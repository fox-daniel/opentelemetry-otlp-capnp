fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("../schema")
        .file("../schema/opentelemetry/capnp/trace/v1/trace.capnp")
        .file("../schema/opentelemetry/capnp/common/v1/common.capnp")
        .file("../schema/opentelemetry/capnp/resource/v1/resource.capnp")
        .file("../schema/opentelemetry/capnp/collector/trace/v1/trace_service.capnp")
        .run()
        .expect("schema should compile");
}
