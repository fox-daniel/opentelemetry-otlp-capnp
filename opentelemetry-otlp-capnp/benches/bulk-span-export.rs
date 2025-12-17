use criterion::Throughput;
use criterion::{criterion_group, criterion_main};
use criterion::{BatchSize, Criterion};

#[derive(Clone)]
struct TestInput {}

fn export_spans(c: &mut Criterion) {
    let req_small = FakeCapnp::trace_service_request();
    let input = [("small", req_small)];
    let mut group = c.benchmark_group("export spans");
    for size_and_req in input.iter() {
        group.throughput(Throughput::Bytes(size_and_req.1.encoded_len() as u64));
        group.bench_function();
    }
    group.finish();
}

criterion_group!(benches, export_spans);
criterion_main!(benches);
