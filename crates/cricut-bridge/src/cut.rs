//! Turning a design into a cut: polylines -> draw records -> command frames,
//! and the order of the cut session.
//!
//! The geometry (SVG -> polylines in inches) is solid. The draw-vector wire
//! bytes and the full command set are the hypothesis in `cricut-proto::wire`,
//! to confirm against a decrypted cut. The cut-session order below follows the
//! observed Design Space flow.

use cricut_design::Polyline;
use cricut_proto::{encode_draw_vectors, CutVectorType, DrawRecord};

/// Convert polylines into draw records: a MoveTo to each subpath's start, then
/// LineTo through its points, and EndPath at the very end.
pub fn polylines_to_records(polylines: &[Polyline]) -> Vec<DrawRecord> {
    let mut recs = Vec::new();
    for pl in polylines {
        for (i, p) in pl.points.iter().enumerate() {
            let kind = if i == 0 { CutVectorType::MoveTo } else { CutVectorType::LineTo };
            recs.push(DrawRecord { kind, x: p.x as f32, y: p.y as f32 });
        }
        if pl.closed {
            if let Some(first) = pl.points.first() {
                recs.push(DrawRecord { kind: CutVectorType::LineTo, x: first.x as f32, y: first.y as f32 });
            }
        }
    }
    if !recs.is_empty() {
        recs.push(DrawRecord { kind: CutVectorType::EndPath, x: 0.0, y: 0.0 });
    }
    recs
}

/// The draw-vector command frames (plaintext bodies) for a set of polylines.
/// The session encrypts each body before sending.
pub fn plan_draw_frames(polylines: &[Polyline]) -> Vec<Vec<u8>> {
    encode_draw_vectors(&polylines_to_records(polylines))
}

/// The order of a cut session, from the observed Design Space flow. Each step is
/// a command (or group) to send; the draw step uses [`plan_draw_frames`]. The
/// exact bodies for the non-draw steps are still being decoded from the capture.
///
/// 1. feature flags + log level, start session (carries user settings as authData)
/// 2. device list / select
/// 3. machine summary request/answer
/// 4. open device
/// 5. material selection (mat size, mode, margins)
/// 6. read pressure table from the machine
/// 7. tool list (pressure, passes, speeds per tool)
/// 8. path data  <-- plan_draw_frames
/// 9. Load, Go, progress, tool-change, pause, unload, complete
pub const CUT_FLOW: &[&str] = &[
    "start-session(authData)",
    "select-device",
    "machine-summary",
    "open-device",
    "material-select",
    "read-pressure-table",
    "tool-list",
    "path-data",
    "load/go/progress/complete",
];

#[cfg(test)]
mod tests {
    use super::*;
    use cricut_design::{path_to_polylines, Units};

    #[test]
    fn square_makes_records_and_frames() {
        let pls = path_to_polylines("M0,0 L72,0 L72,72 L0,72 Z", Units::POINTS, 0.01).unwrap();
        let recs = polylines_to_records(&pls);
        assert_eq!(recs.first().unwrap().kind, CutVectorType::MoveTo);
        assert_eq!(recs.last().unwrap().kind, CutVectorType::EndPath);
        let frames = plan_draw_frames(&pls);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0][0], 0x42);
    }
}
