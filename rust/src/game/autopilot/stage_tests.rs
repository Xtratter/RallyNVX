//! The autopilot against the real generated stages (slow: ignored by default, run with `--ignored`).
use crate::game::{input::Input, world::World, vehicle::VehicleSpec};

struct Report { seed: u64, finished: bool, time: f32, mean_kmh: f32, worst_edge: f32, flips: u32, recoveries: u32, length: f32 }

fn run(seed: u64, from_field: bool) -> Report { run_from(seed, from_field, false) }

/// `spawn`: start where the level starts (on the liaison) instead of at the stage start.
fn run_from(seed: u64, from_field: bool, spawn: bool) -> Report {
    if let Ok(p) = std::env::var("AP_JSON") { crate::game::autopilot::cfg::load_override(std::path::Path::new(&p)); }
    let mut w = World::with_roads(seed, VehicleSpec::offroader());
    if !spawn { assert!(w.place_on_road(2)); }
    if from_field {
        let p = w.vehicle.pos;
        // 250 m to the side of the stage start
        let (x, z) = (p.x + 250.0, p.z + 250.0);
        w.vehicle = crate::game::vehicle::Vehicle::new(w.car_spec, crate::game::math::v3(x, w.terrain.height(x, z) + 1.1, z), 0.0);
    }
    let length = w.terrain.roads().unwrap().roads.iter().map(|r| r.length()).sum::<f32>();
    assert!(w.set_autopilot(true), "seed {seed}");
    let (mut t, mut dist, mut worst, mut flips) = (0.0f32, 0.0f32, f32::MIN, 0u32);
    let first = if from_field { f32::MAX } else { -1.0e9 };   // set below once the stage is joined
    let mut first = first;
    let mut was_flipped = false; let mut joined = false;   // the edge is only judged once the car has reached the road
    while w.autopilot_on() && t < 1500.0 {
        let before = w.vehicle.pos;
        w.step(&Input::default(), 1.0 / 60.0);
        dist += (w.vehicle.pos - before).len(); t += 1.0 / 60.0;
        let ap_progress = w.autopilot.as_ref().map_or(0.0, |a| a.progress());
        let on_stage_route = w.autopilot.as_ref().map_or(false, |a| a.route().length() > 3000.0 && !w.terrain.in_city(w.vehicle.pos.x, w.vehicle.pos.z, 20.0));
        if on_stage_route && first == f32::MAX { first = ap_progress; }
        if on_stage_route { if let Some(e) = w.terrain.road_edge_distance(w.vehicle.pos.x, w.vehicle.pos.z) { if e < 0.0 && ap_progress >= first + 100.0 { joined = true; } if joined { worst = worst.max(e); if e > 0.3 && std::env::var("DBG").is_ok() { let ap = w.autopilot.as_ref().unwrap(); eprintln!("OFF {:.1} at t {:.0} s {:.0} v {:.1} want {:.1}", e, t, ap.progress(), w.vehicle.speed(), ap.route().speed[ap.route().nearest(w.vehicle.pos.x, w.vehicle.pos.z).0]); } } } }
        if std::env::var("DBG").is_ok() && ((t * 60.0) as u32) % 600 == 0 { let ap = w.autopilot.as_ref().unwrap(); eprintln!("t {:.0} s {:.0}/{:.0} v {:.1} want {:.1} rec {} surf {:?}", t, ap.progress(), ap.route().length(), w.vehicle.speed(), ap.route().speed[ap.route().nearest(w.vehicle.pos.x, w.vehicle.pos.z).0], ap.recoveries, w.vehicle.surface); }
        let f = w.vehicle.flipped(); if f && !was_flipped { flips += 1; } was_flipped = f;
    }
    Report { seed, finished: !w.autopilot_on(), time: t, mean_kmh: dist / t.max(0.1) * 3.6, worst_edge: worst, flips, recoveries: 0, length }
}

fn check(r: &Report) {
    println!("seed {} finished {} t {:.0}s len {:.0} m mean {:.0} km/h worst edge {:.1} m flips {}", r.seed, r.finished, r.time, r.length, r.mean_kmh, r.worst_edge, r.flips);
    assert!(r.finished, "seed {}: did not finish", r.seed);
    assert!(r.flips == 0, "seed {}: flipped", r.seed);
    assert!(r.worst_edge < 0.5, "seed {}: left the road by {} m", r.seed, r.worst_edge);
    assert!(r.mean_kmh > 42.0, "seed {}: only {} km/h", r.seed, r.mean_kmh);
}

#[test] #[ignore] fn whole_stage_seeds_in_parallel() {
    let hs: Vec<_> = [1u64, 2, 3, 5, 8, 21].iter().map(|&s| std::thread::spawn(move || run(s, false))).collect();
    let rs: Vec<Report> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    for r in &rs { check(r); }
}
#[test] #[ignore] fn from_the_field_to_the_finish() {
    let hs: Vec<_> = [4u64, 21].iter().map(|&s| std::thread::spawn(move || run(s, true))).collect();
    for h in hs { let r = h.join().unwrap(); check(&r); }
}

#[test] #[ignore] fn the_whole_drive_from_the_spawn_through_the_city_to_the_finish() {
    let hs: Vec<_> = [1u64, 2, 3, 5, 8, 21].iter().map(|&s| std::thread::spawn(move || run_from(s, false, true))).collect();
    let rs: Vec<Report> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    for r in &rs { check(r); }
}
