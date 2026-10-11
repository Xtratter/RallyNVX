//! Dynamic daylight: the sun moves along a day arc; sky, fog and light colours follow its elevation. No night.
use super::math::{v3, V3};
use super::terrain::hash01;

pub const DAY_SECONDS: f32 = 720.0;
pub const ELEV_MIN_DEG: f32 = 10.0;
pub const ELEV_MAX_DEG: f32 = 60.0;

#[derive(Clone, Copy, Debug)]
pub struct Daylight { pub sun: V3, pub sun_col: [f32; 3], pub sky_top: [f32; 3], pub sky_hor: [f32; 3] }

fn mix(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] { [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k] }
fn smooth(a: f32, b: f32, x: f32) -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

/// Sun elevation over the cycle: dawn (min) at phase 0, noon (max) at 0.5.
pub fn elevation_deg(phase: f32) -> f32 {
    let mid = (ELEV_MIN_DEG + ELEV_MAX_DEG) * 0.5; let amp = (ELEV_MAX_DEG - ELEV_MIN_DEG) * 0.5;
    mid - amp * (std::f32::consts::TAU * phase).cos()
}

pub fn daylight_at_phase(phase: f32) -> Daylight { daylight_at(phase, 0.9) }

fn daylight_at(phase: f32, azimuth0: f32) -> Daylight {
    let e = elevation_deg(phase).to_radians();
    let az = azimuth0 + 0.5 * std::f32::consts::PI * (std::f32::consts::TAU * phase).sin();
    let sun = v3(e.cos() * az.cos(), e.sin(), e.cos() * az.sin()).norm();
    let warm = 1.0 - smooth(10.0, 40.0, elevation_deg(phase));
    let k = 2.6 - 0.9 * warm;
    let base = mix([1.0, 0.93, 0.80], [1.0, 0.62, 0.36], warm);
    Daylight {
        sun, sun_col: [base[0] * k, base[1] * k, base[2] * k],
        sky_top: mix([0.22, 0.42, 0.80], [0.30, 0.34, 0.62], warm),
        sky_hor: mix([0.70, 0.80, 0.90], [0.96, 0.68, 0.50], warm),
    }
}

/// Daylight `t` seconds into a level; the start phase and the sun's side come from the level seed.
pub fn daylight(seed: u64, t: f32) -> Daylight {
    let phase0 = hash01(seed ^ 0xDA7, 3, 5);
    let az0 = hash01(seed ^ 0xDA8, 7, 1) * std::f32::consts::TAU;
    daylight_at(((phase0 + t / DAY_SECONDS) % 1.0 + 1.0) % 1.0, az0)
}

#[cfg(test)]
#[path = "daylight_tests.rs"]
mod tests;
