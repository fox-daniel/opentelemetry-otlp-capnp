fn main() {
    capnpc::CompilerCommand::new()
        .file("../schema/opentelemetry/capnp/trace/v1/trace.capnp")
        .run()
        .expect("schema should compile");
}
