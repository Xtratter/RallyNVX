//! Generated tileable material maps (RGBA8) and their mip chains. No files: everything comes from fixed seeds.
use crate::game::terrain::hash01;

pub const SIZE: usize = 256;
pub const LAYERS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material { Grass, Dirt, Sand, Rock, Asphalt }
impl Material { pub const ALL: [Material; 5] = [Material::Grass, Material::Dirt, Material::Sand, Material::Rock, Material::Asphalt]; }

fn smooth(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

/// Value noise on a lattice of `period` x `period` cells that wraps around, so the result tiles.
fn tile_noise(seed: u64, x: f32, y: f32, period: i32) -> f32 {
    let (fx, fy) = (x.floor(), y.floor());
    let (ix, iy) = (fx as i32, fy as i32);
    let (u, v) = (smooth(x - fx), smooth(y - fy));
    let w = |a: i32, b: i32| hash01(seed, a.rem_euclid(period), b.rem_euclid(period));
    let top = w(ix, iy) + (w(ix + 1, iy) - w(ix, iy)) * u;
    let bot = w(ix, iy + 1) + (w(ix + 1, iy + 1) - w(ix, iy + 1)) * u;
    top + (bot - top) * v
}

/// Tileable fBm in 0..1 over the unit square; `base` is the lattice period of the first octave.
fn fbm(seed: u64, x: f32, y: f32, base: i32, octaves: u32) -> f32 {
    let (mut sum, mut norm, mut amp, mut p) = (0.0, 0.0, 1.0, base);
    for o in 0..octaves { sum += amp * tile_noise(seed.wrapping_add(o as u64 * 977), x * p as f32, y * p as f32, p); norm += amp; amp *= 0.5; p *= 2; }
    sum / norm
}

pub fn make(m: Material, size: usize) -> Vec<u8> {
    let mut out = vec![255u8; size * size * 4];
    for py in 0..size { for px in 0..size {
        let (x, y) = (px as f32 / size as f32, py as f32 / size as f32);
        let broad = fbm(11, x, y, 4, 4);
        let fine = fbm(23, x, y, 32, 2);
        let col: [f32; 3] = match m {
            Material::Grass => { let t = (0.35 + 0.8 * broad) * (0.85 + 0.3 * fine); let tuft = (fbm(31, x, y, 64, 1) - 0.5) * 0.35; [0.26 * t + tuft * 0.2, 0.46 * t + tuft, 0.14 * t] }
            Material::Dirt => { let t = 0.55 + 0.7 * broad; let pebble = if fine > 0.78 { 0.18 } else { 0.0 }; [0.45 * t + pebble, 0.33 * t + pebble * 0.9, 0.21 * t + pebble * 0.7] }
            Material::Sand => { let ripple = (x * 18.0 * std::f32::consts::TAU + broad * 6.0).sin() * 0.04; let t = 0.85 + 0.3 * broad + ripple; [0.80 * t, 0.71 * t, 0.50 * t] }
            Material::Rock => { let strata = ((y * 6.0 * std::f32::consts::TAU + broad * 5.0).sin() * 0.5 + 0.5) * 0.35 + 0.65; let t = strata * (0.8 + 0.4 * fine); [0.47 * t, 0.45 * t, 0.42 * t] }
            Material::Asphalt => { let speck = if fine > 0.8 { 0.12 } else { 0.0 }; let t = 0.8 + 0.25 * broad; [0.17 * t + speck, 0.17 * t + speck, 0.18 * t + speck] }
        };
        let i = (py * size + px) * 4;
        for c in 0..3 { out[i + c] = (col[c].clamp(0.0, 1.0) * 255.0 + 0.5) as u8; }
    } }
    out
}

pub fn mips(level0: &[u8], size: usize) -> Vec<Vec<u8>> {
    let mut levels = vec![level0.to_vec()];
    let mut s = size;
    while s > 1 {
        let prev = levels.last().unwrap(); let n = s / 2;
        let mut next = vec![255u8; n * n * 4];
        for y in 0..n { for x in 0..n { for c in 0..3 {
            let at = |xx: usize, yy: usize| prev[(yy * s + xx) * 4 + c] as u32;
            next[(y * n + x) * 4 + c] = ((at(2 * x, 2 * y) + at(2 * x + 1, 2 * y) + at(2 * x, 2 * y + 1) + at(2 * x + 1, 2 * y + 1) + 2) / 4) as u8;
        } } }
        levels.push(next); s = n;
    }
    levels
}

#[cfg(test)]
#[path = "texgen_tests.rs"]
mod tests;
