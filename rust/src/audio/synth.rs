//! The synthesiser: smoothed continuous voices (engine, rolling, squeal, wind) plus a pool of one-shots.
use super::dsp::{coef, frac, Noise, OnePole};
use super::engine::voice::EngineVoice;
use super::engine::{Controls, EngineSpec};
use super::reverb::Reverb;
use super::spatial::{path, Emitter, MAX_DELAY};
use super::{AudioParams, Event, Machine, Scene, Surface, N_SRC, RATE, SRC_BAY, SRC_BODY, SRC_EXHAUST, SRC_WHEEL};

const TAU: f32 = std::f32::consts::TAU;
const MAX_VOICES: usize = 8;
const FADE: f32 = 0.05;      // seconds for mute/inactive fades
const SMOOTH: f32 = 0.08;    // parameter smoothing time constant, seconds

#[derive(Clone, Copy, Default)]
struct Voice { kind: u8, t: f32, strength: f32, phase: f32, lp1: OnePole, lp2: OnePole, live: bool }

#[derive(Clone, Copy, Default)]
struct Smoothed { drive1: f32, drive2: f32, load: f32, roll: f32, water: f32, slip: f32, wind: f32, car: f32 }

pub struct Synth {
    target: AudioParams, s: Smoothed, master: f32, muted: bool, volume: f32, vol_s: f32,
    noise: Noise, noise_w: [Noise; 4], noise_wind: Noise, rate: f32, a: f32,
    gurgle: f32, squeal: f32, voice: EngineVoice,
    f_roll: [OnePole; 4], f_roll_w: [OnePole; 4], f_sq1: [OnePole; 4], f_sq2: [OnePole; 4], f_wind: OnePole, f_wind_r: OnePole,
    voices: [Voice; MAX_VOICES],
    scene: Scene, emitters: Vec<Emitter>, block: usize, wind_k: f32, lp_wind: [OnePole; 2], reverb: Reverb, burned_pub: f64, load: f32,
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
            gurgle: 0.0, squeal: 0.0, voice: EngineVoice::new(EngineSpec::rally_four()),
            f_roll: [OnePole::default(); 4], f_roll_w: [OnePole::default(); 4], f_sq1: [OnePole::default(); 4], f_sq2: [OnePole::default(); 4],
            f_wind: OnePole::default(), f_wind_r: OnePole::default(),
            voices: [Voice::default(); MAX_VOICES],
            scene: Scene::default(), emitters: (0..N_SRC).map(|_| Emitter::new()).collect(), block: 0, wind_k: 1.0, lp_wind: [OnePole::default(); 2], reverb: Reverb::new(), burned_pub: 0.0, load: 0.0,
        }
    }

    pub fn set_params(&mut self, p: &AudioParams) {
        let c = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        let rpm = |v: f32| if v.is_finite() { v.clamp(0.0, 12000.0) } else { 0.0 };
        self.target = AudioParams { machine: p.machine, als: p.als, cut: p.cut, stalled: p.stalled, no_bov: p.no_bov, drive_rpm: rpm(p.drive_rpm), clutch: c(p.clutch), load: c(p.load), roll: c(p.roll), surface: p.surface, slip: c(p.slip), wind: c(p.wind), active: p.active };
    }

    /// Where everything is (the listener and the sources): the paths are renewed at the next block.
    pub fn set_scene(&mut self, sc: &Scene) {
        let ok = |v: crate::game::math::V3| v.x.is_finite() && v.y.is_finite() && v.z.is_finite();
        if ok(sc.listener) && ok(sc.fwd) && sc.src.iter().all(|p| ok(*p)) { self.scene = *sc; }
    }

    /// The fuel (mol) burnt in the exhaust since the last call.
    pub fn take_burned_delta(&mut self) -> f64 { let b = self.voice.engine.exhaust_burned; let d = (b - self.burned_pub).max(0.0); self.burned_pub = b; d }

    /// The revs of the simulated engine (rpm).
    pub fn engine_rpm(&self) -> f32 { self.voice.rpm() }

    pub fn set_muted(&mut self, m: bool) { self.muted = m; }
    /// Master volume 0..1 (applied after the soft clip, smoothed).
    pub fn set_volume(&mut self, v: f32) { self.volume = if v.is_finite() { v.clamp(0.0, 1.0) } else { 1.0 }; }

    /// Stops everything at once: no one-shots, no machine, inactive (the activity left the game).
    pub fn silence(&mut self) { self.voices = [Voice::default(); MAX_VOICES]; self.target = AudioParams::default(); self.emitters.iter_mut().for_each(|e| e.reset()); self.reverb.reset(); }

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
        self.s.load += (t.load - self.s.load) * a * 3.0;
        // the revs the game asks for arrive 60 times a second, each a step: two cascaded smoothers (about 12 ms each) turn the stairs into a ramp, or the engine would rev in steps
        { let k = 1.0 - (-1.0 / (0.012 * rate)).exp(); self.s.drive1 += (t.drive_rpm - self.s.drive1) * k; self.s.drive2 += (self.s.drive1 - self.s.drive2) * k; }
        self.s.roll += (t.roll - self.s.roll) * a; self.s.slip += (t.slip - self.s.slip) * a; self.s.wind += (t.wind - self.s.wind) * a;
        self.s.water += ((t.surface == Surface::Water) as u8 as f32 - self.s.water) * a;
        self.s.car += ((t.machine == Machine::Car) as u8 as f32 - self.s.car) * a;
        let goal = if self.muted || !t.active { 0.0 } else { 1.0 };
        self.master = if self.master < goal { (self.master + fade).min(goal) } else { (self.master - fade).max(goal) };
        let n = self.noise.next();
        let mut f = Frame::default();
        // car: the engine (the model runs on this thread: the exhaust pipe's sound and the intake's and the block's, which fill the engine bay), the tyres' rolling noise and squeal
        if self.s.car > 0.001 {
            let running = self.master > 0.0 || goal > 0.0;
            if !running { self.voice.flame = 0.0; }
            if running {
                let ignition = t.active && !self.muted && !t.stalled;
                let starter = ignition && self.voice.engine.rpm() < 350.0;   // it cranks by itself when it is not running (the first start, a stall)
                self.voice.sample(&Controls { throttle: self.s.load as f64, starter, clutch: t.clutch as f64, drive_rpm: self.s.drive2 as f64, ignition, als: t.als, cut: t.cut, bov: !t.no_bov });
            }
            f.exhaust = self.voice.exhaust_out * 0.7 * self.s.car;
            f.bay = (self.voice.intake_out * 0.35 + self.voice.block_out * 0.35 + self.voice.turbo_out + self.voice.bov_out + self.voice.whine_out) * self.s.car;
            self.gurgle = frac(self.gurgle + 3.0 / rate);
            self.squeal = frac(self.squeal + 9.0 / rate);
            let wobble = 0.6 + 0.4 * (self.squeal * TAU).sin();
            let gur = 0.7 + 0.3 * (self.gurgle * TAU).sin();
            for k in 0..4 {
                let nk = self.noise_w[k].next();
                let ground = self.f_roll[k].run(nk, c_roll) * 0.35;
                let wet = self.f_roll_w[k].run(nk, c_roll_w) * 0.45 * gur;
                let roll = (ground * (1.0 - self.s.water) + wet * self.s.water) * self.s.roll * self.scene.wheel_on[k] * 0.45;   // the tyres under the engine: a hush, not a roar
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
                self.reverb.set_env(&self.scene.env);
            }
            self.block = (self.block + 1) % BLOCK;
            let f = self.frame();
            let g = self.scene.wind_gain;
            let (mut l, mut r) = (self.lp_wind[0].run(f.wind[0] * g, self.wind_k), self.lp_wind[1].run(f.wind[1] * g, self.wind_k));
            let mut add = |k: usize, x: f32| { let (a, b) = self.emitters[k].process(x); l += a; r += b; };
            add(SRC_EXHAUST, f.exhaust); add(SRC_BAY, f.bay); add(SRC_BODY, f.body);
            for k in 0..4 { add(SRC_WHEEL + k, f.wheels[k]); }
            let rin = (f.exhaust + f.bay + f.body + f.wheels.iter().sum::<f32>()) * 0.5;
            let (tl, tr) = self.reverb.process(rin);
            fr[0] = self.finish(l + tl); fr[1] = self.finish(r + tr);
        }
    }

    /// Interleaved stereo PCM16.
    pub fn render(&mut self, out: &mut [i16]) {
        let t0 = std::time::Instant::now();
        let mut buf = [0.0f32; 512];
        for chunk in out.chunks_mut(buf.len()) {
            self.render_stereo_f32(&mut buf[..chunk.len()]);
            for (o, s) in chunk.iter_mut().zip(&buf) { *o = (s * 32767.0) as i16; }
        }
        // a phone too slow for the engine (a block takes over half of its own length) simulates it with one step per sample instead of two; when it has room again it goes back
        let frames = (out.len() / 2).max(1) as f32;
        if frames >= 256.0 {
            let load = t0.elapsed().as_secs_f32() / (frames / RATE as f32);
            self.load = self.load * 0.8 + load * 0.2;
            if self.load > 0.55 { self.voice.substeps = 1; } else if self.load < 0.22 { self.voice.substeps = 2; }
        }
    }

    /// How much of its own length a block takes to render (smoothed, 1 = just in time), and the engine's steps per sample in use.
    pub fn load(&self) -> (f32, usize) { (self.load, self.voice.substeps) }
}
