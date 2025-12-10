use std::io;
use std::io::{ErrorKind, Write};
use std::net::SocketAddr;
use std::time::Duration;
pub(crate) mod trace;

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
    // pub(crate) retry_policy: Option<RetryPolicy>,
}

// #[derive(Debug, Default)]
// pub struct CapnpExporterBuilder {
//     pub(crate) capnp_config: CapnpConfig,
//     pub(crate) exporter_config: ExportConfig,
// }

// Expose interface for modifying [CapnpConfig] fields within the exporter builders.
// pub trait HasCapnpConfig {
//     /// Return a mutable reference to the export config within the exporter builders.
//     fn capnp_config(&mut self) -> &mut CapnpConfig;
// }

// Expose interface for modifying [CapnpConfig] fields within the [CapnpExporterBuilder].
// impl HasCapnpConfig for CapnpExporterBuilder {
//     fn capnp_config(&mut self) -> &mut CapnpConfig {
//         &mut self.capnp_config
//     }
// }

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
