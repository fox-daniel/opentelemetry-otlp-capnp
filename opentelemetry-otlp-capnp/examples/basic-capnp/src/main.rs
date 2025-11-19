use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::io::AsyncReadExt;
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::KeyValue;
use opentelemetry::{global, InstrumentationScope};
use opentelemetry_capnp::{span_export, trace_service};
use opentelemetry_otlp_capnp::SpanExporter;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::error::Error;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

const TEST_ADDRESS: &str = "127.0.0.1:8080";

struct SpanReceiver;

// impl trace_service::Server for SpanReceiver {}

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

fn get_resource() -> Resource {
    static RESOURCE: OnceLock<Resource> = OnceLock::new();
    RESOURCE
        .get_or_init(|| {
            Resource::builder()
                .with_service_name("basic-otlp-example-grpc")
                .build()
        })
        .clone()
}

fn init_traces() -> SdkTracerProvider {
    let addr = TEST_ADDRESS.to_socket_addrs().unwrap().next().unwrap();
    // first build a little server to receive exported span data
    // this will eventually be put into a SpanReceiver or similar
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

                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

                let rpc_network = twoparty::VatNetwork::new(
                    futures::io::BufReader::new(reader),
                    futures::io::BufWriter::new(writer),
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );

                let rpc_system = RpcSystem::new(Box::new(rpc_network), Some(client.clone().client));
                tokio::task::spawn_local(rpc_system);
            }
        })
    });

    let exporter = SpanExporter::new(&addr);
    SdkTracerProvider::builder()
        .with_resource(get_resource())
        .with_batch_exporter(exporter)
        .build()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let tracer_provider = init_traces();
    global::set_tracer_provider(tracer_provider.clone());

    let filter_fmt = EnvFilter::new("info").add_directive("opentelemetry=debug".parse().unwrap());
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_filter(filter_fmt);

    tracing_subscriber::registry().with(fmt_layer).init();

    let common_scope_attributes = vec![KeyValue::new("scope-key", "scope-value")];
    let scope = InstrumentationScope::builder("basic")
        .with_version("1.0")
        .with_attributes(common_scope_attributes)
        .build();

    let tracer = global::tracer_with_scope(scope.clone());

    tracer.in_span("Main operation", |cx| {
        let span = cx.span();
        span.add_event(
            "Nice operation!".to_string(),
            vec![KeyValue::new("bogons", 100)],
        );
        span.set_attribute(KeyValue::new("another.key", "yes"));

        info!(name: "my-event-inside-span", target: "my-target", "hello from {}. My price is {}. I am also inside a Span!", "banana", 2.99);

        tracer.in_span("Sub operation...", |cx| {
            let span = cx.span();
            span.set_attribute(KeyValue::new("another.key", "yes"));
            span.add_event("Sub span event", vec![]);
        });
    });

    info!(name: "my-event", target: "my-target", "hello from {}. My price is {}", "apple", 1.99);

    // Collect all shutdown errors
    let mut shutdown_errors = Vec::new();
    if let Err(e) = tracer_provider.shutdown() {
        shutdown_errors.push(format!("tracer provider: {e}"));
    }

    // Return an error if any shutdown failed
    if !shutdown_errors.is_empty() {
        return Err(format!(
            "Failed to shutdown providers:{}",
            shutdown_errors.join("\n")
        )
        .into());
    }
    // this keeps the process alive long enough for all telemetry to be exported;
    // need to make this unnecessary
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}
