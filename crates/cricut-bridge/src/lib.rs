//! Session and command logic on top of a raw device transport.
//!
//! Layers:
//! - [`codec`]   — the 2-byte SPP frame header (encrypted flag + length)
//! - [`cipher`]  — [`SessionCipher`], [`JoyXtraCipher`], and the key exchange
//! - [`session`] — [`MachineSession`]: open phase, key exchange, encrypted phase
//!
//! [`cipher::NullCipher`] is the plaintext open phase. [`cipher::JoyXtraCipher`]
//! is the encrypted phase. Key material is loaded at runtime and is not in this
//! repository.

pub mod cipher;
pub mod codec;
pub mod session;

pub use cipher::{
    load_seed_file, JoyXtraCipher, JoyXtraKeyExchange, KeyExchange, KeyExchangeInputs, NullCipher,
    SessionCipher, SessionKey,
};
pub use codec::{Frame, FrameReader};
pub use session::MachineSession;

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error(transparent)]
    Transport(#[from] cricut_transport::TransportError),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("not implemented yet: {0}")]
    Unimplemented(&'static str),
}
