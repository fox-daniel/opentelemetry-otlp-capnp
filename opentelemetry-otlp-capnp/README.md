# opentelemetry-otlp-capnp
## OpenTelemetry Exporters and Recievers that use Cap'N Proto
This workspace is for developing OpenTelemetry Exporters and Receivers that use
Cap'n Proto in place of ProtoBuf and gRPC for OpenTelemetry. Cap'n Proto should provide
performance gains both in terms of speed and memory usage. Preliminary benchmarks show that
exporting Spans is almost 2x faster for batches of 1000 spans. The gains are closer to 1.5x faster 
for smaller batches. Even greater gains are expected on the receiver side. 


![Span Export Performance](docs/benchmarks/large_batch_violin_plot.png)

The performance gains mean that your monitoring is using less resources.

- The `opentelemetry-otlp-capnp` crate defines the exporters and receivers needed to use
Cap'N Proto for the over-the-wire protocol for telemetry. It is
the Cap'N Proto equivalent of the `opentelemetry-otlp` crate which uses
`tonic` for `gRPC`.
- The Cap'N Proto schema that follows the OTEL spec is defined in
the `opentelemetry-capnp` crate. That crate is
the Cap'N Proto equivalent of the `opentelemetry-proto` crate
which has the ProtoBuf schema.

This project is in early development.

## Usage
Clone the repo
```bash
git clone https://github.com/fox-daniel/opentelemetry-otlp-capnp.git
```
Install `capnp` usning [just](https://github.com/casey/just):
```bash
just install-capnp
```
Then you can build the project with
```bash
cargo build
```
There are examples in `opentelemetry-otlp-capnp/opentelemetry-otlp-capnp/examples` with instructions in their `README`s.

## Benchmarks
Run the benchmarks from `opentelemetry-otlp-capnp/opentelemetry-otlp-capnp` with
```bash
cargo bench  
```
A report from the benchmarks can be found in
```
opentelemetry-otlp-capnp/target/criterion/report/index.html
```
