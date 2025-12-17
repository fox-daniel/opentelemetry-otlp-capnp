use opentelemetry::KeyValue;
use opentelemetry::{global, InstrumentationScope};
use opentelemetry_otlp_capnp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::io;
use std::io::Write;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::{event, field, info, info_span, instrument, Instrument, Level, Span};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

const ADDRESS: &str = "127.0.0.1:4317";

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    writeln!(
        io::stdout(),
        "app running on process {}",
        std::process::id()
    )
    .ok();

    // Set up telemetry
    let tracer_provider = init_traces()?;
    global::set_tracer_provider(tracer_provider.clone());

    let tracer = global::tracer("my-app");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let filter_fmt = EnvFilter::new("info").add_directive("opentelemetry=debug".parse().unwrap());
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_filter(filter_fmt);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    let common_scope_attributes = vec![KeyValue::new("scope-key", "scope-value")];
    let scope = InstrumentationScope::builder("basic")
        .with_version("1.0")
        .with_attributes(common_scope_attributes)
        .build();

    let _tracer = global::tracer_with_scope(scope.clone());
    // begin app logic
    top_level_function().await;

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

#[instrument(name = "app top level function", fields(nested_attr=tracing::field::Empty))]
async fn top_level_function() {
    let mut set = JoinSet::new();
    let nested = vec![vec![0, 1], vec![2, 3]];
    Span::current().record("nested_attr", field::debug(&nested));
    for i in 0..2 {
        set.spawn(
            async move {
                writeln!(io::stdout(), "hi from task {i}").ok();
                inner_function(i as u64).await;
            }
            .instrument(info_span!("spawned task", task_id = i)),
        );
    }

    while let Some(result) = set.join_next().await {
        if let Err(e) = result {
            writeln!(io::stdout(), "Task Error: {e}").ok();
        }
    }
}

#[instrument]
async fn inner_function(i: u64) {
    let dur = (i + 1) * 10;
    tokio::time::sleep(Duration::from_millis(dur)).await;
    info!("i slept for {dur}ms");
    let data = vec![i; 2];
    event!(Level::INFO, name = "event", ?data);
}

fn init_traces() -> io::Result<SdkTracerProvider> {
    let exporter = SpanExporter::builder()
        .with_capnp()
        .with_endpoint(ADDRESS)
        .build()
        .unwrap();
    Ok(SdkTracerProvider::builder()
        .with_resource(get_resource())
        .with_batch_exporter(exporter)
        .build())
}

fn get_resource() -> Resource {
    static RESOURCE: OnceLock<Resource> = OnceLock::new();
    RESOURCE
        .get_or_init(|| {
            Resource::builder()
                .with_service_name("bulk-capnp-example")
                .build()
        })
        .clone()
}
