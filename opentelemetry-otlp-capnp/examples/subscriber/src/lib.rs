use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::io::AsyncReadExt;
use std::error::Error;
use std::io;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, span, subscriber::Subscriber};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

pub struct CapnpSubscriber;

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

// struct SpanReceiver;

// /// To demonstrate using Cap'n Proto over the wire we need a receiver that
// /// can handle Cap'n Proto client requests. This is a mini-server that does that.
// impl SpanReceiver {
//     fn new() -> std::io::Result<SpanReceiver> {
//         let addr = TEST_ADDRESS.to_socket_addrs().unwrap().next().unwrap();
//         // TODO
//         // integrate into the OTEL API/SDK. There appears to be no SpanReceiver!
//         //
//         // TODO
//         // this uses the minimal span_export interface; implement the full trace_service interface.
//         std::thread::spawn(move || {
//             let rt = tokio::runtime::Builder::new_current_thread()
//                 .enable_all()
//                 .build()
//                 .unwrap();
//             let local = tokio::task::LocalSet::new();

//             local.block_on(&rt, async {
//                 let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
//                 // let client: trace_service::Client = capnp_rpc::new_client(SpanReceiver);
//                 let client: span_export::Client = capnp_rpc::new_client(SpanReceiver);

//                 loop {
//                     let (stream, _) = listener.accept().await.unwrap();
//                     stream.set_nodelay(true).unwrap();

//                     spawn_local_rpc_system_to_handle_stream(stream, client.clone()).await;
//                 }
//             })
//         });
//         Ok(SpanReceiver)
//     }
// }

// // impl trace_service::Server for SpanReceiver {}

// /// Give the SpanReceiver the capability of receiving a
// /// `send_span_data` call from the client.
// ///
// /// Capabilities of the server are implemented from the
// /// perspective of the client calling those capabilities.
// impl span_export::Server for SpanReceiver {
//     fn send_span_data(
//         self: std::rc::Rc<Self>,
//         params: span_export::SendSpanDataParams,
//         mut results: span_export::SendSpanDataResults,
//     ) -> Promise<(), capnp::Error> {
//         let request = pry!(params.get());
//         let request_data = pry!(request.get_request());
//         let spans = pry!(request_data.get_spans());
//         pry!(writeln!(
//             std::io::stdout(),
//             "received {} spans",
//             spans.len()
//         ));
//         for span in spans.iter() {
//             pry!(writeln!(std::io::stdout(), "{:#?}", span));
//         }
//         pry!(writeln!(std::io::stdout(), "finished receiving spans"));

//         let mut reply = results.get().init_reply();
//         reply.set_count(spans.len() as u16);
//         Promise::ok(())
//     }
// }

// async fn spawn_local_rpc_system_to_handle_stream(
//     stream: tokio::net::TcpStream,
//     client: span_export::Client,
// ) {
//     let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

//     let rpc_network = twoparty::VatNetwork::new(
//         futures::io::BufReader::new(reader),
//         futures::io::BufWriter::new(writer),
//         rpc_twoparty_capnp::Side::Server,
//         Default::default(),
//     );

//     let rpc_system = RpcSystem::new(Box::new(rpc_network), Some(client.clone().client));
//     tokio::task::spawn_local(rpc_system);
// }
