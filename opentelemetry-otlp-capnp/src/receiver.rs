use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::io::AsyncReadExt;
use opentelemetry_capnp::capnp::capnp_rpc::trace_service;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};

/// A Span receiver for Cap'n Proto RPC. This is a sketch and needs to be
/// developed.
///
/// ```rust, no_run
/// use opentelemetry_otlp_capnp::SpanReceiver;
/// const TEST_ADDRESS: &str = "127.0.0.1:8080";
///
/// #[tokio::main]
/// pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let span_receiver = SpanReceiver::new(TEST_ADDRESS)
///         .start()
///         .map_err(|e| format!("Failed to start SpanReceiver: {e}"))?;
///     Ok(())
/// }
/// ```
pub struct SpanReceiver {
    addr: SocketAddr,
}

/// To demonstrate using Cap'n Proto over the wire we need a receiver that
/// can handle Cap'n Proto client requests. This is a mini-server that does that.
impl SpanReceiver {
    pub fn new(addr: &str) -> Self {
        let addr = addr
            .to_socket_addrs()
            .expect("Valid socket address")
            .next()
            .expect("At least one address");
        Self { addr }
    }

    pub fn start(self) -> std::io::Result<std::thread::JoinHandle<()>> {
        // TODO
        // integrate into the OTEL API/SDK. There appears to be no SpanReceiver!
        //
        // TODO
        // this uses the minimal span_export interface; implement the full trace_service interface.
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();

            local.block_on(&rt, async {
                let listener = tokio::net::TcpListener::bind(self.addr).await.unwrap();
                // let client: trace_service::Client = capnp_rpc::new_client(SpanReceiver);
                let client: trace_service::Client = capnp_rpc::new_client(self);

                loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    stream.set_nodelay(true).unwrap();

                    spawn_local_rpc_system_to_handle_stream(stream, client.clone()).await;
                }
            })
        });
        Ok(handle)
    }
}

/// Give the SpanReceiver the capability of receiving a
/// `export` call from the client.
///
/// Capabilities of the server are implemented from the
/// perspective of the client calling those capabilities.
impl trace_service::Server for SpanReceiver {
    fn export(
        self: std::rc::Rc<Self>,
        params: trace_service::ExportParams,
        mut results: trace_service::ExportResults,
    ) -> impl futures::Future<Output = Result<(), capnp::Error>> + 'static {
        let request = pry!(params.get());
        let request_data = pry!(request.get_request());
        let resource_spans = pry!(request_data.get_resource_spans());
        let first_resource_span = resource_spans.get(0);
        let scope_spans = first_resource_span.get_scope_spans();
        pry!(writeln!(
            std::io::stdout(),
            "received {} spans on {}",
            scope_spans.iter().count(),
            std::process::id()
        ));
        for span in scope_spans.iter() {
            pry!(writeln!(std::io::stdout(), "{:#?}", span));
        }
        pry!(writeln!(std::io::stdout(), "finished receiving spans"));

        let response_builder = results.get().init_response();
        let mut partial_success_builder = response_builder.init_partial_success();
        // This needs to be calculated instead of set arbitrarily!
        let num_rejected_spans = 0;
        partial_success_builder
            .reborrow()
            .set_rejected_spans(num_rejected_spans);
        Promise::ok(())
    }
}

async fn spawn_local_rpc_system_to_handle_stream(
    stream: tokio::net::TcpStream,
    client: trace_service::Client,
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
