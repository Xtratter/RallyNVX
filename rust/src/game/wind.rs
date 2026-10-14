//! The level's wind: one slowly wandering direction and speed with gusts, shared by the dust, the clouds, the sway of the
//! plants and the wind sound (see `surfaces.json`, `wind`).
use super::grip;
use super::math::{v3, V3};
use super::terrain::hash01;

/// The mean wind (horizontal, m/s): a direction from the seed that wanders over minutes and a speed between 0.6 and 1.0
/// of the table's `speed`.
pub fn wind_at(seed: u64, time: f32) -> V3 {
    let t = if time.is_finite() { time } else { 0.0 };
    let a = hash01(seed ^ 0x71D, 1, 2) * std::f32::consts::TAU + 0.6 * (t * 0.011).sin() + 0.4 * (t * 0.0043 + 1.0).sin();
    let k = 0.7 + 0.3 * (t * 0.037 + 2.0).sin();
    let speed = grip::table().wind_speed * (0.6 + 0.4 * (0.5 + 0.5 * k)).min(1.0);
    v3(a.cos() * speed, 0.0, a.sin() * speed)
}

/// The gust factor: 1 +- `gust` varying over a few seconds to a quarter of a minute.
pub fn gust(seed: u64, time: f32) -> f32 {
    let t = if time.is_finite() { time } else { 0.0 };
    let g = grip::table().wind_gust;
    let s = hash01(seed ^ 0x6057, 3, 4) * 10.0;
    let w = 0.5 * (t * 0.43 + s).sin() + 0.3 * (t * 0.97 + 2.0 * s).sin() + 0.2 * (t * 0.23 + 3.0 * s).sin();
    1.0 + g * w
}

/// The wind the world feels now: the mean wind times the gust factor.
pub fn wind_now(seed: u64, time: f32) -> V3 { wind_at(seed, time) * gust(seed, time) }

/// How far the clouds have drifted (in the sky shader's noise units) after `dt` seconds of this wind.
pub const CLOUD_DRIFT: f32 = 0.005;

#[cfg(test)]
#[path = "wind_tests.rs"]
mod tests;
