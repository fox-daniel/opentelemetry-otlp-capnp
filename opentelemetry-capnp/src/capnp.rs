pub mod trace_capnp {
    include!(concat!(env!("OUT_DIR"), "/trace_capnp.rs"))    
}

pub use trace_capnp::*;
