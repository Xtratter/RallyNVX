//! The exhaust gas itself: what comes out of the tail pipe all the time, not only when it burns. Hot exhaust is invisible; the water in it (about one part in eight of the gas) condenses
//! into a white vapour where it meets cold or humid air, so the plume shows on a cold morning, at night, in the rain and in the fog, and hardly at all on a dry hot day; a rich mixture
//! (unburnt fuel, the anti-lag's) makes dark soot. The mass flow of the gas comes from the engine model (moles out of the collector), so it follows the revs and the load: a thin wisp at idle, a
//! thick plume at full throttle. Each puff leaves at the speed the flow gives, slows in the air, rises while it is hot, spreads as it takes in air and thins to nothing.
use crate::game::math::{v3, V3};
use super::dustgen::DustInst;

pub const MAX_PUFFS: usize = 480;
/// The seconds a puff of vapour lasts at most.
const LIFE: f32 = 2.4;

#[derive(Clone, Copy, Debug)]
pub struct Puff { pub pos: V3, pub vel: V3, pub temp: f32, pub size: f32, pub size0: f32, pub age: f32, pub seed: f32, pub strength: f32, pub soot: f32 }

/// What the air is like: its temperature (degrees C) and how humid it is (0..1).
#[derive(Clone, Copy, Debug)]
pub struct Air { pub celsius: f32, pub humidity: f32 }

impl Air {
    /// An estimate from the sky: the sun's height (a cold night, a warm noon), the clouds, the rain and the fog.
    pub fn from_sky(sun_height: f32, cloud: f32, rain: f32, fog: f32) -> Air {
        let warm = (sun_height * 1.6).clamp(0.0, 1.0);
        Air { celsius: 5.0 + 17.0 * warm - 3.0 * cloud - 2.0 * rain, humidity: (0.40 + 0.25 * cloud + 0.45 * rain + 0.5 * fog).clamp(0.0, 1.0) }
    }
    /// How much of the exhaust's water shows as vapour (0..1): the colder and the more humid the air the more.
    pub fn vapour(&self) -> f32 { (((20.0 - self.celsius) / 24.0).clamp(0.0, 1.0) * (0.3 + 0.7 * self.humidity) + 0.03).clamp(0.0, 1.0) }
}

#[derive(Default)]
pub struct Smoke { pub parts: Vec<Puff>, rng: u32 }

impl Smoke {
    pub fn new() -> Smoke { Smoke { parts: Vec::new(), rng: 0x85EB_CA6B } }
    fn rand(&mut self) -> f32 { self.rng ^= self.rng << 13; self.rng ^= self.rng >> 17; self.rng ^= self.rng << 5; (self.rng >> 8) as f32 / 16_777_216.0 }

    /// The gas that left the pipe in the last `dt` seconds: `moles` of it at the mean temperature `kelvin`, of which `fuel` moles were unburnt fuel. It comes out of `tip` along `dir` (unit,
    /// backwards), on top of the car's speed.
    pub fn emit(&mut self, tip: V3, dir: V3, moles: f32, kelvin: f32, fuel: f32, dt: f32, car_vel: V3, air: Air) {
        if !(moles > 1e-6) || !moles.is_finite() || !(dt > 1e-4) { return; }
        let vis = air.vapour();
        // soot: the share of the gas that is unburnt fuel, with a dark smoke at a few parts in a hundred
        let soot = (fuel / moles * 25.0).clamp(0.0, 1.0);
        let strength = (vis * 0.55 + soot * 0.9).clamp(0.0, 1.0);
        if strength < 0.02 { return; }   // invisible gas costs nothing
        let n = ((moles / 1.2e-3).ceil() as usize).clamp(1, 20);
        // the speed of the jet: the mass flow (0.029 kg/mol) over the density of the hot gas and the pipe's area (a 50 mm pipe)
        let rho = 101_325.0 * 0.029 / (8.314 * kelvin.clamp(300.0, 1500.0));
        let speed = ((moles / dt * 0.029) / (rho * 0.002)).clamp(2.0, 55.0);
        let side = dir.cross(v3(0.0, 1.0, 0.0)); let side = if side.len() > 1e-3 { side.norm() } else { v3(1.0, 0.0, 0.0) };
        let up = side.cross(dir);
        for k in 0..n {
            if self.parts.len() >= MAX_PUFFS { self.parts.remove(0); }
            let (a, b, v) = (self.rand() - 0.5, self.rand() - 0.5, 0.8 + 0.4 * self.rand());
            let d = (dir + side * (a * 0.12) + up * (b * 0.12)).norm();
            let along = (k as f32 + self.rand()) / n as f32 * speed * dt;   // spread along the path the gas took in this time
            let s0 = 0.06 + 0.05 * (speed / 55.0);
            let seed = self.rand();
            self.parts.push(Puff { pos: tip + d * along, vel: d * (speed * v) + car_vel, temp: kelvin.clamp(300.0, 1500.0), size: s0, size0: s0, age: 0.0, seed, strength, soot });
        }
    }

    /// The puffs by `dt`: drag against the air (`wind` moves it), buoyancy while hot, the spread and the cooling.
    pub fn update(&mut self, dt: f32, wind: V3, air: Air) {
        let dt = dt.clamp(0.0, 0.05);
        let ta = air.celsius + 273.15;
        for p in self.parts.iter_mut() {
            p.age += dt;
            let rel = p.vel - wind;
            let k = 3.0 + 0.06 * rel.len();
            p.vel = p.vel - rel * (k * dt).min(0.95);
            p.vel.y += 9.81 * (p.temp / ta - 1.0).max(0.0) * 0.12 * dt;
            p.pos = p.pos + p.vel * dt;
            p.size += (0.07 * rel.len() + 0.22) * dt;
            p.temp = ta + (p.temp - ta) * (-dt / 0.6).exp();
        }
        self.parts.retain(|p| p.age < LIFE && p.size < 3.0);
    }

    pub fn is_active(&self) -> bool { !self.parts.is_empty() }

    /// The puffs for the dust pipeline, far ones first (they blend over each other).
    pub fn instances(&self, cam: V3) -> Vec<DustInst> {
        let mut v: Vec<(f32, DustInst)> = self.parts.iter().map(|p| {
            // the vapour evaporates (the whole of its life), and the plume dilutes with the square of its growth
            let dil = (p.size0 / p.size).powf(1.6);
            let fade = (1.0 - p.age / LIFE).clamp(0.0, 1.0);
            let alpha = (p.strength * dil * fade * 1.6).clamp(0.0, 0.85);
            let white = [0.93f32, 0.95, 0.97];
            let tone = p.soot / (p.soot + 0.35).max(1e-3);   // the soot darkens the puff
            let c = [white[0] * (1.0 - tone) + 0.07 * tone, white[1] * (1.0 - tone) + 0.07 * tone, white[2] * (1.0 - tone) + 0.07 * tone];
            ((p.pos - cam).len(), DustInst { p: [p.pos.x, p.pos.y, p.pos.z, p.size], c: [c[0], c[1], c[2], alpha], e: [p.seed, (p.age / LIFE).min(1.0), 0.0, 1.0] })
        }).filter(|x| x.1.c[3] > 0.004).collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        v.into_iter().map(|x| x.1).collect()
    }
}

#[cfg(test)]
#[path = "smoke_tests.rs"]
mod tests;
