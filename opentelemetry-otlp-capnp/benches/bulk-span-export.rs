use criterion::{criterion_group, criterion_main, BenchmarkId};
use criterion::{BatchSize, Criterion};
use opentelemetry_capnp::transform::trace::SpanRequest;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp_capnp::{SpanExporter, SpanReceiver, WithExportConfig as _};
use opentelemetry_sdk::trace::SpanExporter as _;
use tokio::runtime::Runtime;
use utilities::capnp::FakeCapnp;
use utilities::otlp;

const CAPNP_ENDPOINT: &str = "127.0.0.1:4318";
const OTLP_ENDPOINT: &str = "http://127.0.0.1:4317";
const OTLP_RECEIVER_ADDR: &str = "127.0.0.1:4317";

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
    let _span_receiver = SpanReceiver::new(CAPNP_ENDPOINT)
        .start()
        .map_err(|e| format!("Failed to start SpanReceiver: {e}"));
    std::thread::sleep(std::time::Duration::from_millis(100));
    let req_small = FakeCapnp::trace_service_request_with_spans(1);
    let input = [("small", req_small)];
    let mut group = c.benchmark_group("export spans");
    for (name, req) in input.into_iter() {
        group.bench_function(BenchmarkId::new("export_spans", name), |b| {
            let exporter = SpanExporter::builder()
                .with_capnp()
                .with_endpoint(CAPNP_ENDPOINT)
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

fn span_export_comparison(c: &mut Criterion) {
    let rt = Runtime::new().expect("able to create new runtime");
    // TODO
    // check that receivers are doing a similar amount of work
    let _capnp_span_receiver = SpanReceiver::new(CAPNP_ENDPOINT)
        .start()
        .map_err(|e| format!("Failed to start SpanReceiver: {e}"));
    let _otlp_receiver = otlp::MinimalOtlpReceiver::new(OTLP_RECEIVER_ADDR)
        .start()
        .expect("Failed to start OTLP receiver");
    std::thread::sleep(std::time::Duration::from_millis(100));
    let req_small = FakeCapnp::trace_service_request_with_spans(1);
    let req_medium = FakeCapnp::trace_service_request_with_spans(10);
    let req_large = FakeCapnp::trace_service_request_with_spans(100);
    let req_huge = FakeCapnp::trace_service_request_with_spans(1000);
    let input: [(&str, SpanRequest); 4] = [
        ("small", req_small),
        ("medium", req_medium),
        ("large", req_large),
        ("huge", req_huge),
    ];
    let mut group = c.benchmark_group("span export comparison");
    let capnp_exporter = SpanExporter::builder()
        .with_capnp()
        .with_endpoint(CAPNP_ENDPOINT)
        .build()
        .expect("build Capnp SpanExporter with endpoint: {ENDPOINT}");
    for (name, req) in input.iter() {
        group.bench_with_input(
            BenchmarkId::new("Cap'n Proto span export", name),
            req,
            |b, req| {
                b.iter(|| rt.block_on(async { capnp_exporter.export(req.batch.clone()).await }))
            },
        );
    }
    let otlp_exporter = rt.block_on(async {
        opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(OTLP_ENDPOINT)
            .build()
            .expect("build OTLP SpanExporter with endpoint: {ENDPOINT}")
    });
    for (name, req) in input.iter() {
        group.bench_with_input(BenchmarkId::new("OTLP span export", name), req, |b, req| {
            b.iter(|| rt.block_on(async { otlp_exporter.export(req.batch.clone()).await }))
        });
    }
    group.finish();
}

criterion_group!(benches, span_export_comparison);
criterion_main!(benches);
