//! SPP frame codec: the 2-byte big-endian header.
//!
//! Every Cricut SPP message = 2-byte big-endian header + body:
//!   - top bit (0x8000) = ENCRYPTED flag
//!   - low 15 bits        = body length
//!
//! This layer is transport-agnostic and has nothing to do with the cipher: it
//! only frames bytes. Whether a body is encrypted is read from the flag; the
//! session decides whether to run it through the [`crate::SessionCipher`].

use crate::BridgeError;

pub const ENCRYPTED_FLAG: u16 = 0x8000;
pub const MAX_BODY_LEN: usize = 0x7fff;

/// One framed SPP message: its encrypted flag and its body bytes (post-header).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub encrypted: bool,
    pub body: Vec<u8>,
}

impl Frame {
    pub fn plain(body: impl Into<Vec<u8>>) -> Self {
        Frame { encrypted: false, body: body.into() }
    }
    pub fn encrypted(body: impl Into<Vec<u8>>) -> Self {
        Frame { encrypted: true, body: body.into() }
    }

    /// Serialize to bytes on the wire (header + body).
    pub fn encode(&self) -> Result<Vec<u8>, BridgeError> {
        if self.body.len() > MAX_BODY_LEN {
            return Err(BridgeError::Protocol(format!(
                "body {} exceeds max {}",
                self.body.len(),
                MAX_BODY_LEN
            )));
        }
        let mut hdr = self.body.len() as u16;
        if self.encrypted {
            hdr |= ENCRYPTED_FLAG;
        }
        let mut out = Vec::with_capacity(2 + self.body.len());
        out.extend_from_slice(&hdr.to_be_bytes());
        out.extend_from_slice(&self.body);
        Ok(out)
    }

    /// Try to parse one frame from the front of `buf`. Returns the frame and the
    /// number of bytes consumed, or `None` if `buf` does not yet hold a complete
    /// frame (need more bytes).
    pub fn decode(buf: &[u8]) -> Result<Option<(Frame, usize)>, BridgeError> {
        if buf.len() < 2 {
            return Ok(None);
        }
        let hdr = u16::from_be_bytes([buf[0], buf[1]]);
        let encrypted = hdr & ENCRYPTED_FLAG != 0;
        let len = (hdr & 0x7fff) as usize;
        if buf.len() < 2 + len {
            return Ok(None);
        }
        let body = buf[2..2 + len].to_vec();
        Ok(Some((Frame { encrypted, body }, 2 + len)))
    }
}

/// A streaming reassembler: feed bytes, pull complete frames.
#[derive(Default)]
pub struct FrameReader {
    buf: Vec<u8>,
}

impl FrameReader {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }
    /// Pull the next complete frame, if one is buffered.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, BridgeError> {
        match Frame::decode(&self.buf)? {
            Some((frame, used)) => {
                self.buf.drain(..used);
                Ok(Some(frame))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_plain_and_encrypted() {
        let p = Frame::plain(vec![0xcf, 0x10, 0xda]);
        let e = Frame::encrypted(vec![0x59, 0x77, 0xcc, 0xfa]);
        for f in [p, e] {
            let bytes = f.encode().unwrap();
            let (back, used) = Frame::decode(&bytes).unwrap().unwrap();
            assert_eq!(used, bytes.len());
            assert_eq!(back, f);
        }
    }

    #[test]
    fn header_bits_match_capture() {
        // from the capture: 0x0010 = plaintext len 16; 0x8010 = encrypted len 16
        let mut plain_bytes = vec![0x00u8, 0x10];
        plain_bytes.extend_from_slice(&[0u8; 16]);
        let (plain, _) = Frame::decode(&plain_bytes).unwrap().unwrap();
        assert!(!plain.encrypted);
        assert_eq!(plain.body.len(), 16);

        let mut enc_bytes = vec![0x80u8, 0x10];
        enc_bytes.extend_from_slice(&[0u8; 16]);
        let (enc, _) = Frame::decode(&enc_bytes).unwrap().unwrap();
        assert!(enc.encrypted);
        assert_eq!(enc.body.len(), 16);
    }

    #[test]
    fn reader_handles_partial_and_multiple() {
        let mut r = FrameReader::new();
        let f1 = Frame::plain(vec![1, 2, 3]).encode().unwrap();
        let f2 = Frame::encrypted(vec![4, 5]).encode().unwrap();
        r.feed(&f1[..1]); // partial header
        assert!(r.next_frame().unwrap().is_none());
        r.feed(&f1[1..]);
        r.feed(&f2);
        assert_eq!(r.next_frame().unwrap().unwrap().body, vec![1, 2, 3]);
        assert_eq!(r.next_frame().unwrap().unwrap().body, vec![4, 5]);
        assert!(r.next_frame().unwrap().is_none());
    }
}
