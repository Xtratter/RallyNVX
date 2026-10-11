//! Pure city instance builder: culling by radius, nearest-first order, a hard budget, and the shadow subset.
use super::models::Part;
use crate::game::city::{City, TileKind};
use crate::game::math::{v3, Q, V3};

pub const PALETTE: [[f32; 3]; 6] = [[0.62, 0.60, 0.57], [0.55, 0.30, 0.24], [0.45, 0.52, 0.60], [0.70, 0.66, 0.55], [0.38, 0.42, 0.48], [0.58, 0.62, 0.58]];

pub struct CityParts { pub parts: Vec<Part>, pub near: usize }

fn dist(p: &Part, cam: V3) -> f32 { (p.pos.x - cam.x).hypot(p.pos.z - cam.z) }

/// Instances of the cities within `radius` of the camera: buildings (facade flag) nearest first, then street, slab and
/// park tiles; at most `budget` parts. `near` counts the leading buildings within `shadow_radius` (the shadow casters).
pub fn city_parts(cities: &[City], cam: V3, radius: f32, budget: usize, shadow_radius: f32) -> CityParts {
    let mut buildings: Vec<Part> = Vec::new(); let mut tiles: Vec<Part> = Vec::new();
    for c in cities {
        if (cam.x - c.site.centre.x).abs().max((cam.z - c.site.centre.z).abs()) > c.site.half + radius { continue; }
        for b in &c.buildings {
            let (ctr, size) = (b.centre(), b.size());
            let p = Part { mesh: 0, rot: Q::IDENT, pos: ctr, scale: size, color: PALETTE[b.color as usize % 6], glow: false, facade: true };
            if dist(&p, cam) <= radius + size.x.max(size.z) { buildings.push(p); }
        }
        for t in &c.tiles {
            let (cx, cz) = ((t.min_x + t.max_x) * 0.5, (t.min_z + t.max_z) * 0.5);
            let (sx, sz) = (t.max_x - t.min_x, t.max_z - t.min_z);
            let (h, color) = match t.kind { TileKind::Asphalt => (0.04, [0.16, 0.16, 0.18]), TileKind::Slab => (0.12, [0.55, 0.55, 0.52]), TileKind::Park => (0.15, [0.22, 0.45, 0.20]) };
            let lift = if t.kind == TileKind::Asphalt { 0.02 } else { 0.0 };
            let p = Part { mesh: 0, rot: Q::IDENT, pos: v3(cx, t.y + h * 0.5 + lift, cz), scale: v3(sx, h, sz), color, glow: false, facade: false };
            if dist(&p, cam) <= radius + sx.max(sz) * 0.5 { tiles.push(p); }
        }
    }
    let by = |a: &Part, b: &Part| dist(a, cam).partial_cmp(&dist(b, cam)).unwrap_or(std::cmp::Ordering::Equal);
    buildings.sort_by(by); tiles.sort_by(by);
    let mut parts = buildings; parts.extend(tiles); parts.truncate(budget);
    let near = parts.iter().take_while(|p| p.facade && dist(p, cam) <= shadow_radius + p.scale.x.max(p.scale.z)).count();
    CityParts { parts, near }
}

#[cfg(test)]
#[path = "citygen_tests.rs"]
mod tests;
