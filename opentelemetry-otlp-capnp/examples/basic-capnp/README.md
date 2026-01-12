# Basic OTLP Exporter Example

This example is derived from [basic-otlp](opentelemetry-otlp/examples/basic-otlp)
in the `opentelemetry-rust/opentelemetry-otlp` create.

This example demonstrates how to set up an `opentelemetry-otlp-capnp` SpanExporter.
Additionally, the example configures a `tracing::fmt` layer to output logs
emitted via `tracing` to `stdout`. For demonstration, this layer uses a filter
to display `DEBUG` level logs from various OpenTelemetry components. In real
applications, these filters should be adjusted appropriately.

The example employs a `BatchExporter` for logs and traces, which is the
recommended approach when using OTLP exporters. 

## Usage
Run the app which exports logs, metrics and traces via OTLP to the collector

```shell
cargo run
```

## View results

You should be able to see something similar to the following in stdout:
```
[
  (
    scope = (
      name = "basic",
      version = "1.0",
      attributes = [
        (
          key = "scope-key",
          value = (
            value = (
              stringValue = "scope-value"
            )
          )
        )
      ],
      droppedAttributesCount = 0
    ),
    spans = [
      (
        traceId = 0x"4c8b37200d46302daa95e44f6163a94a",
        spanId = 0x"28e7a5b8873feb50",
        traceState = "",
        parentSpanId = 0x"0f7fe653935339d8",
        name = "Sub operation...",
        kind = spanKindInternal,
        startTimeUnixNano = 1768240501853034000,
        endTimeUnixNano = 1768240501853040000,
        attributes = [
          (
            key = "another.key",
            value = (
              value = (
                stringValue = "yes"
              )
            )
          )
        ],
        droppedAttributesCount = 0,
        events = [
          (
            timeUnixNano = 1768240501853038000,
            name = "Sub span event",
            attributes = [],
            droppedAttributesCount = 0
          )
        ],
        droppedEventsCount = 0,
        links = [],
        droppedLinksCount = 0,
        status = (
          reserved0 = (),
          message = "",
          code = unset
        ),
        flags = 0
      ),
      (
        traceId = 0x"4c8b37200d46302daa95e44f6163a94a",
        spanId = 0x"0f7fe653935339d8",
        traceState = "",
        parentSpanId = 0x"0000000000000000",
        name = "Main operation",
        kind = spanKindInternal,
        startTimeUnixNano = 1768240501852796000,
        endTimeUnixNano = 1768240501853044000,
        attributes = [
          (
            key = "another.key",
            value = (
              value = (
                stringValue = "yes"
              )
            )
          )
        ],
        droppedAttributesCount = 0,
        events = [
          (
            timeUnixNano = 1768240501852802000,
            name = "Nice operation!",
            attributes = [
              (
                key = "bogons",
                value = (
                  value = (
                    intValue = 100
                  )
                )
              )
            ],
            droppedAttributesCount = 0
          )
        ],
        droppedEventsCount = 0,
        links = [],
        droppedLinksCount = 0,
        status = (
          reserved0 = (),
          message = "",
          code = unset
        ),
        flags = 0
      )
    ],
    schemaUrl = ""
  )
]     
```
