use opentelemetry::trace::{SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId};
use opentelemetry::InstrumentationScope;
use opentelemetry_capnp::transform::trace::SpanRequest;
use opentelemetry_sdk::trace::{SpanData, SpanEvents, SpanLinks};
use opentelemetry_sdk::Resource;
use std::borrow::Cow;
use std::time::SystemTime;

fn create_test_span_data() -> SpanData {
    let trace_id = TraceId::from(0x0123456789abcdef0123456789abcdef);
    let span_id = SpanId::from(0x0123456789abcdef);
    let instrumentation_scope = InstrumentationScope::builder("my app")
        .with_version("1.0")
        .with_schema_url("www.myapp")
        .build();
    SpanData {
        span_context: SpanContext::new(
            trace_id,
            span_id,
            TraceFlags::SAMPLED,
            false,              // is_remote
            Default::default(), // trace_state
        ),
        parent_span_id: SpanId::INVALID,
        parent_span_is_remote: false,
        instrumentation_scope,
        dropped_attributes_count: 0,
        span_kind: SpanKind::Internal,
        name: Cow::Borrowed("benchmark-span"),
        start_time: SystemTime::now(),
        end_time: SystemTime::now(),
        attributes: Vec::new(),
        events: SpanEvents::default(),
        links: SpanLinks::default(),
        status: Status::Unset,
    }
}

pub struct FakeCapnp;
impl FakeCapnp {
    pub fn trace_service_request_with_spans(num_spans: usize) -> SpanRequest {
        let batch = build_batch(num_spans);
        let resource = build_resource();
        SpanRequest { batch, resource }
    }
}

fn build_batch(num_spans: usize) -> Vec<SpanData> {
    let mut batch = Vec::<SpanData>::with_capacity(num_spans);
    for _ in 0..num_spans {
        batch.push(create_test_span_data());
    }
    batch
}

fn build_resource() -> Resource {
    Resource::builder().build()
}
