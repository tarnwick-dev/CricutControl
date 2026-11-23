//! Session and command logic on top of a raw device transport.
//!
//! Layers:
//! - [`codec`]   — the 2-byte SPP frame header (encrypted flag + length)
//! - [`cipher`]  — the [`SessionCipher`] trait and the developer's cipher slot
//! - [`session`] — [`MachineSession`]: open phase, key exchange, encrypted phase
//!
//! The only unfinished piece is the concrete cipher in [`cipher`]; everything
//! here works with [`cipher::NullCipher`] for the plaintext open phase.

pub mod cipher;
pub mod codec;
pub mod cut;
pub mod session;

pub use cipher::{JoyXtraCipher, JoyXtraKeyExchange, KeyExchange, NullCipher, SessionCipher, SessionKey};
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
