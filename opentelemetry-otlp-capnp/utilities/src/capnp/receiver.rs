use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::io::AsyncReadExt;
use opentelemetry_capnp::capnp::capnp_rpc::trace_service;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};

/// A No-op Span receiver for Cap'n Proto RPC for benchmarking.
///
/// ```rust
/// // import stuff
/// const TEST_ADDRESS: &str = "127.0.0.1:8080";
///
/// #[tokio::main]
/// pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let _span_receiver = NoOpSpanReceiver::new()
///         .start()
///         .map_err(|e| format!("Failed to start SpanReceiver: {e}"))?;
///
///     tokio::signal::ctrl_c().await?;
///     Ok(())
/// }
/// ```
pub struct NoOpSpanReceiver {
    addr: SocketAddr,
}

impl NoOpSpanReceiver {
    pub fn new(addr: &str) -> Self {
        let addr = addr
            .to_socket_addrs()
            .expect("Valid socket address")
            .next()
            .expect("At least one address");
        Self { addr }
    }

    pub fn start(self) -> std::io::Result<std::thread::JoinHandle<()>> {
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
impl trace_service::Server for NoOpSpanReceiver {
    fn export(
        self: std::rc::Rc<Self>,
        params: trace_service::ExportParams,
        mut results: trace_service::ExportResults,
    ) -> impl futures::Future<Output = Result<(), capnp::Error>> + 'static {
        let response_builder = results.get().init_response();
        let mut partial_success_builder = response_builder.init_partial_success();
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
