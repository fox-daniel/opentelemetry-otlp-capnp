//! # CAPNP - Span Exporter
//!
//! Defines a [SpanExporter] to send trace data via an extended
//! OpenTelemetry Protocol using Cap'n Proto.

use futures::io::AsyncReadExt;
use opentelemetry_capnp::{trace_service, transform::trace::populate_span_minimal};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::trace::SpanData;
use std::fmt::Debug;
use std::io::Write;

// this is a temporary interface to get an example working
use opentelemetry_capnp::span_export;

// use crate::exporter::capnp::trace::CapnpTracesClient;
// use crate::{
//     exporter::capnp::{CapnpExporterBuilder, HasCapnpConfig},
//     CapnpExporterBuilderSet,
// };
use crate::ShutDown;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::LocalSet;

/// Target to which the exporter is going to send spans, defaults to https://localhost:4317/v1/traces.
/// Learn about the relationship between this constant and default/metrics/logs at
/// <https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/protocol/exporter.md#endpoint-urls-for-otlphttp>
pub const OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT: &str = "OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT";
/// Max waiting time for the backend to process each spans batch, defaults to 10s.
pub const OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT: &str = "OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT";
// pub const OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION: &str = "OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION";
// pub const OTEL_EXPORTER_CAPNP_TRACES_HEADERS: &str = "OTEL_EXPORTER_CAPNP_TRACES_HEADERS";

/// CAPNP exporter that sends tracing data
///
/// Forwards SpanData over a tokio channel to the thread dedicated to
/// a Cap'n Proto client for further export.
///
/// The internals do not parallel opentelemetry-otlp using Prost and Tonic.
/// The change is required for Cap'n Proto because the Cap'n Proto SpanExporter
/// is not Send. The Cap'n Proto RPC client used to export SpanData
/// is placed on a dedicated thread and all SpanData is sent to it
/// for export using CapnpForwardingCliet.
#[derive(Debug)]
pub struct SpanExporter {
    tx_export: tokio::sync::mpsc::UnboundedSender<Vec<SpanData>>,
    tx_shutdown: tokio::sync::mpsc::UnboundedSender<ShutDown>,
}

// #[derive(Debug)]
// enum SupportedTransportClient {
//     Capnp(crate::exporter::capnp::trace::CapnpTracesClient),
// }

impl SpanExporter {
    // TODO
    // should boot up in unconnected state and be able to cache span data
    // when are able to connect, then do so and start exporting
    // need to handle disconnect and cache full
    pub fn new(endpoint: &SocketAddr) -> Self {
        let (tx_export, rx_export) = unbounded_channel::<Vec<SpanData>>();
        let (tx_shutdown, rx_shutdown) = unbounded_channel();

        let addr = endpoint
            .to_socket_addrs()
            .unwrap()
            .next()
            .expect("parse to socket address");
        println!("addr: {addr}");

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let local = LocalSet::new();

            local.block_on(&rt, async {
                let stream = tokio::net::TcpStream::connect(&addr)
                    .await
                    .expect("should connect to address");
                println!("stream established for exporter");
                stream.set_nodelay(true).expect("no delay set");

                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

                let rpc_network = Box::new(twoparty::VatNetwork::new(
                    futures::io::BufReader::new(reader),
                    futures::io::BufWriter::new(writer),
                    rpc_twoparty_capnp::Side::Client,
                    Default::default(),
                ));

                println!("rpc network established for exporter");
                let mut rpc_system = RpcSystem::new(rpc_network, None);
                // let client: trace_service::Client =
                //     rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

                let client: span_export::Client =
                    rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
                tokio::task::spawn_local(rpc_system);

                export_loop(client, rx_export, rx_shutdown).await;
            });
        });
        SpanExporter {
            tx_export,
            tx_shutdown,
        }
    }
}

async fn export_loop(
    // client: trace_service::Client,
    client: span_export::Client,
    mut rx_export: UnboundedReceiver<Vec<SpanData>>,
    mut rx_shutdown: UnboundedReceiver<ShutDown>,
) {
    loop {
        tokio::select! {
            Some(batch) = rx_export.recv() => {
                if let Err(e) = export_batch(&client, batch).await {
                    eprintln!("Export failed: {}", e);
                }
            },
            Some(ShutDown) = rx_shutdown.recv() => {
                while let Ok(batch) = rx_export.try_recv() {
                    let _ = export_batch(&client, batch).await;
                }
                break;
            },
            // TODO
            // make this a graceful shutdown
            else => { break;}
        }
    }
}

async fn export_batch(
    // client: &trace_service::Client,
    client: &span_export::Client,
    batch: Vec<SpanData>,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO
    // implement the request
    // TODO
    // switch all println to writeln
    let mut request = client.send_span_data_request();
    {
        let span_data_builder = request.get().init_request();
        let mut spans_builder = span_data_builder.init_spans(batch.len() as u32);
        for (idx, span) in batch.into_iter().enumerate() {
            let span_builder = spans_builder.reborrow().get(idx as u32);
            populate_span_minimal(span_builder, span)?;
        }
    }

    let response = request.send().promise.await?;
    let reply = response.get()?.get_reply()?.get_count();
    writeln!(std::io::stdout(), "{}", reply)?;
    // "this would be a good time to export the batch over the wire");
    Ok(())
}

impl opentelemetry_sdk::trace::SpanExporter for SpanExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        self.tx_export.send(batch).map_err(|e| {
            OTelSdkError::InternalFailure(format!(
                "Failed to send over MPSC to Cap'n Proto Exporter Thread: {e}"
            ))
        })
    }
}
