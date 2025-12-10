use crate::retry::RetryPolicy;
use core::fmt;
use opentelemetry_capnp::collector::trace::v1::{
    trace_service_client::TraceServiceClient, ExportTraceServiceRequest,
};
use opentelemetry_sdk::{
    error::{OTelSdkError, OTelSdkResult},
    trace::{SpanData, SpanExporter},
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::connect_with_retry;
use crate::ShutDown;
use futures::io::AsyncReadExt;
use opentelemetry_capnp::{trace_service, transform::trace::populate_span};
use std::io;
use std::io::Write;
use std::time::Duration;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio::task::LocalSet;

pub const OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT: &str = "OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT";
/// Max waiting time for the backend to process each spans batch, defaults to 10s.
pub const OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT: &str = "OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT";
// pub const OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION: &str = "OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION";
// pub const OTEL_EXPORTER_CAPNP_TRACES_HEADERS: &str = "OTEL_EXPORTER_CAPNP_TRACES_HEADERS";
pub const SPAN_EXPORTER_TIMEOUT: u64 = 30_000;
/// Buffer size is the count of Vec<SpanData>:
/// Batch size = 512
/// Span size = 2KB
/// Max memory footprint for buffer: SpanSize x BatchSize x BufferSize = 2KB x 512 x 32 ~ 32MB
pub const SPAN_EXPORTER_MPSC_CHANNEL_BUFFER_SIZE: usize = 32;
pub const SPAN_EXPORTER_SHUTDOWN_CHANNEL_BUFFER_SIZE: usize = 256;
pub const CAPNP_EXPORTER_RPC_TRACES_TIMEOUT: u64 = 10;

pub(crate) struct CapnpTracesClient {
    inner: Option<ClientInner>,
    retry_policy: RetryPolicy,
    // #[allow(dead_code)]
    // <allow dead> would be removed once we support set_resource for metrics.
    resource: opentelemetry_capnp::transform::common::capnp::ResourceAttributesWithSchema,
}

// TODO
// create one more layer of indirection with the message passing:
// tx_export: tokio::sync::mpsc::Sender<Vec<SpanData>>,
// tx_shutdown: tokio::sync::mpsc::Sender<ShutDown>,

#[derive(Debug)]
struct ClientInner {
    // this may need to be generic over a channel
    client: TraceServiceClient,
    // include an interceptor?
}

impl fmt::Debug for CapnpTracesClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CapnpTracesClient")
    }
}

// this is not right, need to use the client and inner client correctly with the message passing layer
impl opentelemetry_sdk::trace::SpanExporter for CapnpTracesClient {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        self.tx_export.send(batch).await.map_err(|e| {
            OTelSdkError::InternalFailure(format!(
                "Failed to send span batch over MPSC to Cap'n Proto Exporter Thread: {e}"
            ))
        })
    }
}
// this will become an impl on the message passing client
impl SpanExporter {
    // TODO
    // should boot up in unconnected state and be able to cache span data
    // when are able to connect, then do so and start exporting
    // need to handle disconnect and cache full
    pub fn new(endpoint: &SocketAddr) -> Self {
        // switch to bounded channels; careful to not have channel-loops
        let (tx_export, rx_export) =
            mpsc::channel::<Vec<SpanData>>(SPAN_EXPORTER_MPSC_CHANNEL_BUFFER_SIZE);
        let (tx_shutdown, rx_shutdown) = mpsc::channel(SPAN_EXPORTER_SHUTDOWN_CHANNEL_BUFFER_SIZE);

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

                let client: trace_service::Client =
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
    client: trace_service::Client,
    mut rx_export: mpsc::Receiver<Vec<SpanData>>,
    mut rx_shutdown: mpsc::Receiver<ShutDown>,
) {
    loop {
        tokio::select! {
            // The recv method is cancel safe: if the other branch completes first,
            // then no messages will have been received.
            Some(batch) = rx_export.recv() => {
                if let Err(e) = export_batch(&client, batch).await {
                    let _ = writeln!(io::stdout(), "Export failed: {}", e);
                }
            },
            Some(ShutDown) = rx_shutdown.recv() => {
                rx_export.close();
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
// - add retry with exponential backoff; use Arc::new(batch) and clone it for retries
// - group spans by resource and scope
//   - how to do this without unnecessary copies?
// - add partial success handling
// - allow some kind of interceptor so users can inject metadata and context
// - put resource spans as message into a Request that includes metadata, extensions, and the message
// - need to return Success or Error for SpanExporter export without blocking or causing resource bloat
async fn export_batch(
    client: &trace_service::Client,
    batch: Vec<SpanData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut request = client.export_request();
    {
        let span_data_builder = request.get().init_request();
        let mut spans_builder = span_data_builder.init_resource_spans(batch.len() as u32);
        // TODO: modify below: need build resource spans with scope
        for (idx, span) in batch.into_iter().enumerate() {
            let span_builder = spans_builder.reborrow().get(idx as u32);
            populate_span(span_builder, span)?;
        }
    }
    // need to make OTEL complient by returning SUCCESS or FAILURE to SpanExporter.export()
    tokio::time::timeout(
        Duration::from_secs(CAPNP_EXPORTER_RPC_TRACES_TIMEOUT),
        request.send().promise,
    )
    .await??;
    // let reply = response.get()?.get_reply()?.get_count();
    // writeln!(std::io::stdout(), "{}", reply)?;
    Ok(())
}

// impl SpanExporter for CapnpTracesClient {
//     async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
//         // let batch = Arc::new(batch);
//         Err(OTelSdkError::InternalFailure(String::from(
//             "need to implement export for CapnpTracesClient",
//         )))
//     }

//     fn shutdown(&mut self) -> OTelSdkResult {
//         match self.inner.take() {
//             Some(_) => Ok(()), // Successfully took `inner`, indicating a successful shutdown.
//             None => Err(OTelSdkError::AlreadyShutdown), // `inner` was already `None`, meaning it's already shut down.
//         }
//     }

//     // fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
//     //     self.resource = resource.into();
//     // }
// }
