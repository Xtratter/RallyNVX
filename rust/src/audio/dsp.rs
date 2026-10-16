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

/// A delay line with a fractional read (linear interpolation): the sound's travel time from a source, so a moving source or listener bends the pitch by itself (the Doppler effect).
pub struct FracDelay { buf: Vec<f32>, mask: usize, w: usize }
impl FracDelay {
    /// Room for `1 << bits` samples (the longest delay is a little less).
    pub fn new(bits: u32) -> FracDelay { FracDelay { buf: vec![0.0; 1 << bits], mask: (1 << bits) - 1, w: 0 } }
    pub fn push(&mut self, x: f32) { self.w = (self.w + 1) & self.mask; self.buf[self.w] = x; }
    /// The sample `d` samples ago (`d` in 0..len-2; clamped).
    pub fn read(&self, d: f32) -> f32 {
        let d = if d.is_finite() { d.clamp(0.0, self.mask as f32 - 2.0) } else { 0.0 };
        let i = d as usize; let f = d - i as f32;
        let a = self.buf[(self.w.wrapping_sub(i)) & self.mask]; let b = self.buf[(self.w.wrapping_sub(i + 1)) & self.mask];
        a + (b - a) * f
    }
    pub fn clear(&mut self) { self.buf.iter_mut().for_each(|x| *x = 0.0); }
}
