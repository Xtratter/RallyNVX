//! Lines across the road or street: the start, the check lines and the finish of a stage.
use super::math::V3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind { Start, Check, Finish }

/// A line across the road: its centre, the unit direction of travel (horizontal), the width and the kind.
#[derive(Clone, Copy, Debug)]
pub struct Line { pub pos: V3, pub dir: V3, pub width: f32, pub kind: LineKind }

impl Line {
    /// (metres past the line along the direction of travel, metres to the right of its centre) of a point.
    pub fn local(&self, p: V3) -> (f32, f32) {
        let (dx, dz) = (p.x - self.pos.x, p.z - self.pos.z);
        (dx * self.dir.x + dz * self.dir.z, dx * self.dir.z - dz * self.dir.x)
    }
    /// True when the step from `a` to `b` crosses the line forwards inside its width (plus a metre and a half of slack).
    pub fn crossed(&self, a: V3, b: V3) -> bool {
        let (la, lb) = (self.local(a), self.local(b));
        if !(la.0 < 0.0 && lb.0 >= 0.0) { return false; }
        let t = -la.0 / (lb.0 - la.0);
        (la.1 + (lb.1 - la.1) * t).abs() <= self.width * 0.5 + 1.5
    }
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
