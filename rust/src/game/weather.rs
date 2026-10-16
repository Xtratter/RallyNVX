//! The weather of a level: clear, cloudy, rain, heavy rain or fog, chosen from the seed ("auto") or by the player, blended smoothly, with the wet ground that stays
//! after the rain. Its numbers feed the sky and fog (`daylight`), the rain pass, the wet shading, the grip (`surfaces.json` -> `wet`) and the dust.
use super::daylight::Daylight;
use super::terrain::hash01;
use std::sync::atomic::{AtomicU8, Ordering};

/// The player's choice: 0 auto (from the level's seed), 1 clear, 2 cloudy, 3 rain, 4 heavy rain, 5 fog.
pub const MODES: u8 = 6;
static MODE: AtomicU8 = AtomicU8::new(0);
pub fn set_mode(m: u8) { MODE.store(m.min(MODES - 1), Ordering::Relaxed); }
pub fn mode() -> u8 { MODE.load(Ordering::Relaxed) }

/// What the sky and the ground are like right now: every field 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Weather { pub rain: f32, pub cloud: f32, pub fog: f32, pub wet: f32 }

/// The weather a mode asks for (auto: from the seed: 50 % clear, 22 % cloudy, 18 % rain, 5 % heavy rain, 5 % fog).
pub fn target(mode: u8, seed: u64) -> Weather {
    let m = if mode == 0 {
        let r = hash01(seed ^ 0x57EA, 11, 3);
        if r < 0.50 { 1 } else if r < 0.72 { 2 } else if r < 0.90 { 3 } else if r < 0.95 { 4 } else { 5 }
    } else { mode.min(MODES - 1) };
    match m {
        2 => Weather { rain: 0.0, cloud: 0.7, fog: 0.0, wet: 0.0 },
        3 => Weather { rain: 0.5, cloud: 0.9, fog: 0.15, wet: 0.0 },
        4 => Weather { rain: 1.0, cloud: 1.0, fog: 0.3, wet: 0.0 },
        5 => Weather { rain: 0.0, cloud: 0.6, fog: 0.9, wet: 0.0 },
        _ => Weather::default(),
    }
}

/// The weather over time: it moves towards the target of the chosen mode (the rain and the clouds in a few seconds), the ground gets wet fast in rain and dries in minutes.
#[derive(Clone, Copy, Debug)]
pub struct Climate { seed: u64, mode: u8, now: Weather }

impl Climate {
    /// A level's weather at its start (the ground is already wet if it is raining).
    pub fn new(seed: u64) -> Climate { Climate::with_mode(seed, mode()) }
    pub fn with_mode(seed: u64, mode: u8) -> Climate {
        let mut now = target(mode, seed);
        now.wet = now.rain.max(if now.fog > 0.5 { 0.3 } else { 0.0 });
        Climate { seed, mode, now }
    }
    pub fn weather(&self) -> Weather { self.now }
    pub fn step(&mut self, dt: f32) { self.step_with(dt, mode()); }
    pub fn step_with(&mut self, dt: f32, m: u8) {
        if !dt.is_finite() || dt <= 0.0 { return; }
        let dt = dt.min(0.25);
        self.mode = m.min(MODES - 1);
        let t = target(self.mode, self.seed);
        let approach = |x: f32, to: f32, rate: f32| x + (to - x).clamp(-rate * dt, rate * dt);
        self.now.rain = approach(self.now.rain, t.rain, 0.25);
        self.now.cloud = approach(self.now.cloud, t.cloud, 0.2);
        self.now.fog = approach(self.now.fog, t.fog, 0.15);
        // wet follows the rain up quickly and dries slowly (about 100 s from soaked)
        let wet_to = self.now.rain.max(if self.now.fog > 0.5 { 0.3 } else { 0.0 });
        self.now.wet = if wet_to > self.now.wet { approach(self.now.wet, wet_to, 0.3) } else { approach(self.now.wet, wet_to, 0.01) };
    }
}

fn luma(c: [f32; 3]) -> f32 { 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2] }

/// The sky and the light under the weather: the sun's light dims, the sky greys and darkens, the sun disc fades (clouds, rain and fog all count, the night stays dark).
pub fn apply(dl: &Daylight, w: Weather) -> Daylight {
    let veil = w.cloud.max(w.rain).max(0.8 * w.fog).clamp(0.0, 1.0);
    if veil <= 0.0 { return *dl; }
    let mut out = *dl;
    let grey = |c: [f32; 3], k: f32| -> [f32; 3] { let l = luma(c); [0, 1, 2].map(|i| c[i] + (l * 0.9 - c[i]) * k * 0.8 + 0.0) };
    out.sky_top = grey(dl.sky_top, veil).map(|x| x * (1.0 - 0.3 * veil));
    out.sky_hor = grey(dl.sky_hor, veil).map(|x| x * (1.0 - 0.15 * veil));
    out.sun_col = dl.sun_col.map(|x| x * (1.0 - 0.6 * veil));
    out.sun_disc = dl.sun_disc;
    out
}

/// How the sky's veil dims the sun disc and the moon: 0 (hidden) .. 1 (clear sky).
pub fn disc_visibility(w: Weather) -> f32 { (1.0 - 1.4 * w.cloud.max(w.rain).max(w.fog)).clamp(0.0, 1.0) }

/// The fog coefficient of the renderer: the clear value `1/330`, up to 4x in heavy rain and 10x in thick fog.
pub fn fog_k(w: Weather) -> f32 { (1.0 / 330.0) * (1.0 + 3.0 * w.rain + 9.0 * w.fog) }

#[cfg(test)]
#[path = "weather_tests.rs"]
mod tests;
