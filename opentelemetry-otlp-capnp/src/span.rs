//! # CAPNP - Span Exporter
//!
//! Defines a [SpanExporter] to send trace data via an extended
//! OpenTelemetry Protocol using Cap'n Proto.

use crate::exporter::capnp::trace::CapnpTracesClient;
use crate::{
    exporter::capnp::{CapnpExporterBuilder, HasCapnpConfig},
    CapnpExporterBuilderSet,
};
use crate::{exporter::HasExportConfig, ExporterBuildError, NoExporterBuilderSet};
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::SpanData;
use std::fmt::Debug;

pub const OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT: &str = "OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT";
/// Max waiting time for the backend to process each spans batch, defaults to 10s.
pub const OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT: &str = "OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT";

/// Target to which the exporter is going to send spans, defaults to https://localhost:4317/v1/traces.
/// Learn about the relationship between this constant and default/metrics/logs at
/// <https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/protocol/exporter.md#endpoint-urls-for-otlphttp>
/// Cap'n Proto exporter builder
#[derive(Debug, Default, Clone)]
pub struct SpanExporterBuilder<C> {
    client: C,
}

impl SpanExporterBuilder<NoExporterBuilderSet> {
    /// Create a new [SpanExporterBuilder] with default settings.
    pub fn new() -> Self {
        SpanExporterBuilder::default()
    }

    /// With the Cap'n Proto transport.
    pub fn with_capnp(self) -> SpanExporterBuilder<CapnpExporterBuilderSet> {
        SpanExporterBuilder {
            client: CapnpExporterBuilderSet(CapnpExporterBuilder::default()),
        }
    }
}

impl SpanExporterBuilder<CapnpExporterBuilderSet> {
    /// Build the [SpanExporter] with the Cap'n Proto transport.
    pub fn build(self) -> Result<SpanExporter, ExporterBuildError> {
        let span_exporter = self.client.0.build_span_exporter()?;
        opentelemetry::otel_debug!(name: "SpanExporterBuilt");
        Ok(span_exporter)
    }
}

impl HasExportConfig for SpanExporterBuilder<CapnpExporterBuilderSet> {
    fn export_config(&mut self) -> &mut crate::ExportConfig {
        &mut self.client.0.exporter_config
    }
}

impl HasCapnpConfig for SpanExporterBuilder<CapnpExporterBuilderSet> {
    fn capnp_config(&mut self) -> &mut crate::CapnpConfig {
        &mut self.client.0.capnp_config
    }
}

/// CAPNP exporter that sends tracing data
///
/// Forwards SpanData over a tokio channel to the thread dedicated to
/// a Cap'n Proto client for further export.
///
/// The internals do not parallel opentelemetry-otlp using Prost and Tonic.
/// The change is required for Cap'n Proto because the Cap'n Proto SpanExporter
/// is not Send. The Cap'n Proto RPC client used to export SpanData
/// is placed on a dedicated thread and all SpanData is sent to it
#[derive(Debug)]
pub struct SpanExporter {
    client: SupportedTransportClient,
}

#[derive(Debug)]
enum SupportedTransportClient {
    Capnp(CapnpTracesClient),
}

impl SpanExporter {
    /// Obtain a builder to configure a [SpanExporter].
    pub fn builder() -> SpanExporterBuilder<NoExporterBuilderSet> {
        SpanExporterBuilder::default()
    }

    pub(crate) fn from_capnp(client: CapnpTracesClient) -> Self {
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
