//! Pure city instance builder: culling by radius, nearest-first order, a hard budget, and the shadow subset.
use super::models::Part;
use crate::game::city::{City, TileKind, BLOCKS, HALF, PITCH, STREET};
use crate::game::math::{v3, Q, V3};

pub const PALETTE: [[f32; 3]; 10] = [[0.62, 0.60, 0.57], [0.55, 0.30, 0.24], [0.45, 0.52, 0.60], [0.70, 0.66, 0.55], [0.38, 0.42, 0.48], [0.58, 0.62, 0.58],
    [0.72, 0.60, 0.42], [0.42, 0.28, 0.20], [0.80, 0.78, 0.72], [0.30, 0.38, 0.34]];
/// Streets are asphalt strips of this pitch/width (the shader draws lane marks from these, keep in sync with city.rs).
const LAMP_RADIUS: f32 = 150.0;

pub struct CityParts { pub parts: Vec<Part>, pub near: usize }

fn dist(p: &Part, cam: V3) -> f32 { (p.pos.x - cam.x).hypot(p.pos.z - cam.z) }

/// Instances of the cities within `radius` of the camera: buildings (facade flag) nearest first, then street, slab and
/// park tiles; at most `budget` parts. `near` counts the leading buildings within `shadow_radius` (the shadow casters).
pub fn city_parts(cities: &[City], cam: V3, radius: f32, budget: usize, shadow_radius: f32) -> CityParts {
    let mut buildings: Vec<Part> = Vec::new(); let mut tiles: Vec<Part> = Vec::new(); let mut lamps: Vec<Part> = Vec::new();
    for c in cities {
        if (cam.x - c.site.centre.x).abs().max((cam.z - c.site.centre.z).abs()) > c.site.half + radius { continue; }
        for b in &c.buildings {
            let (ctr, size) = (b.centre(), b.size());
            let col = PALETTE[b.color as usize % PALETTE.len()];
            let p = Part { mesh: 0, rot: Q::IDENT, pos: ctr, scale: size, color: col, glow: false, facade: true, street: false };
            if dist(&p, cam) > radius + size.x.max(size.z) { continue; }
            if size.y > 8.0 {
                // cornice: a slightly wider dark slab on the roof edge
                let (cc, y, w) = ([col[0] * 0.65, col[1] * 0.65, col[2] * 0.65], b.max.y + 0.2, 0.35);
                for (dx, dz, sx, sz) in [(0.0, 1.0, size.x + 0.8, w), (0.0, -1.0, size.x + 0.8, w), (1.0, 0.0, w, size.z + 0.8 - 2.0 * w), (-1.0, 0.0, w, size.z + 0.8 - 2.0 * w)] {
                    let rim = Part { mesh: 0, rot: Q::IDENT, pos: v3(ctr.x + dx * (size.x * 0.5 + 0.4 - w * 0.5), y, ctr.z + dz * (size.z * 0.5 + 0.4 - w * 0.5)),
                        scale: v3(sx, 0.4, sz), color: cc, glow: false, facade: false, street: false };
                    if dist(&rim, cam) <= radius + sx.max(sz) { tiles.push(rim); }
                }
            }
            buildings.push(p);
        }
        for t in &c.tiles {
            let (cx, cz) = ((t.min_x + t.max_x) * 0.5, (t.min_z + t.max_z) * 0.5);
            let (sx, sz) = (t.max_x - t.min_x, t.max_z - t.min_z);
            let (h, color) = match t.kind { TileKind::Asphalt => (0.04, [0.16, 0.16, 0.18]), TileKind::Slab => (0.12, [0.55, 0.55, 0.52]), TileKind::Park => (0.15, [0.22, 0.45, 0.20]) };
            let lift = if t.kind == TileKind::Asphalt { 0.02 } else { 0.0 };
            let p = Part { mesh: 0, rot: Q::IDENT, pos: v3(cx, t.y + h * 0.5 + lift, cz), scale: v3(sx, h, sz), color, glow: false, facade: false, street: t.kind == TileKind::Asphalt };
            if dist(&p, cam) <= radius + sx.max(sz) * 0.5 { tiles.push(p); }
        }
        // street lamps at the corners of the intersections near the camera
        let (ox, oz) = (c.site.centre.x - HALF, c.site.centre.z - HALF);
        let off = STREET / 2.0 + 0.8;
        for k in 0..=BLOCKS { for l in 0..=BLOCKS {
            let (ix, iz) = (ox + k as f32 * PITCH, oz + l as f32 * PITCH);
            if (ix - cam.x).hypot(iz - cam.z) > LAMP_RADIUS - off { continue; }
            for (sx, sz) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                let (x, z) = (ix + sx * off, iz + sz * off);
                if (x - cam.x).hypot(z - cam.z) > LAMP_RADIUS { continue; }
                let y = c.site.centre.y;
                lamps.push(Part { mesh: 1, rot: Q::IDENT, pos: v3(x, y + 3.0, z), scale: v3(0.25, 6.0, 0.25), color: [0.2, 0.2, 0.22], glow: false, facade: false, street: false });
                lamps.push(Part { mesh: 3, rot: Q::IDENT, pos: v3(x, y + 6.2, z), scale: v3(0.7, 0.7, 0.7), color: [1.0, 0.85, 0.5], glow: true, facade: false, street: false });
            }
        } }
    }
    let by = |a: &Part, b: &Part| dist(a, cam).partial_cmp(&dist(b, cam)).unwrap_or(std::cmp::Ordering::Equal);
    buildings.sort_by(by); tiles.sort_by(by);
    let mut parts = buildings; parts.extend(tiles); parts.extend(lamps); parts.truncate(budget);
    let near = parts.iter().take_while(|p| p.facade && dist(p, cam) <= shadow_radius + p.scale.x.max(p.scale.z)).count();
    CityParts { parts, near }
}

#[cfg(test)]
#[path = "citygen_tests.rs"]
mod tests;
