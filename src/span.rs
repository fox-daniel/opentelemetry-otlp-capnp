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

/// CAPNP exporter that sends tracing data
#[derive(Debug)]
pub struct SpanExporter {
    client: SuppoertedTransportClient,
}

#[derive(Debug)]
enum SupportedTransportClient {
    Capnp(crate::exporter::capnp::trace::CapnpTracesClient),
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
            SupportedTransportClient::Tonic(client) => client.export(batch).await,
        }
    }

    fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
        match &mut self.client {
            SupportedTransportClient::Capnp(client) => client.set_resource(resource),
        }
    }
}
