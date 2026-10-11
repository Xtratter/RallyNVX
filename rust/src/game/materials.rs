//! Per-vertex ground material weights: grass, dirt, sand, rock (sum 255).
use super::math::V3;
use super::terrain::hash01;

fn smoothstep(a: f32, b: f32, x: f32) -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

/// Smooth value noise over 24 m cells.
fn patch_noise(x: f32, z: f32) -> f32 {
    let (u, v) = (x / 24.0, z / 24.0);
    let (fx, fz) = (u.floor(), v.floor()); let (ix, iz) = (fx as i32, fz as i32);
    let (a, b) = (u - fx, v - fz); let (a, b) = (a * a * (3.0 - 2.0 * a), b * b * (3.0 - 2.0 * b));
    let h = |i: i32, j: i32| hash01(0xD1, i, j);
    let top = h(ix, iz) + (h(ix + 1, iz) - h(ix, iz)) * a; let bot = h(ix, iz + 1) + (h(ix + 1, iz + 1) - h(ix, iz + 1)) * a;
    top + (bot - top) * b
}

fn raw(h: f32, n: V3, x: f32, z: f32) -> [f32; 4] {
    let rock = smoothstep(0.86, 0.74, n.y).max(smoothstep(32.0, 46.0, h) * 0.6).clamp(0.0, 1.0);
    let sand = (1.0 - rock) * smoothstep(-3.0, -7.0, h);
    let dirt_f = (smoothstep(0.55, 0.72, patch_noise(x, z)) * 0.8 + smoothstep(0.95, 0.86, n.y) * 0.5).clamp(0.0, 1.0);
    let dirt = (1.0 - rock - sand).max(0.0) * dirt_f;
    let grass = (1.0 - rock - sand - dirt).max(0.0);
    [grass, dirt, sand, rock]
}

pub fn terrain_weights(h: f32, normal: V3, x: f32, z: f32) -> [u8; 4] {
    let w = raw(h, normal, x, z);
    let total: f32 = w.iter().sum::<f32>().max(1e-6);
    let mut out = [0u8; 4];
    let mut acc = 0u32;
    for i in 0..4 { out[i] = (w[i] / total * 255.0).round() as u8; acc += out[i] as u32; }
    // fix rounding so the sum is exactly 255 (adjust the largest weight)
    let big = (0..4).max_by_key(|&i| out[i]).unwrap();
    out[big] = (out[big] as i32 + 255 - acc as i32).clamp(0, 255) as u8;
    out
}

pub fn rock_dominant(h: f32, normal: V3, x: f32, z: f32) -> bool { terrain_weights(h, normal, x, z)[3] > 128 }

#[cfg(test)]
#[path = "materials_tests.rs"]
mod tests;
