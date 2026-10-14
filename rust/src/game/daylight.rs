//! Day and night: the sun and the moon travel a full circle; sky, fog and light colours follow the sun's elevation.
//! The world uses ONE light: the sun above -4 degrees, the moon below it (both are zero at the switch, so shadows never jump).
use super::math::{v3, V3};
use super::terrain::hash01;
use std::f32::consts::{PI, TAU};
use std::sync::atomic::{AtomicU32, Ordering};

pub const DAY_SECONDS: f32 = 720.0;
/// Elevation `MID - AMP cos(2 pi phase)`: phase 0 is midnight (-43 deg), 0.5 noon (+67 deg).
pub const MID_DEG: f32 = 12.0;
pub const AMP_DEG: f32 = 55.0;
/// A level starts with the sun between these elevations: never at night.
pub const START_MIN_DEG: f32 = 8.0;
pub const START_MAX_DEG: f32 = 50.0;
const MOON_COL: [f32; 3] = [0.36, 0.44, 0.62];

#[derive(Clone, Copy, Debug)]
pub struct Daylight {
    /// Direction and colour of the one light the world uses (the sun by day, the moon by night).
    pub sun: V3, pub sun_col: [f32; 3],
    pub sky_top: [f32; 3], pub sky_hor: [f32; 3],
    /// Where the sun disc and the moon are in the sky (unit vectors, possibly below the horizon).
    pub sun_disc: V3, pub moon: V3,
    /// 0 by day .. 1 in the deep night.
    pub night: f32,
}

fn mix(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] { [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k] }
fn scale(a: [f32; 3], k: f32) -> [f32; 3] { [a[0] * k, a[1] * k, a[2] * k] }
/// Smoothstep that also works for `a > b` (a falling edge).
fn smooth(a: f32, b: f32, x: f32) -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

pub fn elevation_deg(phase: f32) -> f32 { MID_DEG - AMP_DEG * (TAU * phase).cos() }

/// The phase at which the sun is `deg` high, on the morning (`rising`) or the evening side.
pub fn phase_at_elevation(deg: f32, rising: bool) -> f32 {
    let x = ((deg - MID_DEG) / AMP_DEG).clamp(-1.0, 1.0);
    let p = (-x).acos() / TAU;
    if rising { p } else { 1.0 - p }
}

/// The level's starting phase: a day phase (sun between `START_MIN_DEG` and `START_MAX_DEG`), morning or evening.
pub fn start_phase(seed: u64) -> f32 {
    let e = START_MIN_DEG + (START_MAX_DEG - START_MIN_DEG) * hash01(seed ^ 0xDA7, 3, 5);
    phase_at_elevation(e, hash01(seed ^ 0xDA9, 2, 6) < 0.5)
}
fn azimuth0(seed: u64) -> f32 { hash01(seed ^ 0xDA8, 7, 1) * TAU }
/// The moon trails or leads the exact opposite of the sun by up to 70 degrees: other levels, other phases of the moon.
fn moon_offset(seed: u64) -> f32 { (hash01(seed ^ 0xDAA, 4, 8) * 2.0 - 1.0) * 70f32.to_radians() }

pub fn daylight_at_phase(phase: f32) -> Daylight { daylight_at(phase, 0.9, 0.0) }

pub fn daylight_at(phase: f32, az0: f32, moon_off: f32) -> Daylight {
    let e_deg = elevation_deg(phase);
    let e = e_deg.to_radians();
    let az = az0 + 0.5 * PI * (TAU * phase).sin();
    let sun_disc = v3(e.cos() * az.cos(), e.sin(), e.cos() * az.sin()).norm();
    let (so, co) = moon_off.sin_cos();
    let a = sun_disc * -1.0;
    let moon = v3(a.x * co + a.z * so, a.y, -a.x * so + a.z * co).norm();

    let warm = 1.0 - smooth(10.0, 40.0, e_deg);
    let k = 2.6 - 0.9 * warm;
    let base = mix([1.0, 0.93, 0.80], [1.0, 0.62, 0.36], warm);
    let (sun, sun_col) = if e_deg > -4.0 {
        (sun_disc, scale(base, k * smooth(-4.0, 12.0, e_deg)))
    } else {
        (moon, scale(MOON_COL, smooth(4.0, 14.0, -e_deg)))
    };

    // sky keys by sun elevation: day -> golden hour -> twilight (below 0) -> night (below -6)
    let mut top = mix([0.22, 0.42, 0.80], [0.30, 0.34, 0.62], warm);
    let mut hor = mix([0.70, 0.80, 0.90], [0.96, 0.68, 0.50], warm);
    let t1 = smooth(0.0, -6.0, e_deg);
    top = mix(top, [0.10, 0.12, 0.30], t1); hor = mix(hor, [0.55, 0.28, 0.30], t1);
    let t2 = smooth(-6.0, -16.0, e_deg);
    top = mix(top, [0.012, 0.02, 0.05], t2); hor = mix(hor, [0.03, 0.05, 0.10], t2);
    Daylight { sun, sun_col, sky_top: top, sky_hor: hor, sun_disc, moon, night: smooth(4.0, -14.0, e_deg) }
}

/// Debug/test clock: a fixed start phase for every level (bits of an f32; `u32::MAX` = none). Set over JNI only.
static DEBUG_PHASE: AtomicU32 = AtomicU32::new(u32::MAX);
/// `Some(phase)` makes every level start at `phase` (0 midnight .. 1); `None` (or a negative/NaN value) clears it.
pub fn set_debug_phase(p: Option<f32>) {
    let bits = match p { Some(x) if x.is_finite() && x >= 0.0 => (x % 1.0).to_bits(), _ => u32::MAX };
    DEBUG_PHASE.store(bits, Ordering::Relaxed);
}
fn debug_phase() -> Option<f32> { let b = DEBUG_PHASE.load(Ordering::Relaxed); if b == u32::MAX { None } else { Some(f32::from_bits(b)) } }

/// Daylight `t` seconds into a level; the start phase, the sun's side and the moon's phase come from the level seed.
pub fn daylight(seed: u64, t: f32) -> Daylight {
    let p0 = debug_phase().unwrap_or_else(|| start_phase(seed));
    daylight_at(((p0 + t / DAY_SECONDS) % 1.0 + 1.0) % 1.0, azimuth0(seed), moon_offset(seed))
}

#[cfg(test)]
#[path = "daylight_tests.rs"]
mod tests;
