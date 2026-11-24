//! Joy Xtra session cipher.
//!
//! Command bodies are AES-256-ECB with a one-byte checksum. The session key is
//! derived from the machine's 48-byte key-exchange body and from material that
//! is loaded at runtime. No key bytes, unwrap seed, or materials string belong
//! in this repository. See `SessionKey::load_from_file` and [`KeyExchangeInputs`].

use crate::BridgeError;

/// A 32-byte session key, loaded at runtime. Never hard-coded, never committed.
#[derive(Clone)]
pub struct SessionKey(pub [u8; 32]);

impl SessionKey {
    /// Load a raw 32-byte key, or a 64-char hex line, from a file outside the repo.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, BridgeError> {
        let raw =
            std::fs::read(path).map_err(|e| BridgeError::Protocol(format!("key file: {e}")))?;
        let key = parse_fixed_bytes::<32>(&raw, "key")?;
        Ok(SessionKey(key))
    }
}

impl std::fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionKey(<32 bytes, redacted>)")
    }
}

/// Load the 33-byte unwrap seed from a file outside the repo.
///
/// Raw 33 bytes, or 66 hex digits. The seed is not stored in this repository.
pub fn load_seed_file(path: &std::path::Path) -> Result<Vec<u8>, BridgeError> {
    let raw = std::fs::read(path).map_err(|e| BridgeError::Protocol(format!("seed file: {e}")))?;
    Ok(parse_fixed_bytes::<33>(&raw, "seed")?.to_vec())
}

fn parse_fixed_bytes<const N: usize>(raw: &[u8], what: &str) -> Result<[u8; N], BridgeError> {
    if raw.len() == N {
        let mut out = [0u8; N];
        out.copy_from_slice(raw);
        return Ok(out);
    }
    let text = String::from_utf8_lossy(raw);
    let hex: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != N * 2 {
        return Err(BridgeError::Protocol(format!(
            "{what} must be {N} bytes or {} hex chars",
            N * 2
        )));
    }
    let mut out = [0u8; N];
    for i in 0..N {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| BridgeError::Protocol(format!("bad hex in {what}")))?;
    }
    Ok(out)
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

const BLOCK: usize = 16;

/// AES-256-ECB session cipher. ECB has no chaining; each call owns the key the
/// way the device object owns its heap key.
pub struct JoyXtraCipher {
    pub key: SessionKey,
}

impl JoyXtraCipher {
    pub fn new(key: SessionKey) -> Self {
        Self { key }
    }
}

impl SessionCipher for JoyXtraCipher {
    fn encrypt_body(&mut self, plain: &[u8]) -> Result<Vec<u8>, BridgeError> {
        let sealed = seal(plain);
        aes256_ecb(&self.key.0, &sealed, true)
    }

    fn decrypt_body(&mut self, cipher: &[u8]) -> Result<Vec<u8>, BridgeError> {
        let raw = aes256_ecb(&self.key.0, cipher, false)?;
        // The old decrypt path drops trailing 0x00 before the checksum check.
        // A checksum of 0 is therefore removed along with the padding. Real
        // command frames in the Joy Xtra capture have a non-zero checksum, so
        // the plaintext is left intact.
        let mut plain = strip_trailing_zeros(&raw);
        if !checksum_ok(&plain) {
            return Err(BridgeError::Protocol("checksum mismatch".into()));
        }
        // Machine responses arrive with the length prefix swapped. Host
        // commands already start with 0x00, so this leaves them alone.
        swap_response_prefix(&mut plain);
        Ok(plain)
    }
}

/// Zero-pad out to a multiple of 16, add one extra block, and store the 8-bit
/// sum of every preceding byte in the last byte.
fn seal(plain: &[u8]) -> Vec<u8> {
    let n = plain.len() & !0x0fusize;
    let mut buf = vec![0u8; n + BLOCK];
    buf[..plain.len()].copy_from_slice(plain);
    let last = buf.len() - 1;
    let mut acc = 0u8;
    for b in &buf[..last] {
        acc = acc.wrapping_add(*b);
    }
    buf[last] = acc;
    buf
}

fn strip_trailing_zeros(raw: &[u8]) -> Vec<u8> {
    if raw.is_empty() || raw[raw.len() - 1] != 0 {
        return raw.to_vec();
    }
    let mut i = raw.len();
    while i > 0 && raw[i - 1] == 0 {
        i -= 1;
    }
    raw[..i].to_vec()
}

fn checksum_ok(body: &[u8]) -> bool {
    if body.is_empty() {
        return false;
    }
    let sum = body[..body.len() - 1]
        .iter()
        .fold(0u8, |acc, b| acc.wrapping_add(*b));
    sum == body[body.len() - 1]
}

fn swap_response_prefix(body: &mut [u8]) {
    if body.len() > 1 && body[0] != 0 {
        body.swap(0, 1);
    }
}

fn aes256_ecb(key: &[u8; 32], input: &[u8], encrypt: bool) -> Result<Vec<u8>, BridgeError> {
    use aes::cipher::generic_array::GenericArray;
    use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
    use aes::Aes256;

    if input.is_empty() || !input.len().is_multiple_of(BLOCK) {
        return Err(BridgeError::Protocol(
            "block length must be a positive multiple of 16".into(),
        ));
    }
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut out = input.to_vec();
    for chunk in out.chunks_mut(BLOCK) {
        let block = GenericArray::from_mut_slice(chunk);
        if encrypt {
            cipher.encrypt_block(block);
        } else {
            cipher.decrypt_block(block);
        }
    }
    Ok(out)
}

/// Builds the exchange key from the materials record and derives the session key.
/// The materials text, firmware version, serial and the unwrap seed are supplied
/// at construction from outside the repo; nothing secret is stored here.
pub struct JoyXtraKeyExchange {
    pub inputs: KeyExchangeInputs,
}

/// Runtime-supplied, never-committed inputs for the key derivation.
#[derive(Default)]
pub struct KeyExchangeInputs {
    /// Exact materials text. A trailing newline changes the unwrap window.
    pub materials_text: Option<String>,
    /// Firmware as a decimal number, for example `8.618`.
    pub firmware_version: Option<String>,
    /// Machine serial. Used only when the matching record's machine id is 5.
    pub serial: Option<String>,
    /// 33-byte unwrap seed.
    pub unwrap_seed: Option<Vec<u8>>,
    /// Device type compared with field 43. `None` means Joy Xtra (`0x13`).
    pub machine_type: Option<u32>,
}

impl KeyExchange for JoyXtraKeyExchange {
    fn derive_session_key(&mut self, exchange_body: &[u8]) -> Result<SessionKey, BridgeError> {
        let text = self
            .inputs
            .materials_text
            .as_deref()
            .ok_or_else(|| BridgeError::Protocol("materials text is required".into()))?;
        let seed = self
            .inputs
            .unwrap_seed
            .as_deref()
            .ok_or_else(|| BridgeError::Protocol("unwrap seed is required".into()))?;
        let firmware_text = self
            .inputs
            .firmware_version
            .as_deref()
            .ok_or_else(|| BridgeError::Protocol("firmware version is required".into()))?;
        let firmware: f64 = firmware_text.trim().parse().map_err(|_| {
            BridgeError::Protocol(format!("firmware version is not a number: {firmware_text}"))
        })?;
        let serial = self.inputs.serial.as_deref().unwrap_or("").as_bytes();
        let machine_type = u64::from(self.inputs.machine_type.unwrap_or(0x13));
        let settings = unwrap_materials(text, seed)?;
        let records = custom_material_records(&settings)?;
        let wrap = wrap_key_for_device(&records, machine_type, firmware, serial)?;
        session_key_from_exchange(&wrap, exchange_body)
    }
}

fn session_key_from_exchange(
    wrap_key: &[u8; 32],
    exchange: &[u8],
) -> Result<SessionKey, BridgeError> {
    if exchange.len() != 0x30 {
        return Err(BridgeError::Protocol(format!(
            "exchange body must be 48 bytes, got {}",
            exchange.len()
        )));
    }
    // The offset is read from the raw decrypted buffer. Trailing zeros stay,
    // so a zero inside the 32-byte key stays.
    let plain = aes256_ecb(wrap_key, exchange, false)?;
    let off = plain[0] as usize;
    if off + 32 > plain.len() {
        return Err(BridgeError::Protocol(format!(
            "session key offset {off} exceeds plaintext ({})",
            plain.len()
        )));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&plain[off..off + 32]);
    Ok(SessionKey(key))
}

/// Keep even or odd indexes. `reverse` walks from the last byte toward the first.
fn select_key_bytes(material: &[u8], even: bool, reverse: bool) -> Vec<u8> {
    let want = if even { 0 } else { 1 };
    let indexes: Vec<usize> = if reverse {
        (0..material.len()).rev().collect()
    } else {
        (0..material.len()).collect()
    };
    indexes
        .into_iter()
        .filter(|i| i & 1 == want)
        .map(|i| material[i])
        .collect()
}

fn splice_serial(key: &mut [u8], serial: &[u8]) {
    if key.len() < 0x20 || serial.len() < 12 {
        return;
    }
    for (dst, src) in [(2, 0), (5, 2), (7, 5), (10, 9), (13, 11)] {
        key[dst] = serial[src];
    }
}

fn unwrap_materials(text: &str, seed: &[u8]) -> Result<Vec<u8>, BridgeError> {
    if seed.len() != 33 {
        return Err(BridgeError::Protocol(format!(
            "seed must be 33 bytes, got {}",
            seed.len()
        )));
    }
    if text.len() < 0x19 {
        return Err(BridgeError::Protocol(
            "materials text shorter than 25 characters".into(),
        ));
    }
    let window_len = (text.len() - 0x19).min(0x2c);
    let window = &text[0x19..0x19 + window_len];
    let rest = format!("{}{}", &text[..0x19], &text[0x19 + window_len..]);
    let mut material = seed.to_vec();
    material.extend(b64_decode(window)?);
    let key_mat = select_key_bytes(&material, true, true);
    if key_mat.len() < 32 {
        return Err(BridgeError::Protocol(format!(
            "unwrap key is {} bytes",
            key_mat.len()
        )));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_mat[..32]);
    let cipher = b64_decode(&rest)?;
    let raw = aes256_ecb(&key, &cipher, false)?;
    Ok(raw[..payload_len(&raw)].to_vec())
}

fn payload_len(buf: &[u8]) -> usize {
    if buf.is_empty() || buf[buf.len() - 1] != 0 {
        return buf.len();
    }
    let mut i = buf.len() - 1;
    loop {
        if buf[i] != 0 {
            return i + 1;
        }
        if i == 0 {
            return buf.len();
        }
        i -= 1;
    }
}

/// `std::stoll` base 10: skip the C whitespace set, require one digit, keep a
/// leading sign, and stop at the first other character.
fn stoll(text: &str) -> Result<i64, BridgeError> {
    let rest = text.trim_start_matches([' ', '\t', '\r', '\n', '\u{0b}', '\u{0c}']);
    let bytes = rest.as_bytes();
    if bytes.is_empty() {
        return Err(BridgeError::Protocol("invalid stoll argument".into()));
    }
    let (neg, start) = match bytes[0] {
        b'+' => (false, 1),
        b'-' => (true, 1),
        b'0'..=b'9' => (false, 0),
        _ => return Err(BridgeError::Protocol("invalid stoll argument".into())),
    };
    if start >= bytes.len() || !bytes[start].is_ascii_digit() {
        return Err(BridgeError::Protocol("invalid stoll argument".into()));
    }
    let mut n: i128 = 0;
    for &b in &bytes[start..] {
        if !b.is_ascii_digit() {
            break;
        }
        n = n * 10 + i128::from(b - b'0');
        if n > i128::from(i64::MAX) + 1 {
            return Err(BridgeError::Protocol("stoll argument out of range".into()));
        }
    }
    let signed = if neg { -n } else { n };
    i64::try_from(signed).map_err(|_| BridgeError::Protocol("stoll argument out of range".into()))
}

fn version_matches(version_text: &str, firmware: f64) -> Result<bool, BridgeError> {
    let n = stoll(version_text)?;
    let bits = n as u64;
    let lo = f64::from(f32::from_le_bytes((bits as u32).to_le_bytes()));
    let hi = f64::from(f32::from_le_bytes(((bits >> 32) as u32).to_le_bytes()));
    Ok(lo < firmware && firmware < hi)
}

#[derive(Clone)]
struct MaterialRecord {
    material_b64: String,
    version_text: String,
    splice_id: u64,
    direction: u64,
    device_type: u64,
    parity: u64,
}

fn custom_material_records(settings: &[u8]) -> Result<Vec<MaterialRecord>, BridgeError> {
    let mut out = Vec::new();
    for (field, val) in proto_fields(settings)? {
        if field == 2 {
            if let ProtoVal::Bytes(buf) = val {
                out.push(record_from_message(&buf)?);
            }
        }
    }
    Ok(out)
}

fn record_from_message(buf: &[u8]) -> Result<MaterialRecord, BridgeError> {
    let mut material = String::new();
    let mut version = String::new();
    let mut splice_id = 0u64;
    let mut direction = 0u64;
    let mut device_type = 0u64;
    let mut parity = 0u64;
    for (field, val) in proto_fields(buf)? {
        match (field, val) {
            (2, ProtoVal::Bytes(b)) => material = String::from_utf8_lossy(&b).into_owned(),
            (5, ProtoVal::Bytes(b)) => version = String::from_utf8_lossy(&b).into_owned(),
            (4, ProtoVal::Varint(n)) => splice_id = n,
            (18, ProtoVal::Varint(n)) => direction = n,
            (43, ProtoVal::Varint(n)) => device_type = n,
            (50, ProtoVal::Varint(n)) => parity = n,
            _ => {}
        }
    }
    Ok(MaterialRecord {
        material_b64: material,
        version_text: version,
        splice_id,
        direction,
        device_type,
        parity,
    })
}

fn wrap_key_for_device(
    records: &[MaterialRecord],
    machine_type: u64,
    firmware: f64,
    serial: &[u8],
) -> Result<[u8; 32], BridgeError> {
    let mut chosen: Option<&MaterialRecord> = None;
    let mut primary = false;
    for rec in records {
        if rec.device_type != machine_type {
            continue;
        }
        // A machine-type match whose version text is not a number aborts.
        if !version_matches(&rec.version_text, firmware)? {
            continue;
        }
        chosen = Some(rec);
        primary = true;
        break;
    }
    if chosen.is_none() {
        chosen = records.iter().find(|rec| rec.device_type == 7);
    }
    let chosen = chosen
        .ok_or_else(|| BridgeError::Protocol("no material record for this machine".into()))?;
    let raw = b64_decode(&chosen.material_b64)?;
    let even = chosen.parity & 1 == 0;
    let reverse = chosen.direction & 1 == 0;
    let mut key = select_key_bytes(&raw, even, reverse);
    if primary && chosen.splice_id == 5 {
        splice_serial(&mut key, serial);
    }
    if key.len() < 32 {
        return Err(BridgeError::Protocol(format!(
            "wrap key is {} bytes",
            key.len()
        )));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&key[..32]);
    Ok(out)
}

enum ProtoVal {
    Varint(u64),
    Bytes(Vec<u8>),
}

fn proto_fields(buf: &[u8]) -> Result<Vec<(u64, ProtoVal)>, BridgeError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < buf.len() {
        let (tag, next) = read_varint(buf, i)?;
        i = next;
        let field = tag >> 3;
        let wire = tag & 7;
        if field == 0 {
            return Err(BridgeError::Protocol("protobuf field 0".into()));
        }
        match wire {
            0 => {
                let (val, next) = read_varint(buf, i)?;
                i = next;
                out.push((field, ProtoVal::Varint(val)));
            }
            2 => {
                let (ln, next) = read_varint(buf, i)?;
                i = next;
                let ln = usize::try_from(ln)
                    .map_err(|_| BridgeError::Protocol("protobuf length".into()))?;
                if i + ln > buf.len() {
                    return Err(BridgeError::Protocol(
                        "truncated length-delimited field".into(),
                    ));
                }
                out.push((field, ProtoVal::Bytes(buf[i..i + ln].to_vec())));
                i += ln;
            }
            5 => {
                if i + 4 > buf.len() {
                    return Err(BridgeError::Protocol("truncated fixed32".into()));
                }
                i += 4;
            }
            1 => {
                if i + 8 > buf.len() {
                    return Err(BridgeError::Protocol("truncated fixed64".into()));
                }
                i += 8;
            }
            other => {
                return Err(BridgeError::Protocol(format!(
                    "unsupported wire type {other}"
                )));
            }
        }
    }
    Ok(out)
}

fn read_varint(buf: &[u8], mut i: usize) -> Result<(u64, usize), BridgeError> {
    let mut n = 0u64;
    let mut shift = 0;
    loop {
        if i >= buf.len() {
            return Err(BridgeError::Protocol("truncated varint".into()));
        }
        let b = buf[i];
        i += 1;
        if shift < 64 {
            n |= u64::from(b & 0x7f) << shift;
        }
        if b < 0x80 {
            return Ok((n, i));
        }
        shift += 7;
        if shift > 63 {
            return Err(BridgeError::Protocol("varint too long".into()));
        }
    }
}

fn b64_decode(text: &str) -> Result<Vec<u8>, BridgeError> {
    let cleaned: String = text
        .chars()
        .filter(|c| !matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{0b}' | '\u{0c}'))
        .collect();
    let n = cleaned.len() / 4 * 4;
    if n == 0 {
        return Ok(Vec::new());
    }
    let bytes = cleaned.as_bytes();
    let mut out = Vec::with_capacity(n / 4 * 3);
    let mut i = 0;
    while i < n {
        let quad = &bytes[i..i + 4];
        i += 4;
        let (a, b, c, d) = (quad[0], quad[1], quad[2], quad[3]);
        let av = b64_val(a)?;
        let bv = b64_val(b)?;
        out.push((av << 2) | (bv >> 4));
        if c != b'=' {
            let cv = b64_val(c)?;
            out.push((bv << 4) | (cv >> 2));
            if d != b'=' {
                let dv = b64_val(d)?;
                out.push((cv << 6) | dv);
            }
        }
    }
    Ok(out)
}

fn b64_val(c: u8) -> Result<u8, BridgeError> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(BridgeError::Protocol("bad base64".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn hex_decode(text: &str) -> Vec<u8> {
        let t = text.trim();
        (0..t.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&t[i..i + 2], 16).unwrap())
            .collect()
    }

    fn b64_encode(data: &[u8]) -> String {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        let mut i = 0;
        while i + 3 <= data.len() {
            let n =
                (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8) | u32::from(data[i + 2]);
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push(T[((n >> 6) & 63) as usize] as char);
            out.push(T[(n & 63) as usize] as char);
            i += 3;
        }
        let rem = data.len() - i;
        if rem == 1 {
            let n = u32::from(data[i]) << 16;
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        } else if rem == 2 {
            let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8);
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push(T[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        out
    }

    fn enc_varint(mut n: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut b = (n & 0x7f) as u8;
            n >>= 7;
            if n != 0 {
                b |= 0x80;
                out.push(b);
            } else {
                out.push(b);
                break;
            }
        }
        out
    }

    fn field_bytes(field: u64, data: &[u8]) -> Vec<u8> {
        let mut out = enc_varint((field << 3) | 2);
        out.extend(enc_varint(data.len() as u64));
        out.extend_from_slice(data);
        out
    }

    fn field_varint(field: u64, n: u64) -> Vec<u8> {
        let mut out = enc_varint(field << 3);
        out.extend(enc_varint(n));
        out
    }

    fn version_text(lo: f32, hi: f32) -> String {
        let mut bits = [0u8; 8];
        bits[..4].copy_from_slice(&lo.to_le_bytes());
        bits[4..].copy_from_slice(&hi.to_le_bytes());
        i64::from_le_bytes(bits).to_string()
    }

    #[test]
    fn select_version_and_stoll() {
        assert_eq!(
            select_key_bytes(&[0, 1, 2, 3, 4], true, true),
            vec![4, 2, 0]
        );
        assert_eq!(select_key_bytes(&[0, 1, 2, 3, 4], false, false), vec![1, 3]);
        let version = version_text(1.0, 20.0);
        assert!(version_matches(&version, 8.618).unwrap());
        assert!(!version_matches(&version, 1.0).unwrap());
        assert!(!version_matches(&version, 20.0).unwrap());
        assert!(stoll("nope").is_err());
        assert_eq!(stoll("  +12abc").unwrap(), 12);
    }

    #[test]
    fn wrap_key_splices_serial_on_the_primary_match_only() {
        let material: Vec<u8> = (0..64).collect();
        let version = version_text(1.0, 20.0);
        let mut record = field_bytes(2, b64_encode(&material).as_bytes());
        record.extend(field_bytes(5, version.as_bytes()));
        record.extend(field_varint(4, 5));
        record.extend(field_varint(18, 1));
        record.extend(field_varint(43, 0x13));
        record.extend(field_varint(50, 0));

        let mut fallback = field_bytes(2, b64_encode(&material).as_bytes());
        fallback.extend(field_varint(4, 5));
        fallback.extend(field_varint(43, 7));
        fallback.extend(field_varint(18, 1));
        fallback.extend(field_varint(50, 0));

        let outside_version = version_text(0.0, 1.0);
        let mut outside = field_bytes(2, b64_encode(&material).as_bytes());
        outside.extend(field_bytes(5, outside_version.as_bytes()));
        outside.extend(field_varint(43, 0x13));
        outside.extend(field_varint(18, 1));
        outside.extend(field_varint(50, 0));

        let mut settings = field_bytes(2, &outside);
        settings.extend(field_bytes(2, &record));
        settings.extend(field_bytes(2, &fallback));
        let records = custom_material_records(&settings).unwrap();
        let got = wrap_key_for_device(&records, 0x13, 8.618, b"ABCDEFGHIJKL").unwrap();
        assert_eq!(got[0], 0);
        assert_eq!(got[1], 2);
        assert_eq!(got[2], b'A');
        assert_eq!(got[5], b'C');
        assert_eq!(got[7], b'F');
        assert_eq!(got[10], b'J');
        assert_eq!(got[13], b'L');

        let plain_only = wrap_key_for_device(
            &custom_material_records(&field_bytes(2, &fallback)).unwrap(),
            0x13,
            8.618,
            b"ABCDEFGHIJKL",
        )
        .unwrap();
        assert_eq!(plain_only[2], 4);
    }

    #[test]
    fn unwrap_and_exchange_match_the_reference_vectors() {
        let seed: Vec<u8> = (0..33).collect();
        let window_raw: Vec<u8> = (0..33).collect();
        let window = b64_encode(&window_raw);
        assert_eq!(window.len(), 44);
        let mut body = b"\x12\x04test".to_vec();
        body.extend(0..42u8);
        let material = {
            let mut m = seed.clone();
            m.extend_from_slice(&window_raw);
            m
        };
        let key_mat = select_key_bytes(&material, true, true);
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_mat[..32]);
        let cipher = aes256_ecb(&key, &body, true).unwrap();
        let coded = b64_encode(&cipher);
        let text = format!("{}{}{}", &coded[..0x19], window, &coded[0x19..]);
        let unwrapped = unwrap_materials(&text, &seed).unwrap();
        assert_eq!(unwrapped, body[..payload_len(&body)]);

        let mut plain = vec![0u8; 48];
        plain[0] = 4;
        plain[4..35].fill(b'K');
        let wrap = [b'W'; 32];
        let blob = aes256_ecb(&wrap, &plain, true).unwrap();
        let session = session_key_from_exchange(&wrap, &blob).unwrap();
        assert_eq!(session.0.as_slice(), &plain[4..36]);

        let mut kx = JoyXtraKeyExchange {
            inputs: KeyExchangeInputs {
                materials_text: Some(text),
                firmware_version: Some("8.618".into()),
                serial: Some("ABCDEFGHIJKL".into()),
                unwrap_seed: Some(seed),
                machine_type: Some(0x13),
            },
        };
        // The unwrap fixture is not a materials protobuf. Derivation must fail
        // closed instead of inventing a key.
        assert!(kx.derive_session_key(&blob).is_err());
    }

    #[test]
    fn derive_session_key_from_synthetic_materials() {
        let material: Vec<u8> = (0..64).collect();
        let version = version_text(1.0, 20.0);
        let mut record = field_bytes(2, b64_encode(&material).as_bytes());
        record.extend(field_bytes(5, version.as_bytes()));
        record.extend(field_varint(4, 5));
        record.extend(field_varint(18, 1));
        record.extend(field_varint(43, 0x13));
        // Bit 0 clear keeps even indexes. A non-zero last byte survives the
        // unwrap helper, which drops trailing 0x00 bytes.
        record.extend(field_varint(50, 2));
        let settings = field_bytes(2, &record);

        let seed: Vec<u8> = (0..33).collect();
        let window_raw: Vec<u8> = (0..33).collect();
        let window = b64_encode(&window_raw);
        let mut padded = settings.clone();
        // At least 48 bytes so the base64 form is longer than the 25-char prefix.
        let blocks = padded.len().div_ceil(16).max(3);
        padded.resize(blocks * 16, 0);
        let key_src = {
            let mut m = seed.clone();
            m.extend_from_slice(&window_raw);
            m
        };
        let key_mat = select_key_bytes(&key_src, true, true);
        let mut unwrap_key = [0u8; 32];
        unwrap_key.copy_from_slice(&key_mat[..32]);
        let cipher = aes256_ecb(&unwrap_key, &padded, true).unwrap();
        let coded = b64_encode(&cipher);
        let text = format!("{}{}{}", &coded[..0x19], window, &coded[0x19..]);

        let records = custom_material_records(&settings).unwrap();
        let wrap = wrap_key_for_device(&records, 0x13, 8.618, b"ABCDEFGHIJKL").unwrap();
        let mut exchange_plain = vec![0u8; 48];
        exchange_plain[0] = 8;
        exchange_plain[8..40].fill(0x5a);
        let exchange = aes256_ecb(&wrap, &exchange_plain, true).unwrap();

        let mut kx = JoyXtraKeyExchange {
            inputs: KeyExchangeInputs {
                materials_text: Some(text),
                firmware_version: Some("8.618".into()),
                serial: Some("ABCDEFGHIJKL".into()),
                unwrap_seed: Some(seed),
                machine_type: Some(0x13),
            },
        };
        let session = kx.derive_session_key(&exchange).unwrap();
        assert_eq!(session.0, [0x5a; 32]);
        assert!(!format!("{session:?}").contains("5a"));
    }

    #[test]
    fn command_round_trip_checks_the_checksum_and_swaps_responses() {
        let mut cipher = JoyXtraCipher::new(SessionKey([0x11; 32]));
        let host = [0x00, 0x04, 0x60];
        let ct = cipher.encrypt_body(&host).unwrap();
        let pt = cipher.decrypt_body(&ct).unwrap();
        assert!(pt.starts_with(&host));
        assert_eq!(listed(&pt), "000460");

        let machine = [0x09, 0x00, 0x60, 0x68, 0x10];
        let ct = cipher.encrypt_body(&machine).unwrap();
        let pt = cipher.decrypt_body(&ct).unwrap();
        assert_eq!(pt[0], 0x00);
        assert_eq!(pt[1], 0x09);
        assert_eq!(pt[2], 0x60);

        let mut flipped = ct.clone();
        flipped[0] ^= 0xff;
        assert!(cipher.decrypt_body(&flipped).is_err());
    }

    fn listed(body: &[u8]) -> String {
        let msg = &body[..body.len() - 1];
        let end = msg
            .iter()
            .rposition(|b| *b != 0)
            .map(|i| i + 1)
            .unwrap_or(0);
        hex_encode(&msg[..end])
    }

    #[test]
    fn session_key_file_round_trip_and_debug_is_redacted() {
        let dir = std::env::temp_dir();
        let raw_path = dir.join(format!("cricut-test-key-raw-{}", std::process::id()));
        let hex_path = dir.join(format!("cricut-test-key-hex-{}", std::process::id()));
        let seed_path = dir.join(format!("cricut-test-seed-{}", std::process::id()));
        let raw = [0x3c; 32];
        std::fs::write(&raw_path, raw).unwrap();
        std::fs::write(&hex_path, hex_encode(&raw)).unwrap();
        std::fs::write(&seed_path, hex_encode(&(0..33).collect::<Vec<_>>())).unwrap();
        let from_raw = SessionKey::load_from_file(&raw_path).unwrap();
        let from_hex = SessionKey::load_from_file(&hex_path).unwrap();
        let seed = load_seed_file(&seed_path).unwrap();
        let _ = std::fs::remove_file(&raw_path);
        let _ = std::fs::remove_file(&hex_path);
        let _ = std::fs::remove_file(&seed_path);
        assert_eq!(from_raw.0, raw);
        assert_eq!(from_hex.0, raw);
        assert_eq!(seed, (0..33).collect::<Vec<_>>());
        let shown = format!("{:?}", SessionKey(raw));
        assert_eq!(shown, "SessionKey(<32 bytes, redacted>)");
        assert!(!shown.contains("3c"));
    }

    fn crypto_pack() -> Option<std::path::PathBuf> {
        if let Some(p) = std::env::var_os("CRICUT_CRYPTO_PACK") {
            let p = std::path::PathBuf::from(p);
            return p.is_dir().then_some(p);
        }
        let home = std::env::var_os("HOME")?;
        let p = std::path::PathBuf::from(home).join("Projects/cricut-joyxtra-crypto");
        p.is_dir().then_some(p)
    }

    #[test]
    fn python_reference_agrees() {
        let Some(pack) = crypto_pack() else {
            eprintln!("skipping python cross-check: crypto pack not found");
            return;
        };
        let script = pack.join("tools/session_cipher.py");
        if !script.is_file() {
            eprintln!("skipping python cross-check: session_cipher.py not found");
            return;
        }
        let selftest = std::process::Command::new("python3")
            .arg(&script)
            .arg("selftest")
            .output()
            .expect("python3");
        if !selftest.status.success() {
            panic!(
                "python selftest failed: {}",
                String::from_utf8_lossy(&selftest.stderr)
            );
        }
        assert_eq!(String::from_utf8_lossy(&selftest.stdout).trim(), "ok");

        let key = [b'W'; 32];
        let samples: [&[u8]; 3] = [
            &[0x00, 0x04, 0x60],
            &[0x09, 0x00, 0x60, 0x68, 0x10],
            &[0x11; 20],
        ];
        let mut cipher = JoyXtraCipher::new(SessionKey(key));
        for plain in samples {
            let ours = cipher.encrypt_body(plain).unwrap();
            let theirs = python_cipher(&script, "encrypt", &key, plain);
            assert_eq!(ours, theirs, "encrypt diverged");
            let opened = python_cipher(&script, "decrypt", &key, &ours);
            let raw = aes256_ecb(&key, &ours, false).unwrap();
            assert_eq!(strip_trailing_zeros(&raw), opened);
        }

        let mut exchange_plain = vec![0u8; 48];
        exchange_plain[0] = 4;
        exchange_plain[4..36].fill(0x44);
        let blob = aes256_ecb(&key, &exchange_plain, true).unwrap();
        let ours = session_key_from_exchange(&key, &blob).unwrap();
        let theirs = python_cipher(&script, "exchange", &key, &blob);
        assert_eq!(ours.0.as_slice(), theirs.as_slice());
    }

    fn python_cipher(script: &std::path::Path, cmd: &str, key: &[u8], body: &[u8]) -> Vec<u8> {
        let path = std::env::temp_dir().join(format!(
            "cricut-synth-{}-{}-{}",
            std::process::id(),
            cmd,
            body.len()
        ));
        std::fs::write(&path, key).unwrap();
        let out = std::process::Command::new("python3")
            .arg(script)
            .arg(cmd)
            .arg(&path)
            .arg(hex_encode(body))
            .output()
            .expect("python3");
        let _ = std::fs::remove_file(&path);
        assert!(
            out.status.success(),
            "python {cmd} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        hex_decode(&String::from_utf8_lossy(&out.stdout))
    }

    #[test]
    fn joyxtra2_capture_matches_decoded_listing() {
        let Some(pack) = crypto_pack() else { return };
        let pcap = std::env::var_os("CRICUT_JOYXTRA_PCAP")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| pack.join("captures/joyxtra2.pcapng"));
        if !pcap.is_file() {
            return;
        }
        let frames = extract_spp_frames(&pcap);
        let encrypted: Vec<_> = frames.into_iter().filter(|(_, enc, _)| *enc).collect();
        assert_eq!(encrypted.len(), 1051, "joyxtra2 encrypted frame count");

        let Ok(key_path) = std::env::var("CRICUT_SESSION_KEY") else {
            eprintln!(
                "skipping joyxtra2 decrypt: set CRICUT_SESSION_KEY to a key file outside the repo"
            );
            return;
        };
        let key = SessionKey::load_from_file(std::path::Path::new(&key_path)).unwrap();
        let mut cipher = JoyXtraCipher::new(key);
        let mut rows = Vec::new();
        for (direction, _, body) in &encrypted {
            let pt = cipher.decrypt_body(body).expect("checksum");
            let hx = listed(&pt);
            let msg = hex_decode(&hx);
            let op = if msg.len() > 2 {
                msg[2]
            } else {
                msg.first().copied().unwrap_or(0)
            };
            let name = opcode_name(op);
            let who = if *direction == 1 { "host" } else { "machine" };
            rows.push(format!("{who:<8} {name:<22} {hx}"));
        }
        let decoded = std::env::var_os("CRICUT_JOYXTRA_DECODED")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| pack.join("decoded/joyxtra2-first.txt"));
        let text = std::fs::read_to_string(&decoded).unwrap();
        let expected: Vec<&str> = text
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        let shown = collapse_rows(&rows);
        assert_eq!(&shown[..expected.len()], expected.as_slice());
    }

    fn collapse_rows(rows: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < rows.len() && out.len() < 30 {
            let mut j = i + 1;
            while j < rows.len() && rows[j] == rows[i] {
                j += 1;
            }
            if j - i > 1 {
                out.push(format!("{}  x{}", rows[i], j - i));
            } else {
                out.push(rows[i].clone());
            }
            i = j;
        }
        out
    }

    fn opcode_name(op: u8) -> String {
        match op {
            0x30 => "SetMachineState",
            0x31 => "SetSpeedState",
            0x37 => "SetToolState",
            0x60 => "GetMachineStatus",
            0x68 => "AbortCutting",
            0x70 => "MoveZ",
            0x73 => "GetInterfaceState",
            0xA4 => "GetPersistentValueCount",
            0xBB => "ReadPosition4D",
            0xC7 => "NewAES",
            0xCF => "ResetAES",
            0xE2 => "ScanForEdgeFloat",
            0xE5 => "BluetoothAlert",
            0xEA => "ToolDetect",
            0xEE => "MeasureMaterial",
            0xF0 => "HomeTheta",
            0xF1 => "Move4D",
            0xFF => "SendZeros",
            0x0A => "ClearError",
            0x0B => "ReadErrorSeverity",
            _ => return format!("0x{op:02x}"),
        }
        .into()
    }

    fn extract_spp_frames(pcap: &std::path::Path) -> Vec<(u8, bool, Vec<u8>)> {
        let script =
            std::env::temp_dir().join(format!("joyxtra2-frames-{}.py", std::process::id()));
        std::fs::write(&script, include_str!("frame_extract.py")).unwrap();
        let out = std::process::Command::new("python3")
            .arg(&script)
            .arg(pcap)
            .output()
            .expect("python3");
        let _ = std::fs::remove_file(&script);
        assert!(
            out.status.success(),
            "frame extract failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                let mut parts = line.split('\t');
                let direction: u8 = parts.next().unwrap().parse().unwrap();
                let enc = parts.next().unwrap() == "1";
                let body = hex_decode(parts.next().unwrap());
                (direction, enc, body)
            })
            .collect()
    }
}
