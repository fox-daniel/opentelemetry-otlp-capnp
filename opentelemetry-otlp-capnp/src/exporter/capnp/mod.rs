pub(crate) mod trace;

use super::ExporterBuildError;
use crate::retry::RetryPolicy;
use crate::{ExportConfig, OTEL_EXPORTER_CAPNP_ENDPOINT};
use opentelemetry::otel_debug;
/// Configuration for [capnp]
///
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct CapnpConfig {
    /// Custom metadata entries to send to the collector.
    // pub(crate) metadata: Option<MetadataMap>,
    /// TLS settings for the collector endpoint.
    #[cfg(feature = "tls")]
    pub(crate) tls_config: Option<ClientTlsConfig>,
    // The compression algorithm to use when communicating with the collector.
    // pub(crate) compression: Option<Compression>,
    // pub(crate) channel: Option<tonic::transport::Channel>,
    // pub(crate) interceptor: Option<BoxInterceptor>,
    // The retry policy to use for gRPC requests.
    // #[cfg(feature = "experimental-grpc-retry")]
    // pub(crate) retry_policy: Option<RetryPolicy>,
}

#[derive(Debug, Default)]
pub struct CapnpExporterBuilder {
    pub(crate) capnp_config: CapnpConfig,
    pub(crate) exporter_config: ExportConfig,
}

/// Expose interface for modifying [CapnpConfig] fields within the exporter builders.
pub trait HasCapnpConfig {
    /// Return a mutable reference to the export config within the exporter builders.
    fn capnp_config(&mut self) -> &mut CapnpConfig;
}

/// Expose interface for modifying [CapnpConfig] fields within the [CapnpExporterBuilder].
impl HasCapnpConfig for CapnpExporterBuilder {
    fn capnp_config(&mut self) -> &mut CapnpConfig {
        &mut self.capnp_config
    }
}

// impl CapnpExporterBuilder {
//     /// Build a new tonic span exporter
//     pub(crate) fn build_span_exporter(self) -> Result<crate::SpanExporter, ExporterBuildError> {
//         use crate::exporter::capnp::trace::CapnpTracesClient;

//         otel_debug!(name: "TracesCapnpChannelBuilding");

//         // let (channel, interceptor, retry_policy) = self.build_channel(
//         //     crate::span::OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT,
//         //     crate::span::OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT,
//         //     // crate::span::OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION,
//         //     crate::span::OTEL_EXPORTER_CAPNP_TRACES_HEADERS,
//         // )?;

//         let client = CapnpTracesClient::new(channel, retry_policy);

//         Ok(crate::SpanExporter::from_capnp(client))
//     }
// }
