//! The synthesiser: smoothed continuous voices (engine, rolling, squeal, wind) plus a pool of one-shots.
use super::dsp::{coef, frac, Noise, OnePole};
use super::spatial::{path, Emitter, MAX_DELAY};
use super::{AudioParams, Event, Machine, Scene, Surface, N_SRC, RATE, SRC_BAY, SRC_BODY, SRC_EXHAUST, SRC_WHEEL};

const TAU: f32 = std::f32::consts::TAU;
const MAX_VOICES: usize = 8;
const FADE: f32 = 0.05;      // seconds for mute/inactive fades
const SMOOTH: f32 = 0.08;    // parameter smoothing time constant, seconds

/// Fundamental of the engine for `rpm` in 0..1.
pub fn engine_freq(rpm: f32) -> f32 { 40.0 + 140.0 * rpm.clamp(0.0, 1.0) }

#[derive(Clone, Copy, Default)]
struct Voice { kind: u8, t: f32, strength: f32, phase: f32, lp1: OnePole, lp2: OnePole, live: bool }

#[derive(Clone, Copy, Default)]
struct Smoothed { rpm: f32, load: f32, roll: f32, water: f32, slip: f32, wind: f32, car: f32 }

pub struct Synth {
    target: AudioParams, s: Smoothed, master: f32, muted: bool, volume: f32, vol_s: f32,
    noise: Noise, noise_w: [Noise; 4], noise_wind: Noise, rate: f32, a: f32,
    engine_phase: f32, gurgle: f32, squeal: f32,
    f_roll: [OnePole; 4], f_roll_w: [OnePole; 4], f_sq1: [OnePole; 4], f_sq2: [OnePole; 4], f_wind: OnePole, f_wind_r: OnePole,
    voices: [Voice; MAX_VOICES],
    scene: Scene, emitters: Vec<Emitter>, block: usize, wind_k: f32, lp_wind: [OnePole; 2],
}

/// What the sources emit at one instant, before the room and the listener's head: the exhaust, the engine bay, the four tyres, the car's body (the one-shots) and the wind at the listener's ears.
#[derive(Clone, Copy, Default)]
pub struct Frame { pub exhaust: f32, pub bay: f32, pub wheels: [f32; 4], pub body: f32, pub wind: [f32; 2] }
impl Frame { fn mono(&self) -> f32 { self.exhaust + self.bay + self.wheels.iter().sum::<f32>() + self.body + 0.5 * (self.wind[0] + self.wind[1]) } }

/// Samples per block between two updates of the paths.
const BLOCK: usize = 128;

impl Default for Synth { fn default() -> Self { Synth::new() } }

impl Synth {
    pub fn new() -> Synth {
        let rate = RATE as f32;
        Synth {
            target: AudioParams::default(), s: Smoothed::default(), master: 0.0, muted: false, volume: 1.0, vol_s: 1.0,
            noise: Noise::new(0x1234_5678), noise_w: [Noise::new(0x2345_6789), Noise::new(0x3456_789A), Noise::new(0x4567_89AB), Noise::new(0x5678_9ABC)], noise_wind: Noise::new(0x6789_ABCD),
            rate, a: 1.0 - (-1.0 / (SMOOTH * rate)).exp(),
            engine_phase: 0.0, gurgle: 0.0, squeal: 0.0,
            f_roll: [OnePole::default(); 4], f_roll_w: [OnePole::default(); 4], f_sq1: [OnePole::default(); 4], f_sq2: [OnePole::default(); 4],
            f_wind: OnePole::default(), f_wind_r: OnePole::default(),
            voices: [Voice::default(); MAX_VOICES],
            scene: Scene::default(), emitters: (0..N_SRC).map(|_| Emitter::new()).collect(), block: 0, wind_k: 1.0, lp_wind: [OnePole::default(); 2],
        }
    }

    pub fn set_params(&mut self, p: &AudioParams) {
        let c = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        self.target = AudioParams { machine: p.machine, rpm: c(p.rpm), load: c(p.load), roll: c(p.roll), surface: p.surface, slip: c(p.slip), wind: c(p.wind), active: p.active };
    }

    /// Where everything is (the listener and the sources): the paths are renewed at the next block.
    pub fn set_scene(&mut self, sc: &Scene) {
        let ok = |v: crate::game::math::V3| v.x.is_finite() && v.y.is_finite() && v.z.is_finite();
        if ok(sc.listener) && ok(sc.fwd) && sc.src.iter().all(|p| ok(*p)) { self.scene = *sc; }
    }

    pub fn set_muted(&mut self, m: bool) { self.muted = m; }
    /// Master volume 0..1 (applied after the soft clip, smoothed).
    pub fn set_volume(&mut self, v: f32) { self.volume = if v.is_finite() { v.clamp(0.0, 1.0) } else { 1.0 }; }

    /// Stops everything at once: no one-shots, no machine, inactive (the activity left the game).
    pub fn silence(&mut self) { self.voices = [Voice::default(); MAX_VOICES]; self.target = AudioParams::default(); self.emitters.iter_mut().for_each(|e| e.reset()); }

    /// Number of one-shot voices currently sounding.
    pub fn voices(&self) -> usize { self.voices.iter().filter(|v| v.live).count() }

    pub fn push(&mut self, e: Event) {
        if self.muted { return; }
        let (kind, strength) = match e {
            Event::Touchdown(s) => (0u8, s), Event::Crash => (1, 1.0), Event::Splash(s) => (2, s),
        };
        let strength = if strength.is_finite() { strength.clamp(0.0, 1.0) } else { 0.0 };
        let slot = self.voices.iter().position(|v| !v.live)
            .unwrap_or_else(|| (0..MAX_VOICES).max_by(|&a, &b| self.voices[a].t.partial_cmp(&self.voices[b].t).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(0));
        self.voices[slot] = Voice { kind, t: 0.0, strength, live: true, ..Voice::default() };
    }

    fn voice_sample(v: &mut Voice, noise: f32, rate: f32) -> f32 {
        let dt = 1.0 / rate;
        let t = v.t; v.t += dt;
        let (out, end) = match v.kind {
            0 => { // touchdown: falling thump plus a little noise
                v.phase = frac(v.phase + (45.0 + 35.0 * (-t * 18.0).exp()) * dt);
                ((TAU * v.phase).sin() * (-t * 11.0).exp() * 0.5 + (TAU * v.phase * 4.0).sin() * (-t * 14.0).exp() * 0.35 + v.lp1.run(noise, 0.45) * (-t * 22.0).exp() * 0.45, t > 0.5)
            }
            1 => (v.lp1.run(noise, 0.2) * (-t * 3.5).exp() * 2.4 * (1.0 + 0.3 * (t * 90.0).sin()), t > 1.2),
            2 => { let hi = v.lp1.run(noise, 0.5) - v.lp2.run(noise, 0.05); (hi * (-t * 7.0).exp() * 1.4, t > 0.7) }
            _ => (0.0, true),
        };
        if end { v.live = false; }
        out * (0.25 + 0.75 * v.strength).min(1.0) * 0.55
    }

    /// One instant of every source (the smoothing, the voices, the master fade run here). `y` of the mix is `finish(frame.mono())`.
    pub fn frame(&mut self) -> Frame {
        let (rate, a) = (self.rate, self.a);
        let c_roll = coef(1500.0, rate); let c_roll_w = coef(600.0, rate);
        let (c_s1, c_s2) = (coef(1400.0, rate), coef(900.0, rate));
        let c_wind = coef(500.0, rate);
        let fade = 1.0 / (FADE * rate);
        let t = self.target;
        self.s.rpm += (t.rpm - self.s.rpm) * a; self.s.load += (t.load - self.s.load) * a;
        self.s.roll += (t.roll - self.s.roll) * a; self.s.slip += (t.slip - self.s.slip) * a; self.s.wind += (t.wind - self.s.wind) * a;
        self.s.water += ((t.surface == Surface::Water) as u8 as f32 - self.s.water) * a;
        self.s.car += ((t.machine == Machine::Car) as u8 as f32 - self.s.car) * a;
        let goal = if self.muted || !t.active { 0.0 } else { 1.0 };
        self.master = if self.master < goal { (self.master + fade).min(goal) } else { (self.master - fade).max(goal) };
        let n = self.noise.next();
        let mut f = Frame::default();
        // car: the engine's pulses (the exhaust carries the low harmonics, the engine bay the upper ones), the tyres' rolling noise and squeal
        if self.s.car > 0.001 {
            self.engine_phase = frac(self.engine_phase + engine_freq(self.s.rpm) / rate);
            let p = self.engine_phase * TAU;
            let w = [0.6f32, 0.7, 1.0, 0.8, 0.6, 0.4];   // the mids carry the sound on a phone speaker
            let (mut lo, mut hi) = (0.0, 0.0);
            for (k, wk) in w.iter().enumerate() { let h = (p * (k + 1) as f32).sin() * wk; if k < 3 { lo += h; } else { hi += h; } }
            let amp = 0.10 + 0.22 * self.s.load;
            f.exhaust = (lo * 0.30 + n * 0.04) * amp * self.s.car;
            f.bay = hi * 0.30 * amp * self.s.car;
            self.gurgle = frac(self.gurgle + 3.0 / rate);
            self.squeal = frac(self.squeal + 9.0 / rate);
            let wobble = 0.6 + 0.4 * (self.squeal * TAU).sin();
            let gur = 0.7 + 0.3 * (self.gurgle * TAU).sin();
            for k in 0..4 {
                let nk = self.noise_w[k].next();
                let ground = self.f_roll[k].run(nk, c_roll) * 0.35;
                let wet = self.f_roll_w[k].run(nk, c_roll_w) * 0.45 * gur;
                let roll = (ground * (1.0 - self.s.water) + wet * self.s.water) * self.s.roll * self.scene.wheel_on[k];
                let band = self.f_sq1[k].run(nk, c_s1) - self.f_sq2[k].run(nk, c_s2);
                let squeal = band * self.s.slip * self.scene.wheel_slip[k] * 0.9 * wobble;
                f.wheels[k] = (roll + squeal) * 0.5 * self.s.car;   // four incoherent tyres: half the amplitude each keeps the total power of one
            }
        }
        // wind at the ears: two decorrelated streams
        let nw = self.noise_wind.next();
        f.wind = [self.f_wind.run(n, c_wind) * self.s.wind * 0.3, self.f_wind_r.run(nw, c_wind) * self.s.wind * 0.3];
        // one-shots, from the car's body
        let mut shots = 0.0;
        for v in self.voices.iter_mut() { if v.live { shots += Self::voice_sample(v, n, rate); } }
        f.body = shots;
        self.vol_s += (self.volume - self.vol_s) * a;
        if self.volume == 0.0 && self.vol_s < 1e-4 { self.vol_s = 0.0; }
        f
    }

    fn finish(&self, x: f32) -> f32 { let y = (x * 0.9).tanh() * self.master * self.vol_s; if y.is_finite() { y.clamp(-1.0, 1.0) } else { 0.0 } }

    /// Renders float samples in -1..1, mono, straight from the sources (no distance, no head): kept for tests; the game plays `render_stereo`.
    pub fn render_f32(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() { let f = self.frame(); *o = self.finish(f.mono()); }
    }

    /// Interleaved stereo samples in -1..1 (`out.len()` / 2 frames): each source is heard along its own path (travel time, distance, the head).
    pub fn render_stereo_f32(&mut self, out: &mut [f32]) {
        let (l0, fwd) = (self.scene.listener, self.scene.fwd);
        for fr in out.chunks_exact_mut(2) {
            if self.block == 0 {
                for k in 0..N_SRC {
                    let ex = self.scene.extra[k];
                    let mut direct = path(l0, fwd, self.scene.src[k]).tap();
                    direct.delay = (direct.delay + ex.extra_delay).min(MAX_DELAY); direct.gain *= ex.gain; direct.hz = direct.hz.min(ex.hz);
                    let echo = ex.echo.map(|(image, coeff, hz)| {
                        let mut t = path(l0, fwd, image).tap();
                        t.delay = (t.delay + ex.extra_delay).min(MAX_DELAY); t.gain *= ex.gain * coeff; t.hz = t.hz.min(hz).min(ex.hz); t
                    });
                    self.emitters[k].set_taps(&direct, echo.as_ref());
                }
                let c = coef(self.scene.wind_hz, self.rate); self.wind_k = c;
            }
            self.block = (self.block + 1) % BLOCK;
            let f = self.frame();
            let g = self.scene.wind_gain;
            let (mut l, mut r) = (self.lp_wind[0].run(f.wind[0] * g, self.wind_k), self.lp_wind[1].run(f.wind[1] * g, self.wind_k));
            let mut add = |k: usize, x: f32| { let (a, b) = self.emitters[k].process(x); l += a; r += b; };
            add(SRC_EXHAUST, f.exhaust); add(SRC_BAY, f.bay); add(SRC_BODY, f.body);
            for k in 0..4 { add(SRC_WHEEL + k, f.wheels[k]); }
            fr[0] = self.finish(l); fr[1] = self.finish(r);
        }
    }

    /// Interleaved stereo PCM16.
    pub fn render(&mut self, out: &mut [i16]) {
        let mut buf = [0.0f32; 512];
        for chunk in out.chunks_mut(buf.len()) {
            self.render_stereo_f32(&mut buf[..chunk.len()]);
            for (o, s) in chunk.iter_mut().zip(&buf) { *o = (s * 32767.0) as i16; }
        }
    }
}
