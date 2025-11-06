# opentelemetry-otlp-capnp
## OpenTelemetry Exporters and Recievers that use Cap'N Proto
Cap'N Proto has the potential to offer improvements in performance
and security in the OpenTelemetry ecosystem. This crate will have a
minimal implementation.

- This crate defines the exporters and receivers needed to use
Cap'N Proto for the over-the-wire protocol for telemetry. It is
the Cap'N Proto equivalent of `opentelemetry-otlp` which uses
`tonic` for `gRPC`.
- The Cap'N Proto schema that follows the OTEL spec is defined in
the `opentelemetry-capnp` crate and is used here. That crate is
the Cap'N Proto equivalent of the `opentelemetry-proto` crate
which has the ProtoBuf schema.
