//! Protobuf messages for the app-to-helper layer.
//!
//! The types come from `transport.proto`, which is converted with prost during
//! the build. The rich machine commands (NativeModel.*) are still a separate
//! schema in `proto/reference/` and will be added later.

pub mod bridge {
    include!(concat!(env!("OUT_DIR"), "/cricut.bridge.rs"));
}

pub mod commands;

pub use bridge::{device_message, Device, DeviceHandle, DeviceMessage};
pub use commands::Opcode;
