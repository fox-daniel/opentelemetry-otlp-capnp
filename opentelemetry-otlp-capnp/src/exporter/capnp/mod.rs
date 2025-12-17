use std::env;
use std::io;
use std::io::{ErrorKind, Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
pub(crate) mod trace;
use crate::retry::RetryPolicy;
use crate::span::OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT;
use crate::{ExportConfig, ExporterBuildError};
use crate::{OTEL_EXPORTER_CAPNP_ENDPOINT, OTEL_EXPORTER_CAPNP_ENDPOINT_DEFAULT};

// use crate::ExportConfig;
/// Configuration for [capnp]
///
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct CapnpConfig {
    // The compression algorithm to use when communicating with the collector.
    // pub(crate) compression: Option<Compression>,
    // pub(crate) channel: Option<tonic::transport::Channel>,
    // pub(crate) interceptor: Option<BoxInterceptor>,
    // The retry policy to use for gRPC requests.
    // #[cfg(feature = "experimental-grpc-retry")]
    pub(crate) retry_policy: Option<RetryPolicy>,
}

#[derive(Debug, Default)]
pub struct CapnpExporterBuilder {
    pub(crate) capnp_config: CapnpConfig,
    pub(crate) exporter_config: ExportConfig,
}

// Expose interface for modifying [CapnpConfig] fields within the exporter builders.
pub trait HasCapnpConfig {
    /// Return a mutable reference to the export config within the exporter builders.
    fn capnp_config(&mut self) -> &mut CapnpConfig;
}

// Expose interface for modifying [CapnpConfig] fields within the [CapnpExporterBuilder].
impl HasCapnpConfig for CapnpExporterBuilder {
    fn capnp_config(&mut self) -> &mut CapnpConfig {
        &mut self.capnp_config
    }
}

impl CapnpExporterBuilder {
    /// Build a new tonic span exporter
    pub(crate) fn build_span_exporter(self) -> Result<crate::SpanExporter, ExporterBuildError> {
        use crate::exporter::capnp::trace::CapnpTracesClient;

        // otel_debug!(name: "TracesCapnpChannelBuilding");
        let config = self.exporter_config;
        let endpoint = Self::resolve_endpoint(OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT, config.endpoint);
        let endpoint = endpoint
            .to_socket_addrs()
            .expect("endpoint should convert to at least one socket address")
            .next()
            .expect("endpoint should be syntactically correct socket address");
        let retry_policy = self.capnp_config.retry_policy.clone();
        let client = CapnpTracesClient::new(endpoint, retry_policy);

        Ok(crate::SpanExporter::from_capnp(client))
    }

    fn resolve_endpoint(default_endpoint_var: &str, provided_endpoint: Option<String>) -> String {
        // resolving endpoint string
        // grpc doesn't have a "path" like http(See https://github.com/grpc/grpc/blob/master/doc/PROTOCOL-HTTP2.md)
        // the path of grpc calls are based on the protobuf service definition
        // so we won't append one for default grpc endpoints
        // If users for some reason want to use a custom path, they can use env var or builder to pass it
        //
        // programmatic configuration overrides any value set via environment variables
        if let Some(endpoint) = provided_endpoint.filter(|s| !s.is_empty()) {
            endpoint
        } else if let Ok(endpoint) = env::var(default_endpoint_var) {
            endpoint
        } else if let Ok(endpoint) = env::var(OTEL_EXPORTER_CAPNP_ENDPOINT) {
            endpoint
        } else {
            OTEL_EXPORTER_CAPNP_ENDPOINT_DEFAULT.to_string()
        }
    }
}

pub async fn connect_with_retry(
    addr: &SocketAddr,
    timeout_ms: u64,
) -> io::Result<tokio::net::TcpStream> {
    let mut delay = Duration::from_millis(1);
    loop {
        tokio::time::sleep(delay).await;

        match tokio::net::TcpStream::connect(&addr).await {
            Ok(stream) => {
                return io::Result::Ok(stream);
            }
            Err(e) => {
                let _ = writeln!(
                    io::stdout(),
                    "Connection attempt failed for SpanExporter: {e}"
                );
                if delay > Duration::from_millis(timeout_ms) {
                    return Err(io::Error::new(
                        ErrorKind::TimedOut,
                        "Connection retry timeout exceeded",
                    ));
                }
                delay *= 2;
            }
        }
    }
}
