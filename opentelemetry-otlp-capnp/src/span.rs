//! # CAPNP - Span Exporter
//!
//! Defines a [SpanExporter] to send trace data via an extended
//! OpenTelemetry Protocol using Cap'n Proto.

use std::fmt::Debug;
use futures::io::AsyncReadExt;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::SpanData;

use crate::exporter::capnp::trace::CapnpTracesClient;
use crate::{ExporterBuildError, ShutDown};
use crate::{
    exporter::capnp::{CapnpExporterBuilder, HasCapnpConfig},
    CapnpExporterBuilderSet,
};

use crate::{exporter::HasExportConfig, NoExporterBuilderSet};

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::LocalSet;

/// Target to which the exporter is going to send spans, defaults to https://localhost:4317/v1/traces.
/// Learn about the relationship between this constant and default/metrics/logs at
/// <https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/protocol/exporter.md#endpoint-urls-for-otlphttp>
pub const OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT: &str = "OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT";
/// Max waiting time for the backend to process each spans batch, defaults to 10s.
pub const OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT: &str = "OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT";
pub const OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION: &str = "OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION";
pub const OTEL_EXPORTER_CAPNP_TRACES_HEADERS: &str = "OTEL_EXPORTER_CAPNP_TRACES_HEADERS";


/// Forwards SpanData over a tokio channel to the thread dedicated to
/// a Cap'n Proto client for further export.
///
/// This has no parallel in opentelemetry-otlp using Prost and Tonic.
/// It is required for Cap'n Proto becuase the Cap'n Proto SpanExporter
/// is not Send. The Cap'n Proto RPC client used to export SpanData
/// is placed on a dedicated thread and all SpanData is sent to it
/// for export using CapnpForwardingCliet.
struct CapnpForwardingClient {
    tx_export: tokio::sync::mpsc::UnboundedSender<Vec<SpanData>>,
    tx_shutdown: tokio::sync::mpsc::UnboundedSender<ShutDown>,
}

/// CAPNP exporter that sends tracing data
#[derive(Debug)]
pub struct SpanExporter {
    client: SupportedTransportClient,
    // tx_export: tokio::sync::mpsc::UnboundedSender<Vec<SpanData>>,
    // tx_shutdown: tokio::sync::mpsc::UnboundedSender<ShutDown>,
}

#[derive(Debug)]
enum SupportedTransportClient {
    Capnp(crate::exporter::capnp::trace::CapnpTracesClient),
}

impl SpanExporter {
    pub fn new(endpoint: String) -> Self {
        let (tx_export, rx_export) = unbounded_channel::<Vec<SpanData>
    >();
        let (tx_shutdown, rx_shutdown) = unbounded_channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let local = LocalSet::new();

            local.block_on(&rt, async {
                let addr = endpoint.parse().expect("valide socket address");
                let stream = tokio::net::TcpStream::connect(&addr)
                    .await
                    .expect("connected to address");
                stream.set_nodelay(true).expect("no delay set");

                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

                let rpc_network = Box::new(twoparty::VatNetwork::new(
                    futures::io::BufReader::new(reader),
                    futures::io::BufWriter::new(writer),
                    rpc_twoparty_capnp::Side::Client,
                    Default::default(),
                ));

                let mut rpc_system = RpcSystem::new(rpc_network, None);
                let client: trace_service_capnp::trace_service::Client =
                    rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

                tokio::task::spawn_local(rpc_system);

                export_loop(client, rx_export, rx_shutdown).await;
            });
        });
        // construct supported_transport_client inner client from tx_export and tx_shutdown
        let supported_transport_client= SupportedTransportClient::Capnp(CapnpTracesClient {
            inner: ,
            retry_policy: ,
        })
        SpanExporter { client: supported_transport_client   }
    }
}

async fn export_loop(
    client: trace_service_capnp::trace_service::Client,
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
            }
        }
    }
}

async fn export_batch(
    client: &trace_service_capnp::trace_service::Client,
    batch: Vec<SpanData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut request = client.export_request();
    // implement the request
    let response = request.send().promise.await?;
    Ok(())
}

impl opentelemetry_sdk::trace::SpanExporter for SpanExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        match &self.client {
            SupportedTransportClient::Capnp(client) => client.export(batch).await,
        }
    }
}
