@0x8fa361a9b01b7dc0;

using Common = import "../../common/v1/common.capnp";
using Resource = import "../../resource/v1/resource.capnp";


# BEGIN Temporary schema elements to get an example working; later this SpanData will
# get replaced by using TracesData -> ResourceSpans -> ScopeSpans
interface SpanExport {
  
  struct SpanData {
    spans @0 :List(Span);
  }

  struct SpanDataReply {
    count @0 :UInt16;
  }

  sendSpanData @0 (request: SpanData) -> (reply: SpanDataReply);
}

# END Temporary schema elements


# TracesData represents the traces data that can be stored in a persistent storage,
# OR can be embedded by other protocols that transfer OTLP traces data but do
# not implement the OTLP protocol.
#
# The main difference between this message and collector protocol is that
# in this message there will not be any "control" or "metadata" specific to
# OTLP protocol.
#
# When new fields are added into this message, the OTLP request MUST be updated
# as well.
struct TracesData {
  # An array of ResourceSpans.
  # For data coming from a single resource this array will typically contain
  # one element. Intermediary nodes that receive data from multiple origins
  # typically batch the data before forwarding further and in that case this
  # array will contain multiple elements.
  resourceSpans @0 :List(ResourceSpans);
}

# A collection of ScopeSpans from a Resource.
struct ResourceSpans {
  
  # The resource for the spans in this message.
  # If this field is not set then no resource info is known.
  resource @0 :Resource.Resource;

  # A list of ScopeSpans that originate from a resource.
  scopeSpans @1 :List(ScopeSpans);

  # The Schema URL, if known. This is the identifier of the Schema that the resource data
  # is recorded in. Notably, the last part of the URL path is the version number of the
  # schema: http[s]:#server[:port]/path/<version>. To learn more about Schema URL see
  # https:#opentelemetry.io/docs/specs/otel/schemas/#schema-url
  # This schema_url applies to the data in the "resource" field. It does not apply
  # to the data in the "scope_spans" field which have their own schema_url field.
  schemaUrl @2 :Text;
}

# A collection of Spans produced by an InstrumentationScope.
struct ScopeSpans {
  # The instrumentation scope information for the spans in this message.
  # Semantically when InstrumentationScope isn't set, it is equivalent with
  # an empty instrumentation scope name (unknown).
  scope @0 :Common.InstrumentationScope;

  # A list of Spans that originate from an instrumentation scope.
  spans @1 :List(Span);

  # The Schema URL, if known. This is the identifier of the Schema that the span data
  # is recorded in. Notably, the last part of the URL path is the version number of the
  # schema: http[s]:#server[:port]/path/<version>. To learn more about Schema URL see
  # https:#opentelemetry.io/docs/specs/otel/schemas/#schema-url
  # This schema_url applies to the data in the "scope" field and all spans and span
  # events in the "spans" field.
  schemaUrl @2 :Text;
}

# A Span represents a single operation performed by a single component of the system.
#
# The next available field id is 17.
struct Span {
  # A unique identifier for a trace. All spans from the same trace share
  # the same `trace_id`. The ID is a 16-byte array. An ID with all zeroes OR
  # of length other than 16 bytes is considered invalid (empty string in OTLP/JSON
  # is zero-length and thus is also invalid).
  #
  # This field is required.
  traceId @0 :Data;

  # A unique identifier for a span within a trace, assigned when the span
  # is created. The ID is an 8-byte array. An ID with all zeroes OR of length
  # other than 8 bytes is considered invalid (empty string in OTLP/JSON
  # is zero-length and thus is also invalid).
  #
  # This field is required.
  spanId @1 :Data;

  # trace_state conveys information about request position in multiple distributed tracing graphs.
  # It is a trace_state in w3c-trace-context format: https:#www.w3.org/TR/trace-context/#tracestate-header
  # See also https:#github.com/w3c/distributed-tracing for more details about this field.
  traceState @2 :Text;

  # The `span_id` of this span's parent span. If this is a root span, then this
  # field must be empty. The ID is an 8-byte array.
  parentSpanId @3 :Data;

  # Flags, a bit field.
  #
  # Bits 0-7 (8 least significant bits) are the trace flags as defined in W3C Trace
  # Context specification. To read the 8-bit W3C trace flag, use
  # `flags & SPAN_FLAGS_TRACE_FLAGS_MASK`.
  #
  # See https:#www.w3.org/TR/trace-context-2/#trace-flags for the flag definitions.
  #
  # Bits 8 and 9 represent the 3 states of whether a span's parent
  # is remote. The states are (unknown, is not remote, is remote).
  # To read whether the value is known, use `(flags & SPAN_FLAGS_CONTEXT_HAS_IS_REMOTE_MASK) != 0`.
  # To read whether the span is remote, use `(flags & SPAN_FLAGS_CONTEXT_IS_REMOTE_MASK) != 0`.
  #
  # When creating span messages, if the message is logically forwarded from another source
  # with an equivalent flags fields (i.e., usually another OTLP span message), the field SHOULD
  # be copied as-is. If creating from a source that does not have an equivalent flags field
  # (such as a runtime representation of an OpenTelemetry span), the high 22 bits MUST
  # be set to zero.
  # Readers MUST NOT assume that bits 10-31 (22 most significant bits) will be zero.
  #
  # [Optional].
  flags @15 :UInt32;

  # A description of the span's operation.
  #
  # For example, the name can be a qualified method name or a file name
  # and a line number where the operation is called. A best practice is to use
  # the same display name at the same call point in an application.
  # This makes it easier to correlate spans in different traces.
  #
  # This field is semantically required to be set to non-empty string.
  # Empty value is equivalent to an unknown span name.
  #
  # This field is required.
  name @4 :Text;

  # SpanKind is the type of span. Can be used to specify additional relationships between spans
  # in addition to a parent/child relationship.
  enum SpanKind {
    # Unspecified. Do NOT use as default.
    # Implementations MAY assume SpanKind to be INTERNAL when receiving UNSPECIFIED.
    spanKindUnspecified @0;

    # Indicates that the span represents an internal operation within an application,
    # as opposed to an operation happening at the boundaries. Default value.
    spanKindInternal @1;

    # Indicates that the span covers server-side handling of an RPC or other
    # remote network request.
    spanKindServer @2;

    # Indicates that the span describes a request to some remote service.
    spanKindClient @3;

    # Indicates that the span describes a producer sending a message to a broker.
    # Unlike CLIENT and SERVER, there is often no direct critical path latency relationship
    # between producer and consumer spans. A PRODUCER span ends when the message was accepted
    # by the broker while the logical processing of the message might span a much longer time.
    spanKindProducer @4;

    # Indicates that the span describes consumer receiving a message from a broker.
    # Like the PRODUCER kind, there is often no direct critical path latency relationship
    # between producer and consumer spans.
    spanKindConsumer @5;
  }

  # Distinguishes between spans generated in a particular context. For example,
  # two spans with the same name may be distinguished using `CLIENT` (caller)
  # and `SERVER` (callee) to identify queueing latency associated with the span.
  kind @5 :SpanKind;

  # The start time of the span. On the client side, this is the time
  # kept by the local machine where the span execution starts. On the server side, this
  # is the time when the server's application handler starts running.
  # Value is UNIX Epoch time in nanoseconds since 00:00:00 UTC on 1 January 1970.
  #
  # This field is semantically required and it is expected that end_time >= start_time.
  startTimeUnixNano @6 :UInt64;

  # The end time of the span. On the client side, this is the time
  # kept by the local machine where the span execution ends. On the server side, this
  # is the time when the server application handler stops running.
  # Value is UNIX Epoch time in nanoseconds since 00:00:00 UTC on 1 January 1970.
  #
  # This field is semantically required and it is expected that end_time >= start_time.
  endTimeUnixNano @7 :UInt64;

  # A collection of key/value pairs. Note, global attributes
  # like server name can be set using the resource API. Examples of attributes:
  #
  #     "/http/user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/71.0.3578.98 Safari/537.36"
  #     "/http/server_latency": 300
  #     "example.com/myattribute": true
  #     "example.com/score": 10.239
  #
  # Attribute keys MUST be unique (it is not allowed to have more than one
  # attribute with the same key).
  # The behavior of software that receives duplicated keys can be unpredictable.
  attributes @8 :List(Common.KeyValue); 

  # The number of attributes that were discarded. Attributes
  # can be discarded because their keys are too long or because there are too many
  # attributes. If this value is 0, then no attributes were dropped.
  droppedAttributesCount @9 :UInt32;

  # Event is a time-stamped annotation of the span, consisting of user-supplied
  # text description and key-value pairs.
  struct Event {
    # The time the event occurred.
    timeUnixNano @0 :UInt64;

    # The name of the event.
    # This field is semantically required to be set to non-empty string.
    name @1 :Text;

    # A collection of attribute key/value pairs on the event.
    # Attribute keys MUST be unique (it is not allowed to have more than one
    # attribute with the same key).
    # The behavior of software that receives duplicated keys can be unpredictable.
    attributes @2 :List(Common.KeyValue);

    # The number of dropped attributes. If the value is 0,
    # then no attributes were dropped.
    droppedAttributesCount @3 :UInt32;
  }

  # A collection of Event items.
  events @10 :List(Event);

  # The number of dropped events. If the value is 0, then no
  # events were dropped.
  droppedEventsCount @11 :UInt32;

  # A pointer from the current span to another span in the same trace or in a
  # different trace. For example, this can be used in batching operations,
  # where a single batch handler processes multiple requests from different
  # traces or when the handler receives a request from a different project.
  struct Link {
    # A unique identifier of a trace that this linked span is part of. The ID is a
    # 16-byte array.
    traceId @0 :Data;

    # A unique identifier for the linked span. The ID is an 8-byte array.
    spanId @1 :Data;

    # The trace_state associated with the link.
    traceState @2 :Text;

    # A collection of attribute key/value pairs on the link.
    # Attribute keys MUST be unique (it is not allowed to have more than one
    # attribute with the same key).
    # The behavior of software that receives duplicated keys can be unpredictable.
    attributes @3 :List(Common.KeyValue);

    # The number of dropped attributes. If the value is 0,
    # then no attributes were dropped.
    droppedAttributesCount @4 :UInt32;

    # Flags, a bit field.
    #
    # Bits 0-7 (8 least significant bits) are the trace flags as defined in W3C Trace
    # Context specification. To read the 8-bit W3C trace flag, use
    # `flags & SPAN_FLAGS_TRACE_FLAGS_MASK`.
    #
    # See https:#www.w3.org/TR/trace-context-2/#trace-flags for the flag definitions.
    #
    # Bits 8 and 9 represent the 3 states of whether the link is remote.
    # The states are (unknown, is not remote, is remote).
    # To read whether the value is known, use `(flags & SPAN_FLAGS_CONTEXT_HAS_IS_REMOTE_MASK) != 0`.
    # To read whether the link is remote, use `(flags & SPAN_FLAGS_CONTEXT_IS_REMOTE_MASK) != 0`.
    #
    # Readers MUST NOT assume that bits 10-31 (22 most significant bits) will be zero.
    # When creating new spans, bits 10-31 (most-significant 22-bits) MUST be zero.
    #
    # [Optional].
    flags @5 :UInt32;
  }

  # A collection of Links, which are references from this span to a span
  # in the same or different trace.
  links @12 :List(Link);

  # The number of dropped links after the maximum size was
  # enforced. If this value is 0, then no links were dropped.
  droppedLinksCount @13 :UInt32;

  # An optional final status for this span. Semantically when Status isn't set, it means
  # span's status code is unset, i.e. assume STATUS_CODE_UNSET (code = 0).
  status @14 :Status;
}

# The Status type defines a logical error model that is suitable for different
# programming environments, including REST APIs and RPC APIs.
struct Status {
  reserved0 @0 :Void;

  # A developer-facing human readable error message.
  message @1 :Text;

  # For the semantics of status codes see
  # https:#github.com/open-telemetry/opentelemetry-specification/blob/main/specification/trace/api.md#set-status
  enum StatusCode {
    # The default status.
    unset @0;
    # The Span has been validated by an Application developer or Operator to 
    # have completed successfully.
    ok @1;
    # The Span contains an error.
    error @2;
  }

  # The status code.
  code @2 :StatusCode;
}

# SpanFlags represents constants used to interpret the
# Span.flags field, which is protobuf 'fixed32' type and is to
# be used as bit-fields. Each non-zero value defined in this enum is
# a bit-mask.  To extract the bit-field, for example, use an
# expression like:
#
#   (span.flags & SPAN_FLAGS_TRACE_FLAGS_MASK)
#
# See https:#www.w3.org/TR/trace-context-2/#trace-flags for the flag definitions.
#
# Note that Span flags were introduced in version 1.1 of the
# OpenTelemetry protocol.  Older Span producers do not set this
# field, consequently consumers should not rely on the absence of a
# particular flag bit to indicate the presence of a particular feature.
# enum SpanFlags {
  # The zero value for the enum. Should not be used for comparisons.
  # Instead use bitwise "and" with the appropriate mask as shown above.
  # do_not_use @0;

  # Bits 0-7 are used for trace flags.
  # trace_flags_mask 
  # SPAN_FLAGS_TRACE_FLAGS_MASK = 0x000000FF;

  # Bits 8 and 9 are used to indicate that the parent span or link span is remote.
  # Bit 8 (`HAS_IS_REMOTE`) indicates whether the value is known.
  # Bit 9 (`IS_REMOTE`) indicates whether the span or link is remote.
  # SPAN_FLAGS_CONTEXT_HAS_IS_REMOTE_MASK = 0x00000100;
  # SPAN_FLAGS_CONTEXT_IS_REMOTE_MASK = 0x00000200;

  # Bits 10-31 are reserved for future use.
#} 
