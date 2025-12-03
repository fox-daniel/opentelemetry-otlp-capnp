use crate::common_capnp::{self, any_value::Builder};
use crate::trace_capnp;
use opentelemetry::trace::{self, SpanKind};
use opentelemetry::{KeyValue, Value};
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

impl From<&trace::Status> for trace_capnp::status::StatusCode {
    fn from(status: &trace::Status) -> Self {
        match status {
            trace::Status::Ok => trace_capnp::status::StatusCode::Ok,
            trace::Status::Unset => trace_capnp::status::StatusCode::Unset,
            trace::Status::Error { .. } => trace_capnp::status::StatusCode::Error,
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

    let attributes = source_span.attributes;
    let mut attributes_builder = builder.reborrow().init_attributes(attributes.len() as u32);
    // TODO: should this be more similar to the OTLP transform?
    // `impl From<Value> for AnyValue` instead of populate_value_builder
    for (id, attr) in attributes.into_iter().enumerate() {
        let mut kv_builder = attributes_builder.reborrow().get(id as u32);
        kv_builder.reborrow().set_key(attr.key.as_str());
        populate_value_builder(kv_builder.init_value(), &attr.value)?;
    }
    builder.set_dropped_events_count(source_span.events.dropped_count);
    builder.reborrow().init_events(0);
    builder.reborrow().init_links(0);
    //TODO:
    // - events
    // - links
    builder.set_dropped_links_count(source_span.links.dropped_count);
    let mut status = builder.init_status();
    status.set_code(trace_capnp::status::StatusCode::from(&source_span.status));
    status.set_message(match &source_span.status {
        trace::Status::Error { description } => description.to_string(),
        _ => Default::default(),
    });

    Ok(())
}

fn populate_value_builder(
    value_builder: Builder<'_>,
    value: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    use opentelemetry::Value;
    let mut value_builder = value_builder.init_value();
    match value {
        Value::Bool(val) => value_builder.set_bool_value(*val),
        Value::I64(val) => value_builder.set_int_value(*val),
        Value::F64(val) => value_builder.set_double_value(*val),
        Value::String(val) => value_builder.set_string_value(val),
        Value::Array(arr) => {
            populate_array(value_builder.init_array_value(), arr)?;
        }
        _ => {
            value_builder.set_string_value("unsupported");
        }
    }
    Ok(())
}

fn populate_array(
    array_value_builder: common_capnp::array_value::Builder<'_>,
    array: &opentelemetry::Array,
) -> Result<(), Box<dyn std::error::Error>> {
    use opentelemetry::Array;

    match array {
        Array::Bool(bools) => {
            let mut values = array_value_builder.init_values(bools.len() as u32);
            for (idx, &b) in bools.iter().enumerate() {
                values
                    .reborrow()
                    .get(idx as u32)
                    .init_value()
                    .set_bool_value(b);
            }
        }
        Array::I64(ints) => {
            let mut values = array_value_builder.init_values(ints.len() as u32);
            for (idx, &i) in ints.iter().enumerate() {
                values
                    .reborrow()
                    .get(idx as u32)
                    .init_value()
                    .set_int_value(i);
            }
        }
        Array::F64(floats) => {
            let mut values = array_value_builder.init_values(floats.len() as u32);
            for (idx, &f) in floats.iter().enumerate() {
                values
                    .reborrow()
                    .get(idx as u32)
                    .init_value()
                    .set_double_value(f);
            }
        }
        Array::String(strings) => {
            let mut values = array_value_builder.init_values(strings.len() as u32);
            for (idx, s) in strings.iter().enumerate() {
                values
                    .reborrow()
                    .get(idx as u32)
                    .init_value()
                    .set_string_value(s.as_ref());
            }
        }
        _ => {}
    }
    Ok(())
}
