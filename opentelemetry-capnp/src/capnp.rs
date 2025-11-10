pub mod trace_capnp {
    include!(concat!(
        env!("OUT_DIR"),
        "/opentelemetry/capnp/trace/v1/trace_capnp.rs"
    ));
}

pub mod common_capnp {
    include!(concat!(
        env!("OUT_DIR"),
        "/opentelemetry/capnp/common/v1/common_capnp.rs"
    ));
}

pub mod resource_capnp {
    include!(concat!(
        env!("OUT_DIR"),
        "/opentelemetry/capnp/resource/v1/resource_capnp.rs"
    ));
}
pub use common_capnp::*;
pub use resource_capnp::*;
pub use trace_capnp::*;
