//! Procedural roads: a tarmac liaison from the spawn plateau to the first city and a gravel special stage (see
//! docs/superpowers/specs/2026-10-14-roads-design.md). This file holds the data and the spatial lookup.
use std::collections::HashMap;

use super::math::{v3, V3};
use super::surface::Surface;

pub mod cfg;
pub mod gen;
pub mod path;
pub mod shape;
pub mod side;

/// Spatial hash cell, the shoulder where the ground eases back to nature, the widest half-width and the reach of a road.
pub const CELL: f32 = 32.0;
pub const SHOULDER: f32 = 8.0;
pub const MAX_HALF: f32 = 5.0;
pub const INFLUENCE: f32 = MAX_HALF + SHOULDER;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoadKind { Liaison, Stage, Side }
impl RoadKind {
    pub fn surface(self) -> Surface { match self { RoadKind::Liaison => Surface::Asphalt, RoadKind::Stage | RoadKind::Side => Surface::Gravel } }
    /// The stage and its side roads: gravel with a loose layer, deep ditches and a rocky edge.
    pub fn is_gravel(self) -> bool { self != RoadKind::Liaison }
}

/// A centre-line point: `pos.y` is the road height (the profile), `s` the distance along the road in metres.
#[derive(Clone, Copy, Debug)]
pub struct RoadPt { pub pos: V3, pub width: f32, pub s: f32 }

#[derive(Clone, Debug)]
pub struct Road { pub kind: RoadKind, pub pts: Vec<RoadPt> }
impl Road { pub fn length(&self) -> f32 { self.pts.last().map_or(0.0, |p| p.s) } }

/// The lines of a special stage: the start at 25 m, a check line every 1000 m, the finish 12 m before the end.
pub fn stage_lines(stage: &Road) -> Vec<super::lines::Line> {
    use super::lines::{Line, LineKind};
    let len = stage.length();
    let at = |s: f32, kind: LineKind| -> Option<Line> {
        let i = stage.pts.iter().position(|p| p.s >= s)?;
        let (a, b) = (stage.pts[i.saturating_sub(1)], stage.pts[(i + 1).min(stage.pts.len() - 1)]);
        let (dx, dz) = (b.pos.x - a.pos.x, b.pos.z - a.pos.z);
        let l = dx.hypot(dz).max(1e-3);
        Some(Line { pos: stage.pts[i].pos, dir: v3(dx / l, 0.0, dz / l), width: stage.pts[i].width + 2.0, kind })
    };
    let mut out: Vec<Line> = at(25.0, LineKind::Start).into_iter().collect();
    let mut s = 1000.0;
    while s < len - 250.0 { out.extend(at(s, LineKind::Check)); s += 1000.0; }
    out.extend(at((len - 12.0).max(26.0), LineKind::Finish));
    out
}

/// The closest road point to a query: `dist` to the centre line, the road height `y`, the half-width, `s` along the road.
#[derive(Clone, Copy, Debug)]
pub struct RoadHit { pub road: usize, pub seg: usize, pub t: f32, pub dist: f32, pub y: f32, pub half: f32, pub s: f32, pub tangent: V3, pub side: f32 }

#[derive(Clone, Debug, Default)]
pub struct RoadNet { pub roads: Vec<Road>, grid: HashMap<(i32, i32), Vec<(u16, u32)>> }

fn cell_of(v: f32) -> i32 { (v / CELL).floor() as i32 }

impl RoadNet {
    pub fn new(roads: Vec<Road>) -> RoadNet {
        let mut grid: HashMap<(i32, i32), Vec<(u16, u32)>> = HashMap::new();
        for (ri, r) in roads.iter().enumerate() {
            for si in 0..r.pts.len().saturating_sub(1) {
                let (a, b) = (r.pts[si].pos, r.pts[si + 1].pos);
                let (x0, x1) = (a.x.min(b.x) - INFLUENCE, a.x.max(b.x) + INFLUENCE);
                let (z0, z1) = (a.z.min(b.z) - INFLUENCE, a.z.max(b.z) + INFLUENCE);
                for cz in cell_of(z0)..=cell_of(z1) { for cx in cell_of(x0)..=cell_of(x1) { grid.entry((cx, cz)).or_default().push((ri as u16, si as u32)); } }
            }
        }
        RoadNet { roads, grid }
    }
    pub fn is_empty(&self) -> bool { self.roads.is_empty() }

    /// The nearest road point within `INFLUENCE` metres: one hash cell and its few segments.
    pub fn nearest(&self, x: f32, z: f32) -> Option<RoadHit> {
        let cands = self.grid.get(&(cell_of(x), cell_of(z)))?;
        let mut best: Option<RoadHit> = None;
        for &(ri, si) in cands {
            if let Some(h) = self.hit(ri as usize, si as usize, x, z) { if best.map_or(true, |b| h.dist < b.dist) { best = Some(h); } }
        }
        best
    }

    /// Brute force over every segment (tests).
    pub fn nearest_slow(&self, x: f32, z: f32) -> Option<RoadHit> {
        let mut best: Option<RoadHit> = None;
        for (ri, r) in self.roads.iter().enumerate() { for si in 0..r.pts.len().saturating_sub(1) {
            if let Some(h) = self.hit(ri, si, x, z) { if best.map_or(true, |b| h.dist < b.dist) { best = Some(h); } }
        } }
        best
    }

    fn hit(&self, ri: usize, si: usize, x: f32, z: f32) -> Option<RoadHit> {
        let r = &self.roads[ri];
        let (a, b) = (r.pts[si], r.pts[si + 1]);
        let (dx, dz) = (b.pos.x - a.pos.x, b.pos.z - a.pos.z);
        let l2 = dx * dx + dz * dz;
        let t = if l2 < 1e-9 { 0.0 } else { (((x - a.pos.x) * dx + (z - a.pos.z) * dz) / l2).clamp(0.0, 1.0) };
        let (px, pz) = (a.pos.x + dx * t, a.pos.z + dz * t);
        let dist = (x - px).hypot(z - pz);
        if dist > INFLUENCE { return None; }
        let lerp = |p: f32, q: f32| p + (q - p) * t;
        let l = l2.sqrt().max(1e-6);
        let side = (x - px) * (-dz / l) + (z - pz) * (dx / l);   // positive to the left of the travel direction
        Some(RoadHit { road: ri, seg: si, t, dist, y: lerp(a.pos.y, b.pos.y), half: lerp(a.width, b.width) * 0.5, s: lerp(a.s, b.s), tangent: v3(dx, 0.0, dz).norm(), side })
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
