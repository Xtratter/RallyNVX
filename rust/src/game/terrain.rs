//! Seeded procedural terrain: hash-based value noise, fBm heights, normals.
use std::sync::Arc;

use super::city::{city_sites, CitySite};
use super::math::{v3, V3};
use super::roads::{self, RoadHit, RoadNet, SHOULDER};

pub const TILE_CELLS: usize = 64;
pub const CELL: f32 = 2.0;
pub const TILE_SIZE: f32 = TILE_CELLS as f32 * CELL;
/// Deepest point of a lake below its surface, metres (the car drives on the bed).
pub const LAKE_DEPTH: f32 = 1.2;

/// Deterministic hash of (seed, x, z) -> [0, 1). Integer arithmetic only, safe for negative coordinates.
pub fn hash01(seed: u64, x: i32, z: i32) -> f32 {
    let mut h = seed ^ (x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ ((z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)).rotate_left(21);
    h ^= h >> 33; h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33; h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h >> 40) as f32 / 16_777_216.0
}

fn smooth(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }
fn smoothstep(a: f32, b: f32, x: f32) -> f32 { smooth(((x - a) / (b - a)).clamp(0.0, 1.0)) }

fn vnoise(seed: u64, x: f32, z: f32) -> f32 {
    let (fx, fz) = (x.floor(), z.floor());
    let (ix, iz) = (fx as i32, fz as i32);
    let (u, v) = (smooth(x - fx), smooth(z - fz));
    let a = hash01(seed, ix, iz); let b = hash01(seed, ix + 1, iz);
    let c = hash01(seed, ix, iz + 1); let d = hash01(seed, ix + 1, iz + 1);
    let top = a + (b - a) * u; let bot = c + (d - c) * u;
    top + (bot - top) * v
}

fn fbm(seed: u64, x: f32, z: f32, octaves: u32) -> f32 {
    let (mut sum, mut norm, mut amp, mut f) = (0.0, 0.0, 1.0, 1.0);
    for o in 0..octaves {
        sum += amp * (2.0 * vnoise(seed.wrapping_add(o as u64 * 0x9E37), x * f, z * f) - 1.0);
        norm += amp; amp *= 0.5; f *= 2.0;
    }
    sum / norm
}

#[derive(Clone, Debug)]
pub struct Terrain { pub seed: u64, scale: f32, sites: [Option<CitySite>; 2], water: Option<f32>, roads: Option<Arc<RoadNet>> }

impl Terrain {
    pub fn new(seed: u64) -> Terrain {
        let raw = Terrain { seed, scale: 1.0, sites: [None; 2], water: None, roads: None };
        let with_sites = Terrain { sites: city_sites(seed, &raw), ..raw.clone() };
        Terrain { water: with_sites.pick_water_level(), ..with_sites }
    }

    /// The 12th percentile of heights sampled over the playable area, kept clear of the spawn plateau and the city
    /// plateaus; `None` for flat levels (a lake needs a clear drop below the spawn height).
    fn pick_water_level(&self) -> Option<f32> {
        let mut hs: Vec<f32> = Vec::with_capacity(41 * 41);
        for j in -20..=20 { for i in -20..=20 { hs.push(self.height(i as f32 * 60.0, j as f32 * 60.0)); } }
        hs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let cand = hs[hs.len() * 12 / 100];
        if cand > -1.0 { return None; }
        let low_city = self.sites.iter().flatten().map(|s| s.centre.y).fold(f32::MAX, f32::min);
        Some(cand.min(low_city - 1.5))
    }

    /// The lake surface height of this level, if it has lakes.
    pub fn water_level(&self) -> Option<f32> { self.water }
    /// The ground is below the lake surface (plus `margin` metres).
    pub fn is_water(&self, x: f32, z: f32, margin: f32) -> bool { self.water.map_or(false, |l| self.height(x, z) < l + margin) }
    /// Flat terrain under water up to `level` (tests).
    pub fn flooded(level: f32) -> Terrain { Terrain { water: Some(level), ..Terrain::flat() } }
    /// Height 0 everywhere (tests, vehicle tuning).
    pub fn flat() -> Terrain { Terrain { seed: 0, scale: 0.0, sites: [None; 2], water: None, roads: None } }

    /// The heightfield without city plateaus.
    pub fn raw_height(&self, x: f32, z: f32) -> f32 {
        if self.scale == 0.0 { return 0.0; }
        let big = vnoise(self.seed ^ 0xB16, x / 700.0, z / 700.0);
        let amp = (6.0 + 44.0 * big * big) * self.scale;
        let k = smoothstep(10.0, 70.0, x.hypot(z)); // flat spawn plateau
        fbm(self.seed, x / 140.0, z / 140.0, 5) * amp * k
    }

    /// Terrain height with every city plateau blended in (flat inside a city, smooth 100 m rim).
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let mut h = self.raw_height(x, z);
        for s in self.sites.iter().flatten() {
            let d = (x - s.centre.x).abs().max((z - s.centre.z).abs()) - s.half;   // distance beyond the city square
            if d >= 100.0 { continue; }
            let k = 1.0 - smoothstep(0.0, 100.0, d.max(0.0));
            h += (s.centre.y - h) * k;
        }
        // lakes are shallow: the lake bed never lies more than LAKE_DEPTH below the surface
        if let Some(l) = self.water { h = h.max(l - LAKE_DEPTH); }
        // roads: the ground equals the road profile inside half-width + 1 m and eases back to nature over the shoulder
        if let Some(hit) = self.road_hit(x, z) {
            let w = 1.0 - smoothstep(hit.half + 1.0, hit.half + SHOULDER, hit.dist);
            if w > 0.0 {
                h += (hit.y - h) * w;
                // a ditch runs along the road edge (none on a causeway)
                let d = self.ditch(&hit);
                if d > 0.0 { h -= d * w; }
            }
        }
        h
    }

    /// How deep the roadside ditch is at this road hit (0 on a causeway or far from the edge).
    fn ditch(&self, hit: &RoadHit) -> f32 {
        let Some(net) = self.roads.as_ref() else { return 0.0 };
        let cfg = roads::cfg::cfg();
        let d = if net.roads[hit.road].kind == roads::RoadKind::Stage { cfg.ditch_stage } else { cfg.ditch_liaison };
        if let Some(l) = self.water { if hit.y - l < cfg.crest.causeway_clear { return 0.0; } }
        let t = ((hit.dist - hit.half - d.offset).abs() / d.width).min(1.0);
        d.depth * (1.0 - smooth(t))
    }
    /// The level with its road network (the game); `Terrain::new` has none and is cheap (tests, planning).
    pub fn with_roads(seed: u64) -> Terrain {
        let base = Terrain::new(seed);
        let net = roads::gen::generate(seed, &base);
        Terrain { roads: Some(Arc::new(net)), ..base }
    }
    /// Tests: this terrain with the given road network.
    #[cfg(test)]
    pub fn with_net(self, net: RoadNet) -> Terrain { Terrain { roads: Some(Arc::new(net)), ..self } }
    /// The plain test terrain (no relief): off the roads a neutral asphalt for the physics tests.
    pub fn is_flat_test(&self) -> bool { self.scale == 0.0 }
    pub fn roads(&self) -> Option<&RoadNet> { self.roads.as_deref() }
    pub fn road_hit(&self, x: f32, z: f32) -> Option<RoadHit> { self.roads.as_ref()?.nearest(x, z) }
    /// Distance from (x, z) to the edge of the nearest road (negative on the road); `None` when no road is within reach.
    pub fn road_edge_distance(&self, x: f32, z: f32) -> Option<f32> { self.road_hit(x, z).map(|h| h.dist - h.half) }
    pub fn near_road(&self, x: f32, z: f32, margin: f32) -> bool { self.road_edge_distance(x, z).map_or(false, |d| d < margin) }

    pub fn sites(&self) -> Vec<CitySite> { self.sites.iter().flatten().copied().collect() }

    /// Inside any city square grown by `margin` metres.
    pub fn in_city(&self, x: f32, z: f32, margin: f32) -> bool {
        self.sites.iter().flatten().any(|s| (x - s.centre.x).abs() <= s.half + margin && (z - s.centre.z).abs() <= s.half + margin)
    }

    pub fn normal(&self, x: f32, z: f32) -> V3 {
        let e = 1.0;
        v3(self.height(x - e, z) - self.height(x + e, z), 2.0 * e, self.height(x, z - e) - self.height(x, z + e)).norm()
    }
}

/// Vertices are `[x, y, z, nx, ny, nz]`.
pub struct Mesh { pub verts: Vec<[f32; 6]>, pub idx: Vec<u32> }

impl Terrain {
    pub fn tile_mesh(&self, tx: i32, tz: i32, lod: u32) -> Mesh { self.tile_mesh_with_weights(tx, tz, lod).0 }

    /// Grid of `TILE_CELLS >> lod` cells over tile (tx, tz), plus downward skirts on all four edges (hide cracks
    /// between tiles of different LOD), with one material weight entry per vertex (skirts copy their parent vertex).
    pub fn tile_mesh_with_weights(&self, tx: i32, tz: i32, lod: u32) -> (Mesh, Vec<[u8; 4]>) {
        let n = TILE_CELLS >> lod;
        let step = TILE_SIZE / n as f32;
        let (ox, oz) = (tx as f32 * TILE_SIZE, tz as f32 * TILE_SIZE);
        let mut verts = Vec::with_capacity((n + 1) * (n + 1) + 4 * (n + 1));
        let mut weights: Vec<[u8; 4]> = Vec::with_capacity(verts.capacity());
        for j in 0..=n { for i in 0..=n {
            let (x, z) = (ox + i as f32 * step, oz + j as f32 * step);
            let nm = self.normal(x, z);
            let h = self.height(x, z);
            verts.push([x, h, z, nm.x, nm.y, nm.z]);
            weights.push(crate::game::materials::terrain_weights(h, nm, x, z));
        } }
        let w = (n + 1) as u32;
        let mut idx = Vec::with_capacity(n * n * 6 + 4 * n * 6);
        for j in 0..n as u32 { for i in 0..n as u32 {
            let a = j * w + i; let (b, c, d) = (a + 1, a + w, a + w + 1);
            idx.extend_from_slice(&[a, c, b, b, c, d]);
        } }
        let drop = 2.0 * step + 2.0;
        let mut skirt = |edge: Vec<u32>| {
            let base = verts.len() as u32;
            for &v in &edge { let mut p = verts[v as usize]; p[1] -= drop; verts.push(p); weights.push(weights[v as usize]); }
            for k in 0..edge.len() - 1 {
                let (a, b, c, d) = (edge[k], edge[k + 1], base + k as u32, base + k as u32 + 1);
                idx.extend_from_slice(&[a, c, b, b, c, d]);
            }
        };
        skirt((0..w).collect());
        skirt((0..w).map(|i| n as u32 * w + i).collect());
        skirt((0..w).map(|j| j * w).collect());
        skirt((0..w).map(|j| j * w + n as u32).collect());
        (Mesh { verts, idx }, weights)
    }
}

#[cfg(test)]
#[path = "terrain_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "terrain_roads_tests.rs"]
mod road_tests;
