pub(crate) mod trace;

use super::ExporterBuildError;
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

impl CapnpExporterBuilder {
    // This is for clippy to work with only the grpc-tonic feature enabled
    #[allow(unused)]
    fn build_channel(
        self,
        signal_endpoint_var: &str,
        signal_timeout_var: &str,
        signal_compression_var: &str,
        signal_headers_var: &str,
    ) -> Result<
        (
            Channel,
            BoxInterceptor,
            Option<CompressionEncoding>,
            Option<RetryPolicy>,
        ),
        ExporterBuildError,
    > {
        let compression = self.resolve_compression(signal_compression_var)?;

        let (headers_from_env, headers_for_logging) = parse_headers_from_env(signal_headers_var);
        let metadata = merge_metadata_with_headers_from_env(
            self.tonic_config.metadata.unwrap_or_default(),
            headers_from_env,
        );

        let add_metadata = move |mut req: tonic::Request<()>| {
            for key_and_value in metadata.iter() {
                match key_and_value {
                    KeyAndValueRef::Ascii(key, value) => {
                        req.metadata_mut().append(key, value.to_owned())
                    }
                    KeyAndValueRef::Binary(key, value) => {
                        req.metadata_mut().append_bin(key, value.to_owned())
                    }
                };
            }

            Ok(req)
        };

        let interceptor = match self.tonic_config.interceptor {
            Some(mut interceptor) => {
                BoxInterceptor(Box::new(move |req| interceptor.call(add_metadata(req)?)))
            }
            None => BoxInterceptor(Box::new(add_metadata)),
        };

        // Get retry policy before consuming self
        #[cfg(feature = "experimental-grpc-retry")]
        let retry_policy = self.tonic_config.retry_policy.clone();

        // If a custom channel was provided, use that channel instead of creating one
        if let Some(channel) = self.tonic_config.channel {
            return Ok((
                channel,
                interceptor,
                compression,
                #[cfg(feature = "experimental-grpc-retry")]
                retry_policy,
                #[cfg(not(feature = "experimental-grpc-retry"))]
                None,
            ));
        }

        let config = self.exporter_config;

        let endpoint = Self::resolve_endpoint(signal_endpoint_var, config.endpoint);

        // Used for logging the endpoint
        let endpoint_clone = endpoint.clone();

        let endpoint = Channel::from_shared(endpoint)
            .map_err(|op| ExporterBuildError::InvalidUri(endpoint_clone.clone(), op.to_string()))?;
        let timeout = resolve_timeout(signal_timeout_var, config.timeout.as_ref());

        #[cfg(feature = "tls")]
        let channel = match self.tonic_config.tls_config {
            Some(tls_config) => endpoint
                .tls_config(tls_config)
                .map_err(|er| ExporterBuildError::InternalFailure(er.to_string()))?,
            None => endpoint,
        }
        .timeout(timeout)
        .connect_lazy();

        #[cfg(not(feature = "tls"))]
        let channel = endpoint.timeout(timeout).connect_lazy();

        otel_debug!(name: "TonicChannelBuilt", endpoint = endpoint_clone, timeout_in_millisecs = timeout.as_millis(), compression = format!("{:?}", compression), headers = format!("{:?}", headers_for_logging));
        Ok((
            channel,
            interceptor,
            compression,
            #[cfg(feature = "experimental-grpc-retry")]
            retry_policy,
            #[cfg(not(feature = "experimental-grpc-retry"))]
            None,
        ))
    }

    /// Build a new tonic span exporter
    pub(crate) fn build_span_exporter(self) -> Result<crate::SpanExporter, ExporterBuildError> {
        use crate::exporter::capnp::trace::CapnpTracesClient;

        otel_debug!(name: "TracesCapnpChannelBuilding");

        let (channel, interceptor, compression, retry_policy) = self.build_channel(
            crate::span::OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT,
            crate::span::OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT,
            crate::span::OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION,
            crate::span::OTEL_EXPORTER_CAPNP_TRACES_HEADERS,
        )?;

        let client = CapnpTracesClient::new(channel, interceptor, compression, retry_policy);

        Ok(crate::SpanExporter::from_capnp(client))
    }
}
