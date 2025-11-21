use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::io::AsyncReadExt;
use opentelemetry_sdk::trace::SpanData;
use std::error::Error;
use std::io;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, span, subscriber::Subscriber};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::LocalSet;

use opentelemetry_capnp::{span_export, trace_service, transform::trace::populate_span_minimal};

pub struct CapnpSubscriber {
    tx_export: tokio::sync::mpsc::UnboundedSender<Vec<SpanData>>,
    tx_shutdown: tokio::sync::mpsc::UnboundedSender<ShutDown>,
}

impl CapnpSubscriber {
    pub fn new(endpoint: &SocketAddr) -> Self {
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
            Some(batch) = rx_export.recv() => {
                if let Err(e) = export_batch(&client, batch).await {
                    let _ = writeln!(io::stdout(), "Export failed: {}", e);
                }
            },
            Some(ShutDown) = rx_shutdown.recv() => {
                while let Ok(batch) = rx_export.try_recv() {
                    let _ = export_batch(&client, batch).await;
                }
                break;
            },
            else => { break;}
        }
    }
}

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

    let response = request.send().promise.await?;
    let reply = response.get()?.get_reply()?.get_count();
    writeln!(std::io::stdout(), "{}", reply)?;
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

impl Subscriber for CapnpSubscriber {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(17u64)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {}

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {}

    fn enter(&self, span: &span::Id) {}

    fn exit(&self, span: &span::Id) {}
}

struct SpanReceiver;

/// To demonstrate using Cap'n Proto over the wire we need a receiver that
/// can handle Cap'n Proto client requests. This is a mini-server that does that.
impl SpanReceiver {
    fn new(addr: &SocketAddr) -> std::io::Result<SpanReceiver> {
        // TODO
        // integrate into the OTEL API/SDK. There appears to be no SpanReceiver!
        //
        // TODO
        // this uses the minimal span_export interface; implement the full trace_service interface.
        let addr = addr.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();

            local.block_on(&rt, async {
                let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
                // let client: trace_service::Client = capnp_rpc::new_client(SpanReceiver);
                let client: span_export::Client = capnp_rpc::new_client(SpanReceiver);

                loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    stream.set_nodelay(true).unwrap();

                    spawn_local_rpc_system_to_handle_stream(stream, client.clone()).await;
                }
            })
        });
        Ok(SpanReceiver)
    }
}

// impl trace_service::Server for SpanReceiver {}

/// Give the SpanReceiver the capability of receiving a
/// `send_span_data` call from the client.
///
/// Capabilities of the server are implemented from the
/// perspective of the client calling those capabilities.
impl span_export::Server for SpanReceiver {
    fn send_span_data(
        self: std::rc::Rc<Self>,
        params: span_export::SendSpanDataParams,
        mut results: span_export::SendSpanDataResults,
    ) -> Promise<(), capnp::Error> {
        let request = pry!(params.get());
        let request_data = pry!(request.get_request());
        let spans = pry!(request_data.get_spans());
        pry!(writeln!(
            std::io::stdout(),
            "received {} spans",
            spans.len()
        ));
        for span in spans.iter() {
            pry!(writeln!(std::io::stdout(), "{:#?}", span));
        }
        pry!(writeln!(std::io::stdout(), "finished receiving spans"));

        let mut reply = results.get().init_reply();
        reply.set_count(spans.len() as u16);
        Promise::ok(())
    }
}

async fn spawn_local_rpc_system_to_handle_stream(
    stream: tokio::net::TcpStream,
    client: span_export::Client,
) {
    let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

    let rpc_network = twoparty::VatNetwork::new(
        futures::io::BufReader::new(reader),
        futures::io::BufWriter::new(writer),
        rpc_twoparty_capnp::Side::Server,
        Default::default(),
    );

    let rpc_system = RpcSystem::new(Box::new(rpc_network), Some(client.clone().client));
    tokio::task::spawn_local(rpc_system);
}
