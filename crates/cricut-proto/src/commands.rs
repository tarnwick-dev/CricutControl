//! Machine command opcodes — the first byte of an SPP message body.
//!
//! These are observed in the capture (`decoded/joyxtra2-first.txt`) and named
//! from the static analysis of the helper. Not exhaustive; extend as more of the
//! command stream is decoded. The rich payloads are described by the protobuf
//! schemas in `proto/reference/` (NativeModel.Commands).

/// Known command opcodes (body[0]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    /// Key-exchange reset (plaintext). Body: 0xCF + random bytes.
    ResetAes = 0xCF,
    /// New-AES key exchange (not used in the captured session).
    NewAes = 0xC7,
    /// Register read/write (plaintext open phase).
    Register = 0xA0,
    /// Poll machine status.
    GetMachineStatus = 0x60,
    /// Read the machine interface state.
    GetInterfaceState = 0x73,
    /// Set machine state.
    SetMachineState = 0x30,
    /// Vector motion (4D).
    Move4D = 0xF1,
    /// Draw/cut vector, float coordinates.
    DrawVectorFloat = 0x42,
    /// Load the mat.
    LoadMat = 0x84,
}

impl Opcode {
    pub fn from_body(body: &[u8]) -> Option<Opcode> {
        let b = *body.first()?;
        use Opcode::*;
        Some(match b {
            0xCF => ResetAes,
            0xC7 => NewAes,
            0xA0 => Register,
            0x60 => GetMachineStatus,
            0x73 => GetInterfaceState,
            0x30 => SetMachineState,
            0xF1 => Move4D,
            0x42 => DrawVectorFloat,
            0x84 => LoadMat,
            _ => return None,
        })
    }
}
