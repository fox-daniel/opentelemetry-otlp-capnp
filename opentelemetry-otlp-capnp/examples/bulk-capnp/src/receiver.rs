use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::io::AsyncReadExt;
use opentelemetry_capnp::span_export;
use std::io::Write;
use std::net::ToSocketAddrs;

const TEST_ADDRESS: &str = "127.0.0.1:8080";

struct SpanReceiver;

/// To demonstrate using Cap'n Proto over the wire we need a receiver that
/// can handle Cap'n Proto client requests. This is a mini-server that does that.
impl SpanReceiver {
    fn new() -> Self {
        Self
    }

    fn start(&self) -> std::io::Result<std::thread::JoinHandle<()>> {
        let addr = TEST_ADDRESS.to_socket_addrs().unwrap().next().unwrap();
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
        Ok(handle)
    }
}

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
            "received {} spans on {}",
            spans.len(),
            std::process::id()
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

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _span_receiver = SpanReceiver::new()
        .start()
        .map_err(|e| format!("Failed to start SpanReceiver: {e}"))?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}
