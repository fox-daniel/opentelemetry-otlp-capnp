use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::KeyValue;
use opentelemetry::{global, InstrumentationScope};
use opentelemetry_otlp_capnp::SpanReceiver;
use opentelemetry_otlp_capnp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::error::Error;
use std::io;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

const TEST_ADDRESS: &str = "127.0.0.1:8080";

fn get_resource() -> Resource {
    static RESOURCE: OnceLock<Resource> = OnceLock::new();
    RESOURCE
        .get_or_init(|| {
            Resource::builder()
                .with_service_name("basic-otlp-capnp-example")
                .build()
        })
        .clone()
}

fn init_traces() -> io::Result<SdkTracerProvider> {
    let exporter = SpanExporter::builder()
        .with_capnp()
        .with_endpoint(TEST_ADDRESS)
        .build()
        .unwrap();
    Ok(SdkTracerProvider::builder()
        .with_resource(get_resource())
        .with_batch_exporter(exporter)
        .build())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    // Normally the SpanReceiver would run in a different process, often
    // in a different VM or machine.
    let _span_receiver = SpanReceiver::new(TEST_ADDRESS)
        .start()
        .map_err(|e| format!("Failed to start SpanReceiver: {e}"))?;

    let tracer_provider = init_traces()?;
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
