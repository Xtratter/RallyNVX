//! Seeded procedural terrain: hash-based value noise, fBm heights, normals.
use super::math::{v3, V3};

pub const TILE_CELLS: usize = 64;
pub const CELL: f32 = 2.0;
pub const TILE_SIZE: f32 = TILE_CELLS as f32 * CELL;

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

#[derive(Clone, Copy, Debug)]
pub struct Terrain { pub seed: u64, scale: f32 }

impl Terrain {
    pub fn new(seed: u64) -> Terrain { Terrain { seed, scale: 1.0 } }
    /// Height 0 everywhere (tests, vehicle tuning).
    pub fn flat() -> Terrain { Terrain { seed: 0, scale: 0.0 } }

    pub fn height(&self, x: f32, z: f32) -> f32 {
        if self.scale == 0.0 { return 0.0; }
        let big = vnoise(self.seed ^ 0xB16, x / 700.0, z / 700.0);
        let amp = (6.0 + 44.0 * big * big) * self.scale;
        let k = smoothstep(10.0, 70.0, x.hypot(z)); // flat spawn plateau
        fbm(self.seed, x / 140.0, z / 140.0, 5) * amp * k
    }

    pub fn normal(&self, x: f32, z: f32) -> V3 {
        let e = 1.0;
        v3(self.height(x - e, z) - self.height(x + e, z), 2.0 * e, self.height(x, z - e) - self.height(x, z + e)).norm()
    }
}

#[cfg(test)]
#[path = "terrain_tests.rs"]
mod tests;
