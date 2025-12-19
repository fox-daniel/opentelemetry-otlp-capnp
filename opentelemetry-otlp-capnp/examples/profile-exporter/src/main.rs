use opentelemetry_otlp_capnp::{SpanExporter, SpanReceiver, WithExportConfig as _};
use opentelemetry_sdk::trace::SpanExporter as _;
use utilities::capnp::{receiver::NoOpSpanReceiver, span::FakeCapnp};

const CAPNP_ENDPOINT: &str = "127.0.0.1:4318";
const NUM_ITERS: usize = 100;

async fn span_export() -> Result<(), Box<dyn std::error::Error>> {
    let _capnp_span_receiver = SpanReceiver::new(CAPNP_ENDPOINT)
        .start()
        .map_err(|e| format!("Failed to start SpanReceiver: {e}"));
    std::thread::sleep(std::time::Duration::from_millis(100));
    let req = FakeCapnp::trace_service_request_with_spans(10);
    let capnp_exporter = SpanExporter::builder()
        .with_capnp()
        .with_endpoint(CAPNP_ENDPOINT)
        .build()
        .expect("build Capnp SpanExporter with endpoint: {ENDPOINT}");
    for _ in 0..NUM_ITERS {
        capnp_exporter.export(req.batch.clone()).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let _ = span_export().await;
}
