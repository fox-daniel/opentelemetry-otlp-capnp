use crate::capnp::capnp_rpc::common_capnp::{self, any_value::Builder};
use crate::capnp::capnp_rpc::trace_capnp;
use crate::transform::common::to_nanos;
use opentelemetry::trace::{self, SpanKind};
use opentelemetry::{InstrumentationScope, KeyValue, Value};
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

pub fn populate_scope_spans(
    mut builder: trace_capnp::scope_spans::Builder,
    span_records: Vec<&SpanData>,
    instrumentation: &InstrumentationScope,
) -> Result<(), Box<dyn std::error::Error>> {
    let instrumentation_builder = builder.reborrow().init_scope();
    populate_instrumentation_scope(instrumentation_builder, instrumentation);
    let scope_spans_builder = builder.reborrow().init_spans(span_records.len() as u32);
    populate_scope_spans_builder(scope_spans_builder, span_records);
    Ok(())
}

fn populate_scope_spans_builder(
    scope_spans_builder: capnp::struct_list::Builder<'_, trace_capnp::span::Owned>,
    span_records: Vec<&SpanData>) -> Result<(), Box<dyn std::error::Error>> {
    // TOOD
    // implementat using populate spans functions
    Ok(())
}

fn populate_instrumentation_scope(mut instrumentation_builder: common_capnp::instrumentation_scope::Builder<'_> , instrumentation_scope: &InstrumentationScope) -> Result<(), Box<dyn std::error::Error>> {
    instrumentation_builder.set_name(instrumentation_scope.name());
    // Check that this default is correct
    instrumentation_builder.reborrow()
        .set_version(instrumentation_scope.version().unwrap_or_else(Default::default));
    let instrumentation_attributes_builder = instrumentation_builder.reborrow().init_attributes(instrumentation_scope.attributes().count() as u32)
    populate_instrumentation_attributes(instrumentation_attributes_builder, instrumentation_scope);
    instrumentation_builder.set_dropped_attributes_count(0);
    Ok(())
}
 
fn populate_instrumentation_attributes(mut instrumentation_attributes_builder: capnp::struct_list::Builder<'_, common_capnp::key_value::Owned>, instrumentation: &InstrumentationScope) -> Result<(), Box<dyn std::error::Error>> {
    // TODO
    // implement
    Ok(())
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
    populate_attributes(attributes_builder, attributes);
    builder.set_dropped_events_count(source_span.events.dropped_count);
    // TODO: events builder refactor into abstractions
    let mut events_builder = builder
        .reborrow()
        .init_events(source_span.events.len() as u32);
    for (id, event) in source_span.events.events.into_iter().enumerate() {
        let mut event_builder = events_builder.reborrow().get(id as u32);
        event_builder
            .reborrow()
            .set_time_unix_nano(to_nanos(event.timestamp));
        event_builder.reborrow().set_name(event.name.into_owned());
        let mut event_attributes_builder = event_builder
            .reborrow()
            .init_attributes(event.attributes.len() as u32);
        populate_attributes(event_attributes_builder, event.attributes);
        event_builder
            .reborrow()
            .set_dropped_attributes_count(event.dropped_attributes_count);
    }

    builder.set_dropped_links_count(source_span.links.dropped_count);
    let mut links_builder = builder
        .reborrow()
        .init_links(source_span.links.len() as u32);
    for (id, link) in source_span.links.into_iter().enumerate() {
        let mut link_builder = links_builder.reborrow().get(id as u32);
        link_builder
            .reborrow()
            .set_trace_id(&link.span_context.trace_id().to_bytes());
        link_builder
            .reborrow()
            .set_span_id(&link.span_context.span_id().to_bytes());
        link_builder
            .reborrow()
            .set_trace_state(link.span_context.trace_state().header());
        link_builder
            .reborrow()
            .set_dropped_attributes_count(link.dropped_attributes_count);
        let mut attr_builder = link_builder
            .reborrow()
            .init_attributes(link.attributes.len() as u32);
        populate_attributes(attr_builder, link.attributes);
        // link_builder.set_flags();
    }
    //TODO:
    // - links: spanflags
    let mut status = builder.init_status();
    status.set_code(trace_capnp::status::StatusCode::from(&source_span.status));
    status.set_message(match &source_span.status {
        trace::Status::Error { description } => description.to_string(),
        _ => Default::default(),
    });

    Ok(())
}

fn populate_attributes(
    mut attributes_builder: capnp::struct_list::Builder<'_, crate::common_capnp::key_value::Owned>,
    attributes: Vec<KeyValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    for (id, attr) in attributes.into_iter().enumerate() {
        let mut kv_builder = attributes_builder.reborrow().get(id as u32);
        kv_builder.reborrow().set_key(attr.key.as_str());
        populate_value_builder(kv_builder.init_value(), &attr.value)?;
    }
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

// pub(crate) fn build_span_flags(parent_span_is_remote: bool, base_flags: u32) -> u32 {
//     use crate::;
//     let mut flags = base_flags;
//     flags |= SpanFlags::ContextHasIsRemoteMask as u32;
//     if parent_span_is_remote {
//         flags |= SpanFlags::ContextIsRemoteMask as u32;
//     }
//     flags
// }
