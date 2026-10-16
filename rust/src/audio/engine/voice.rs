//! The engine as a sound source: the simulation runs on the audio thread, two steps per output sample; the pressure at the exhaust collector goes through the exhaust pipe and what
//! leaves its open end is the tail pipe's sound; the pressure in the intake plenum goes through the intake pipe, and the push of the combustion on the block rattles the engine bay.
use super::pipe::Pipe;
use super::{Controls, Engine, EngineSpec};
use crate::audio::dsp::{coef, OnePole};
use crate::audio::RATE;

/// Simulation steps per output sample.
pub const SUBSTEPS: usize = 2;

pub struct EngineVoice {
    pub engine: Engine,
    exhaust: Pipe, intake: Pipe,
    lp_struct: OnePole, hp_struct: OnePole, lp_noise: OnePole, bov_lo: OnePole, bov_hi: OnePole, whoosh_lo: OnePole, whoosh_hi: OnePole,
    /// The phases of the turbo's whistle and the gearbox's whine (0..1).
    ph_turbo: f32, ph_gear: f32, rev_s: f32,
    last_sum: f64, noise: u32,
    /// The running mean of the exhaust's power: a partial leveller (as the simulator it follows has) keeps the idle audible and the roar from deafening.
    level: f32,
    /// The sound of the tail pipe, the intake and the block (set by `sample`).
    pub exhaust_out: f32, pub intake_out: f32, pub block_out: f32,
    /// The turbo's whistle and whoosh, the blow-off valve's hiss and the gearbox's whine: they come from the engine bay.
    pub turbo_out: f32, pub bov_out: f32, pub whine_out: f32,
}

/// The pressure that makes a unit of sound: the collector's swing in the exhaust is a few tenths of a bar.
const EXHAUST_SCALE: f64 = 1.0 / 1_000.0;
const INTAKE_SCALE: f64 = 1.0 / 500.0;
const BLOCK_SCALE: f64 = 1.0 / 6.0e5;

impl EngineVoice {
    pub fn new(spec: EngineSpec) -> EngineVoice {
        EngineVoice { engine: Engine::new(spec), exhaust: Pipe::new(2.2, 0.55, -0.85, 0.35), intake: Pipe::new(0.55, 0.8, -0.7, 0.25),
            lp_struct: OnePole::default(), hp_struct: OnePole::default(), lp_noise: OnePole::default(), bov_lo: OnePole::default(), bov_hi: OnePole::default(), whoosh_lo: OnePole::default(), whoosh_hi: OnePole::default(), ph_turbo: 0.0, ph_gear: 0.0, rev_s: 0.0, last_sum: 0.0, noise: 0x1357_9BDF, level: 0.01,
            exhaust_out: 0.0, intake_out: 0.0, block_out: 0.0, turbo_out: 0.0, bov_out: 0.0, whine_out: 0.0 }
    }

    pub fn rpm(&self) -> f32 { self.engine.rpm() as f32 }

    pub fn reset(&mut self) { self.exhaust.clear(); self.intake.clear(); self.exhaust_out = 0.0; self.intake_out = 0.0; self.block_out = 0.0; self.turbo_out = 0.0; self.bov_out = 0.0; self.whine_out = 0.0; }

    /// One output sample (44.1 kHz): advances the engine by `SUBSTEPS` steps with the controls and works out the three sounds.
    pub fn sample(&mut self, ctl: &Controls) {
        self.engine.controls = *ctl;
        let dt = 1.0 / (RATE as f64 * SUBSTEPS as f64);
        let (mut pe, mut pi, mut ps) = (0.0, 0.0, 0.0);
        for _ in 0..SUBSTEPS { self.engine.step(dt); pe += self.engine.exhaust_pressure(); pi += self.engine.intake_pressure(); ps += self.engine.cylinder_pressure_sum(); }
        let k = 1.0 / SUBSTEPS as f64;
        let raw = self.exhaust.process((pe * k * EXHAUST_SCALE) as f32);
        self.level += (raw * raw - self.level) * (1.0 / (0.3 * RATE as f32));
        let gain = (0.1 / self.level.sqrt().max(1e-4)).sqrt().clamp(0.5, 4.0);   // the loudness changes by the square root of what the engine really does
        self.exhaust_out = raw * gain;
        // the intake: the plenum's swing, and the rush of the air through the throttle (more when it is open and the engine is breathing hard)
        self.noise = self.noise.wrapping_mul(1664525).wrapping_add(1013904223);
        let n = (self.noise >> 8) as f32 / 8_388_608.0 - 1.0;
        let flow = (self.engine.air_flow / 4.0).clamp(0.0, 1.5) as f32;
        let rush = self.lp_noise.run(n, coef(2500.0, RATE as f32)) * flow.powf(0.8) * 0.03;
        self.intake_out = self.intake.process((pi * k * INTAKE_SCALE) as f32) + rush;
        // the block: the rate of change of the pressure in all the cylinders, taken through the walls (low-passed, with the rumble taken out)
        let d = ps * k - self.last_sum; self.last_sum = ps * k;
        let s = self.lp_struct.run((d * BLOCK_SCALE) as f32, coef(1800.0, RATE as f32));
        self.block_out = s - self.hp_struct.run(s, coef(60.0, RATE as f32));
        let rate = RATE as f32;
        // the turbo: a whistle at the compressor's blade-pass frequency (5 blades of the shaft speed) that grows with the boost and the flow, and a whoosh of the air
        let (air, boost) = ((self.engine.air_flow / 3.0).clamp(0.0, 1.5) as f32, self.engine.boost_bar().max(0.0) as f32);
        let f_t = (self.engine.turbo_omega as f32 / std::f32::consts::TAU * 5.0).clamp(500.0, 14000.0);
        self.ph_turbo = (self.ph_turbo + f_t / rate).fract();
        let spool = ((self.engine.turbo_omega as f32 - 3000.0) / 9000.0).clamp(0.0, 1.0);
        let whistle = (self.ph_turbo * std::f32::consts::TAU).sin() * 0.7 + (self.ph_turbo * 2.0 * std::f32::consts::TAU).sin() * 0.2;
        let whoosh = self.whoosh_lo.run(n, coef(6000.0, rate)) - self.whoosh_hi.run(n, coef(1200.0, rate));
        self.turbo_out = (whistle * 0.030 * spool * (0.35 + 0.65 * air) + whoosh * 0.08 * spool * air) * (0.4 + boost.min(0.8));
        // the blow-off valve: the rush of the air let out of the charge volume
        let bov = (self.engine.bov_flow / 6.0).clamp(0.0, 1.5) as f32;
        let hiss = self.bov_lo.run(n, coef(9000.0, rate)) - self.bov_hi.run(n, coef(2500.0, rate));
        self.bov_out = hiss * bov.sqrt() * 0.7;
        // the gearbox: spur gears whine at a multiple of the revs, louder under torque and when the clutch holds
        self.rev_s += (self.engine.rpm() as f32 / 60.0 - self.rev_s) * 0.003;   // the revs without the ripple of the firing strokes (a gear's whine is steady)
        let rev = self.rev_s;
        self.ph_gear = (self.ph_gear + rev * 21.0 / rate).fract();
        let torque = (self.engine.clutch_torque.abs() / 300.0).clamp(0.0, 1.0) as f32 * ctl.clutch as f32;
        self.whine_out = ((self.ph_gear * std::f32::consts::TAU).sin() + 0.4 * (self.ph_gear * 2.0 * std::f32::consts::TAU).sin()) * 0.012 * (0.15 + torque) * (rev / 100.0).min(1.0);
    }
}

#[cfg(test)]
#[path = "voice_tests.rs"]
mod tests;
