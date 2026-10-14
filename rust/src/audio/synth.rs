//! The synthesiser: smoothed continuous voices (engine, rolling, squeal, rotor, drone, wind) plus a pool of one-shots.
use super::dsp::{coef, frac, Noise, OnePole};
use super::{AudioParams, Event, Machine, Surface, RATE};

const TAU: f32 = std::f32::consts::TAU;
const MAX_VOICES: usize = 8;
const FADE: f32 = 0.05;      // seconds for mute/inactive fades
const SMOOTH: f32 = 0.08;    // parameter smoothing time constant, seconds

/// Fundamental of the engine for `rpm` in 0..1.
pub fn engine_freq(rpm: f32) -> f32 { 40.0 + 140.0 * rpm.clamp(0.0, 1.0) }

#[derive(Clone, Copy, Default)]
struct Voice { kind: u8, t: f32, strength: f32, phase: f32, lp1: OnePole, lp2: OnePole, live: bool }

#[derive(Clone, Copy, Default)]
struct Smoothed { rpm: f32, load: f32, roll: f32, water: f32, slip: f32, wind: f32, car: f32, heli: f32, drone: f32 }

pub struct Synth {
    target: AudioParams, s: Smoothed, master: f32, muted: bool, volume: f32, vol_s: f32,
    noise: Noise, rate: f32, a: f32,
    engine_phase: f32, rotor_phase: f32, rotor_tone: f32, drone_phase: [f32; 4], gurgle: f32, squeal: f32,
    f_roll: OnePole, f_roll_w: OnePole, f_sq1: OnePole, f_sq2: OnePole, f_thump: OnePole, f_whoosh: OnePole, f_wind: OnePole, f_drone: OnePole,
    voices: [Voice; MAX_VOICES],
}

impl Default for Synth { fn default() -> Self { Synth::new() } }

impl Synth {
    pub fn new() -> Synth {
        let rate = RATE as f32;
        Synth {
            target: AudioParams::default(), s: Smoothed::default(), master: 0.0, muted: false, volume: 1.0, vol_s: 1.0,
            noise: Noise::new(0x1234_5678), rate, a: 1.0 - (-1.0 / (SMOOTH * rate)).exp(),
            engine_phase: 0.0, rotor_phase: 0.0, rotor_tone: 0.0, drone_phase: [0.0; 4], gurgle: 0.0, squeal: 0.0,
            f_roll: OnePole::default(), f_roll_w: OnePole::default(), f_sq1: OnePole::default(), f_sq2: OnePole::default(),
            f_thump: OnePole::default(), f_whoosh: OnePole::default(), f_wind: OnePole::default(), f_drone: OnePole::default(),
            voices: [Voice::default(); MAX_VOICES],
        }
    }

    pub fn set_params(&mut self, p: &AudioParams) {
        let c = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        self.target = AudioParams { machine: p.machine, rpm: c(p.rpm), load: c(p.load), roll: c(p.roll), surface: p.surface, slip: c(p.slip), wind: c(p.wind), active: p.active };
    }

    pub fn set_muted(&mut self, m: bool) { self.muted = m; }
    /// Master volume 0..1 (applied after the soft clip, smoothed).
    pub fn set_volume(&mut self, v: f32) { self.volume = if v.is_finite() { v.clamp(0.0, 1.0) } else { 1.0 }; }

    /// Stops everything at once: no one-shots, no machine, inactive (the activity left the game).
    pub fn silence(&mut self) { self.voices = [Voice::default(); MAX_VOICES]; self.target = AudioParams::default(); }

    /// Number of one-shot voices currently sounding.
    pub fn voices(&self) -> usize { self.voices.iter().filter(|v| v.live).count() }

    pub fn push(&mut self, e: Event) {
        if self.muted { return; }
        let (kind, strength) = match e {
            Event::Touchdown(s) => (0u8, s), Event::Crash => (1, 1.0), Event::Splash(s) => (2, s), Event::Checkpoint => (3, 1.0),
            Event::Board => (4, 1.0), Event::Success => (5, 1.0), Event::Fail => (6, 1.0),
        };
        let strength = if strength.is_finite() { strength.clamp(0.0, 1.0) } else { 0.0 };
        let slot = self.voices.iter().position(|v| !v.live)
            .unwrap_or_else(|| (0..MAX_VOICES).max_by(|&a, &b| self.voices[a].t.partial_cmp(&self.voices[b].t).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(0));
        self.voices[slot] = Voice { kind, t: 0.0, strength, live: true, ..Voice::default() };
    }

    fn voice_sample(v: &mut Voice, noise: f32, rate: f32) -> f32 {
        let dt = 1.0 / rate;
        let t = v.t; v.t += dt;
        let note = |f: f32, start: f32, len: f32, t: f32| -> f32 {
            if t < start || t >= start + len { return 0.0; }
            let l = t - start;
            (TAU * f * l).sin() * (l * 120.0).min(1.0) * (-(l / len) * 3.0).exp()
        };
        let (out, end) = match v.kind {
            0 => { // touchdown: falling thump plus a little noise
                v.phase = frac(v.phase + (45.0 + 35.0 * (-t * 18.0).exp()) * dt);
                ((TAU * v.phase).sin() * (-t * 11.0).exp() * 0.5 + (TAU * v.phase * 4.0).sin() * (-t * 14.0).exp() * 0.35 + v.lp1.run(noise, 0.45) * (-t * 22.0).exp() * 0.45, t > 0.5)
            }
            1 => (v.lp1.run(noise, 0.2) * (-t * 3.5).exp() * 2.4 * (1.0 + 0.3 * (t * 90.0).sin()), t > 1.2),
            2 => { let hi = v.lp1.run(noise, 0.5) - v.lp2.run(noise, 0.05); (hi * (-t * 7.0).exp() * 1.4, t > 0.7) }
            3 => (note(660.0, 0.0, 0.14, t) + note(880.0, 0.12, 0.28, t), t > 0.45),
            4 => (note(1000.0, 0.0, 0.07, t) * 0.8, t > 0.1),
            5 => (note(523.0, 0.0, 0.16, t) + note(659.0, 0.15, 0.16, t) + note(784.0, 0.30, 0.4, t), t > 0.75),
            _ => (note(330.0, 0.0, 0.24, t) + note(247.0, 0.22, 0.4, t), t > 0.65),
        };
        if end { v.live = false; }
        out * (0.25 + 0.75 * v.strength).min(1.0) * 0.55
    }

    /// Renders float samples in -1..1 (kept for tests; `render` converts).
    pub fn render_f32(&mut self, out: &mut [f32]) {
        let (rate, a) = (self.rate, self.a);
        let c_roll = coef(1500.0, rate); let c_roll_w = coef(600.0, rate);
        let (c_s1, c_s2) = (coef(1400.0, rate), coef(900.0, rate));
        let (c_thump, c_whoosh, c_wind, c_drone) = (coef(700.0, rate), coef(1200.0, rate), coef(500.0, rate), coef(2000.0, rate));
        let fade = 1.0 / (FADE * rate);
        for o in out.iter_mut() {
            let t = self.target;
            self.s.rpm += (t.rpm - self.s.rpm) * a; self.s.load += (t.load - self.s.load) * a;
            self.s.roll += (t.roll - self.s.roll) * a; self.s.slip += (t.slip - self.s.slip) * a; self.s.wind += (t.wind - self.s.wind) * a;
            self.s.water += ((t.surface == Surface::Water) as u8 as f32 - self.s.water) * a;
            self.s.car += ((t.machine == Machine::Car) as u8 as f32 - self.s.car) * a;
            self.s.heli += ((t.machine == Machine::Heli) as u8 as f32 - self.s.heli) * a;
            self.s.drone += ((t.machine == Machine::Drone) as u8 as f32 - self.s.drone) * a;
            let goal = if self.muted || !t.active { 0.0 } else { 1.0 };
            self.master = if self.master < goal { (self.master + fade).min(goal) } else { (self.master - fade).max(goal) };
            let n = self.noise.next();
            let mut x = 0.0f32;
            // car: engine stack, rolling noise, squeal
            if self.s.car > 0.001 {
                self.engine_phase = frac(self.engine_phase + engine_freq(self.s.rpm) / rate);
                let p = self.engine_phase * TAU;
                let w = [0.6f32, 0.7, 1.0, 0.8, 0.6, 0.4];   // the mids carry the sound on a phone speaker
                let mut e = 0.0; for (k, wk) in w.iter().enumerate() { e += (p * (k + 1) as f32).sin() * wk; }
                let engine = (e * 0.30 + n * 0.04) * (0.10 + 0.22 * self.s.load);
                self.gurgle = frac(self.gurgle + 3.0 / rate);
                let ground = self.f_roll.run(n, c_roll) * 0.35;
                let wet = self.f_roll_w.run(n, c_roll_w) * 0.45 * (0.7 + 0.3 * (self.gurgle * TAU).sin());
                let roll = (ground * (1.0 - self.s.water) + wet * self.s.water) * self.s.roll;
                self.squeal = frac(self.squeal + 9.0 / rate);
                let band = self.f_sq1.run(n, c_s1) - self.f_sq2.run(n, c_s2);
                let squeal = band * self.s.slip * 0.9 * (0.6 + 0.4 * (self.squeal * TAU).sin());
                x += (engine + roll + squeal) * self.s.car;
            }
            // helicopter: blade-pass thumps, 55 Hz tone, whoosh
            if self.s.heli > 0.001 {
                self.rotor_phase = frac(self.rotor_phase + (13.0 + 4.0 * self.s.rpm) / rate);
                self.rotor_tone = frac(self.rotor_tone + 55.0 / rate);
                let pulse = (1.0 - self.rotor_phase).powi(4);
                let thump = self.f_thump.run(n, c_thump) * pulse * 2.6;
                let whoosh = self.f_whoosh.run(n, c_whoosh) * 0.18;
                let heli = thump * 0.5 + ((self.rotor_tone * TAU).sin() * 0.06 + (self.rotor_tone * 2.0 * TAU).sin() * 0.05) + whoosh;
                x += heli * (0.4 + 0.6 * self.s.rpm) * self.s.heli;
            }
            // drone: four detuned tones
            if self.s.drone > 0.001 {
                let mut d = 0.0;
                for i in 0..4 {
                    let f = 300.0 + 7.0 * i as f32 + 60.0 * self.s.rpm;
                    self.drone_phase[i] = frac(self.drone_phase[i] + f / rate);
                    d += (self.drone_phase[i] * TAU).sin() * 0.05;
                }
                d += self.f_drone.run(n, c_drone) * 0.03;
                x += d * (0.3 + 0.7 * self.s.rpm) * self.s.drone;
            }
            // wind
            x += self.f_wind.run(n, c_wind) * self.s.wind * 0.3;
            // one-shots
            let mut shots = 0.0;
            for v in self.voices.iter_mut() { if v.live { shots += Self::voice_sample(v, n, rate); } }
            self.vol_s += (self.volume - self.vol_s) * a;
            if self.volume == 0.0 && self.vol_s < 1e-4 { self.vol_s = 0.0; }
            let y = ((x + shots) * 0.9).tanh() * self.master * self.vol_s;
            *o = if y.is_finite() { y.clamp(-1.0, 1.0) } else { 0.0 };
        }
    }

    pub fn render(&mut self, out: &mut [i16]) {
        let mut buf = [0.0f32; 512];
        for chunk in out.chunks_mut(buf.len()) {
            self.render_f32(&mut buf[..chunk.len()]);
            for (o, s) in chunk.iter_mut().zip(&buf) { *o = (s * 32767.0) as i16; }
        }
    }
}
