//! The tail of the environment: a feedback delay network (eight delay lines, a Householder mixing matrix, a low-pass in every loop) whose decay time and darkness come from
//! the rays cast around the listener (`acoustics::environment`), and a bank of echoes (a wall, a cliff, a house across the street) at the delays the rays measured.
use super::dsp::{coef, FracDelay, OnePole};
use super::RATE;

const N: usize = 8;
/// The line lengths, ms (mutually prime in samples so the modes do not pile up).
const LEN_MS: [f32; N] = [29.7, 37.1, 41.1, 43.7, 53.1, 59.9, 67.7, 73.3];
/// The longest decay time the network is asked for, s.
pub const MAX_RT60: f32 = 4.0;
/// The echo bank's taps: the four sides of the listener.
pub const ECHOES: usize = 4;

/// What the environment asks of the tail (the audio thread smooths the changes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Env {
    /// The time the tail takes to fall by 60 dB, s.
    pub rt60: f32,
    /// How loud the tail is against the dry sound (0..1).
    pub level: f32,
    /// The cutoff (Hz) of the loop's low-pass: foliage and grass take the highs of every bounce.
    pub damp_hz: f32,
    /// Echoes off the surroundings: (delay s, gain, pan -1..1); a zero gain is no echo.
    pub echoes: [(f32, f32, f32); ECHOES],
}
impl Default for Env { fn default() -> Env { Env { rt60: 0.3, level: 0.0, damp_hz: 8000.0, echoes: [(0.0, 0.0, 0.0); ECHOES] } } }

pub struct Reverb {
    lines: Vec<FracDelay>, len: [usize; N], lp: [OnePole; N], fb: [f32; N],
    // the smoothed controls
    rt60: f32, level: f32, damp: f32, t_rt60: f32, t_level: f32, t_damp: f32,
    echo_line: FracDelay, echo: [(f32, f32, f32); ECHOES], t_echo: [(f32, f32, f32); ECHOES],
    a: f32,
}

impl Default for Reverb { fn default() -> Reverb { Reverb::new() } }

impl Reverb {
    pub fn new() -> Reverb {
        let rate = RATE as f32;
        let len: [usize; N] = std::array::from_fn(|i| (LEN_MS[i] * 1e-3 * rate) as usize);
        let mut r = Reverb { lines: (0..N).map(|_| FracDelay::new(12)).collect(), len, lp: [OnePole::default(); N], fb: [0.0; N],
            rt60: 0.3, level: 0.0, damp: 8000.0, t_rt60: 0.3, t_level: 0.0, t_damp: 8000.0,
            echo_line: FracDelay::new(16), echo: [(0.0, 0.0, 0.0); ECHOES], t_echo: [(0.0, 0.0, 0.0); ECHOES], a: 1.0 - (-1.0 / (0.25 * rate)).exp() };
        r.update_feedback();
        r
    }

    pub fn set_env(&mut self, e: &Env) {
        let f = |x: f32, lo: f32, hi: f32, d: f32| if x.is_finite() { x.clamp(lo, hi) } else { d };
        self.t_rt60 = f(e.rt60, 0.05, MAX_RT60, 0.3); self.t_level = f(e.level, 0.0, 1.0, 0.0); self.t_damp = f(e.damp_hz, 800.0, 16000.0, 8000.0);
        let rate = RATE as f32;
        for k in 0..ECHOES {
            let (d, g, p) = e.echoes[k];
            self.t_echo[k] = if d.is_finite() && g.is_finite() && p.is_finite() { ((d * rate).clamp(0.0, 60000.0), g.clamp(0.0, 1.0), p.clamp(-1.0, 1.0)) } else { (0.0, 0.0, 0.0) };
        }
    }

    fn update_feedback(&mut self) {
        let rate = RATE as f32;
        for i in 0..N { self.fb[i] = 10f32.powf(-3.0 * self.len[i] as f32 / (self.rt60 * rate)); }
    }

    pub fn reset(&mut self) { for l in self.lines.iter_mut() { l.clear(); } self.echo_line.clear(); self.lp = [OnePole::default(); N]; self.level = 0.0; self.echo = [(0.0, 0.0, 0.0); ECHOES]; }

    /// One mono sample of every source in, the stereo tail and echoes out.
    pub fn process(&mut self, x: f32) -> (f32, f32) {
        let a = self.a;
        self.rt60 += (self.t_rt60 - self.rt60) * a * 0.4; self.level += (self.t_level - self.level) * a * 0.3; self.damp += (self.t_damp - self.damp) * a;
        self.update_feedback();
        let k = coef(self.damp, RATE as f32);
        // read the lines, mix them (Householder: x - (2/N) sum x), feed back with the decay gain and the low-pass, add the input
        let mut y = [0.0f32; N]; let mut sum = 0.0;
        for i in 0..N { y[i] = self.lines[i].read(self.len[i] as f32 - 1.0); sum += y[i]; }
        let m = 2.0 * sum / N as f32;
        for i in 0..N {
            let v = (y[i] - m) * self.fb[i];
            let v = self.lp[i].run(v, k) + x * 0.35;
            self.lines[i].push(if v.is_finite() { v } else { 0.0 });
        }
        let (mut l, mut r) = (0.0, 0.0);
        for i in 0..N { if i % 2 == 0 { l += y[i] } else { r += y[i] } }
        let sc = self.level * 0.5;
        let (mut l, mut r) = (l * sc, r * sc);
        // the echoes off the surroundings: each side of the listener, panned to its side
        self.echo_line.push(x);
        for k in 0..ECHOES {
            let e = &mut self.echo[k]; let t = self.t_echo[k];
            if t.1 == 0.0 && e.1 < 1e-5 { e.1 = 0.0; continue; }
            if e.1 == 0.0 { e.0 = t.0; e.2 = t.2; }
            e.0 += (t.0 - e.0) * a * 0.2; e.1 += (t.1 - e.1) * a; e.2 += (t.2 - e.2) * a;
            let s = self.echo_line.read(e.0) * e.1;
            let p = (e.2 + 1.0) * 0.25 * std::f32::consts::PI;
            l += s * p.cos() * std::f32::consts::SQRT_2; r += s * p.sin() * std::f32::consts::SQRT_2;
        }
        (l, r)
    }
}

#[cfg(test)]
#[path = "reverb_tests.rs"]
mod tests;
