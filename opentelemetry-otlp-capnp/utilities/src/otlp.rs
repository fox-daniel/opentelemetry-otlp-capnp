use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_server::{TraceService, TraceServiceServer},
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use std::net::SocketAddr;
use tonic::{transport::Server, Request, Response, Status};

pub struct MinimalOtlpReceiver {
    addr: SocketAddr,
}

impl MinimalOtlpReceiver {
    pub fn new(addr: &str) -> Self {
        let addr = addr.parse().expect("Valid socket address");
        Self { addr }
    }

    pub fn start(self) -> std::io::Result<std::thread::JoinHandle<()>> {
        let addr = self.addr;
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                Server::builder()
                    .add_service(TraceServiceServer::new(self))
                    .serve(addr)
                    .await
                    .unwrap();
            });
        });
        Ok(handle)
    }
}

#[tonic::async_trait]
impl TraceService for MinimalOtlpReceiver {
    async fn export(
        &self,
        _request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        // Do absolutely no work - don't even access request data
        // This matches the minimal Cap'n Proto receiver pattern
        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}
