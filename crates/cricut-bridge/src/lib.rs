//! Machine session: the layer between a raw [`DeviceTransport`] and commands.
//!
//! We talk to the machine directly and thus take over the role of the Design
//! Space helper. Two things sit deliberately behind a trait, because they are
//! only filled in once confirmed from our own machine:
//!
//! * [`FrameCodec`] — the framing towards the machine (length header, byte
//!   order, checksum). The exact shape comes from the transport analysis.
//! * [`SessionCipher`] — the encryption of the connection. This repository
//!   contains no keys and no cryptography code taken from Design Space. The
//!   implementation and key material are supplied separately, from a capture of
//!   our own machine. See `docs/protocol.md`, chapter Encryption.

use cricut_transport::{DeviceTransport, TransportError};

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("not implemented yet: {0}")]
    Unimplemented(&'static str),
}

/// Wrapping and unwrapping of frames on the line to the machine.
/// Pending: the concrete header/length/checksum from the transport analysis.
pub trait FrameCodec {
    /// Put a payload into a frame ready to send.
    fn encode(&self, payload: &[u8], out: &mut Vec<u8>) -> Result<(), BridgeError>;
    /// Take one complete frame out of the incoming buffer, if present.
    fn decode(&self, inbuf: &mut Vec<u8>) -> Result<Option<Vec<u8>>, BridgeError>;
}

/// The encrypted layer on the connection. Implementation and keys are supplied
/// separately (see the module description). The no-op variant is for the
/// plaintext phase of the handshake and for testing against a capture.
pub trait SessionCipher {
    fn encrypt(&mut self, plain: &[u8]) -> Result<Vec<u8>, BridgeError>;
    fn decrypt(&mut self, cipher: &[u8]) -> Result<Vec<u8>, BridgeError>;
}

/// No encryption: for the open setup phase and for replay tests.
pub struct NullCipher;
impl SessionCipher for NullCipher {
    fn encrypt(&mut self, plain: &[u8]) -> Result<Vec<u8>, BridgeError> {
        Ok(plain.to_vec())
    }
    fn decrypt(&mut self, cipher: &[u8]) -> Result<Vec<u8>, BridgeError> {
        Ok(cipher.to_vec())
    }
}

/// A running session with one machine.
pub struct MachineSession<T: DeviceTransport, C: FrameCodec, S: SessionCipher> {
    transport: T,
    codec: C,
    cipher: S,
}

impl<T: DeviceTransport, C: FrameCodec, S: SessionCipher> MachineSession<T, C, S> {
    pub fn new(transport: T, codec: C, cipher: S) -> Self {
        Self { transport, codec, cipher }
    }

    /// Placeholder for startup: reading status, registers, key exchange.
    /// Filled in after the transport analysis and the crypto capture.
    pub fn handshake(&mut self) -> Result<(), BridgeError> {
        let _ = (&mut self.transport, &mut self.codec, &mut self.cipher);
        Err(BridgeError::Unimplemented("handshake"))
    }
}
