use criterion::{criterion_group, criterion_main};
use criterion::{BatchSize, Criterion};
use opentelemetry_capnp::transform::trace::SpanRequest;
use opentelemetry_otlp_capnp::{SpanExporter, SpanReceiver, WithExportConfig};
use opentelemetry_sdk::trace::SpanExporter as _;
use tokio::runtime::Runtime;
use utilities::capnp::FakeCapnp;

const ENDPOINT: &str = "127.0.0.1:4317";

#[derive(Clone)]
struct TestInput {
    // may want to switch this field to a builder that can be used for each element of the bench groups independently
    span_exporter: SpanExporter,
    request: SpanRequest,
}

impl TestInput {
    fn new(span_exporter: SpanExporter, request: SpanRequest) -> Self {
        TestInput {
            span_exporter,
            request,
        }
    }
}

fn export_spans(c: &mut Criterion) {
    let rt = Runtime::new().expect("able to create new runtime");
    let _span_receiver = SpanReceiver::new(ENDPOINT)
        .start()
        .map_err(|e| format!("Failed to start SpanReceiver: {e}"));
    std::thread::sleep(std::time::Duration::from_millis(100));
    let req_small = FakeCapnp::trace_service_request_with_spans(1);
    let input = [("small", req_small)];
    let mut group = c.benchmark_group("export spans");
    for (name, req) in input.into_iter() {
        group.bench_function(format!("export spans {}", name), |b| {
            let exporter = SpanExporter::builder()
                .with_capnp()
                .with_endpoint(ENDPOINT)
                .build()
                .expect("build SpanExporter with endpoint: {ENDPOINT}");
            b.iter_batched(
                || TestInput::new(exporter.clone(), req.clone()),
                |ti| rt.block_on(async { ti.span_exporter.export(ti.request.batch).await }),
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, export_spans);
criterion_main!(benches);
