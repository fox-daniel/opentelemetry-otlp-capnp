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

### Basic Design

```rust
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use std::thread;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
use tokio::task::LocalSet;

pub struct CapnpSpanExporter {
    tx_export: UnboundedSender<Vec<SpanData>>,
    tx_shutdown: UnboundedSender<()>,
}

impl CapnpSpanExporter {
    pub fn new(endpoint: String) -> Self {
        let (tx_export, rx_export) = unbounded_channel();
        let (tx_shutdown, rx_shutdown) = unbounded_channel();
        
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            
            let local = LocalSet::new();
            
            local.block_on(&rt, async {
                // Connect to server
                let addr = endpoint.parse().expect("valid socket address");
                let stream = tokio::net::TcpStream::connect(&addr)
                    .await
                    .expect("connection failed");
                stream.set_nodelay(true).expect("set_nodelay failed");
                
                let (reader, writer) = 
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                
                // Setup Cap'n Proto RPC
                let rpc_network = Box::new(twoparty::VatNetwork::new(
                    futures::io::BufReader::new(reader),
                    futures::io::BufWriter::new(writer),
                    rpc_twoparty_capnp::Side::Client,
                    Default::default(),
                ));
                
                let mut rpc_system = RpcSystem::new(rpc_network, None);
                let client: trace_service_capnp::trace_service::Client = 
                    rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
                
                // Spawn RPC system
                tokio::task::spawn_local(rpc_system);
                
                // Run export loop (blocks until shutdown)
                export_loop(client, rx_export, rx_shutdown).await;
            });
        });
        
        CapnpSpanExporter { tx_export, tx_shutdown }
    }
}

async fn export_loop(
    client: trace_service_capnp::trace_service::Client,
    mut rx_export: UnboundedReceiver<Vec<SpanData>>,
    mut rx_shutdown: UnboundedReceiver<()>,
) {
    loop {
        tokio::select! {
            Some(batch) = rx_export.recv() => {
                if let Err(e) = export_batch(&client, batch).await {
                    eprintln!("Export failed: {}", e);
                }
            }
            Some(()) = rx_shutdown.recv() => {
                // Drain remaining exports
                while let Ok(batch) = rx_export.try_recv() {
                    let _ = export_batch(&client, batch).await;
                }
                break;
            }
        }
    }
}

async fn export_batch(
    client: &trace_service_capnp::trace_service::Client,
    batch: Vec<SpanData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut request = client.export_request();
    // ... serialize batch into request ...
    let response = request.send().promise.await?;
    Ok(())
}
```

(older) Basic design:

```rust
pub struct CapnpSpanExporter {
    tx_export: tokio::sync::mpsc::UnboundedSender<Vec<SpanData>>,
    tx_shutdown: tokio::sync::mpsc::UnboundedSender<ShutDown>,
    // Track thread handle for clean shutdown
    _thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

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

impl CapnpSpanExporter {
    pub fn new(endpoint: String) -> Self {
        let (tx_export, rx_export) = tokio::sync::mpsc::unbounded_channel();
        let (tx_shutdown, rx_shutdown) = tokio::sync::mpsc::unbounded_channel();
        
        // Spawn dedicated OS thread
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            
            let local = tokio::task::LocalSet::new();
            
            local.block_on(&rt, async {
                // Create Cap'n Proto client (lives only on this thread)
                let client = create_capnp_client(&endpoint).await.unwrap();
                
                // Export task
                tokio::task::spawn_local(export_loop(
                    client,
                    rx_export,
                    rx_shutdown,
                ));
            });
        });
        
        CapnpSpanExporter { tx_export, tx_shutdown }
    }
}

async fn export_loop(
    client: trace_service_capnp::trace_service::Client,
    mut rx_export: UnboundedReceiver<ExportMessage>,
    mut rx_shutdown: UnboundedReceiver<()>,
) {
    loop {
        tokio::select! {
            Some(ExportMessage::Batch(spans)) = rx_export.recv() => {
                if let Err(e) = export_batch(&client, spans).await {
                    eprintln!("Export failed: {}", e);
                }
            }
            Some(()) = rx_shutdown.recv() => {
                break;
            }
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
