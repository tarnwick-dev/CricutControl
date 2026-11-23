//! A live session with one machine: open phase, key exchange, encrypted phase.

use std::time::Duration;

use cricut_transport::DeviceTransport;

use crate::cipher::{KeyExchange, NullCipher, SessionCipher};
use crate::codec::{Frame, FrameReader};
use crate::BridgeError;

const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// A framed, optionally-encrypted session on top of a raw transport.
///
/// During the open phase the cipher is [`NullCipher`] and frames are plaintext.
/// After the key exchange, install the real cipher with [`Self::set_cipher`] and
/// encrypted command bodies are run through it automatically.
pub struct MachineSession<T: DeviceTransport> {
    transport: T,
    reader: FrameReader,
    cipher: Box<dyn SessionCipher + Send>,
}

impl<T: DeviceTransport> MachineSession<T> {
    pub fn new(transport: T) -> Self {
        Self { transport, reader: FrameReader::new(), cipher: Box::new(NullCipher) }
    }

    /// Install the session cipher (after the key exchange).
    pub fn set_cipher(&mut self, cipher: Box<dyn SessionCipher + Send>) {
        self.cipher = cipher;
    }

    /// Send a command body. If `encrypted`, the body is run through the cipher
    /// and the encrypted-flag is set on the frame.
    pub fn send(&mut self, body: &[u8], encrypted: bool) -> Result<(), BridgeError> {
        let frame = if encrypted {
            Frame::encrypted(self.cipher.encrypt_body(body)?)
        } else {
            Frame::plain(body.to_vec())
        };
        let bytes = frame.encode()?;
        let mut off = 0;
        while off < bytes.len() {
            off += self.transport.write(&bytes[off..])?;
        }
        Ok(())
    }

    /// Receive the next frame. An encrypted frame's body is decrypted before
    /// return; the returned bool says whether it arrived encrypted.
    pub fn recv(&mut self) -> Result<(Vec<u8>, bool), BridgeError> {
        loop {
            if let Some(frame) = self.reader.next_frame()? {
                return if frame.encrypted {
                    Ok((self.cipher.decrypt_body(&frame.body)?, true))
                } else {
                    Ok((frame.body, false))
                };
            }
            let mut buf = [0u8; 512];
            let n = self.transport.read(&mut buf, READ_TIMEOUT)?;
            if n == 0 {
                return Err(BridgeError::Protocol("connection closed mid-frame".into()));
            }
            self.reader.feed(&buf[..n]);
        }
    }

    /// Run the key exchange: read the machine's key-exchange frame, derive the
    /// session key, and install the cipher. `make_cipher` turns the derived key
    /// into the concrete [`SessionCipher`] (the developer's JoyXtraCipher).
    pub fn establish<K, F>(&mut self, exchange: &mut K, make_cipher: F) -> Result<(), BridgeError>
    where
        K: KeyExchange,
        F: FnOnce(crate::cipher::SessionKey) -> Box<dyn SessionCipher + Send>,
    {
        let (body, _) = self.recv()?; // the plaintext key-exchange body (the 48-byte frame)
        let key = exchange.derive_session_key(&body)?;
        self.cipher = make_cipher(key);
        Ok(())
    }
}
