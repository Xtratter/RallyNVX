//! The autopilot against the real generated stages (slow: ignored by default, run with `--ignored`).
use crate::game::{input::Input, world::World, vehicle::VehicleSpec};

struct Report { damage: f32, coolant: f32, clutch: f32, tyre_temp: f32, tyre_wear: f32, slide_pct: f32, max_lat: f32, seed: u64, finished: bool, time: f32, mean_kmh: f32, worst_edge: f32, flips: u32, recoveries: u32, length: f32 }

/// A thread that waits for a free slot first (`AP_PAR`, 4 by default): a stage world is big, and a phone must stay alive while the seeds run.
fn spawn_lim<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> std::thread::JoinHandle<T> {
    use std::sync::{Condvar, Mutex};
    static SLOTS: (Mutex<Option<usize>>, Condvar) = (Mutex::new(None), Condvar::new());
    std::thread::spawn(move || {
        let (m, c) = &SLOTS;
        let mut g = m.lock().unwrap();
        loop { let free = g.get_or_insert_with(|| std::env::var("AP_PAR").ok().and_then(|s| s.parse().ok()).unwrap_or(4)); if *free > 0 { *free -= 1; break; } g = c.wait(g).unwrap(); }
        drop(g);
        let r = f();
        *m.lock().unwrap().as_mut().unwrap() += 1; c.notify_one();
        r
    })
}

fn car() -> VehicleSpec { VehicleSpec::by_id(&std::env::var("AP_CAR").unwrap_or_else(|_| "offroader".into())).expect("a car id") }
fn env_style() -> i32 { std::env::var("AP_STYLE").ok().and_then(|s| s.parse().ok()).unwrap_or(1) }
fn run(seed: u64, from_field: bool) -> Report { run_from(seed, from_field, false, env_style()) }

/// `spawn`: start where the level starts (on the liaison) instead of at the stage start.
fn run_from(seed: u64, from_field: bool, spawn: bool, style: i32) -> Report {
    if let Ok(p) = std::env::var("AP_JSON") { crate::game::autopilot::cfg::load_override(std::path::Path::new(&p)); }
    let mut w = World::with_roads(seed, car());
    if !spawn { assert!(w.place_on_road(2)); }
    if from_field {
        let p = w.vehicle.pos;
        // 250 m to the side of the stage start
        let (x, z) = (p.x + 250.0, p.z + 250.0);
        w.vehicle = crate::game::vehicle::Vehicle::new(w.car_spec, crate::game::math::v3(x, w.terrain.height(x, z) + 1.1, z), 0.0);
    }
    let length = w.terrain.roads().unwrap().roads.iter().map(|r| r.length()).sum::<f32>();
    assert!(w.set_autopilot_style(style), "seed {seed}");
    let (mut t, mut dist, mut worst, mut flips) = (0.0f32, 0.0f32, f32::MIN, 0u32);
    let first = if from_field { f32::MAX } else { -1.0e9 };   // set below once the stage is joined
    let mut first = first;
    let (mut slide_steps, mut steps, mut max_lat) = (0u32, 0u32, 0.0f32);
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
        steps += 1; let lat = w.vehicle.wheels.iter().map(|x| x.lat_slip).fold(0.0, f32::max); max_lat = max_lat.max(lat);
        if lat > 1.0 { slide_steps += 1; }   // a side slip over 1 m/s (the real wheels, ABS and traction control hold a slide short)
        let f = w.vehicle.flipped(); if f && !was_flipped { flips += 1; } was_flipped = f;
    }
    Report { damage: w.vehicle.damage.total(), coolant: w.drivetrain.coolant.temp, clutch: w.drivetrain.clutch.temp, tyre_temp: w.vehicle.tyres.iter().map(|y| y.temp).sum::<f32>() / 4.0, tyre_wear: w.vehicle.tyres.iter().map(|y| y.wear).sum::<f32>() / 4.0, slide_pct: 100.0 * slide_steps as f32 / steps.max(1) as f32, max_lat, seed, finished: !w.autopilot_on(), time: t, mean_kmh: dist / t.max(0.1) * 3.6, worst_edge: worst, flips, recoveries: 0, length }
}

fn print(r: &Report) {
    println!("seed {} finished {} t {:.0}s len {:.0} m mean {:.0} km/h worst edge {:.1} m flips {} sliding {:.0}% max side slip {:.1} m/s, tyres {:.0} C worn {:.0}%, coolant {:.0} C clutch {:.0} C damage {:.0}%", r.seed, r.finished, r.time, r.length, r.mean_kmh, r.worst_edge, r.flips, r.slide_pct, r.max_lat, r.tyre_temp, r.tyre_wear * 100.0, r.coolant, r.clutch, r.damage * 100.0);
}
fn check(r: &Report) {
    println!("seed {} finished {} t {:.0}s len {:.0} m mean {:.0} km/h worst edge {:.1} m flips {}", r.seed, r.finished, r.time, r.length, r.mean_kmh, r.worst_edge, r.flips);
    assert!(r.finished, "seed {}: did not finish", r.seed);
    assert!(r.flips == 0, "seed {}: flipped", r.seed);
    assert!(r.worst_edge < 0.5, "seed {}: left the road by {} m", r.seed, r.worst_edge);
    assert!(r.mean_kmh > 42.0, "seed {}: only {} km/h", r.seed, r.mean_kmh);
}

#[test] #[ignore] fn whole_stage_seeds_in_parallel() {
    let hs: Vec<_> = [1u64, 2, 3, 5, 8, 21].iter().map(|&s| spawn_lim(move || run(s, false))).collect();
    let rs: Vec<Report> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    for r in &rs { print(r); }
    for r in &rs { check(r); }
}
#[test] #[ignore] fn from_the_field_to_the_finish() {
    let hs: Vec<_> = [4u64, 21].iter().map(|&s| spawn_lim(move || run(s, true))).collect();
    for h in hs { let r = h.join().unwrap(); check(&r); }
}

#[test] #[ignore] fn the_whole_drive_from_the_spawn_through_the_city_to_the_finish() {
    let hs: Vec<_> = [1u64, 2, 3, 5, 8, 21].iter().map(|&s| spawn_lim(move || run_from(s, false, true, env_style()))).collect();
    let rs: Vec<Report> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    for r in &rs { check(r); }
}

#[test] #[ignore] fn the_styles_order_calm_normal_aggressive() {
    let hs: Vec<_> = [3u64, 8].iter().flat_map(|&s| (0..3).map(move |st| (s, st))).map(|(s, st)| spawn_lim(move || (s, st, run_from(s, false, false, st)))).collect();
    let rs: Vec<(u64, i32, Report)> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    for (s, st, r) in &rs { println!("style {st}: "); print(r); let _ = s; }
    for seed in [3u64, 8] {
        let get = |st: i32| rs.iter().find(|(s, t, _)| *s == seed && *t == st).map(|(_, _, r)| r).unwrap();
        let (calm, normal, aggr) = (get(0), get(1), get(2));
        assert!(calm.finished && normal.finished && aggr.finished, "seed {seed}");
        assert!(calm.time > normal.time * 1.04 && aggr.time <= normal.time * 1.03, "seed {seed}: times {:.0} {:.0} {:.0}", calm.time, normal.time, aggr.time);
        assert!(calm.worst_edge < 0.0 && normal.worst_edge < 0.5, "seed {seed}: the calm and normal styles stay on the road");
        assert!(aggr.worst_edge < 3.0 && aggr.flips == 0, "seed {seed}: aggressive edge {} flips {}", aggr.worst_edge, aggr.flips);
        assert!(normal.slide_pct < 4.0 && calm.slide_pct < 1.0, "seed {seed}: the normal style slides {:.1}%, the calm one {:.1}%", normal.slide_pct, calm.slide_pct);
    }
}

#[test] #[ignore] fn two_laps_of_the_city_loop() {
    let hs: Vec<_> = [1u64, 2, 3, 5].iter().map(|&seed| spawn_lim(move || {
        let mut w = World::with_roads(seed, VehicleSpec::offroader());
        assert!(w.place_on_road(5), "seed {seed}");
        let pts = w.terrain.city_loop().unwrap().pts.clone();
        let len = w.terrain.city_loop().unwrap().length;
        assert!(w.set_autopilot(true));
        let (mut t, mut worst, mut flips, mut dist) = (0.0f32, 0.0f32, 0u32, 0.0f32);
        let mut was = false;
        while w.autopilot_on() && t < 400.0 {
            let b = w.vehicle.pos;
            w.step(&Input::default(), 1.0 / 60.0); t += 1.0 / 60.0; dist += (w.vehicle.pos - b).len();
            let off = pts.iter().step_by(2).map(|p| (p.x - w.vehicle.pos.x).hypot(p.z - w.vehicle.pos.z)).fold(f32::INFINITY, f32::min);
            worst = worst.max(off);
            let f = w.vehicle.flipped(); if f && !was { flips += 1; } was = f;
        }
        (seed, !w.autopilot_on(), t, len, dist, worst, flips)
    })).collect();
    for h in hs {
        let (seed, done, t, len, dist, worst, flips) = h.join().unwrap();
        println!("loop seed {seed}: finished {done} t {t:.0}s loop {len:.0} m driven {dist:.0} m worst offset {worst:.1} m flips {flips}");
        assert!(done && flips == 0 && worst < 6.5 && dist > len * 1.8 && dist < len * 2.4, "seed {seed}");
    }
}

/// A rally stage driven by the autopilot: returns (finished, time, penalty, splits, lines, assisted).
fn rally_run(seed: u64, stage: u32, style: i32) -> (bool, f32, f32, usize, usize, bool) {
    let mut w = World::with_roads(seed, VehicleSpec::offroader());
    assert!(w.rally_start(stage));
    let lines = w.rally.as_ref().unwrap().run.line_count();
    for _ in 0..(4.0 * 60.0) as usize { w.step(&Input::default(), 1.0 / 60.0); }
    assert!(w.set_autopilot_style(style), "the autopilot starts");
    let mut t = 0.0;
    while w.rally.as_ref().unwrap().run.phase != crate::game::rally::Phase::Finished && t < 900.0 { w.step(&Input::default(), 1.0 / 60.0); t += 1.0 / 60.0; }
    let r = w.rally.as_ref().unwrap();
    (r.run.phase == crate::game::rally::Phase::Finished, r.run.time, r.run.penalty, r.run.splits.len(), lines, r.run.assisted)
}

#[test] #[ignore] fn the_autopilot_finishes_both_rally_stages_without_penalties() {
    let hs: Vec<_> = [(21u64, 0u32), (3, 0), (21, 1), (3, 1)].iter().map(|&(s, st)| spawn_lim(move || (s, st, rally_run(s, st, 1)))).collect();
    for h in hs {
        let (seed, stage, (done, time, penalty, splits, lines, assisted)) = h.join().unwrap();
        println!("rally seed {seed} stage {stage}: finished {done} time {time:.1}s penalty {penalty} splits {splits}/{lines} assisted {assisted}");
        assert!(done && penalty == 0.0 && splits + 2 == lines && assisted, "seed {seed} stage {stage}");
    }
}


/// A diagnosis of one stage drive: where the car brakes with no bend ahead, how long the handbrake is held and how it keeps to the planned speed.
fn diagnose(seed: u64, style: i32) -> String {
    if let Ok(p) = std::env::var("AP_JSON") { crate::game::autopilot::cfg::load_override(std::path::Path::new(&p)); }
    let mut w = World::with_roads(seed, car());
    assert!(w.place_on_road(2));
    assert!(w.set_autopilot_style(style));
    let (mut t, mut brake_t, mut hand_t, mut hand_n, mut was_hand) = (0.0f32, 0.0f32, 0.0f32, 0u32, false);
    let (mut brake_straight, mut slow_t, mut road_t, mut ratio_sum) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut events: Vec<String> = Vec::new();
    let mut last_event = -10.0f32;
    while w.autopilot_on() && t < 1500.0 {
        w.step(&Input::default(), 1.0 / 60.0); t += 1.0 / 60.0;
        let Some(ap) = w.autopilot.as_ref() else { break };
        let r = ap.route(); if r.length() < 3000.0 { continue; }
        let (i, _) = r.nearest(w.vehicle.pos.x, w.vehicle.pos.z);
        let v = w.vehicle.speed(); let plan = r.speed[i].max(1.0);
        let ahead = r.ahead(i, (v * 3.0).max(30.0)).0;
        let kmax = r.kappa[i..=ahead].iter().cloned().fold(0.0, f32::max);
        road_t += 1.0 / 60.0; ratio_sum += (v / plan).min(1.5) / 60.0;
        if std::env::var("OFFLOG").is_ok() { if let Some(rm) = w.terrain.road_edge_distance(w.vehicle.pos.x, w.vehicle.pos.z).map(|e| -e) { if rm < -0.1 && t - last_event > 1.5 { last_event = t; let lr = r.room(i, r.p[i].x, r.p[i].z).unwrap_or(0.0); eprintln!("OUT seed {seed} t {:.0} s {:.0} terrain-room {:.1} route-room {:.1} line-room {:.1} cross {:.1} v {:.1} plan {:.1} kraw {:.3} steer {:.2} slip {:.1}", t, r.s[i], rm, r.room(i, w.vehicle.pos.x, w.vehicle.pos.z).unwrap_or(9.9), lr, r.cross(i, w.vehicle.pos.x, w.vehicle.pos.z), v, plan, r.kraw[i], w.vehicle.steer, w.vehicle.wheels.iter().map(|x| x.lat_slip.abs()).fold(0.0, f32::max)); } } }
        if w.vehicle.braking { brake_t += 1.0 / 60.0; }
        if w.vehicle.braking && kmax < 0.006 && v > 8.0 { brake_straight += 1.0 / 60.0; if t - last_event > 4.0 && events.len() < 6 { last_event = t; events.push(format!("brake on a straight at t {:.0} s {:.0} v {:.1} plan {:.1} kmax {:.4}", t, r.s[i], v, plan, kmax)); } }
        if w.vehicle.parked { hand_t += 1.0 / 60.0; if !was_hand { hand_n += 1; if events.len() < 12 { events.push(format!("handbrake at t {:.0} s {:.0} v {:.1} plan {:.1} kmax {:.4}", t, r.s[i], v, plan, kmax)); } } }
        was_hand = w.vehicle.parked;
        if v < 0.6 * plan && plan > 12.0 { slow_t += 1.0 / 60.0; if t - last_event > 4.0 && events.len() < 18 { last_event = t; events.push(format!("slow at t {:.0} s {:.0} v {:.1} plan {:.1} kmax {:.4}", t, r.s[i], v, plan, kmax)); } }
    }
    format!("seed {seed} style {style}: t {:.0}s brake {:.0}s (on straights {:.0}s) handbrake {:.1}s in {} flicks, under 60% of plan {:.0}s, v/plan {:.2}\n  {}", t, brake_t, brake_straight, hand_t, hand_n, slow_t, ratio_sum / road_t.max(1.0), events.join("\n  "))
}

#[test] #[ignore] fn diagnose_the_autopilot() {
    let st = env_style();
    let seeds: Vec<u64> = std::env::var("AP_SEEDS").map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect()).unwrap_or_else(|_| vec![1, 2, 3]);
    let hs: Vec<_> = seeds.iter().map(|&s| spawn_lim(move || diagnose(s, st))).collect();
    for h in hs { println!("{}", h.join().unwrap()); }
}

#[test] #[ignore] fn max_attack_is_faster_slides_and_cuts_the_corners_without_wrecking_the_car() {
    let hs: Vec<_> = [3u64, 8].iter().flat_map(|&s| [1, 5].map(move |st| (s, st))).map(|(s, st)| spawn_lim(move || (s, st, run_from(s, false, false, st)))).collect();
    let rs: Vec<(u64, i32, Report)> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    for seed in [3u64, 8] {
        let get = |st: i32| rs.iter().find(|(s, t, _)| *s == seed && *t == st).map(|(_, _, r)| r).unwrap();
        let (normal, attack) = (get(1), get(5));
        print(attack);
        assert!(normal.finished && attack.finished && attack.flips == 0, "seed {seed}");
        assert!(attack.time < normal.time * 0.95, "seed {seed}: max attack {:.0} s, normal {:.0} s", attack.time, normal.time);
        assert!(attack.slide_pct > 3.0 && attack.slide_pct > normal.slide_pct * 2.0, "seed {seed}: it slides ({:.1}% against {:.1}%)", attack.slide_pct, normal.slide_pct);
        assert!(attack.worst_edge > normal.worst_edge && attack.worst_edge < 6.0, "seed {seed}: it cuts the corners (edge {:.1} m against {:.1} m) but stays near the road", attack.worst_edge, normal.worst_edge);
        assert!(attack.damage < 0.15, "seed {seed}: damage {:.0}%", attack.damage * 100.0);
    }
}

/// Which gears the automatic uses when the autopilot drives, and at what revs (diagnosis).
#[test] #[ignore] fn diagnose_the_gears() {
    let st = env_style();
    let mut w = World::with_roads(2, car());
    if let Ok(p) = std::env::var("AP_PROFILE") { w.drivetrain.profile = crate::game::shiftlogic::Profile::from_i32(p.parse().unwrap_or(1)); }
    if let Ok(m) = std::env::var("AP_ASSISTS") { w.assists = crate::game::assists::Assists::from_mask(m.parse().unwrap_or(255)); }
    assert!(w.place_on_road(2) && w.set_autopilot_style(st));
    let (mut t, mut gears, mut rpm_sum, mut n) = (0.0f32, [0.0f32; 8], 0.0f32, 0.0f32);
    let mut rows: Vec<String> = Vec::new();
    while w.autopilot_on() && t < 200.0 {
        w.step(&Input::default(), 1.0 / 60.0); t += 1.0 / 60.0;
        let g = w.drivetrain.gear.clamp(-1, 6); gears[(g + 1) as usize] += 1.0 / 60.0; rpm_sum += w.drivetrain.rpm(); n += 1.0;
        if ((t * 60.0) as u32) % 600 == 0 { rows.push(format!("t {:.0} v {:.1} m/s gear {} rpm {:.0} mode {:?}", t, w.vehicle.speed(), w.drivetrain.gear, w.drivetrain.rpm(), w.drivetrain.mode)); }
    }
    println!("seconds per gear (R,N,1..6): {:?}; mean rpm {:.0}\n{}", gears.iter().map(|x| x.round()).collect::<Vec<_>>(), rpm_sum / n.max(1.0), rows.join("\n"));
}
