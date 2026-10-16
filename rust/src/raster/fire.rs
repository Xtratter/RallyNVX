//! The fire at the tail pipe: hot gas that burns in the exhaust leaves the pipe as a jet and a fireball. Each particle is a parcel of burning gas with its own temperature, speed and
//! size: it leaves at the speed the pipe's overpressure gives, slows by drag, rises as its buoyancy lifts it, grows as it takes in air (the jet spreads) and cools; its colour is that of
//! a black body at its temperature (white-yellow at 2000 K, orange at 1400, dull red below 1000), its brightness goes with the fourth power of the temperature.
use crate::game::math::{v3, V3};

/// The ambient air, K, the flame's temperature in the exhaust (diluted by the exhaust gas), the temperature below which a parcel no longer glows.
const T_AIR: f32 = 300.0;
const T_FLAME: f32 = 2000.0;
const T_DARK: f32 = 750.0;
pub const MAX_PARTS: usize = 256;

#[derive(Clone, Copy, Debug)]
pub struct Parcel { pub pos: V3, pub vel: V3, pub temp: f32, pub size: f32, pub age: f32, pub seed: f32 }

/// The instance data of one particle for the flame pipeline: position and size, colour and strength, and (seed, age 0..1, 0, brightness).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FireInst { pub p: [f32; 4], pub c: [f32; 4], pub e: [f32; 4] }

#[derive(Default)]
pub struct Fire { pub parts: Vec<Parcel>, rng: u32 }

/// The colour of a black body at `t` kelvin (linear 0..1; the fit of Tanner Helland's, good from 1000 to 6500 K).
pub fn blackbody(t: f32) -> [f32; 3] {
    let k = (t / 100.0).clamp(10.0, 100.0);
    let r = if k <= 66.0 { 255.0 } else { 329.698_73 * (k - 60.0).powf(-0.133_204_76) };
    let g = if k <= 66.0 { 99.470_8 * k.ln() - 161.119_57 } else { 288.122_17 * (k - 60.0).powf(-0.075_514_85) };
    let b = if k >= 66.0 { 255.0 } else if k <= 19.0 { 0.0 } else { 138.517_73 * (k - 10.0).ln() - 305.044_8 };
    let c = |v: f32| (v / 255.0).clamp(0.0, 1.0);
    [c(r), c(g), c(b)]
}

impl Fire {
    pub fn new() -> Fire { Fire { parts: Vec::new(), rng: 0x9E37_79B9 } }
    fn rand(&mut self) -> f32 { self.rng ^= self.rng << 13; self.rng ^= self.rng >> 17; self.rng ^= self.rng << 5; (self.rng >> 8) as f32 / 16_777_216.0 }

    /// Fuel burnt in the exhaust since the last call: `moles` of it. Each mole of this fuel gives about 2.5 MJ; the gas it heats (about 2000 K, 1 bar) takes `n R T / p` of volume, so a
    /// pop of 0.002 mol is a fireball of a few centimetres and a spark cut's 0.02 mol one of ten. It leaves the pipe at `tip` along `dir` (unit, backwards) at about 25 to 70 m/s (the pipe's overpressure),
    /// on top of the car's own speed `car_vel`.
    pub fn emit(&mut self, tip: V3, dir: V3, moles: f32, car_vel: V3) {
        if !(moles > 1e-7) || !moles.is_finite() { return; }
        let n = ((moles / 1.2e-4).ceil() as usize).clamp(1, 36);
        let per = moles / n as f32;
        // the volume of the hot gas of a parcel (m^3): n R T / p; its radius
        let vol = per * 8.314 * T_FLAME / 1.0e5;
        let r0 = (vol * 0.75 / std::f32::consts::PI).powf(1.0 / 3.0).clamp(0.02, 0.22);
        let speed = 25.0 + 45.0 * (moles / 0.02).clamp(0.0, 1.0).sqrt();
        let side = dir.cross(v3(0.0, 1.0, 0.0)); let side = if side.len() > 1e-3 { side.norm() } else { v3(1.0, 0.0, 0.0) };
        let up = side.cross(dir);
        for _ in 0..n {
            if self.parts.len() >= MAX_PARTS { self.parts.remove(0); }
            let (a, b, v) = (self.rand() - 0.5, self.rand() - 0.5, 0.65 + 0.7 * self.rand());
            let d = (dir + side * (a * 0.35) + up * (b * 0.35)).norm();
            let along = self.rand() * 0.25;   // the parcels leave one after the other: a jet, not a ball
            let (seed, t0, sz) = (self.rand(), T_FLAME * (0.82 + 0.18 * self.rand()), r0 * (0.7 + 0.6 * self.rand()));
            self.parts.push(Parcel { pos: tip + d * along, vel: d * (speed * v) + car_vel, temp: t0, size: sz, age: 0.0, seed });
        }
    }

    /// Advances the parcels by `dt`: the drag against the still air (`car_vel` is the air's velocity relative to the car, so a moving car leaves the fire behind), the buoyancy, the spread
    /// and the cooling.
    pub fn update(&mut self, dt: f32, air_vel: V3) {
        let dt = dt.clamp(0.0, 0.05);
        for p in self.parts.iter_mut() {
            p.age += dt;
            let rel = p.vel - air_vel;
            let sp = rel.len();
            // drag: a parcel of this size loses its speed in about a tenth of a second
            let k = 2.5 + 0.12 * sp;
            p.vel = p.vel - rel * (k * dt).min(0.95);
            // buoyancy: hot gas rises (the entrainment takes most of it away)
            p.vel.y += 9.81 * (p.temp / T_AIR - 1.0) * 0.03 * dt;
            p.pos = p.pos + p.vel * dt;
            // the jet spreads as it takes in air, and the air cools it (the parcel's heat is shared with more and more air)
            p.size += (0.08 * sp + 0.15) * dt;
            p.temp = T_AIR + (p.temp - T_AIR) * (-dt / (0.10 + 0.25 * p.size)).exp();
        }
        self.parts.retain(|p| p.temp > T_DARK && p.age < 1.2);
    }

    pub fn is_active(&self) -> bool { !self.parts.is_empty() }

    /// The particles for the additive pipeline.
    pub fn instances(&self) -> Vec<FireInst> {
        self.parts.iter().map(|p| {
            let [r, g, b] = blackbody(p.temp);
            let glow = ((p.temp - T_DARK) / (T_FLAME - T_DARK)).clamp(0.0, 1.0);
            // the radiated power goes with T^4; the cloud thins as it spreads (the same light over a larger sprite)
            let bright = (p.temp / T_FLAME).powi(4) * 5.0;
            let thin = (0.12 / (p.size * p.size + 0.01)).clamp(0.12, 1.0);
            FireInst { p: [p.pos.x, p.pos.y, p.pos.z, p.size], c: [r, g, b, glow * thin], e: [p.seed, (p.age / 0.5).min(1.0), 0.0, bright] }
        }).collect()
    }
}

#[cfg(test)]
#[path = "fire_tests.rs"]
mod tests;
