//! Machine wire encoding of commands (the plaintext body that the session then
//! encrypts). We produce the machine format directly because we replace the
//! Design Space helper.
//!
//! STATUS: the draw-vector byte layout below is the documented hypothesis
//! (opcode 0x42, a count, then records of [kind, f32 x, f32 y] in inches, at most
//! 56 per frame). It must be confirmed against a DECRYPTED full-cut capture
//! before it will cut correctly. Everything marked `VERIFY` is unconfirmed on the
//! wire. The semantic model (vector types, points) is from the NativeModel
//! schemas and is solid; only the exact packing is open.

/// Vector segment type, matching PBDrawVectorType in NativeModel.Commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CutVectorType {
    LineTo = 0,
    CurveTo = 1,
    MoveTo = 2,
    LineIn = 3,
    LineOut = 4,
    CurveIn = 5,
    CurveOut = 6,
    CheckSum = 7,
    None = 8,
    EndPath = 9,
}

/// One draw record: a segment type and a point in inches.
#[derive(Debug, Clone, Copy)]
pub struct DrawRecord {
    pub kind: CutVectorType,
    pub x: f32,
    pub y: f32,
}

/// Opcode for the float draw-vector command (from the static analysis).
pub const OP_DRAW_VECTOR_FLOAT: u8 = 0x42;
/// Max records per draw frame (documented; VERIFY).
pub const MAX_RECORDS_PER_FRAME: usize = 56;

/// Encode draw records into one or more command bodies (opcode + packed records),
/// chunked to [`MAX_RECORDS_PER_FRAME`]. Each body is a plaintext command; the
/// session encrypts it before sending.
///
/// VERIFY against a decrypted cut: the exact header (is there a count byte?), the
/// float endianness (little-endian assumed), and the record size.
pub fn encode_draw_vectors(records: &[DrawRecord]) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    for chunk in records.chunks(MAX_RECORDS_PER_FRAME) {
        let mut body = Vec::with_capacity(2 + chunk.len() * 9);
        body.push(OP_DRAW_VECTOR_FLOAT);
        body.push(chunk.len() as u8); // VERIFY: count byte
        for r in chunk {
            body.push(r.kind as u8);
            body.extend_from_slice(&r.x.to_le_bytes()); // VERIFY: endianness
            body.extend_from_slice(&r.y.to_le_bytes());
        }
        frames.push(body);
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_by_max() {
        let recs: Vec<DrawRecord> = (0..130)
            .map(|i| DrawRecord { kind: CutVectorType::LineTo, x: i as f32, y: 0.0 })
            .collect();
        let frames = encode_draw_vectors(&recs);
        assert_eq!(frames.len(), 3); // 56 + 56 + 18
        assert_eq!(frames[0][0], OP_DRAW_VECTOR_FLOAT);
        assert_eq!(frames[0][1], 56);
        assert_eq!(frames[2][1], 18);
    }
}
