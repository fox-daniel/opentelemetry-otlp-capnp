#[test]
fn verify_capnp_schemas_compile() {
    // This test verifies that:
    // 1. The capnp schemas exist and are valid
    // 2. The generated Rust code compiles
    // 3. The expected modules are accessible

    use opentelemetry_capnp::capnp::capnp_rpc::{
        common_capnp, resource_capnp, trace_capnp, trace_service_capnp,
    };

    // If these type references compile, schema generation succeeded
    let _span = std::any::type_name::<trace_capnp::span::Owned>();
    let _service = std::any::type_name::<trace_service_capnp::trace_service::Client>();
    let _resource = std::any::type_name::<resource_capnp::resource::Owned>();
    let _common = std::any::type_name::<common_capnp::key_value::Owned>();
}
