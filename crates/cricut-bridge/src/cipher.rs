//! The session cipher: the one slot the cipher developer fills in.
//!
//! Everything else in this crate (framing, the open phase, the session) is
//! complete. To drive a machine's encrypted command stream, provide a type that
//! implements [`SessionCipher`] and a [`KeyExchange`] that establishes the
//! session key from the handshake.
//!
//! The concrete Joy Xtra cipher (the algorithm, the plugin addresses and a
//! reference implementation) is documented in the private crypto pack
//! (`cricut-joyxtra-crypto/`), which is not part of this repository.
//!
//! Hard rule (from that pack's CLAUDE.md, and the project's legal note):
//! **no key bytes, no 33-byte unwrap seed, and no materials string belong in
//! this repository.** The session key and any derivation inputs are loaded at
//! runtime from outside version control (see [`SessionKey::load_from_file`]).

use crate::BridgeError;

/// A 32-byte session key, loaded at runtime. Never hard-coded, never committed.
#[derive(Clone)]
pub struct SessionKey(pub [u8; 32]);

impl SessionKey {
    /// Load a raw 32-byte key, or a 64-char hex line, from a file outside the repo.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, BridgeError> {
        let raw = std::fs::read(path).map_err(|e| BridgeError::Protocol(format!("key file: {e}")))?;
        let key = if raw.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&raw);
            k
        } else {
            let text = String::from_utf8_lossy(&raw);
            let hex: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if hex.len() != 64 {
                return Err(BridgeError::Protocol("key must be 32 bytes or 64 hex chars".into()));
            }
            let mut k = [0u8; 32];
            for i in 0..32 {
                k[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                    .map_err(|_| BridgeError::Protocol("bad hex in key".into()))?;
            }
            k
        };
        Ok(SessionKey(key))
    }
}

impl std::fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // never print key bytes
        write!(f, "SessionKey(<32 bytes, redacted>)")
    }
}

/// Encrypt/decrypt one SPP message BODY (the bytes after the 2-byte frame
/// header). Called only for frames whose header had the encrypted bit set.
pub trait SessionCipher {
    /// Encrypt a plaintext command body for sending to the machine.
    fn encrypt_body(&mut self, plain: &[u8]) -> Result<Vec<u8>, BridgeError>;
    /// Decrypt a command/response body received from the machine.
    fn decrypt_body(&mut self, cipher: &[u8]) -> Result<Vec<u8>, BridgeError>;
}

/// Establish the per-connection session key from the handshake. The host
/// decrypts the machine's key-exchange body and installs the session key.
pub trait KeyExchange {
    /// Given the machine's key-exchange body (the plaintext 48-byte frame),
    /// derive and return the session key.
    fn derive_session_key(&mut self, exchange_body: &[u8]) -> Result<SessionKey, BridgeError>;
}

/// No encryption: for the plaintext open phase and for replay/testing.
pub struct NullCipher;
impl SessionCipher for NullCipher {
    fn encrypt_body(&mut self, plain: &[u8]) -> Result<Vec<u8>, BridgeError> {
        Ok(plain.to_vec())
    }
    fn decrypt_body(&mut self, cipher: &[u8]) -> Result<Vec<u8>, BridgeError> {
        Ok(cipher.to_vec())
    }
}

/// ============================================================================
/// DEVELOPER: implement the Joy Xtra cipher here.
/// ============================================================================
///
/// Spec and reference: the private crypto pack (`cricut-joyxtra-crypto`):
///   - `docs/session-cipher.md` + `CLAUDE.md`  — the exact algorithm and addresses
///   - `tools/session_cipher.py`               — a working reference (encrypt,
///                                                decrypt, exchange)
///   - `captures/joyxtra2.pcapng` + `decoded/joyxtra2-first.txt` — test vectors
///
/// Fill in the two `SessionCipher` methods and `derive_session_key` on
/// `JoyXtraKeyExchange`, following the algorithm in the private crypto pack. Any
/// key material (keys, seed, materials) stays OUTSIDE this repo and is passed in
/// at construction, never hard-coded.
///
/// Verify: with a real session key, `decrypt_body` on the encrypted frames of
/// the capture must reproduce the decoded reference in the crypto pack.
pub struct JoyXtraCipher {
    pub key: SessionKey,
}

impl JoyXtraCipher {
    pub fn new(key: SessionKey) -> Self {
        Self { key }
    }
}

impl SessionCipher for JoyXtraCipher {
    fn encrypt_body(&mut self, _plain: &[u8]) -> Result<Vec<u8>, BridgeError> {
        let _ = &self.key;
        Err(BridgeError::Unimplemented("JoyXtraCipher::encrypt_body (developer)"))
    }
    fn decrypt_body(&mut self, _cipher: &[u8]) -> Result<Vec<u8>, BridgeError> {
        Err(BridgeError::Unimplemented("JoyXtraCipher::decrypt_body (developer)"))
    }
}

/// Builds the exchange key from the materials record and derives the session key.
/// The materials text, firmware version, serial and the unwrap seed are supplied
/// at construction from outside the repo; nothing secret is stored here.
pub struct JoyXtraKeyExchange {
    /// Opaque inputs the developer needs for the derivation, loaded at runtime.
    /// (e.g. the EncryptionGet materials string, firmware version, serial, seed.)
    pub inputs: KeyExchangeInputs,
}

/// Runtime-supplied, never-committed inputs for the key derivation.
#[derive(Default)]
pub struct KeyExchangeInputs {
    pub materials_text: Option<String>,
    pub firmware_version: Option<String>,
    pub serial: Option<String>,
    pub unwrap_seed: Option<Vec<u8>>,
}

impl KeyExchange for JoyXtraKeyExchange {
    fn derive_session_key(&mut self, _exchange_body: &[u8]) -> Result<SessionKey, BridgeError> {
        let _ = &self.inputs;
        Err(BridgeError::Unimplemented("JoyXtraKeyExchange::derive_session_key (developer)"))
    }
}
