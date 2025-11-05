use crate::exporter::capnp::CapnpExporterBuilder;
use crate::Protocol;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use thiserror::Error;

/// Target to which the exporter is going to send signals, defaults to https://localhost:4317.
/// Learn about the relationship between this constant and metrics/spans/logs at
/// <https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/protocol/exporter.md#endpoint-urls-for-otlphttp>
pub const OTEL_EXPORTER_CAPNP_ENDPOINT: &str = "OTEL_EXPORTER_CAPNP_ENDPOINT";
/// Default target to which the exporter is going to send signals.
pub const OTEL_EXPORTER_CAPNP_ENDPOINT_DEFAULT: &str = OTEL_EXPORTER_CAPNP_ENDPOINT;
const OTEL_EXPORTER_CAPNP_PROTOCOL_RPC: &str = "capnp";
pub const OTEL_EXPORTER_CAPNP_PROTOCOL: &str = "OTEL_EXPORTER_CAPNP_PROTOCOL";
pub const OTEL_EXPORTER_CAPNP_PROTOCOL_DEFAULT: &str = OTEL_EXPORTER_CAPNP_PROTOCOL_RPC;

/// Max waiting time for the backend to process each signal batch, defaults to 10 seconds.
pub const OTEL_EXPORTER_CAPNP_TIMEOUT: &str = "OTEL_EXPORTER_CAPNP_TIMEOUT";
/// Default max waiting time for the backend to process each signal batch.
pub const OTEL_EXPORTER_CAPNP_TIMEOUT_DEFAULT: Duration = Duration::from_millis(10000);

pub(crate) mod capnp;
/// Configuration for the CAPNP exporter.
#[derive(Debug)]
pub struct ExportConfig {
    /// The address of the CAPNP collector.
    /// Default address will be used based on the protocol.
    ///
    /// Note: Programmatically setting this will override any value set via the environment variable.
    pub endpoint: Option<String>,

    /// The protocol to use when communicating with the collector.
    pub protocol: Protocol,

    /// The timeout to the collector.
    /// The default value is 10 seconds.
    ///
    /// Note: Programmatically setting this will override any value set via the environment variable.
    pub timeout: Option<Duration>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        let protocol = default_protocol();

        Self {
            endpoint: None,
            // don't use default_endpoint(protocol) here otherwise we
            // won't know if user provided a value
            protocol,
            timeout: None,
        }
    }
}

#[derive(Error, Debug)]
/// Errors that can occur while building an exporter.
// TODO: Refine and polish this.
// Non-exhaustive to allow for future expansion without breaking changes.
// This could be refined after polishing and finalizing the errors.
#[non_exhaustive]
pub enum ExporterBuildError {
    /// Spawning a new thread failed.
    #[error("Spawning a new thread failed. Unable to create Reqwest-Blocking client.")]
    ThreadSpawnFailed,

    /// Feature required to use the specified compression algorithm.
    // #[cfg(any(not(feature = "gzip-tonic"), not(feature = "zstd-tonic")))]
    // #[error("feature '{0}' is required to use the compression algorithm '{1}'")]
    // FeatureRequiredForCompressionAlgorithm(&'static str, Compression),

    /// No Http client specified.
    #[error("no http client specified")]
    NoHttpClient,

    /// Unsupported compression algorithm.
    #[error("unsupported compression algorithm '{0}'")]
    UnsupportedCompressionAlgorithm(String),

    /// Invalid URI.
    // #[cfg(any(feature = "rpc-capnp", feature = "http-proto", feature = "http-json"))]
    // #[error("invalid URI {0}. Reason {1}")]
    // InvalidUri(String, String),

    /// Failed due to an internal error.
    /// The error message is intended for logging purposes only and should not
    /// be used to make programmatic decisions. It is implementation-specific
    /// and subject to change without notice. Consumers of this error should not
    /// rely on its content beyond logging.
    #[error("Reason: {0}")]
    InternalFailure(String),
}

/// Provide access to the [ExportConfig] field within the exporter builders.
pub trait HasExportConfig {
    /// Return a mutable reference to the [ExportConfig] within the exporter builders.
    fn export_config(&mut self) -> &mut ExportConfig;
}

/// Provide [ExportConfig] access to the [CapnpExporterBuilder].
impl HasExportConfig for CapnpExporterBuilder {
    fn export_config(&mut self) -> &mut ExportConfig {
        &mut self.exporter_config
    }
}

/// default protocol based on enabled features
fn default_protocol() -> Protocol {
    match OTEL_EXPORTER_CAPNP_PROTOCOL_DEFAULT {
        OTEL_EXPORTER_CAPNP_PROTOCOL_CAPNP => Protocol::Capnp,
        _ => Protocol::Capnp,
    }
}

/// Expose methods to override [ExportConfig].
///
/// This trait will be implemented for every struct that implemented [`HasExportConfig`] trait.
///
/// ## Examples
/// ```
/// # #[cfg(all(feature = "trace", feature = "rpc-capnp"))]
/// # {
/// use crate::opentelemetry_otlp::WithExportConfig;
/// let exporter_builder = opentelemetry_otlp::SpanExporter::builder()
///     .with_capnp()
///     .with_endpoint("http://localhost:7201");
/// # }
/// ```
pub trait WithExportConfig {
    /// Set the address of the CAPNP collector. If not set or set to empty string, the default address is used.
    ///
    /// Note: Programmatically setting this will override any value set via the environment variable.
    fn with_endpoint<T: Into<String>>(self, endpoint: T) -> Self;
    /// Set the protocol to use when communicating with the collector.
    ///
    /// Note that protocols that are not supported by exporters will be ignored. The exporter
    /// will use default protocol in this case.
    ///
    /// ## Note
    /// All exporters in this crate only support one protocol, thus choosing the protocol is a no-op at the moment.
    fn with_protocol(self, protocol: Protocol) -> Self;
    /// Set the timeout to the collector.
    ///
    /// Note: Programmatically setting this will override any value set via the environment variable.
    fn with_timeout(self, timeout: Duration) -> Self;
    /// Set export config. This will override all previous configurations.
    ///
    /// Note: Programmatically setting this will override any value set via environment variables.
    fn with_export_config(self, export_config: ExportConfig) -> Self;
}
