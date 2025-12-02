use crate::trace_capnp;
use opentelemetry::trace::SpanKind;
use opentelemetry_sdk::trace::SpanData;
use std::time::UNIX_EPOCH;

/// Populate a Span with minimal data for testing
pub fn populate_span_minimal(
    mut builder: trace_capnp::span::Builder,
    span: SpanData,
) -> Result<(), Box<dyn std::error::Error>> {
    // Required fields only
    builder.set_trace_id(&span.span_context.trace_id().to_bytes());
    builder.set_span_id(&span.span_context.span_id().to_bytes());
    builder.set_name(&span.name);
    builder.set_parent_span_id(&span.parent_span_id.to_bytes());

    // Timestamps
    let start = span.start_time.duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    let end = span.end_time.duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    builder.set_start_time_unix_nano(start);
    builder.set_end_time_unix_nano(end);

    // Set kind to Internal as default
    builder.set_kind(trace_capnp::span::SpanKind::SpanKindInternal);

    // Empty collections for now
    builder.reborrow().init_attributes(0);
    builder.reborrow().init_events(0);
    builder.reborrow().init_links(0);

    // Set simple status
    let mut status = builder.init_status();
    status.set_code(trace_capnp::status::StatusCode::Unset);
    status.set_message("i am a test span");

    Ok(())
}

impl From<SpanKind> for trace_capnp::span::SpanKind {
    fn from(span_kind: SpanKind) -> Self {
        match span_kind {
            SpanKind::Client => trace_capnp::span::SpanKind::SpanKindClient,
            SpanKind::Consumer => trace_capnp::span::SpanKind::SpanKindConsumer,
            SpanKind::Internal => trace_capnp::span::SpanKind::SpanKindInternal,
            SpanKind::Producer => trace_capnp::span::SpanKind::SpanKindProducer,
            SpanKind::Server => trace_capnp::span::SpanKind::SpanKindServer,
        }
    }
}

pub fn populate_span(
    mut builder: trace_capnp::span::Builder,
    source_span: SpanData,
) -> Result<(), Box<dyn std::error::Error>> {
    let span_kind: trace_capnp::span::SpanKind = source_span.span_kind.into();
    builder.set_trace_id(&source_span.span_context.trace_id().to_bytes());
    builder.set_span_id(&source_span.span_context.span_id().to_bytes());
    builder.set_trace_state(source_span.span_context.trace_state().header());
    builder.set_parent_span_id(&source_span.parent_span_id.to_bytes());
    // TODO: set flags
    builder.set_name(&source_span.name);
    builder.set_kind(span_kind);
    // Timestamps
    let start = source_span
        .start_time
        .duration_since(UNIX_EPOCH)?
        .as_nanos() as u64;
    let end = source_span.end_time.duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    builder.set_start_time_unix_nano(start);
    builder.set_end_time_unix_nano(end);
    builder.set_dropped_attributes_count(source_span.dropped_attributes_count);
    // // Set kind to Internal as default
    // builder.set_kind(trace_capnp::span::SpanKind::SpanKindInternal);

    // Empty collections for now
    builder.reborrow().init_attributes(0);
    // builder.set_attributes(source_span.attributes);
    builder.reborrow().init_events(0);
    builder.reborrow().init_links(0);

    // Set simple status
    let mut status = builder.init_status();
    status.set_code(trace_capnp::status::StatusCode::Unset);
    status.set_message("i am a test span");

    Ok(())
}
