mod exporter;
pub mod retry;
mod span;
pub use crate::exporter::capnp::{CapnpConfig, CapnpExporterBuilder};
pub use crate::exporter::ExporterBuildError;
pub use crate::span::{
    SpanExporter, OTEL_EXPORTER_CAPNP_TRACES_ENDPOINT, OTEL_EXPORTER_CAPNP_TRACES_TIMEOUT,
};
pub use exporter::ExportConfig;

pub struct ShutDown;

/// The communication protocol to use when exporting data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    /// Capnp protocol
    Capnp,
}

pub use crate::exporter::{
    HasExportConfig, WithExportConfig, OTEL_EXPORTER_CAPNP_ENDPOINT,
    OTEL_EXPORTER_CAPNP_ENDPOINT_DEFAULT, OTEL_EXPORTER_CAPNP_PROTOCOL,
    OTEL_EXPORTER_CAPNP_PROTOCOL_DEFAULT, OTEL_EXPORTER_CAPNP_TIMEOUT,
    OTEL_EXPORTER_CAPNP_TIMEOUT_DEFAULT,
};

/// Type to hold the [CapnpExporterBuilder] and indicate it has been set.
///
/// Allowing access to [CapnpExporterBuilder] specific configuration methods.
// #[cfg(feature = "rpc-capnp")]
// This is for clippy to work with only the rpc-capnp feature enabled
#[allow(unused)]
#[derive(Debug, Default)]
pub struct CapnpExporterBuilderSet(CapnpExporterBuilder);

/// Type to indicate the builder does not have a client set.
#[derive(Debug, Default, Clone)]
pub struct NoExporterBuilderSet;
