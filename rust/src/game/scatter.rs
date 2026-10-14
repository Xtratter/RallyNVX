//! Deterministic decorative scatter: bushes per terrain tile and grass tufts around a point (no gameplay effect).
use super::materials::terrain_weights;
use super::math::{v3, V3};
use super::terrain::{hash01, Terrain, TILE_SIZE};

#[derive(Clone, Copy, Debug)]
pub struct Bush { pub pos: V3, pub scale: f32 }

const BUSHES_PER_TILE: i32 = 6;
const GRASS_CELL: f32 = 1.2;

fn gentle(t: &Terrain, x: f32, z: f32, min_y: f32) -> Option<V3> {
    if t.near_road(x, z, 3.0) { return None; }
    if t.in_city(x, z, 0.0) || t.is_water(x, z, 0.5) { return None; }
    let n = t.normal(x, z);
    if n.y < min_y { return None; }
    Some(n)
}

pub fn tile_bushes(t: &Terrain, tx: i32, tz: i32) -> Vec<Bush> {
    let mut out = Vec::new();
    for k in 0..BUSHES_PER_TILE {
        let a = hash01(t.seed ^ 0x61, tx.wrapping_mul(16).wrapping_add(k), tz);
        let b = hash01(t.seed ^ 0x62, tx, tz.wrapping_mul(16).wrapping_add(k));
        let c = hash01(t.seed ^ 0x63, tx.wrapping_add(k), tz.wrapping_sub(k * 3));
        let (x, z) = ((tx as f32 + a) * TILE_SIZE, (tz as f32 + b) * TILE_SIZE);
        if x.hypot(z) < 12.0 || gentle(t, x, z, 0.85).is_none() { continue; }
        out.push(Bush { pos: v3(x, t.height(x, z), z), scale: 0.7 + 0.9 * c });
    }
    out
}

/// Grass tufts on a 1.2 m jittered grid within `radius` of `centre` (which should be snapped by the caller so the
/// result stays stable while the camera moves), nearest first, at most `cap`. Tuft: (position, scale).
pub fn grass_spots(t: &Terrain, centre: V3, radius: f32, cap: usize) -> Vec<(V3, f32)> {
    if cap == 0 { return Vec::new(); }
    let n = (radius / GRASS_CELL).ceil() as i32;
    let (ci, cj) = ((centre.x / GRASS_CELL).floor() as i32, (centre.z / GRASS_CELL).floor() as i32);
    let mut out: Vec<(f32, V3, f32)> = Vec::new();
    for dj in -n..=n { for di in -n..=n {
        let (i, j) = (ci + di, cj + dj);
        let x = (i as f32 + hash01(0x6A, i, j)) * GRASS_CELL;
        let z = (j as f32 + hash01(0x6B, i, j)) * GRASS_CELL;
        let d = (x - centre.x).hypot(z - centre.z);
        if d > radius { continue; }
        // cheap checks first: one height sample, a forward-difference slope (2 more samples)
        if t.in_city(x, z, 0.0) || t.near_road(x, z, 0.3) { continue; }
        let h = t.height(x, z);
        if t.water_level().map_or(false, |l| h < l + 0.3) { continue; }
        let nm = v3(h - t.height(x + 1.0, z), 1.0, h - t.height(x, z + 1.0)).norm();
        if nm.y < 0.8 || terrain_weights(h, nm, x, z)[0] < 128 { continue; }
        out.push((d, v3(x, h, z), 0.8 + 0.6 * hash01(0x6C, i, j)));
    } }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(cap);
    out.into_iter().map(|(_, p, s)| (p, s)).collect()
}

#[cfg(test)]
#[path = "scatter_tests.rs"]
mod tests;
