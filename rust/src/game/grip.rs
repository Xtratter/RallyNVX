//! How the ground grips: multipliers per surface and the loose gravel layer of the special stage.
use super::roads::{RoadKind, RoadNet};
use super::surface::{surface_at, Surface};
use super::terrain::Terrain;

/// Multipliers on the car's own `mu`, `lateral_k` and `rolling` resistance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceGrip { pub mu: f32, pub lateral: f32, pub rolling: f32 }

pub const GRAVEL_PACKED: f32 = 0.78;
pub const GRAVEL_LOOSE: f32 = 0.62;

/// The grip of a surface; `loose` (0 packed .. 1 loose, ignored off gravel) interpolates the gravel grip.
pub fn grip(s: Surface, loose: f32) -> SurfaceGrip {
    let g = |mu, lateral, rolling| SurfaceGrip { mu, lateral, rolling };
    match s {
        Surface::Asphalt => g(1.0, 1.0, 1.0),
        Surface::Gravel => { let l = if loose.is_finite() { loose.clamp(0.0, 1.0) } else { 1.0 }; g(GRAVEL_PACKED + (GRAVEL_LOOSE - GRAVEL_PACKED) * l, 0.75, 1.3) }
        Surface::Dirt => g(0.70, 0.8, 1.2),
        Surface::Grass => g(0.60, 0.7, 1.5),
        Surface::Sand => g(0.50, 0.6, 3.0),
        Surface::Rock => g(0.80, 0.9, 1.0),
        Surface::Water => g(0.55, 0.7, 1.0),
    }
}

pub const CELL_S: f32 = 1.0;
pub const CELL_X: f32 = 0.5;
pub const HALF_W: f32 = 6.0;
const NX: usize = (2.0 * HALF_W / CELL_X) as usize;

/// Looseness (1 = untouched loose gravel .. 0 = packed) of the cells along every stage road.
#[derive(Clone, Debug, Default)]
pub struct LooseMap { roads: Vec<Vec<f32>> }

impl LooseMap {
    pub fn empty() -> LooseMap { LooseMap { roads: Vec::new() } }
    pub fn new(net: &RoadNet) -> LooseMap {
        LooseMap { roads: net.roads.iter().map(|r| if r.kind == RoadKind::Stage { vec![1.0; ((r.length() / CELL_S) as usize + 2) * NX] } else { Vec::new() }).collect() }
    }
    fn index(&self, road: usize, s: f32, side: f32) -> Option<usize> {
        let cells = self.roads.get(road)?;
        if cells.is_empty() || !s.is_finite() || !side.is_finite() || s < 0.0 || side.abs() >= HALF_W { return None; }
        let (i, j) = ((s / CELL_S) as usize, ((side + HALF_W) / CELL_X) as usize);
        let k = i * NX + j.min(NX - 1);
        if k < cells.len() { Some(k) } else { None }
    }
    pub fn loose_at(&self, road: usize, s: f32, side: f32) -> f32 { self.index(road, s, side).map_or(1.0, |k| self.roads[road][k]) }
    /// A wheel at (s, side) moving at `speed` m/s for `dt` s packs the cells under it (about 0.35 per metre travelled).
    pub fn wear(&mut self, road: usize, s: f32, side: f32, speed: f32, dt: f32) {
        if !speed.is_finite() || !dt.is_finite() { return; }
        let amount = (0.35 * speed.abs() * dt).clamp(0.0, 1.0);
        for off in [-0.25f32, 0.25] {   // the tyre's width: two half-metre cells
            if let Some(k) = self.index(road, s, side + off) { let c = &mut self.roads[road][k]; *c = (*c - amount * 0.5).clamp(0.0, 1.0); }
        }
    }
}

/// The surface at a point, its looseness and, on a gravel road, the (road, s, side) cell: what a wheel contact needs.
pub fn ground_at(t: &Terrain, map: &LooseMap, x: f32, z: f32) -> (Surface, f32, Option<(usize, f32, f32)>) {
    let surface = surface_at(t, x, z);
    if surface == Surface::Gravel {
        if let Some(h) = t.road_hit(x, z) { return (surface, map.loose_at(h.road, h.s, h.side), Some((h.road, h.s, h.side))); }
    }
    (surface, 1.0, None)
}

#[cfg(test)]
#[path = "grip_tests.rs"]
mod tests;
