//! # CAPNP - Span Exporter
//!
//! Defines a [SpanExporter] to send trace data via an extended
//! OpenTelemetry Protocol using Cap'n Proto.

use futures::io::AsyncReadExt;
use opentelemetry_capnp::{trace_service, transform::trace::populate_span_minimal};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::trace::SpanData;
use std::fmt::Debug;
use std::io;
use std::io::{ErrorKind, Write};
use std::time::Duration;
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
pub const SPAN_EXPORTER_TIMEOUT: u64 = 30_000;
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

async fn connect_with_retry(
    addr: &SocketAddr,
    timeout_ms: u64,
) -> io::Result<tokio::net::TcpStream> {
    let mut delay = Duration::from_millis(1);
    loop {
        tokio::time::sleep(delay).await;

        match tokio::net::TcpStream::connect(&addr).await {
            Ok(stream) => {
                return io::Result::Ok(stream);
            }
            Err(e) => {
                let _ = writeln!(
                    io::stdout(),
                    "Connection attempt failed for SpanExporter: {e}"
                );
                if delay > Duration::from_millis(timeout_ms) {
                    return Err(io::Error::new(
                        ErrorKind::TimedOut,
                        "Connection retry timeout exceeded",
                    ));
                }
                delay *= 2;
            }
        }
    }
}

impl SpanExporter {
    // TODO
    // should boot up in unconnected state and be able to cache span data
    // when are able to connect, then do so and start exporting
    // need to handle disconnect and cache full
    pub fn new(endpoint: &SocketAddr) -> Self {
        // switch to bounded channels; careful to not have channel-loops
        let (tx_export, rx_export) = unbounded_channel::<Vec<SpanData>>();
        let (tx_shutdown, rx_shutdown) = unbounded_channel();

        let addr = endpoint
            .to_socket_addrs()
            .unwrap()
            .next()
            .expect("parse to socket address");

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let local = LocalSet::new();

            local.block_on(&rt, async {
                let Ok(stream) = connect_with_retry(&addr, SPAN_EXPORTER_TIMEOUT).await else {
                    writeln!(io::stdout(), "Could not build Span Exporter").ok();
                    return;
                };
                stream.set_nodelay(true).expect("no delay set");

                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

                let rpc_network = Box::new(twoparty::VatNetwork::new(
                    futures::io::BufReader::new(reader),
                    futures::io::BufWriter::new(writer),
                    rpc_twoparty_capnp::Side::Client,
                    Default::default(),
                ));

                let _ = writeln!(io::stdout(), "rpc network established for exporter");
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
            // does dropping recv cause data loss? cancel safety
            Some(batch) = rx_export.recv() => {
                if let Err(e) = export_batch(&client, batch).await {
                    let _ = writeln!(io::stdout(), "Export failed: {}", e);
                }
            },
            Some(ShutDown) = rx_shutdown.recv() => {
                // close channel so noone can send: close then flush
                while let Ok(batch) = rx_export.try_recv() {
                    let _ = export_batch(&client, batch).await;
                }
                break;
            },
            else => { break;}
        }
    }
}

// TODO
// - add retry with exponential backoff
// - group spans by resource and scope
// - put resource spans as message into a Request that includes metadata, extensions, and the message
// - need to push responses into async tasks and return Success or Error for SpanExporter export
async fn export_batch(
    // client: &trace_service::Client,
    client: &span_export::Client,
    batch: Vec<SpanData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut request = client.send_span_data_request();
    {
        let span_data_builder = request.get().init_request();
        let mut spans_builder = span_data_builder.init_spans(batch.len() as u32);
        for (idx, span) in batch.into_iter().enumerate() {
            let span_builder = spans_builder.reborrow().get(idx as u32);
            populate_span_minimal(span_builder, span)?;
        }
    }
    // consider dropping promise for open loop; don't want buffer in channel to fill up if receiver is slow

    let _response = request.send().promise.await?;
    // let reply = response.get()?.get_reply()?.get_count();
    // writeln!(std::io::stdout(), "{}", reply)?;
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
