//! Reward-driven training of the net: evolution strategies (the net's own numbers are changed at random in pairs, +/- the same change, and moved towards what drove faster).
//! It starts from the imitating net, so it already drives; the reward is the time over short stretches of a stage, started at random points, plus the penalties of a drive.
use super::cfg::AutoCfg;
use super::nn::{Mlp, Rng};
use super::train::{Drive, Mode};
use crate::game::{input::Input, math::v3, vehicle::{Vehicle, VehicleSpec}, world::World};
use std::sync::Arc;

/// A level and where on its stage a short drive starts.
pub struct Segment { pub world: World, pub seed: u64 }

/// The metres of stage one drive covers.
pub const LENGTH: f32 = 500.0;

impl Segment {
    pub fn new(seed: u64, car: VehicleSpec) -> Option<Segment> {
        let mut world = World::with_roads(seed, car);
        if !world.place_on_road(2) { return None; }
        Some(Segment { world, seed })
    }

    /// Drives `LENGTH` metres of the stage from the route point at `frac` (0..1 of the route) with the net, from the speed the plan gives there; returns the cost in seconds.
    pub fn run(&mut self, car: VehicleSpec, cfg: AutoCfg, net: Option<Arc<Mlp>>, frac: f32, mode: Mode) -> f32 {
        let w = &mut self.world;
        // every drive starts from the same world: the weather and the clock run on while a world is kept, and a rain that has set in would make every later drive slower than the first
        w.climate = crate::game::weather::Climate::new(self.seed); w.time = 0.0;
        // and with a full tank, new tyres and a cool engine: the fuel of a kept world runs out after some hundred drives and then every drive fails (this was the collapse seen at first)
        w.service();
        w.autopilot = None; w.loose = crate::game::grip::LooseMap::new(w.terrain.roads().expect("roads"));
        w.vehicle = Vehicle::new(car, v3(0.0, 0.0, 0.0), 0.0);
        if !w.place_on_road(2) || !w.set_autopilot_cfg(cfg) { return 2000.0; }
        let (k, p, yaw, v0, s0, s_end) = {
            let r = w.autopilot.as_ref().expect("the autopilot").route();
            let n = r.len();
            let last = (0..n).rev().find(|&i| r.length() - r.s[i] >= LENGTH + 60.0).unwrap_or(0);
            let k = ((frac.clamp(0.0, 1.0) * last as f32) as usize).clamp(2, last.max(2));
            let (_, q) = r.ahead(k, 8.0);
            (k, r.p[k], (q.x - r.p[k].x).atan2(q.z - r.p[k].z), r.speed[k] * 0.9, r.s[k], r.s[k] + LENGTH)
        };
        let _ = k;
        let h = w.terrain.height(p.x, p.z);
        w.vehicle = Vehicle::new(car, v3(p.x, h.max(p.y) + 1.1, p.z), yaw);
        let f = w.vehicle.forward(); w.vehicle.vel = f * v0;
        w.drivetrain.reset();
        w.autopilot_policy(net, false);
        let (mut t, mut worst, mut flips, mut was) = (0.0f32, f32::MIN, 0u32, false);
        let mut done = false;
        while w.autopilot_on() && t < 150.0 {
            w.step(&Input::default(), 1.0 / 60.0); t += 1.0 / 60.0;
            if let Some(ap) = w.autopilot.as_ref() { if ap.progress() >= s_end { done = true; break; } }
            if t > 3.0 { if let Some(e) = w.terrain.road_edge_distance(w.vehicle.pos.x, w.vehicle.pos.z) { worst = worst.max(e); } }
            let fl = w.vehicle.flipped(); if fl && !was { flips += 1; } was = fl;
        }
        let takeover = w.autopilot.as_ref().map_or(0.0, |a| a.takeover_time());
        let d = Drive { finished: done, time: t, worst_edge: worst.max(-5.0), flips, damage: w.vehicle.damage.total(), recoveries: 0 };
        let _ = s0;
        d.cost_in(mode) + 6.0 * takeover
    }
}

/// The ranks of `costs` as utilities in -0.5..0.5, the lowest cost the highest.
pub fn utilities(costs: &[f32]) -> Vec<f32> {
    let n = costs.len(); let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| costs[b].partial_cmp(&costs[a]).unwrap_or(std::cmp::Ordering::Equal));
    let mut u = vec![0.0; n];
    for (rank, &i) in order.iter().enumerate() { u[i] = if n > 1 { rank as f32 / (n - 1) as f32 - 0.5 } else { 0.0 }; }
    u
}

/// The evolution strategy. `pool` are the levels kept built (each in its own thread, at most `par` at a time); every generation `pairs` changes of size `sigma` are tried on all of
/// them from random start points. Returns the net after `generations` and the mean cost of the starting net and of the last one (on the same start points of the last round).
pub fn evolve(car: VehicleSpec, cfg: AutoCfg, mode: Mode, start: &Mlp, pool: &mut Vec<Segment>, generations: usize, pairs: usize, sigma: f32, lr: f32, par: usize,
              mut progress: impl FnMut(f32, f32), stop: impl Fn() -> bool) -> Mlp {
    let mut theta = start.to_vec(); let n = theta.len();
    // only the output layer is searched (a few hundred numbers): with a handful of drives per round the whole net's thousands of numbers are far too many to find a direction in
    let free = n - (super::nn::H * super::nn::O + super::nn::O);
    let mut rng = Rng::new(11);
    let (mut m, mut v, mut t) = (vec![0.0f32; n], vec![0.0f32; n], 0i32);
    // the best net so far on the stretches of the round it was measured in; a step that lands on a worse net (a cliff: the car no longer finishes) is taken back and the step size halved
    let mut best: Option<(Vec<f32>, f32)> = None; let mut lr = lr; let mut fracs: Vec<f32> = Vec::new();
    for gen in 0..generations {
        if stop() { break; }
        let eps: Vec<Vec<f32>> = (0..pairs).map(|_| (0..n).map(|k| if k >= free { rng.normal() } else { 0.0 }).collect()).collect();
        let mut cands: Vec<Arc<Mlp>> = Vec::new();
        for e in &eps { for sign in [1.0f32, -1.0] { let x: Vec<f32> = theta.iter().zip(e).map(|(a, b)| a + sign * sigma * b).collect(); cands.push(Arc::new(Mlp::from_vec(&x).expect("a net"))); } }
        cands.push(Arc::new(Mlp::from_vec(&theta).expect("a net")));   // the current net on the same stretches, to watch the progress
        // the stretches change every five rounds (so the net does not learn six stretches by heart); inside a block they are the same, so the costs can be compared and a bad step taken back
        if gen % 5 == 0 { fracs = pool.iter().map(|_| rng.uniform()).collect(); best = None; }
        // costs[c][w]
        let mut costs = vec![vec![0.0f32; pool.len()]; cands.len()];
        let idx: Vec<usize> = (0..pool.len()).collect();
        for chunk in idx.chunks(par.max(1)) {
            let results: Vec<(usize, Vec<f32>)> = std::thread::scope(|sc| {
                // every thread owns one level for the whole round
                let mut refs: Vec<(usize, &mut Segment)> = Vec::new();
                let mut rest: &mut [Segment] = pool.as_mut_slice(); let mut base = 0usize;
                for &i in chunk { let (_, tail) = rest.split_at_mut(i - base); let (one, tail2) = tail.split_first_mut().expect("a level"); refs.push((i, one)); rest = tail2; base = i + 1; }
                let hs: Vec<_> = refs.into_iter().map(|(i, seg)| { let cands = &cands; let f = fracs[i];
                    sc.spawn(move || (i, cands.iter().map(|c| seg.run(car, cfg, Some(c.clone()), f, mode)).collect::<Vec<f32>>())) }).collect();
                hs.into_iter().map(|h| h.join().unwrap_or((0, Vec::new()))).collect()
            });
            for (i, cs) in results { for (c, x) in cs.into_iter().enumerate() { costs[c][i] = x; } }
        }
        let mean: Vec<f32> = costs.iter().map(|c| c.iter().sum::<f32>() / c.len().max(1) as f32).collect();
        let cur = mean[2 * pairs];
        if let Some((bt, bc)) = &best { if cur > *bc * 1.03 + 1.0 { theta = bt.clone(); lr *= 0.5; m.iter_mut().for_each(|x| *x = 0.0); v.iter_mut().for_each(|x| *x = 0.0); t = 0; progress((gen + 1) as f32 / generations as f32, *bc); continue; } }
        if best.as_ref().map_or(true, |(_, bc)| cur < *bc) { best = Some((theta.clone(), cur)); }
        let u = utilities(&mean[..2 * pairs]);
        let mut g = vec![0.0f32; n];
        for (i, e) in eps.iter().enumerate() { let w = u[2 * i] - u[2 * i + 1]; for k in 0..n { g[k] += w * e[k]; } }
        for x in g.iter_mut() { *x /= 2.0 * pairs as f32 * sigma; }
        t += 1; let (b1, b2) = (0.9f32, 0.999f32); let (c1, c2) = (1.0 - b1.powi(t), 1.0 - b2.powi(t));
        for k in 0..n { m[k] = b1 * m[k] + (1.0 - b1) * g[k]; v[k] = b2 * v[k] + (1.0 - b2) * g[k] * g[k]; theta[k] += lr * (m[k] / c1) / ((v[k] / c2).sqrt() + 1e-8); }
        progress((gen + 1) as f32 / generations as f32, mean[2 * pairs]);
    }
    Mlp::from_vec(&best.map_or(theta, |b| b.0)).expect("a net")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn load(path: &str) -> Option<Mlp> { let b = std::fs::read(path).ok()?; let v: Vec<f32> = b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(); Mlp::from_vec(&v) }
    fn save(path: &str, m: &Mlp) { let b: Vec<u8> = m.to_vec().iter().flat_map(|x| x.to_le_bytes()).collect(); let _ = std::fs::write(path, b); }

    /// The evolution strategy against the imitation net (slow; `AP_STYLE` 1 or 5, `AP_GEN`, `AP_PAIRS`, `AP_PAR`): prints segment costs and full-stage drives of unseen levels before and after.
    #[test] #[ignore] fn evolve_the_net_beyond_its_teacher() {
        let car = VehicleSpec::by_id(&std::env::var("AP_CAR").unwrap_or_else(|_| "vulpes".into())).unwrap();
        let style: i32 = std::env::var("AP_STYLE").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let gens: usize = std::env::var("AP_GEN").ok().and_then(|s| s.parse().ok()).unwrap_or(20);
        let pairs: usize = std::env::var("AP_PAIRS").ok().and_then(|s| s.parse().ok()).unwrap_or(8);
        let par: usize = std::env::var("AP_PAR").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
        let sigma: f32 = std::env::var("AP_SIGMA").ok().and_then(|s| s.parse().ok()).unwrap_or(0.03);
        let lr: f32 = std::env::var("AP_LR").ok().and_then(|s| s.parse().ok()).unwrap_or(0.01);
        let cfg = super::super::cfg::cfg_style(style);
        let mode = if style == 5 { Mode::Attack } else { Mode::Safe };
        let imit_path = format!("{}/net_imit_{style}.bin", std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into()));
        let imit = load(&imit_path).unwrap_or_else(|| { let m = super::super::learn::dagger(car, cfg, &[2, 3, 5, 8], 4, |_| {}, || false); save(&imit_path, &m); m });
        let mut pool: Vec<Segment> = [2u64, 3, 5, 8, 13, 34].iter().filter_map(|&s| Segment::new(s, car)).collect();
        let fixed = [0.12f32, 0.3, 0.48, 0.66, 0.8, 0.9];
        let eval = |pool: &mut Vec<Segment>, net: Option<Arc<Mlp>>| -> f32 { pool.iter_mut().zip(fixed).map(|(s, f)| s.run(car, cfg, net.clone(), f, mode)).sum::<f32>() / fixed.len() as f32 };
        let (teacher, before) = (eval(&mut pool, None), eval(&mut pool, Some(Arc::new(imit.clone()))));
        println!("segments: the controller {teacher:.1}, the imitating net {before:.1}");
        let t0 = std::time::Instant::now();
        let evolved = evolve(car, cfg, mode, &imit, &mut pool, gens, pairs, sigma, lr, par, |p, c| eprintln!("progress {:.0}% current net {:.1} ({} s)", p * 100.0, c, t0.elapsed().as_secs()), || false);
        save(&format!("{}/net_es_{style}.bin", std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into())), &evolved);
        let after = eval(&mut pool, Some(Arc::new(evolved.clone())));
        println!("segments after: {after:.1} (the controller {teacher:.1}, the imitating net {before:.1})");
        for (tag, net) in [("imitating", &imit), ("evolved", &evolved)] {
            let rs = super::super::learn::rollouts(&[1, 21], car, cfg, Some(Arc::new(net.clone())), false, 2);
            for (r, s) in rs.iter().zip([1u64, 21]) { println!("{tag} net, full stage {s}: finished {} t {:.0}s edge {:.1} m flips {} damage {:.0}%", r.0.finished, r.0.time, r.0.worst_edge, r.0.flips, r.0.damage * 100.0); }
        }
    }

    #[test] fn the_utilities_rank_the_lowest_cost_highest_and_sum_to_zero() {
        let u = utilities(&[30.0, 10.0, 20.0, 40.0]);
        assert!(u[1] > u[2] && u[2] > u[0] && u[0] > u[3]);
        assert!(u.iter().sum::<f32>().abs() < 1e-5 && (u[1] - 0.5).abs() < 1e-6 && (u[3] + 0.5).abs() < 1e-6);
        assert_eq!(utilities(&[5.0]), vec![0.0]);
    }
}
