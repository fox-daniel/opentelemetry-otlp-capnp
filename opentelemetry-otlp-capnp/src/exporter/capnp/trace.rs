// TODO:
// remove the clones for better performance
use crate::retry::RetryPolicy;
use core::fmt;
// the following path is different than the OTLP because this crate doesn't use an extra module
// indrection for the rpc layer since it is all capnp
use opentelemetry_sdk::{
    error::{OTelSdkError, OTelSdkResult},
    trace::SpanData,
    Resource,
};

use crate::connect_with_retry;
use crate::ShutDown;
use futures::io::AsyncReadExt;
use opentelemetry_capnp::{
    capnp::capnp_rpc::trace_service,
    transform::trace::{
        populate_resource, populate_scope_spans, ResourceSpans, ScopeSpans, SpanRequest,
    },
};
use std::io;
use std::io::Write;
use std::time::Duration;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::LocalSet;

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

#[derive(Clone)]
pub(crate) struct CapnpTracesClient {
    inner: Option<ClientInner>,
    retry_policy: RetryPolicy,
    resource: Resource,
}

impl CapnpTracesClient {
    pub(super) fn new(endpoint: SocketAddr, retry_policy: Option<RetryPolicy>) -> Self {
        let client = CapnpMessageClient::new(&endpoint);
        let resource = Resource::builder().build();
        Self {
            inner: Some(ClientInner { client }),
            retry_policy: retry_policy.unwrap_or(RetryPolicy {
                max_retries: 3,
                initial_delay_ms: 100,
                max_delay_ms: 1600,
                jitter_ms: 100,
            }),
            resource,
        }
    }
}

#[derive(Clone)]
struct ClientInner {
    client: CapnpMessageClient,
    // include an interceptor
}

#[derive(Clone)]
struct CapnpMessageClient {
    // TODO
    // make this generic over the channel so that flume can also be used
    tx_export: tokio::sync::mpsc::Sender<SpanRequest>,
    tx_shutdown: tokio::sync::mpsc::Sender<ShutDown>,
}

impl fmt::Debug for CapnpTracesClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CapnpTracesClient")
    }
}

impl opentelemetry_sdk::trace::SpanExporter for CapnpTracesClient {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        // TODO
        // use retry policy for trying to export
        match &self.inner {
            Some(inner_client) => {
                inner_client
                    .clone()
                    .client
                    .tx_export
                    .send(SpanRequest {
                        batch,
                        resource: self.resource.clone(),
                    })
                    .await
                    .map_err(|e| {
                        OTelSdkError::InternalFailure(format!(
                        "Failed to send span batch over MPSC to Cap'n Proto Exporter Thread: {e}"
                    ))
                    })?;
                // TODO
                // Need to surface errors returned from tx_export.send()
                Ok(())
            }
            None => return OTelSdkResult::Err(OTelSdkError::AlreadyShutdown),
        }
    }
    fn shutdown(&mut self) -> OTelSdkResult {
        // TODO
        // check that this is correct
        match self.inner.take() {
            Some(_) => Ok(()), // Successfully took `inner`, indicating a successful shutdown.
            None => Err(OTelSdkError::AlreadyShutdown), // `inner` was already `None`, meaning it's already shut down.
        }
    }

    fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
        self.resource = resource.clone();
    }
}

impl CapnpMessageClient {
    // TODO
    // should boot up in unconnected state and be able to cache span data
    // when are able to connect, then do so and start exporting
    // need to handle disconnect and cache full
    pub fn new(endpoint: &SocketAddr) -> Self {
        // switch to bounded channels; careful to not have channel-loops
        let (tx_export, rx_export) =
            mpsc::channel::<SpanRequest>(SPAN_EXPORTER_MPSC_CHANNEL_BUFFER_SIZE);
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

                let mut rpc_system = build_capnp_rpc_system(stream);

                let client: trace_service::Client =
                    rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
                tokio::task::spawn_local(rpc_system);

                export_loop(client, rx_export, rx_shutdown).await;
            });
        });
        Self {
            tx_export,
            tx_shutdown,
        }
    }
}

fn build_capnp_rpc_system(stream: TcpStream) -> RpcSystem<twoparty::VatId> {
    let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

    let rpc_network = Box::new(twoparty::VatNetwork::new(
        futures::io::BufReader::new(reader),
        futures::io::BufWriter::new(writer),
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));

    let _ = writeln!(io::stdout(), "rpc network established for exporter");
    RpcSystem::new(rpc_network, None)
}

async fn export_loop(
    client: trace_service::Client,
    mut rx_export: mpsc::Receiver<SpanRequest>,
    mut rx_shutdown: mpsc::Receiver<ShutDown>,
) {
    loop {
        tokio::select! {
            // The recv method is cancel safe: if the other branch completes first,
            // then no messages will have been received.
            Some(span_request) = rx_export.recv() => {
                if let Err(e) = export_batch(&client, span_request).await {
                    let _ = writeln!(io::stdout(), "Export failed: {}", e);
                }
            },
            Some(ShutDown) = rx_shutdown.recv() => {
                rx_export.close();
                while let Ok(span_request) = rx_export.try_recv() {
                    let _ = export_batch(&client, span_request).await;
                }
                break;
            },
            else => { break;}
        }
    }
}

// TODO
// - add retry with exponential backoff; use Arc::new(batch) and clone it for retries
// - add partial success handling
// - allow some kind of interceptor so users can inject metadata and context
// - put resource spans as message into a Request that includes metadata, extensions, and the message
// - need to return Success or Error for SpanExporter export without blocking or causing resource bloat
// - switch types to be impl traits? impl Iter<SpanData> etc
async fn export_batch(
    client: &trace_service::Client,
    span_request: SpanRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    let resource_spans = group_spans_by_resource_and_scope(span_request);
    // currently assuming that group_spans_by_resource_and_scope returns a vec of length 1
    // with a single resource
    let resource_spans = resource_spans[0].clone();
    let resource: Arc<Resource> = resource_spans.resource.clone();
    let mut request = client.export_request();
    {
        let export_trace_service_request_builder = request.get().init_request();
        // we have a single resource, so our resource spans is a vec of length one
        let mut resource_spans_builder =
            export_trace_service_request_builder.init_resource_spans(1u32);
        let mut builder_for_resource_spans = resource_spans_builder.reborrow().get(0);
        {
            let resource_builder = builder_for_resource_spans.reborrow().init_resource();
            populate_resource(resource_builder, resource)?;
        }
        let scope_spans_collection: Vec<ScopeSpans> = resource_spans.scope_spans;
        let mut scope_spans_builder = builder_for_resource_spans
            .reborrow()
            .init_scope_spans(scope_spans_collection.len() as u32);
        {
            for (idx, scope_spans) in scope_spans_collection.into_iter().enumerate() {
                let builder_for_scope_spans = scope_spans_builder.reborrow().get(idx as u32);
                populate_scope_spans(builder_for_scope_spans, scope_spans)?;
            }
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

pub fn group_spans_by_resource_and_scope(span_request: SpanRequest) -> Vec<ResourceSpans> {
    let resource = span_request.resource;
    let scope_map = span_request.batch.iter().fold(
        HashMap::new(),
        |mut scope_map: HashMap<&opentelemetry::InstrumentationScope, Vec<&SpanData>>, span| {
            let instrumentation = &span.instrumentation_scope;
            scope_map.entry(instrumentation).or_default().push(span);
            scope_map
        },
    );

    // Convert the grouped spans into ScopeSpans
    let scope_spans = scope_map
        .into_iter()
        .map(|(instrumentation, span_records)| ScopeSpans {
            scope: Some(instrumentation.clone()),
            schema_url: instrumentation
                .schema_url()
                .map(ToOwned::to_owned)
                .unwrap_or_default(),
            spans: span_records.into_iter().cloned().collect(),
        })
        .collect();

    // Wrap ScopeSpans into a single ResourceSpans
    let schema_url = resource.schema_url().unwrap_or_default().to_string();
    vec![ResourceSpans {
        resource: Arc::new(resource),
        scope_spans,
        schema_url,
    }]
}
