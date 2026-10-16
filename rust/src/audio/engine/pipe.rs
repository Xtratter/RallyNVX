//! A pipe as a waveguide: one delay line for the round trip, a reflection at each end. The entry (the manifold, a big volume) reflects with the same sign, the open exit with the
//! opposite one and loses the highs on the way, so the pipe rings at the odd multiples of c / 4L (a quarter-wave resonator), and what leaves the open end is the derivative of the wave
//! that reaches it (a small opening radiates the highs better than the lows).
use super::super::RATE;
use super::super::spatial::C;

pub struct Pipe { line: Vec<f32>, w: usize, d: usize, half: usize, g: f32, lp: f32, lp_k: f32, prev: f32, dc: f32 }

impl Pipe {
    /// A pipe of `length` metres; `r_entry` and `r_exit` are the reflection coefficients (the exit's is negative); the highs lose `damp` (0..1) in every round trip.
    pub fn new(length: f32, r_entry: f32, r_exit: f32, damp: f32) -> Pipe {
        let d = ((2.0 * length / C * RATE as f32) as usize).max(4);
        Pipe { line: vec![0.0; d + 2], w: 0, d, half: d / 2, g: r_entry * r_exit, lp: 0.0, lp_k: (1.0 - damp).clamp(0.05, 1.0), prev: 0.0, dc: 0.0 }
    }
    /// The round trip's time in samples.
    pub fn round_trip(&self) -> usize { self.d }
    /// One sample of the pressure entering the pipe; the radiated sample out.
    pub fn process(&mut self, x: f32) -> f32 {
        let n = self.line.len();
        let back = self.line[(self.w + n - self.d) % n];
        self.lp += (back - self.lp) * self.lp_k;
        let y = x + self.g * self.lp;
        let y = if y.is_finite() { y } else { 0.0 };
        self.line[self.w] = y;
        let at_exit = self.line[(self.w + n - self.half) % n];
        self.w = (self.w + 1) % n;
        // the open end radiates the rate of change of the wave, plus a little of the wave itself (the low notes of the pulses, which a bare derivative would lose)
        self.dc += (at_exit - self.dc) * 0.0007;   // the steady pressure (the mean back pressure of the exhaust, the depression of the intake) is no sound: about 20 Hz and below goes
        let rad = (at_exit - self.prev) + 0.02 * (at_exit - self.dc); self.prev = at_exit;
        rad
    }
    pub fn clear(&mut self) { self.line.iter_mut().for_each(|x| *x = 0.0); self.lp = 0.0; self.prev = 0.0; self.dc = 0.0; }
}
