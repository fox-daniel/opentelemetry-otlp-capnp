use crate::trace_capnp;
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
