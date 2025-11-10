pub mod trace_capnp {
    include!(concat!(
        env!("OUT_DIR"),
        "/opentelemetry/capnp/trace/v1/trace_capnp.rs"
    ));
}

pub use trace_capnp::*;
