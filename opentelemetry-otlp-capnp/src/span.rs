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
    tx_export: tokio::sync::mpsc::UnboundedSender<Vec<SpanData>>,
    tx_shutdown: tokio::sync::mpsc::UnboundedSender<ShutDown>,
}

struct ShutDown;

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
