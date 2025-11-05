//! # CAPNP - Span Exporter
//!
//! Defines a [SpanExporter] to send trace data via an extended
//! OpenTelemetry Protocol using Cap'n Proto.

use std::fmt::Debug;

use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::SpanData;

use crate::ExporterBuildError;
use crate::{
    exporter::capnp::{CapnpExporterBuilder, HasCapnpConfig},
    CapnpExporterBuilderSet,
};

use crate::{exporter::HasExportConfig, NoExporterBuilderSet};

/// Target to which the exporter is going to send spans, defaults to https://localhost:4317/v1/traces.
/// Learn about the relationship between this constant and default/metrics/logs at
/// <https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/protocol/exporter.md#endpoint-urls-for-otlphttp>
pub const OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT: &str = "OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT";
/// Max waiting time for the backend to process each spans batch, defaults to 10s.
pub const OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT: &str = "OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT";
pub const OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION: &str = "OTEL_EXPORTER_CAPNP_TRACES_COMPRESSION";
pub const OTEL_EXPORTER_CAPNP_TRACES_HEADERS: &str = "OTEL_EXPORTER_CAPNP_TRACES_HEADERS";
/// CAPNP exporter that sends tracing data
#[derive(Debug)]
pub struct SpanExporter {
    client: SupportedTransportClient,
}

#[derive(Debug)]
enum SupportedTransportClient {
    Capnp(crate::exporter::capnp::trace::CapnpTracesClient),
}

/// CAPNP span exporter builder
#[derive(Debug, Default, Clone)]
pub struct SpanExporterBuilder<C> {
    client: C,
}

impl SpanExporterBuilder<NoExporterBuilderSet> {
    /// Create a new [SpanExporterBuilder] with default settings.
    pub fn new() -> Self {
        SpanExporterBuilder::default()
    }

    /// With the RCP Capnp transport.
    pub fn with_capnp(self) -> SpanExporterBuilder<CapnpExporterBuilderSet> {
        SpanExporterBuilder {
            client: CapnpExporterBuilderSet(CapnpExporterBuilder::default()),
        }
    }
}

impl SpanExporterBuilder<CapnpExporterBuilderSet> {
    /// Build the [SpanExporter] with the RPC CAPNP transport.
    pub fn build(self) -> Result<SpanExporter, ExporterBuildError> {
        let span_exporter = self.client.0.build_span_exporter()?;
        opentelemetry::otel_debug!(name: "SpanExporterBuilt");
        Ok(span_exporter)
    }
}

#[cfg(any(feature = "http-proto", feature = "http-json"))]
impl SpanExporterBuilder<HttpExporterBuilderSet> {
    /// Build the [SpanExporter] with the HTTP transport.
    pub fn build(self) -> Result<SpanExporter, ExporterBuildError> {
        let span_exporter = self.client.0.build_span_exporter()?;
        Ok(span_exporter)
    }
}

#[cfg(feature = "grpc-tonic")]
impl HasExportConfig for SpanExporterBuilder<TonicExporterBuilderSet> {
    fn export_config(&mut self) -> &mut crate::ExportConfig {
        &mut self.client.0.exporter_config
    }
}

#[cfg(any(feature = "http-proto", feature = "http-json"))]
impl HasExportConfig for SpanExporterBuilder<HttpExporterBuilderSet> {
    fn export_config(&mut self) -> &mut crate::ExportConfig {
        &mut self.client.0.exporter_config
    }
}

#[cfg(feature = "grpc-tonic")]
impl HasTonicConfig for SpanExporterBuilder<TonicExporterBuilderSet> {
    fn tonic_config(&mut self) -> &mut crate::TonicConfig {
        &mut self.client.0.tonic_config
    }
}

#[cfg(any(feature = "http-proto", feature = "http-json"))]
impl HasHttpConfig for SpanExporterBuilder<HttpExporterBuilderSet> {
    fn http_client_config(&mut self) -> &mut crate::exporter::http::HttpConfig {
        &mut self.client.0.http_config
    }
}

impl SpanExporter {
    pub fn builder() -> SpanExporterBuilder<NoExporterBuilderSet> {
        SpanExporterBuilder::default()
    }

    pub(crate) fn from_capnp(client: crate::exporter::capnp::trace::CapnpTracesClient) -> Self {
        SpanExporter {
            client: SupportedTransportClient::Capnp(client),
        }
    }
}

impl opentelemetry_sdk::trace::SpanExporter for SpanExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        match &self.client {
            SupportedTransportClient::Capnp(client) => client.export(batch).await,
        }
    }

    fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
        match &mut self.client {
            SupportedTransportClient::Capnp(client) => client.set_resource(resource),
        }
    }
}
