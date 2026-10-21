//! The autopilot teaches itself: it drives the stage again and again in the simulation (no drawing, seconds per drive) with different driving numbers and keeps
//! what is faster and stays on the road. The search is a separable CMA-ES over six numbers; a candidate's cost is the drive time plus penalties for leaving the road,
//! flipping, damage and being put back by the recovery.
use super::cfg::AutoCfg;
use crate::game::{input::Input, vehicle::VehicleSpec, world::World};

/// The numbers searched: lower and upper bound of each, in the order of `to_cfg`: cornering grip, braking, how much of the free width the line may use, the margin to the edge, the look-ahead base and gain.
pub const LO: [f32; 6] = [0.8, 0.6, 0.4, 0.8, 3.0, 0.2];
pub const HI: [f32; 6] = [1.8, 1.2, 0.9, 2.0, 12.0, 0.8];

/// The numbers `base` has, with the searched ones taken from `g` (each 0..1 over its bounds).
pub fn to_cfg(base: AutoCfg, g: &[f32; 6]) -> AutoCfg {
    let v = |i: usize| LO[i] + g[i].clamp(0.0, 1.0) * (HI[i] - LO[i]);
    AutoCfg { lat_factor: v(0), brake_factor: v(1), offset_max: v(2), edge_margin: v(3), look_base: v(4), look_gain: v(5), ..base }
}
/// The inverse: where `cfg` lies in the 0..1 box.
pub fn from_cfg(cfg: &AutoCfg) -> [f32; 6] {
    let x = [cfg.lat_factor, cfg.brake_factor, cfg.offset_max, cfg.edge_margin, cfg.look_base, cfg.look_gain];
    std::array::from_fn(|i| ((x[i] - LO[i]) / (HI[i] - LO[i])).clamp(0.0, 1.0))
}

/// The six searched numbers of `cfg` in their own units (the order of `LO`).
pub fn from_cfg_values(c: &AutoCfg) -> [f32; 6] { [c.lat_factor, c.brake_factor, c.offset_max, c.edge_margin, c.look_base, c.look_gain] }

/// How one drive of a stage went.
#[derive(Clone, Copy, Debug, Default)]
pub struct Drive { pub finished: bool, pub time: f32, pub worst_edge: f32, pub flips: u32, pub damage: f32, pub recoveries: u32 }

impl Drive {
    /// The cost of the drive in seconds: the time and what the mistakes are worth (a metre beyond the edge costs 40 s past 0.2 m, a flip 300 s, all the damage 800 s).
    pub fn cost(&self) -> f32 {
        if !self.finished { return 2000.0; }
        self.time + 40.0 * (self.worst_edge - 0.2).max(0.0) + 300.0 * self.flips as f32 + 800.0 * self.damage + 20.0 * self.recoveries as f32
    }
}

/// Drives the stage of level `seed` from its start with `cfg`, in a world that has the car and the setup of `like` (tuning, aids, gearbox).
pub fn drive_stage(seed: u64, car: VehicleSpec, like: Option<&World>, cfg: AutoCfg, limit: f32) -> Drive {
    let mut w = World::with_roads(seed, car);
    if let Some(l) = like { w.tuning = l.tuning; w.assists = l.assists; w.drivetrain = l.drivetrain.clone(); }
    if !w.place_on_road(2) || !w.set_autopilot_cfg(cfg) { return Drive::default(); }
    let (mut t, mut worst, mut flips, mut was) = (0.0f32, f32::MIN, 0u32, false);
    while w.autopilot_on() && t < limit {
        w.step(&Input::default(), 1.0 / 60.0); t += 1.0 / 60.0;
        if let Some(e) = w.terrain.road_edge_distance(w.vehicle.pos.x, w.vehicle.pos.z) { if t > 8.0 { worst = worst.max(e); } }
        let f = w.vehicle.flipped(); if f && !was { flips += 1; } was = f;
    }
    Drive { finished: !w.autopilot_on(), time: t, worst_edge: worst.max(-5.0), flips, damage: w.vehicle.damage.total(), recoveries: 0 }
}

/// A small xorshift generator with normal numbers.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng { Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1) }
    pub fn uniform(&mut self) -> f32 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; ((self.0 >> 40) as f32 + 0.5) / 16_777_216.0 }
    pub fn normal(&mut self) -> f32 { let (a, b) = (self.uniform(), self.uniform()); (-2.0 * a.ln()).sqrt() * (std::f32::consts::TAU * b).cos() }
}

/// Separable CMA-ES in the 0..1 box (the candidates are clamped into it).
pub struct Cma {
    pub mean: [f32; 6], pub sigma: f32, diag: [f32; 6], ps: [f32; 6], pc: [f32; 6], pub lambda: usize, mu: usize, w: Vec<f32>, mueff: f32, rng: Rng, pub generation: u32,
}

impl Cma {
    pub fn new(start: [f32; 6], sigma: f32, seed: u64) -> Cma {
        let lambda = 8usize; let mu = lambda / 2;
        let mut w: Vec<f32> = (0..mu).map(|i| ((lambda as f32 + 1.0) / 2.0).ln() - ((i + 1) as f32).ln()).collect();
        let s: f32 = w.iter().sum(); for x in w.iter_mut() { *x /= s; }
        let mueff = 1.0 / w.iter().map(|x| x * x).sum::<f32>();
        Cma { mean: start, sigma, diag: [1.0; 6], ps: [0.0; 6], pc: [0.0; 6], lambda, mu, w, mueff, rng: Rng::new(seed), generation: 0 }
    }

    /// The next candidates.
    pub fn ask(&mut self) -> Vec<[f32; 6]> {
        (0..self.lambda).map(|_| std::array::from_fn(|i| (self.mean[i] + self.sigma * self.diag[i].sqrt() * self.rng.normal()).clamp(0.0, 1.0))).collect()
    }

    /// Learns from the costs of the candidates `ask` returned (lower is better).
    pub fn tell(&mut self, xs: &[[f32; 6]], costs: &[f32]) {
        let n = 6.0f32;
        let mut order: Vec<usize> = (0..xs.len()).collect();
        order.sort_by(|&a, &b| costs[a].partial_cmp(&costs[b]).unwrap_or(std::cmp::Ordering::Equal));
        let old = self.mean;
        for i in 0..6 { self.mean[i] = (0..self.mu).map(|k| self.w[k] * xs[order[k]][i]).sum(); }
        let me = self.mueff;
        let cs = (me + 2.0) / (n + me + 5.0); let ds = 1.0 + 2.0 * (((me - 1.0) / (n + 1.0)).sqrt() - 1.0).max(0.0) + cs;
        let cc = (4.0 + me / n) / (n + 4.0 + 2.0 * me / n);
        let c1 = 2.0 / ((n + 1.3) * (n + 1.3) + me) * (n + 2.0) / 3.0;
        let cmu = (1.0 - c1).min(2.0 * (me - 2.0 + 1.0 / me) / ((n + 2.0) * (n + 2.0) + me) * (n + 2.0) / 3.0);
        let mut ps2 = 0.0;
        for i in 0..6 {
            let y = (self.mean[i] - old[i]) / (self.sigma * self.diag[i].sqrt());
            self.ps[i] = (1.0 - cs) * self.ps[i] + (cs * (2.0 - cs) * me).sqrt() * y; ps2 += self.ps[i] * self.ps[i];
        }
        let chi = n.sqrt() * (1.0 - 1.0 / (4.0 * n) + 1.0 / (21.0 * n * n));
        let hs = if ps2.sqrt() / (1.0 - (1.0 - cs).powi(2 * (self.generation as i32 + 1))).sqrt() < (1.4 + 2.0 / (n + 1.0)) * chi { 1.0 } else { 0.0 };
        for i in 0..6 {
            let d = self.diag[i].sqrt();
            self.pc[i] = (1.0 - cc) * self.pc[i] + hs * (cc * (2.0 - cc) * me).sqrt() * (self.mean[i] - old[i]) / self.sigma;
            let rank_mu: f32 = (0..self.mu).map(|k| { let y = (xs[order[k]][i] - old[i]) / self.sigma; self.w[k] * y * y }).sum();
            self.diag[i] = ((1.0 - c1 - cmu) * self.diag[i] + c1 * self.pc[i] * self.pc[i] + cmu * rank_mu).clamp(1e-4, 4.0);
            let _ = d;
        }
        self.sigma = (self.sigma * ((cs / ds) * (ps2.sqrt() / chi - 1.0)).exp()).clamp(0.005, 0.5);
        self.generation += 1;
    }
}

/// The self-training of the autopilot for `car`: `generations` rounds of the search, every candidate driving `seeds`. `progress(done, best_cost)` is called after each round and
/// returning `false` from `stop` ends the search early. Returns the best numbers found (the start's own when nothing beat it) and their cost.
pub fn train(car: VehicleSpec, like: Option<&World>, start: AutoCfg, seeds: &[u64], generations: u32, mut progress: impl FnMut(f32, f32), stop: impl Fn() -> bool) -> (AutoCfg, f32) {
    let cost = |cfg: AutoCfg| -> f32 {
        let hs: Vec<_> = seeds.iter().map(|&s| { let tun = like.map(|l| (l.tuning, l.assists, l.drivetrain.clone())); std::thread::spawn(move || {
            let mut w = None::<World>;
            if let Some(t) = tun { let mut x = World::from_terrain(crate::game::terrain::Terrain::flat(), car, (0.0, 0.0), 0.0); x.tuning = t.0; x.assists = t.1; x.drivetrain = t.2; w = Some(x); }
            drive_stage(s, car, w.as_ref(), cfg, 700.0).cost()
        }) }).collect();
        hs.into_iter().map(|h| h.join().unwrap_or(2000.0)).sum::<f32>() / seeds.len().max(1) as f32
    };
    let mut best = (start, cost(start));
    progress(0.0, best.1);
    let mut cma = Cma::new(from_cfg(&start), 0.15, 7);
    for g in 0..generations {
        if stop() { break; }
        let xs = cma.ask();
        let cs: Vec<f32> = xs.iter().map(|x| cost(to_cfg(start, x))).collect();
        for (x, c) in xs.iter().zip(&cs) { if *c < best.1 { best = (to_cfg(start, x), *c); } }
        cma.tell(&xs, &cs);
        progress((g + 1) as f32 / generations as f32, best.1);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn the_numbers_map_to_the_box_and_back() {
        let c = super::super::cfg::cfg();
        let g = from_cfg(&c); assert!(g.iter().all(|x| (0.0..=1.0).contains(x)));
        let back = to_cfg(c, &g);
        assert!((back.lat_factor - c.lat_factor).abs() < 1e-4 && (back.look_base - c.look_base).abs() < 1e-4 && back.slide == c.slide);
        assert_eq!(to_cfg(c, &[5.0; 6]).lat_factor, HI[0]); assert_eq!(to_cfg(c, &[-1.0; 6]).edge_margin, LO[3]);
    }
    #[test] fn the_search_finds_the_minimum_of_a_bowl() {
        let target = [0.3, 0.7, 0.5, 0.2, 0.8, 0.4];
        let f = |x: &[f32; 6]| x.iter().zip(&target).map(|(a, b)| (a - b) * (a - b)).sum::<f32>();
        let mut cma = Cma::new([0.5; 6], 0.2, 1);
        for _ in 0..80 { let xs = cma.ask(); let cs: Vec<f32> = xs.iter().map(f).collect(); cma.tell(&xs, &cs); }
        assert!(f(&cma.mean) < 0.01, "the mean ends near the minimum: {:?}", cma.mean);
    }
    #[test] fn a_failed_drive_costs_more_than_a_slow_one_and_mistakes_cost_time() {
        let ok = Drive { finished: true, time: 300.0, worst_edge: -0.5, ..Default::default() };
        assert!(Drive { finished: false, ..ok }.cost() > 1500.0);
        assert!(Drive { worst_edge: 1.2, ..ok }.cost() > ok.cost() + 30.0 && Drive { flips: 1, ..ok }.cost() >= ok.cost() + 300.0 && Drive { damage: 0.2, ..ok }.cost() > ok.cost() + 100.0);
        assert!(Drive { worst_edge: 0.1, ..ok }.cost() == ok.cost() + 0.0 * 1.0 || Drive { worst_edge: 0.1, ..ok }.cost() - ok.cost() < 1e-3, "no penalty while the edge excursion is under 0.2 m");
    }
    #[test] fn the_random_numbers_are_in_range_and_roughly_normal() {
        let mut r = Rng::new(3); let n = 4000;
        let xs: Vec<f32> = (0..n).map(|_| r.normal()).collect();
        let m = xs.iter().sum::<f32>() / n as f32; let v = xs.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / n as f32;
        assert!(m.abs() < 0.08 && (v - 1.0).abs() < 0.12, "mean {m} var {v}");
        assert!((0..1000).all(|_| { let u = r.uniform(); u > 0.0 && u < 1.0 }));
    }
    /// The training itself (slow; `AP_CAR`, `AP_GEN`, `AP_SEEDS`): prints the numbers it found.
    #[test] #[ignore] fn train_the_autopilot() {
        let car = VehicleSpec::by_id(&std::env::var("AP_CAR").unwrap_or_else(|_| "vulpes".into())).unwrap();
        let gens: u32 = std::env::var("AP_GEN").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
        let seeds: Vec<u64> = std::env::var("AP_SEEDS").map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect()).unwrap_or_else(|_| vec![2, 21]);
        let start = super::super::cfg::cfg();
        let (best, cost) = train(car, None, start, &seeds, gens, |p, b| eprintln!("progress {:.0}% best cost {:.1}", p * 100.0, b), || false);
        let c0 = { let hs: Vec<_> = seeds.iter().map(|&s| std::thread::spawn(move || drive_stage(s, car, None, start, 700.0).cost())).collect(); hs.into_iter().map(|h| h.join().unwrap()).sum::<f32>() / seeds.len() as f32 };
        println!("START cost {c0:.1}\nBEST cost {cost:.1}\n{{\"lat_factor\":{:.3},\"brake_factor\":{:.3},\"offset_max\":{:.3},\"edge_margin\":{:.3},\"look_base\":{:.2},\"look_gain\":{:.3}}}", best.lat_factor, best.brake_factor, best.offset_max, best.edge_margin, best.look_base, best.look_gain);
    }
}
