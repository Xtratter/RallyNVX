//! Deterministic rocks and trees scattered per terrain tile.
use super::math::{v3, V3};
use super::terrain::{hash01, Terrain, TILE_SIZE};

pub const MIN_PROP_DIST: f32 = 25.0;
const PER_TILE: i32 = 28;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropKind { Rock, Tree }

/// `pos` is at ground level; `height` is the trunk height for trees, the rock's visual height for rocks.
#[derive(Clone, Copy, Debug)]
pub struct Prop { pub kind: PropKind, pub pos: V3, pub radius: f32, pub height: f32 }

pub fn tile_props(t: &Terrain, tx: i32, tz: i32) -> Vec<Prop> {
    let mut out = Vec::new();
    for k in 0..PER_TILE {
        let a = hash01(t.seed ^ 0x51, tx.wrapping_mul(64).wrapping_add(k), tz);
        let b = hash01(t.seed ^ 0x52, tx, tz.wrapping_mul(64).wrapping_add(k));
        let c = hash01(t.seed ^ 0x53, tx.wrapping_add(k.wrapping_mul(7)), tz.wrapping_sub(k));
        let (x, z) = ((tx as f32 + a) * TILE_SIZE, (tz as f32 + b) * TILE_SIZE);
        if x.hypot(z) < MIN_PROP_DIST { continue; }
        if t.in_city(x, z, 100.0) || t.is_water(x, z, 1.0) { continue; }
        if t.normal(x, z).y < 0.85 { continue; }
        if t.near_road(x, z, 4.0) { continue; }
        let pos = v3(x, t.height(x, z), z);
        if c < 0.45 {
            let radius = 0.6 + (c / 0.45) * 1.4;
            out.push(Prop { kind: PropKind::Rock, pos, radius, height: radius * 1.4 });
        } else {
            out.push(Prop { kind: PropKind::Tree, pos, radius: 0.35, height: 5.0 + (c - 0.45) / 0.55 * 4.0 });
        }
    }
    // a rocky edge along the stage: extra rock candidates, kept only 4-8 m from the road
    let cfg = super::roads::cfg::cfg();
    if t.roads().is_some() {
        for k in 0..cfg.rock_candidates as i32 {
            let a = hash01(t.seed ^ 0x61, tx.wrapping_mul(97).wrapping_add(k), tz);
            let b = hash01(t.seed ^ 0x62, tx, tz.wrapping_mul(97).wrapping_add(k));
            let c = hash01(t.seed ^ 0x63, tx.wrapping_add(k), tz.wrapping_sub(k.wrapping_mul(5)));
            let (x, z) = ((tx as f32 + a) * TILE_SIZE, (tz as f32 + b) * TILE_SIZE);
            let Some(hit) = t.road_hit(x, z) else { continue };
            let edge = hit.dist - hit.half;
            let stage = t.roads().map_or(false, |n| n.roads[hit.road].kind.is_gravel());
            if !stage || edge < cfg.rock_band.0 || edge > cfg.rock_band.1 { continue; }
            if t.in_city(x, z, 100.0) || t.is_water(x, z, 1.0) || t.normal(x, z).y < 0.85 || x.hypot(z) < MIN_PROP_DIST { continue; }
            let radius = 0.5 + c * 0.9;
            out.push(Prop { kind: PropKind::Rock, pos: v3(x, t.height(x, z), z), radius, height: radius * 1.4 });
        }
    }
    out
}

#[cfg(test)]
#[path = "props_tests.rs"]
mod tests;
