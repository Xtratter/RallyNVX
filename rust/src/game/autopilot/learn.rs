//! Learning to drive from the geometric controller (imitation) with DAgger: the net drives, the controller labels what it would have done in every place the net
//! reached, and the net is trained on all the labels so far. This is how a net learns to recover from its own mistakes instead of only copying a perfect drive.
use super::cfg::AutoCfg;
use super::nn::{Mlp, F, O};
use super::train::Drive;
use crate::game::{input::Input, vehicle::VehicleSpec, world::World};
use std::sync::Arc;

pub type Sample = ([f32; F], [f32; O]);

/// One drive of a stage from its start with `cfg`, the net (when given) driving; the (features, controller's action) pairs recorded when asked.
pub fn rollout(seed: u64, car: VehicleSpec, cfg: AutoCfg, policy: Option<Arc<Mlp>>, record: bool, limit: f32) -> (Drive, Vec<Sample>) {
    let mut w = World::with_roads(seed, car);
    let mut out = Vec::new();
    if !w.place_on_road(2) || !w.set_autopilot_cfg(cfg) { return (Drive::default(), out); }
    w.autopilot_policy(policy, record);
    let (mut t, mut worst, mut flips, mut was) = (0.0f32, f32::MIN, 0u32, false);
    while w.autopilot_on() && t < limit {
        w.step(&Input::default(), 1.0 / 60.0); t += 1.0 / 60.0;
        if record { if let Some(ap) = w.autopilot.as_mut() { out.extend(ap.take_samples()); } }
        if let Some(e) = w.terrain.road_edge_distance(w.vehicle.pos.x, w.vehicle.pos.z) { if t > 8.0 { worst = worst.max(e); } }
        let f = w.vehicle.flipped(); if f && !was { flips += 1; } was = f;
    }
    (Drive { finished: !w.autopilot_on(), time: t, worst_edge: worst.max(-5.0), flips, damage: w.vehicle.damage.total(), recoveries: 0 }, out)
}

/// Rollouts of several seeds, at most `par` at a time.
pub fn rollouts(seeds: &[u64], car: VehicleSpec, cfg: AutoCfg, policy: Option<Arc<Mlp>>, record: bool, par: usize) -> Vec<(Drive, Vec<Sample>)> {
    let mut all = Vec::new();
    for chunk in seeds.chunks(par.max(1)) {
        let hs: Vec<_> = chunk.iter().map(|&s| { let p = policy.clone(); std::thread::spawn(move || rollout(s, car, cfg, p, record, 700.0)) }).collect();
        all.extend(hs.into_iter().map(|h| h.join().unwrap_or_else(|_| (Drive::default(), Vec::new()))));
    }
    all
}

/// The training set from samples.
pub fn to_set(samples: &[Sample]) -> (Vec<[f32; F]>, Vec<[f32; O]>, Vec<f32>) {
    (samples.iter().map(|s| s.0).collect(), samples.iter().map(|s| s.1).collect(), vec![1.0; samples.len()])
}

/// The imitation with DAgger: `iters` rounds after the first (the controller's own drive); the net drives, the controller labels, the net trains on everything so far.
/// `progress` gets 0..1 after each round; returning `true` from `stop` ends it early with the net so far.
pub fn dagger(car: VehicleSpec, cfg: AutoCfg, seeds: &[u64], iters: usize, mut progress: impl FnMut(f32), stop: impl Fn() -> bool) -> Mlp {
    let mut data: Vec<Sample> = Vec::new(); let mut net = Mlp::new(1);
    for it in 0..=iters {
        if stop() { break; }
        let policy = if it == 0 { None } else { Some(Arc::new(net.clone())) };
        for (_, s) in rollouts(seeds, car, cfg, policy, true, 3) { data.extend(s); }
        let (xs, ys, ws) = to_set(&data);
        net.train(&xs, &ys, &ws, if it == 0 { 60 } else { 25 }, 0.002, it as u64 + 3);
        progress((it + 1) as f32 / (iters + 1) as f32);
    }
    net
}

#[cfg(test)]
mod tests {
    use super::*;
    fn started() -> World {
        let mut w = World::with_roads(2, VehicleSpec::by_id("vulpes").unwrap());
        assert!(w.place_on_road(2) && w.set_autopilot(true));
        for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); }
        w
    }
    #[test] fn the_features_are_finite_and_in_range_on_the_road() {
        let w = started();
        let f = w.autopilot.as_ref().unwrap().features(&w.vehicle);
        assert!(f.iter().all(|x| x.is_finite() && x.abs() <= 2.6), "{f:?}");
        assert!(f[0] > 0.0 && f[22] == 1.0 && f[21] > 0.5, "moving, on the ground, daring: {f:?}");
    }
    #[test] fn a_policy_drives_in_place_of_the_controller_and_the_controller_takes_over_when_the_car_is_lost() {
        let mut w = started();
        let zero = Arc::new(Mlp::from_vec(&vec![0.0; Mlp::new(1).params()]).unwrap());   // outputs 0: no steering, no throttle
        let ap = w.autopilot.as_mut().unwrap(); ap.set_policy(Some(zero), true);
        let c = ap.drive(&w.vehicle, 1.0 / 60.0);
        assert!(c.input.throttle == 0.0 && c.input.steer == 0.0 && !c.input.brake, "the net's command: {:?}", c.input);
        // the controller's own action is recorded as the label
        for _ in 0..6 { ap.drive(&w.vehicle, 1.0 / 60.0); }
        let s = ap.take_samples(); assert!(!s.is_empty() && s.iter().all(|(f, y)| f.iter().all(|x| x.is_finite()) && y.iter().all(|x| x.abs() <= 1.0)));
        // 7 m off the line: the controller takes over for a while
        let mut far = w.vehicle; far.pos = far.pos + far.right() * 9.0;
        let c = ap.drive(&far, 1.0 / 60.0);
        let c2 = ap.drive(&far, 1.0 / 60.0);
        assert!(c.teleport.is_some() || c2.input.throttle > 0.0 || c2.input.brake || c2.input.steer != 0.0, "the controller acts: {:?}", c2.input);
    }
    #[test] fn the_neural_style_uses_the_controller_it_copied_and_a_bad_net_is_refused() {
        use super::super::cfg::{cfg_style, set_net, reset_net, net};
        reset_net();
        assert_eq!(cfg_style(6), cfg_style(1), "no net: the normal style's numbers");
        assert!(!set_net(&[0.0; 10], 1) && !set_net(&vec![f32::NAN; Mlp::new(1).params()], 1) && net().is_none());
        let good = Mlp::new(2).to_vec();
        assert!(set_net(&good, 5)); assert_eq!(net().unwrap().1, 5); assert_eq!(cfg_style(6), cfg_style(5));
        assert!(set_net(&good, 77)); assert_eq!(net().unwrap().1, 1, "any other source means the normal style");
        reset_net();
    }
    fn show(tag: &str, rs: &[(Drive, Vec<Sample>)], seeds: &[u64]) { for (r, s) in rs.iter().zip(seeds) { println!("{tag} seed {s}: finished {} t {:.0}s worst edge {:.1} m flips {} damage {:.0}%", r.0.finished, r.0.time, r.0.worst_edge, r.0.flips, r.0.damage * 100.0); } }
    /// The imitation with DAgger (slow; `AP_CAR`, `AP_ITER`): prints the net's drives of unseen levels against the controller's.
    #[test] #[ignore] fn the_net_learns_to_drive_from_the_controller() {
        let car = VehicleSpec::by_id(&std::env::var("AP_CAR").unwrap_or_else(|_| "vulpes".into())).unwrap();
        let iters: usize = std::env::var("AP_ITER").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
        let cfg = super::super::cfg::cfg_style(std::env::var("AP_STYLE").ok().and_then(|s| s.parse().ok()).unwrap_or(1));
        let (train_seeds, test_seeds) = ([2u64, 3, 5, 8], [1u64, 21]);
        let base = rollouts(&test_seeds, car, cfg, None, false, 2); show("controller", &base, &test_seeds);
        let mut data: Vec<Sample> = Vec::new(); let mut net = Mlp::new(1);
        for it in 0..=iters {
            let policy = if it == 0 { None } else { Some(Arc::new(net.clone())) };
            let rs = rollouts(&train_seeds, car, cfg, policy, true, 3);
            for (_, s) in rs { data.extend(s); }
            let (xs, ys, ws) = to_set(&data);
            let loss = net.train(&xs, &ys, &ws, if it == 0 { 60 } else { 25 }, 0.002, it as u64 + 3);
            println!("iteration {it}: {} samples, loss {loss:.4}", data.len());
            let ev = rollouts(&test_seeds, car, cfg, Some(Arc::new(net.clone())), false, 2); show(&format!("net@{it}"), &ev, &test_seeds);
        }
        let _ = (F, O);
    }
}
