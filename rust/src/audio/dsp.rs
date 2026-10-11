//! Small DSP building blocks (no allocation, deterministic).

/// xorshift32 white noise in [-1, 1].
#[derive(Clone, Copy, Debug)]
pub struct Noise(u32);
impl Noise {
    pub fn new(seed: u32) -> Noise { Noise(if seed == 0 { 0x9E37_79B9 } else { seed }) }
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13; x ^= x >> 17; x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// One-pole low-pass filter.
#[derive(Clone, Copy, Debug, Default)]
pub struct OnePole { y: f32 }
impl OnePole {
    /// `k` = 1 - exp(-2 pi fc / rate), see `coef`.
    pub fn run(&mut self, x: f32, k: f32) -> f32 { self.y += (x - self.y) * k; self.y }
}

/// Filter coefficient for a cutoff in Hz at `rate`.
pub fn coef(fc: f32, rate: f32) -> f32 { 1.0 - (-std::f32::consts::TAU * fc / rate).exp() }

/// Fractional part in [0, 1).
pub fn frac(x: f32) -> f32 { x - x.floor() }
