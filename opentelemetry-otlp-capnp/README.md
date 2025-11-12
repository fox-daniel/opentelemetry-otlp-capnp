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


## With Claude
What follows is from an exchange with Claude Desktop about the design of the Cap'n Proto SpanExporter.
It is motivated by the TokioSpanExporter test in the `opentelemetry-sdk` crate.

Basic design:

```rust
pub struct CapnpSpanExporter {
    tx_export: tokio::sync::mpsc::UnboundedSender<Vec<SpanData>>,
    tx_shutdown: tokio::sync::mpsc::UnboundedSender<ShutDown>,
    // Track thread handle for clean shutdown
    _thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

struct ShutDown;

impl CapnpSpanExporter {
    /// Creates a new Cap'n Proto span exporter.
    /// 
    /// This spawns a dedicated OS thread with a single Cap'n Proto client.
    /// One exporter instance = one client = one connection to the collector.
    /// 
    /// Typical usage: create once during application initialization and
    /// install in a global TracerProvider.
    pub fn builder() -> CapnpExporterBuilder {
        CapnpExporterBuilder::default()
    }
}

impl Drop for CapnpSpanExporter {
    fn drop(&mut self) {
        // Signal shutdown
        let _ = self.tx_shutdown.send(());
        
        // Wait for thread to finish (with timeout)
        if let Some(handle) = self._thread_handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}
```
The usage pattern:
```rust
#[tokio::main]
async fn main() {
    // Initialize once
    let exporter = CapnpSpanExporter::builder()
        .with_endpoint("http://localhost:4317")
        .with_timeout(Duration::from_secs(10))
        .build()
        .expect("failed to create exporter");
    
    // ONE provider for entire process
    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter)  // Moves exporter in
        .build();
    
    global::set_tracer_provider(provider);
    
    // Application runs with this single configuration
    run_application().await;
    
    // Cleanup
    global::shutdown_tracer_provider();
}```
