use crate::retry::RetryPolicy;
use core::fmt;
// use optentelemetry_capnp::collector::trace::v1::trace_service_client::TraceServiceClient;
use opentelemetry_capnp::trace_capnp;
use std::sync::Arc;
use tokio::sync::Mutex;

use opentelemetry_sdk::{
    error::{OTelSdkError, OTelSdkResult},
    trace::{SpanData, SpanExporter},
};

pub(crate) struct CapnpTracesClient {
    inner: Option<ClientInner>,
    retry_policy: RetryPolicy,
    // #[allow(dead_code)]
    // <allow dead> would be removed once we support set_resource for metrics.
    // resource: opentelemetry_capnp::transform::common::capnp::ResourceAttributesWithSchema,
}

struct ClientInner {
    client: trace_service_client::TraceServiceClient,
}

impl fmt::Debug for CapnpTracesClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TonicTracesClient")
    }
}

impl SpanExporter for CapnpTracesClient {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        // let batch = Arc::new(batch);
        Err(OTelSdkError::InternalFailure(String::from(
            "need to implement export for CapnpTracesClient",
        )))
    }

    fn shutdown(&mut self) -> OTelSdkResult {
        match self.inner.take() {
            Some(_) => Ok(()), // Successfully took `inner`, indicating a successful shutdown.
            None => Err(OTelSdkError::AlreadyShutdown), // `inner` was already `None`, meaning it's already shut down.
        }
    }

    // fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
    //     self.resource = resource.into();
    // }
}

pub mod trace_service_client {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value
    )]
    use opentelemetry_capnp::trace_service_capnp;
    /// Service that can be used to push spans between one Application instrumented with
    /// OpenTelemetry and a collector, or between a collector and a central collector (in this
    /// case spans are sent/received to/from multiple Applications).
    #[derive(Debug, Clone)]
    pub struct TraceServiceClient {
        inner: trace_service_capnp::Client,
    }
}
