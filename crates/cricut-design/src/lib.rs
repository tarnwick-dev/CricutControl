//! Design input: SVG paths -> flattened polylines in machine units.
//!
//! Design Space sends the cut geometry to the helper as SVG path strings in
//! points (72 per inch), with a per-layer transform, sizes in inches. This crate
//! turns an SVG path `d` string into subpaths of points in inches, flattening
//! curves to line segments. The machine wire encoding of those points lives in
//! `cricut-proto` (the draw-vector frames).
//!
//! This layer is fully determined (standard SVG geometry); it does not depend on
//! any reverse-engineered format.

use kurbo::{BezPath, PathEl, Point as KPoint, Shape};

#[derive(Debug, thiserror::Error)]
pub enum DesignError {
    #[error("svg path parse: {0}")]
    Parse(String),
}

/// A point in inches (machine coordinate space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// One continuous subpath: a MOVE to the first point, then LINEs through the
/// rest. `closed` marks a `Z` close (last point joins the first).
#[derive(Debug, Clone)]
pub struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
}

/// Units per inch of the incoming SVG coordinates. Design Space uses points
/// (72/inch); plain SVG user units are commonly 96/inch. Set to match the source.
#[derive(Debug, Clone, Copy)]
pub struct Units {
    pub per_inch: f64,
}
impl Units {
    pub const POINTS: Units = Units { per_inch: 72.0 };
    pub const CSS_PX: Units = Units { per_inch: 96.0 };
    pub const INCHES: Units = Units { per_inch: 1.0 };
}

/// Parse an SVG path `d` string and flatten it into polylines in inches.
///
/// `tolerance` is the maximum flattening error, in the SVG's own units.
pub fn path_to_polylines(d: &str, units: Units, tolerance: f64) -> Result<Vec<Polyline>, DesignError> {
    let path = BezPath::from_svg(d).map_err(|e| DesignError::Parse(e.to_string()))?;
    let scale = 1.0 / units.per_inch;
    let to_inch = |p: KPoint| Point { x: p.x * scale, y: p.y * scale };

    let mut out: Vec<Polyline> = Vec::new();
    let mut cur: Option<Polyline> = None;

    // flatten turns curves into MoveTo/LineTo/ClosePath only.
    kurbo::flatten(path.iter(), tolerance, |el| match el {
        PathEl::MoveTo(p) => {
            if let Some(pl) = cur.take() {
                if pl.points.len() >= 2 {
                    out.push(pl);
                }
            }
            cur = Some(Polyline { points: vec![to_inch(p)], closed: false });
        }
        PathEl::LineTo(p) => {
            if let Some(pl) = cur.as_mut() {
                pl.points.push(to_inch(p));
            }
        }
        PathEl::ClosePath => {
            if let Some(pl) = cur.as_mut() {
                pl.closed = true;
            }
        }
        // flatten() emits no Quad/Curve elements.
        PathEl::QuadTo(_, _) | PathEl::CurveTo(_, _, _) => unreachable!("flatten removes curves"),
    });
    if let Some(pl) = cur.take() {
        if pl.points.len() >= 2 {
            out.push(pl);
        }
    }
    Ok(out)
}

/// Overall bounding box of a path, in inches (for mat-fit checks).
pub fn bounds_inches(d: &str, units: Units) -> Result<(Point, Point), DesignError> {
    let path = BezPath::from_svg(d).map_err(|e| DesignError::Parse(e.to_string()))?;
    let b = path.bounding_box();
    let s = 1.0 / units.per_inch;
    Ok((Point { x: b.x0 * s, y: b.y0 * s }, Point { x: b.x1 * s, y: b.y1 * s }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_rect_in_points() {
        // a 72x72 point square = 1x1 inch
        let d = "M0,0 L72,0 L72,72 L0,72 Z";
        let pls = path_to_polylines(d, Units::POINTS, 0.01).unwrap();
        assert_eq!(pls.len(), 1);
        assert!(pls[0].closed);
        // corners at 0..1 inch
        let xs: Vec<f64> = pls[0].points.iter().map(|p| p.x).collect();
        assert!(xs.iter().any(|&x| (x - 1.0).abs() < 1e-9));
        assert!(xs.iter().all(|&x| (0.0..=1.0 + 1e-9).contains(&x)));
    }

    #[test]
    fn curve_is_flattened() {
        let d = "M0,0 C0,72 72,72 72,0";
        let pls = path_to_polylines(d, Units::POINTS, 0.1).unwrap();
        assert_eq!(pls.len(), 1);
        assert!(pls[0].points.len() > 2, "curve should flatten to many segments");
    }

    #[test]
    fn two_subpaths() {
        let d = "M0,0 L72,0 M0,72 L72,72";
        let pls = path_to_polylines(d, Units::POINTS, 0.01).unwrap();
        assert_eq!(pls.len(), 2);
    }
}
