//! Procedural city: grid of streets and blocks, lots with buildings, parks, grid queries and collision helpers.
use std::collections::HashMap;

use super::math::{v3, Q, V3};
use super::terrain::{hash01, Terrain};

pub const BLOCKS: i32 = 11;
pub const PITCH: f32 = 48.0;
pub const STREET: f32 = 12.0;
pub const BLOCK: f32 = 36.0;
pub const HALF: f32 = BLOCKS as f32 * PITCH / 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CitySite { pub centre: V3, pub half: f32 }

#[derive(Clone, Copy, Debug)]
pub struct Building { pub min: V3, pub max: V3, pub color: u8 }
impl Building {
    pub fn centre(&self) -> V3 { (self.min + self.max) * 0.5 }
    pub fn size(&self) -> V3 { self.max - self.min }
}

#[derive(Clone, Copy, Debug, PartialEq)] pub enum TileKind { Asphalt, Slab, Park }
#[derive(Clone, Copy, Debug)]
pub struct Tile { pub min_x: f32, pub min_z: f32, pub max_x: f32, pub max_z: f32, pub y: f32, pub kind: TileKind }

pub struct City { pub site: CitySite, pub buildings: Vec<Building>, pub tiles: Vec<Tile>, pub roofs: Vec<V3>, grid: HashMap<(i32, i32), Vec<usize>> }

impl City {
    pub fn build(site: CitySite, seed: u64) -> City {
        let key = seed ^ 0xC17;
        let (ox, oz, y0) = (site.centre.x - HALF, site.centre.z - HALF, site.centre.y);
        let inset = (PITCH - BLOCK) / 2.0;
        let mut buildings = Vec::new(); let mut tiles = Vec::new(); let mut roofs = Vec::new();
        // streets: BLOCKS + 1 strips each way
        for k in 0..=BLOCKS {
            let c = k as f32 * PITCH;
            tiles.push(Tile { min_x: ox + c - STREET / 2.0, max_x: ox + c + STREET / 2.0, min_z: oz, max_z: oz + 2.0 * HALF, y: y0, kind: TileKind::Asphalt });
            tiles.push(Tile { min_x: ox, max_x: ox + 2.0 * HALF, min_z: oz + c - STREET / 2.0, max_z: oz + c + STREET / 2.0, y: y0, kind: TileKind::Asphalt });
        }
        for j in 0..BLOCKS { for i in 0..BLOCKS {
            let (bx, bz) = (ox + i as f32 * PITCH + inset, oz + j as f32 * PITCH + inset);
            let h = hash01(key, i, j);
            let park = (i == BLOCKS / 2 && j == BLOCKS / 2) || h < 1.0 / 7.0;
            tiles.push(Tile { min_x: bx, max_x: bx + BLOCK, min_z: bz, max_z: bz + BLOCK, y: y0, kind: if park { TileKind::Park } else { TileKind::Slab } });
            if park { continue; }
            let (cx, cz) = (bx + BLOCK / 2.0, bz + BLOCK / 2.0);
            let falloff = (1.0 - (cx - site.centre.x).hypot(cz - site.centre.z) / (HALF * 1.2)).clamp(0.0, 1.0);
            // lots inside the block: 2 m inset, 2 m gap
            let (a, b, sz) = (bx + 2.0, bz + 2.0, BLOCK - 4.0);
            let half = (sz - 2.0) / 2.0;
            let lots: Vec<(f32, f32, f32, f32)> = match (hash01(key ^ 1, i, j) * 3.0) as u32 {
                0 => vec![(a, b, a + sz, b + sz)],
                1 => if hash01(key ^ 2, i, j) < 0.5 { vec![(a, b, a + half, b + sz), (a + half + 2.0, b, a + sz, b + sz)] } else { vec![(a, b, a + sz, b + half), (a, b + half + 2.0, a + sz, b + sz)] },
                _ => vec![(a, b, a + half, b + half), (a + half + 2.0, b, a + sz, b + half), (a, b + half + 2.0, a + half, b + sz), (a + half + 2.0, b + half + 2.0, a + sz, b + sz)],
            };
            for (n, (x0, z0, x1, z1)) in lots.into_iter().enumerate() {
                let hh = hash01(key ^ 3, i * 4 + n as i32, j);
                let mut height = 8.0 + 82.0 * falloff * falloff * hh;
                if falloff < 0.35 && hash01(key ^ 4, i * 4 + n as i32, j) < 0.15 { height = 4.0 + 2.0 * hh; }
                let height = height.clamp(4.0, 90.0);
                buildings.push(Building { min: v3(x0, y0, z0), max: v3(x1, y0 + height, z1), color: (hash01(key ^ 5, i * 4 + n as i32, j) * 6.0) as u8 % 6 });
                if height >= 20.0 && x1 - x0 >= 14.0 && z1 - z0 >= 14.0 { roofs.push(v3((x0 + x1) / 2.0, y0 + height, (z0 + z1) / 2.0)); }
            }
        } }
        City::with(site, buildings, tiles, roofs)
    }

    pub fn from_buildings(site: CitySite, buildings: Vec<Building>) -> City { City::with(site, buildings, Vec::new(), Vec::new()) }

    fn with(site: CitySite, buildings: Vec<Building>, tiles: Vec<Tile>, roofs: Vec<V3>) -> City {
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        let (ox, oz) = (site.centre.x - site.half, site.centre.z - site.half);
        for (idx, b) in buildings.iter().enumerate() {
            let (c0, c1) = (((b.min.x - ox) / PITCH).floor() as i32, ((b.max.x - ox) / PITCH).floor() as i32);
            let (r0, r1) = (((b.min.z - oz) / PITCH).floor() as i32, ((b.max.z - oz) / PITCH).floor() as i32);
            for r in r0..=r1 { for c in c0..=c1 { grid.entry((c, r)).or_default().push(idx); } }
        }
        City { site, buildings, tiles, roofs, grid }
    }

    fn cell(&self, x: f32, z: f32) -> (i32, i32) {
        (((x - (self.site.centre.x - self.site.half)) / PITCH).floor() as i32, ((z - (self.site.centre.z - self.site.half)) / PITCH).floor() as i32)
    }

    pub fn inside(&self, x: f32, z: f32) -> bool { (x - self.site.centre.x).abs() <= self.site.half && (z - self.site.centre.z).abs() <= self.site.half }

    pub fn roof_at(&self, x: f32, z: f32) -> Option<f32> {
        let c = self.cell(x, z);
        self.grid.get(&c)?.iter().map(|&i| &self.buildings[i]).find(|b| x >= b.min.x && x <= b.max.x && z >= b.min.z && z <= b.max.z).map(|b| b.max.y)
    }

    pub fn buildings_near(&self, p: V3, r: f32) -> Vec<&Building> {
        let (c0, r0) = self.cell(p.x - r, p.z - r); let (c1, r1) = self.cell(p.x + r, p.z + r);
        let mut seen: Vec<usize> = Vec::new();
        for row in r0..=r1 { for col in c0..=c1 { if let Some(v) = self.grid.get(&(col, row)) { for &i in v { if !seen.contains(&i) { seen.push(i); } } } } }
        seen.into_iter().map(|i| &self.buildings[i]).collect()
    }

    /// The centre of a street intersection chosen by `key`.
    pub fn street_point(&self, key: u64) -> V3 {
        let n = BLOCKS + 1;
        let i = (hash01(key, 7, 1) * n as f32) as i32 % n; let j = (hash01(key, 7, 2) * n as f32) as i32 % n;
        v3(self.site.centre.x - self.site.half + i as f32 * PITCH, self.site.centre.y, self.site.centre.z - self.site.half + j as f32 * PITCH)
    }

    /// Deepest overlap of a sphere with a building: (push-out vector, surface normal).
    pub fn push_out(&self, p: V3, r: f32) -> Option<(V3, V3)> { self.overlap(p, r, true) }

    fn overlap(&self, p: V3, r: f32, roofs: bool) -> Option<(V3, V3)> {
        let mut best: Option<(f32, V3, V3)> = None;
        for b in self.buildings_near(p, r + 1.0) {
            let q = v3(p.x.clamp(b.min.x, b.max.x), p.y.clamp(b.min.y, b.max.y), p.z.clamp(b.min.z, b.max.z));
            let d = p - q; let dist = d.len();
            let (depth, normal) = if dist > 1e-5 {
                if dist >= r { continue; }
                let n = d * (1.0 / dist);
                if !roofs && n.y > 0.5 { continue; }               // the car never rides on a roof: ignore contacts from above
                (r - dist, n)
            } else {
                // centre inside the box: leave through the nearest face (the top face only when roofs count)
                let mut faces = vec![(p.x - b.min.x, v3(-1., 0., 0.)), (b.max.x - p.x, v3(1., 0., 0.)), (p.z - b.min.z, v3(0., 0., -1.)), (b.max.z - p.z, v3(0., 0., 1.))];
                if roofs { faces.push((b.max.y - p.y, v3(0., 1., 0.))); }
                let (pen, n) = faces.iter().cloned().fold((f32::MAX, V3::ZERO), |a, f| if f.0 < a.0 { f } else { a });
                (pen + r, n)
            };
            if best.map_or(true, |(bd, _, _)| depth > bd) { best = Some((depth, normal * depth, normal)); }
        }
        best.map(|(_, push, n)| (push, n))
    }

    pub fn touches_side(&self, p: V3, r: f32) -> bool {
        self.buildings_near(p, r + 1.0).into_iter().any(|b| {
            let q = v3(p.x.clamp(b.min.x, b.max.x), p.y.clamp(b.min.y, b.max.y), p.z.clamp(b.min.z, b.max.z));
            let d = p - q; let dist = d.len();
            if dist >= r { return false; }
            if dist < 1e-5 { return true; }
            (d.y / dist) < 0.7
        })
    }

    /// Pushes the car hull (three spheres along the body) out of the buildings sideways, removes the velocity into the
    /// wall, damps the slip along it and the rotation (so glancing hits do not trip the car).
    pub fn resolve_hull(&self, pos: &mut V3, vel: &mut V3, omega: &mut V3, rot: Q) {
        for _ in 0..3 {
            let mut moved = false;
            for z in [-1.4f32, 0.0, 1.4] {
                let c = *pos + rot.rotate(v3(0.0, 0.5, z));
                if let Some((push, n)) = self.overlap(c, 1.0, false) {
                    *pos = *pos + push;
                    let vn = vel.dot(n);
                    if vn < 0.0 { *vel = *vel - n * (vn * 1.2); }
                    let tangent = *vel - n * vel.dot(n);
                    *vel = *vel - tangent * 0.4;
                    *omega = *omega * 0.4;
                    moved = true;
                }
            }
            if !moved { break; }
        }
    }
}

/// Picks one or two city sites by seed: the flattest of 12 hashed candidates, then the flattest one at least 800 m from it.
pub fn city_sites(seed: u64, raw: &Terrain) -> [Option<CitySite>; 2] {
    let key = seed ^ 0xC17;
    let mut cands: Vec<(f32, CitySite)> = (0..12i32).map(|k| {
        let a = hash01(key, k, 91) * std::f32::consts::TAU;
        let d = 600.0 + 900.0 * hash01(key, k, 92);
        let (cx, cz) = (a.cos() * d, a.sin() * d);
        let samples: Vec<f32> = (-1..=1).flat_map(|i| (-1..=1).map(move |j| (i, j)))
            .map(|(i, j)| raw.raw_height(cx + i as f32 * HALF * 0.8, cz + j as f32 * HALF * 0.8)).collect();
        let spread = samples.iter().cloned().fold(f32::MIN, f32::max) - samples.iter().cloned().fold(f32::MAX, f32::min);
        let base = samples.iter().sum::<f32>() / samples.len() as f32;
        (spread, CitySite { centre: v3(cx, base, cz), half: HALF })
    }).collect();
    cands.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let first = cands[0].1;
    let mut out = [Some(first), None];
    if hash01(key, 90, 0) >= 0.6 {
        out[1] = cands.iter().skip(1).map(|c| c.1).find(|s| {
            let (dx, dz) = ((s.centre.x - first.centre.x).abs(), (s.centre.z - first.centre.z).abs());
            (s.centre - first.centre).len() >= 800.0 && (dx >= 2.0 * HALF + 200.0 || dz >= 2.0 * HALF + 200.0)   // rims never overlap
        });
    }
    out
}

#[cfg(test)]
#[path = "city_tests.rs"]
mod tests;
