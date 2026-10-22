//! A small neural network that drives: what the geometric controller does (steering, throttle and brake, handbrake) learned from its own drives and then trained further.
//! Plain Rust, no libraries: a fully connected net of tanh layers, trained with Adam on the mean squared error (backpropagation by hand).

/// The inputs of the route-based net: see `Autopilot::features` (the sensor-based nets have more, see `sense`).
pub const F: usize = 24;
/// The outputs: steering (-1..1), throttle (+) or brake (-) (-1..1), the handbrake (-1..1, over 0 = on).
pub const O: usize = 3;
/// The hidden size of the nets saved without a header (the first ones); a net may have up to `HMAX` hidden units.
pub const H: usize = 48;
pub const HMAX: usize = 128;
/// The marker at the start of a saved net with a header: `[MAGIC, inputs, hidden, weights...]`.
const MAGIC: f32 = -7777.0;

/// Layers: inputs -> h -> h -> O (tanh on all three).
#[derive(Clone, Debug)]
pub struct Mlp { pub n_in: usize, pub h: usize, pub w1: Vec<f32>, pub b1: Vec<f32>, pub w2: Vec<f32>, pub b2: Vec<f32>, pub w3: Vec<f32>, pub b3: Vec<f32> }

/// Adam's moments for every parameter.
struct Adam { m: Vec<f32>, v: Vec<f32>, t: i32 }

pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng { Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1) }
    pub fn uniform(&mut self) -> f32 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; ((self.0 >> 40) as f32 + 0.5) / 16_777_216.0 }
    pub fn normal(&mut self) -> f32 { let (a, b) = (self.uniform(), self.uniform()); (-2.0 * a.ln()).sqrt() * (std::f32::consts::TAU * b).cos() }
}

impl Mlp {
    /// Small random weights (Xavier-like scale).
    pub fn new(seed: u64) -> Mlp { Mlp::with_shape(seed, F, H) }
    /// A net for `n_in` inputs (the route's 24 or the sensors' number) with the first hidden size.
    pub fn with_inputs(seed: u64, n_in: usize) -> Mlp { Mlp::with_shape(seed, n_in, H) }
    pub fn with_shape(seed: u64, n_in: usize, h: usize) -> Mlp {
        let h = h.clamp(4, HMAX);
        let mut r = Rng::new(seed);
        let mut layer = |a: usize, b: usize| -> Vec<f32> { let s = (1.0 / a as f32).sqrt(); (0..a * b).map(|_| r.normal() * s).collect() };
        Mlp { n_in, h, w1: layer(n_in, h), b1: vec![0.0; h], w2: layer(h, h), b2: vec![0.0; h], w3: layer(h, O), b3: vec![0.0; O] }
    }

    pub fn params(&self) -> usize { self.w1.len() + self.b1.len() + self.w2.len() + self.b2.len() + self.w3.len() + self.b3.len() }

    /// The weights alone as one vector, and back for a known shape (for the optimisers).
    pub fn flat(&self) -> Vec<f32> { [&self.w1, &self.b1, &self.w2, &self.b2, &self.w3, &self.b3].iter().flat_map(|v| v.iter().cloned()).collect() }
    pub fn from_flat(n_in: usize, h: usize, x: &[f32]) -> Option<Mlp> {
        let n = [n_in * h, h, h * h, h, h * O, O];
        if h < 4 || h > HMAX || n_in == 0 || x.len() != n.iter().sum::<usize>() || x.iter().any(|v| !v.is_finite()) { return None; }
        let mut o = 0; let mut take = |k: usize| { let s = x[o..o + k].to_vec(); o += k; s };
        Some(Mlp { n_in, h, w1: take(n[0]), b1: take(n[1]), w2: take(n[2]), b2: take(n[3]), w3: take(n[4]), b3: take(n[5]) })
    }

    /// The saved form: the weights alone for the first hidden size (as the first nets were saved), else with the header `[MAGIC, inputs, hidden]`.
    pub fn to_vec(&self) -> Vec<f32> {
        let mut v = if self.h == H { Vec::new() } else { vec![MAGIC, self.n_in as f32, self.h as f32] };
        v.extend(self.flat()); v
    }
    /// Reads a saved net: with the header, or (without) of the first hidden size, whose input size follows from the length; `None` for a length that fits nothing or non-finite numbers.
    pub fn from_vec(x: &[f32]) -> Option<Mlp> {
        if x.first() == Some(&MAGIC) {
            if x.len() < 4 || !(x[1] >= 1.0 && x[1] <= 1000.0 && x[2] >= 4.0 && x[2] <= HMAX as f32) { return None; }
            return Mlp::from_flat(x[1] as usize, x[2] as usize, &x[3..]);
        }
        let base = H + H * H + H + H * O + O;
        if x.len() <= base || (x.len() - base) % H != 0 { return None; }
        Mlp::from_flat((x.len() - base) / H, H, x)
    }

    fn layer(w: &[f32], b: &[f32], x: &[f32], n_out: usize, out: &mut [f32]) {
        let n_in = x.len();
        for j in 0..n_out { let mut s = b[j]; let row = &w[j * n_in..(j + 1) * n_in]; for i in 0..n_in { s += row[i] * x[i]; } out[j] = s.tanh(); }
    }

    pub fn forward(&self, x: &[f32]) -> [f32; O] {
        let h = self.h;
        let (mut h1, mut h2) = ([0.0f32; HMAX], [0.0f32; HMAX]);
        Mlp::layer(&self.w1, &self.b1, x, h, &mut h1[..h]); Mlp::layer(&self.w2, &self.b2, &h1[..h], h, &mut h2[..h]);
        let mut o = [0.0f32; O]; Mlp::layer(&self.w3, &self.b3, &h2[..h], O, &mut o);
        o
    }

    /// One pass of the gradient of the mean squared error over a batch (weighted per sample); returns the loss and the gradient in the order of `flat`.
    fn grad(&self, xs: &[Vec<f32>], ys: &[[f32; O]], ws: &[f32]) -> (f32, Vec<f32>) {
        let (f, h) = (self.n_in, self.h);
        let mut g = vec![0.0f32; self.params()];
        let (o1, o2, o3, o4, o5) = (f * h, f * h + h, f * h + h + h * h, f * h + 2 * h + h * h, f * h + 2 * h + h * h + h * O);
        let mut loss = 0.0; let mut wsum = 0.0;
        for ((x, y), &sw) in xs.iter().zip(ys).zip(ws) {
            let (mut h1, mut h2) = ([0.0f32; HMAX], [0.0f32; HMAX]);
            Mlp::layer(&self.w1, &self.b1, x, h, &mut h1[..h]); Mlp::layer(&self.w2, &self.b2, &h1[..h], h, &mut h2[..h]);
            let mut o = [0.0f32; O]; Mlp::layer(&self.w3, &self.b3, &h2[..h], O, &mut o);
            let mut d3 = [0.0f32; O];
            for k in 0..O { let e = o[k] - y[k]; loss += sw * e * e; d3[k] = 2.0 * sw * e * (1.0 - o[k] * o[k]); }
            wsum += sw;
            let mut d2 = [0.0f32; HMAX];
            for k in 0..O { for j in 0..h { g[o4 + k * h + j] += d3[k] * h2[j]; d2[j] += d3[k] * self.w3[k * h + j]; } g[o5 + k] += d3[k]; }
            for j in 0..h { d2[j] *= 1.0 - h2[j] * h2[j]; }
            let mut d1 = [0.0f32; HMAX];
            for k in 0..h { for j in 0..h { g[o2 + k * h + j] += d2[k] * h1[j]; d1[j] += d2[k] * self.w2[k * h + j]; } g[o3 + k] += d2[k]; }
            for j in 0..h { d1[j] *= 1.0 - h1[j] * h1[j]; }
            for k in 0..h { for j in 0..f { g[k * f + j] += d1[k] * x[j]; } g[o1 + k] += d1[k]; }
        }
        let n = wsum.max(1e-6);
        for v in g.iter_mut() { *v /= n; }
        (loss / n, g)
    }

    /// Trains on `(xs, ys)` for `epochs` passes in batches of 128 with Adam; `ws` weights each sample (the rare, important ones count more). Returns the loss of the last epoch.
    pub fn train(&mut self, xs: &[Vec<f32>], ys: &[[f32; O]], ws: &[f32], epochs: usize, lr: f32, seed: u64) -> f32 {
        if xs.is_empty() { return 0.0; }
        let (n_in, h) = (self.n_in, self.h);
        let mut p = self.flat();
        let mut adam = Adam { m: vec![0.0; p.len()], v: vec![0.0; p.len()], t: 0 };
        let mut rng = Rng::new(seed); let mut order: Vec<usize> = (0..xs.len()).collect();
        let mut last = 0.0;
        for _ in 0..epochs {
            for i in (1..order.len()).rev() { let j = (rng.uniform() * (i + 1) as f32) as usize % (i + 1); order.swap(i, j); }
            let (mut tot, mut cnt) = (0.0, 0);
            for chunk in order.chunks(128) {
                let bx: Vec<Vec<f32>> = chunk.iter().map(|&i| xs[i].clone()).collect(); let by: Vec<[f32; O]> = chunk.iter().map(|&i| ys[i]).collect(); let bw: Vec<f32> = chunk.iter().map(|&i| ws[i]).collect();
                let (loss, g) = self.grad(&bx, &by, &bw);
                adam.t += 1; let (b1, b2) = (0.9f32, 0.999f32);
                let (c1, c2) = (1.0 - b1.powi(adam.t), 1.0 - b2.powi(adam.t));
                for k in 0..p.len() { adam.m[k] = b1 * adam.m[k] + (1.0 - b1) * g[k]; adam.v[k] = b2 * adam.v[k] + (1.0 - b2) * g[k] * g[k]; p[k] -= lr * (adam.m[k] / c1) / ((adam.v[k] / c2).sqrt() + 1e-8); }
                *self = Mlp::from_flat(n_in, h, &p).unwrap_or_else(|| self.clone());
                tot += loss; cnt += 1;
            }
            last = tot / cnt.max(1) as f32;
        }
        last
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn the_net_has_the_expected_size_and_round_trips_its_parameters() {
        let m = Mlp::new(1);
        assert_eq!(m.params(), F * H + H + H * H + H + H * O + O);
        let back = Mlp::from_vec(&m.to_vec()).unwrap();
        let x = [0.3f32; F];
        assert_eq!(m.forward(&x), back.forward(&x));
        let wide = Mlp::with_inputs(2, 60); assert_eq!(wide.n_in, 60); assert_eq!(Mlp::from_vec(&wide.to_vec()).unwrap().n_in, 60, "the input size follows from the length");
        let big = Mlp::with_shape(3, 67, 96); let back = Mlp::from_vec(&big.to_vec()).unwrap();
        assert_eq!((back.n_in, back.h), (67, 96), "a net with another hidden size keeps its header");
        let xs = vec![0.2f32; 67]; assert_eq!(big.forward(&xs), back.forward(&xs));
        assert!(Mlp::from_vec(&[MAGIC, 0.0, 96.0, 1.0]).is_none() && Mlp::from_vec(&[MAGIC, 5.0, 999.0, 1.0]).is_none(), "a bad header is refused");
        assert!(Mlp::from_vec(&[0.0; 5]).is_none() && Mlp::from_vec(&vec![f32::NAN; m.params()]).is_none());
    }
    #[test] fn the_outputs_stay_in_range() {
        let m = Mlp::new(2);
        for k in 0..20 { let x = vec![(k as f32 - 10.0) * 3.0; F]; assert!(m.forward(&x).iter().all(|o| o.abs() <= 1.0 && o.is_finite())); }
    }
    #[test] fn the_gradient_matches_a_numerical_one() {
        let m = Mlp::new(3); let mut r = Rng::new(9);
        let xs: Vec<Vec<f32>> = (0..4).map(|_| (0..F).map(|_| r.normal()).collect()).collect();
        let ys: Vec<[f32; O]> = (0..4).map(|_| std::array::from_fn(|_| r.uniform() * 2.0 - 1.0)).collect();
        let ws = vec![1.0; 4];
        let (_, g) = m.grad(&xs, &ys, &ws);
        let p = m.flat();
        for &k in &[0usize, 7, F * H + 3, F * H + H + 11, m.params() - 1, m.params() - 40] {
            let eps = 1e-2f32;
            let (mut a, mut b) = (p.clone(), p.clone()); a[k] += eps; b[k] -= eps;
            let la = Mlp::from_flat(F, H, &a).unwrap().grad(&xs, &ys, &ws).0; let lb = Mlp::from_flat(F, H, &b).unwrap().grad(&xs, &ys, &ws).0;
            let num = (la - lb) / (2.0 * eps);
            assert!((num - g[k]).abs() < 0.05 * num.abs().max(g[k].abs()).max(0.02), "parameter {k}: numeric {num} analytic {}", g[k]);
        }
    }
    #[test] fn it_learns_a_simple_function() {
        let mut r = Rng::new(5);
        let xs: Vec<Vec<f32>> = (0..600).map(|_| (0..F).map(|i| if i < 3 { r.uniform() * 2.0 - 1.0 } else { 0.0 }).collect()).collect();
        let ys: Vec<[f32; O]> = xs.iter().map(|x| [0.8 * x[0] - 0.3 * x[1], (x[2] * 1.5).tanh() * 0.9, if x[0] > 0.0 { 0.9 } else { -0.9 }]).collect();
        let mut m = Mlp::new(4);
        let before = m.train(&xs, &ys, &vec![1.0; xs.len()], 1, 0.003, 1);
        let after = m.train(&xs, &ys, &vec![1.0; xs.len()], 150, 0.003, 2);
        assert!(after < before * 0.2 && after < 0.03, "the loss falls from {before} to {after}");
    }
}
