//! Protobuf-berichten voor de app-naar-helper-laag.
//!
//! De types komen uit `transport.proto`, dat tijdens de build met prost wordt
//! omgezet. De rijke machinecommando's (NativeModel.*) staan nog als los
//! schema in `proto/reference/` en worden later toegevoegd.

pub mod bridge {
    include!(concat!(env!("OUT_DIR"), "/cricut.bridge.rs"));
}

pub use bridge::{Device, DeviceHandle, DeviceMessage, device_message};
