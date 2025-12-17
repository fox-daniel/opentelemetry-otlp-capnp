use criterion::Throughput;
use criterion::{criterion_group, criterion_main};
use criterion::{BatchSize, Criterion};
use opentelemetry_capnp::transform::trace::SpanRequest;
use opentelemetry_otlp_capnp::{SpanExporter, SpanReceiver, WithExportConfig};
use utilities::capnp::FakeCapnp;

const ENDPOINT = "127.0.0.1:4317";

#[derive(Clone)]
struct TestInput {
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

    fn export(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

fn export_spans(c: &mut Criterion) {
    let exporter = SpanExporter::builder().with_capnp().with_endpoint(ENDPOINT).build().expect("build SpanExporter with endpoint: {ENDPOINT}");
    let receiver = SpanReceiver::new(ENDPOINT);
    let req_small = FakeCapnp::trace_service_request_with_spans(1, 1);
    let input = [("small", req_small)];
    let mut group = c.benchmark_group("export spans");
    for size_and_req in input.iter() {
        group.throughput(Throughput::Bytes(size_and_req.1.encoded_len() as u64));
        group.bench_function(format!("export spans {}", size_and_req.0), move |b| {
            b.iter_batched(|| TestInput::new(), |ti| ti.export(), BatchSize::SmallInput)
        });
    }
    group.finish();
}

criterion_group!(benches, export_spans);
criterion_main!(benches);
