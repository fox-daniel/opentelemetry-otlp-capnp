use criterion::Throughput;
use criterion::{criterion_group, criterion_main};
use criterion::{BatchSize, Criterion};
use utilities::capnp::FakeCapnp;

#[derive(Clone)]
struct TestInput {
    rb: u32,
}

impl TestInput {
    fn new() -> Self {
        TestInput { rb: 0 }
    }

    fn export(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

fn export_spans(c: &mut Criterion) {
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
