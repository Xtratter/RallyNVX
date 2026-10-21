//! A tyre's temperature, wear and pressure, and the grip they leave. The tread heats with the friction work (the force times the slip speed, in both directions) and the rolling hysteresis,
//! cools in the air faster the faster the car goes, grips best inside a thermal window that depends on the set (tarmac tyres like it hot, snow tyres cold), loses grip as the pressure leaves
//! its optimum (the gas inside heats with the tyre) and as the tread wears out (faster on rough ground and with the wrong set).
use super::surface::Surface;

/// The heat capacity of the tread and the carcass layer that follows it (J/K) and the work (J) that wears a fresh tyre out on the base surface.
const HEAT_CAP: f32 = 5000.0;
const WEAR_J: f32 = 2.5e7;

/// The best tread temperature and the half width of the window (degrees C) of the wheel set: 0 tarmac, 1 gravel, 2 snow.
pub fn window(set: u8) -> (f32, f32) { match set { 1 => (55.0, 50.0), 2 => (10.0, 35.0), _ => (85.0, 45.0) } }
/// The cold pressure (bar) at which a tyre that is at its best temperature has its best hot pressure.
pub fn cold_default(set: u8) -> f32 { match set { 1 => 1.41, 2 => 1.53, _ => 1.69 } }
fn best_bar(set: u8) -> f32 { match set { 1 => 1.6, 2 => 1.5, _ => 2.1 } }
/// The gas inside is heated with the tread: the pressure from the cold one set at 15 degrees C.
pub fn hot_pressure(cold_bar: f32, temp: f32) -> f32 { cold_bar * (temp + 273.0) / 288.0 }

/// The grip of the tyre as a multiplier (about 0.4 to 1): the thermal window, the pressure and the wear.
pub fn grip(set: u8, temp: f32, wear: f32, cold_bar: f32) -> f32 {
    let (opt, half) = window(set);
    let temp = if temp.is_finite() { temp } else { opt };
    let gt = (1.0 - 0.125 * ((temp - opt) / half).powi(2)).clamp(0.55, 1.0);
    let bar = if cold_bar.is_finite() { cold_bar } else { cold_default(set) };
    let po = best_bar(set);
    let gp = (1.0 - 4.0 * ((hot_pressure(bar, temp) - po) / po).powi(2)).clamp(0.7, 1.0);
    let w = if wear.is_finite() { wear.clamp(0.0, 1.0) } else { 0.0 };
    let gw = 1.0 - 0.25 * w - 0.15 * ((w - 0.8) / 0.2).max(0.0);
    gt * gp * gw
}

/// How hard a surface is on the tread (1 on tarmac), more with the wrong set (`set_match` is the grip the set has on it over the best one, 1 = the right one).
pub fn abrasiveness(s: Surface, set_match: f32) -> f32 {
    let base = match s { Surface::Asphalt => 1.0, Surface::Gravel => 1.6, Surface::Dirt => 1.3, Surface::Grass => 0.6, Surface::Sand => 1.8, Surface::Rock => 2.5, Surface::Water => 0.5 };
    let m = if set_match.is_finite() { set_match.clamp(0.0, 1.0) } else { 1.0 };
    base * (1.0 + 3.0 * (1.0 - m))
}

#[derive(Clone, Copy, Debug)]
pub struct Tyre { pub temp: f32, pub wear: f32 }

impl Tyre {
    /// A new tyre at its best temperature (the liaison section warmed it).
    pub fn warm(set: u8) -> Tyre { Tyre { temp: window(set).0, wear: 0.0 } }

    /// One step of `h` s: `work_w` the friction work (W) of the wheel, `v` the car's speed (m/s), `fz` the load (N), `air_c` the air's temperature, `abrasion` the surface's.
    pub fn step(&mut self, h: f32, work_w: f32, v: f32, fz: f32, air_c: f32, abrasion: f32) {
        if !h.is_finite() || h <= 0.0 { return; }
        let h = h.min(0.1);
        let work = if work_w.is_finite() { work_w.clamp(0.0, 2.0e5) } else { 0.0 };
        let speed = if v.is_finite() { v.abs().min(120.0) } else { 0.0 };
        let load = if fz.is_finite() { fz.clamp(0.0, 2.0e4) } else { 0.0 };
        let air = if air_c.is_finite() { air_c.clamp(-40.0, 50.0) } else { 15.0 };
        let heat = work + 0.012 * load * speed;
        let cool = (12.0 + 3.5 * speed) * (self.temp - air);
        self.temp = (self.temp + h * (heat - cool) / HEAT_CAP).clamp(-60.0, 250.0);
        let ab = if abrasion.is_finite() { abrasion.clamp(0.0, 10.0) } else { 1.0 };
        self.wear = (self.wear + h * work * ab / WEAR_J).clamp(0.0, 1.0);
    }
}

#[cfg(test)]
#[path = "tyre_tests.rs"]
mod tests;
