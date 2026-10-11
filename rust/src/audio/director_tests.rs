use super::*;
use crate::game::{flight::{AircraftKind, Status}, input::Input, math::v3, missions::{Mission, MissionKind}, terrain::Terrain, world::World};

fn settled() -> World { let mut w = World::new(7); for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); } w }
fn air_mission(kind: MissionKind, ac: AircraftKind, pts: Vec<crate::game::math::V3>) -> Mission {
    let goal = *pts.last().unwrap();
    Mission { kind, seed: 42, difficulty: 1, start: v3(0., 0., 0.), goal, checkpoints: pts, length_m: 300.0, time_limit: 100.0, reward_base: 100.0, aircraft: Some(ac), urban: false, start_yaw: None, normals: vec![] }
}
fn heli_world() -> World { World::for_mission(air_mission(MissionKind::Rings, AircraftKind::Heli, vec![v3(0., 30., 100.), v3(0., 30., 200.)])) }

#[test] fn free_drive_at_rest_is_quiet_and_driving_raises_the_revs() {
    let mut w = settled(); let mut d = Director::new();
    let (p, ev) = d.update(&w, 0.0, true);
    assert_eq!(p.machine, Machine::Car); assert!(p.active && p.rpm < 0.3 && p.roll < 0.05 && ev.is_empty(), "{p:?} {ev:?}");
    for _ in 0..240 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    let (p2, _) = d.update(&w, 1.0, true);
    assert!(p2.rpm > p.rpm + 0.3 && p2.roll > 0.3 && p2.load > p.load && p2.surface == Surface::Ground, "{p2:?}");
}
#[test] fn landing_after_a_jump_gives_one_touchdown_scaled_by_speed() {
    let mut w = settled(); let mut d = Director::new();
    d.update(&w, 0.0, true);
    w.vehicle.contacts = 0; w.vehicle.vel.y = -9.0; w.time += 0.4;
    assert!(d.update(&w, 0.0, true).1.is_empty());
    w.vehicle.contacts = 4;
    let ev = d.update(&w, 0.0, true).1;
    assert!(matches!(ev.as_slice(), [Event::Touchdown(s)] if *s > 0.7 && *s <= 1.0), "{ev:?}");
    assert!(d.update(&w, 0.0, true).1.is_empty(), "no repeat while grounded");
    w.vehicle.contacts = 0; w.vehicle.vel.y = -3.5; w.time += 0.4; d.update(&w, 0.0, true); w.vehicle.contacts = 4;
    assert!(matches!(d.update(&w, 0.0, true).1.as_slice(), [Event::Touchdown(s)] if *s < 0.5));
    w.vehicle.contacts = 0; w.vehicle.vel.y = -1.0; w.time += 0.4; d.update(&w, 0.0, true); w.vehicle.contacts = 4;
    assert!(d.update(&w, 0.0, true).1.is_empty(), "a gentle landing is silent");
}
#[test] fn a_sudden_stop_is_a_crash_and_entering_a_lake_splashes_once() {
    let mut w = settled(); let mut d = Director::new();
    w.vehicle.vel = v3(0., 0., 15.); d.update(&w, 1.0, true);
    w.vehicle.vel = v3(0., 0., 3.);
    assert!(d.update(&w, 1.0, true).1.contains(&Event::Crash));
    assert!(!d.update(&w, 1.0, true).1.contains(&Event::Crash));
    let mut w = settled(); let mut d = Director::new();
    w.terrain = Terrain::flooded(-5.0);            // same seed as the flooded terrain below, but dry
    w.vehicle.vel = v3(0., 0., 12.); d.update(&w, 1.0, true);
    w.terrain = Terrain::flooded(1.0);
    let (p, ev) = d.update(&w, 1.0, true);
    assert_eq!(p.surface, Surface::Water);
    assert!(matches!(ev.as_slice(), [Event::Splash(s)] if *s > 0.3), "{ev:?}");
    assert!(d.update(&w, 1.0, true).1.is_empty());
}
#[test] fn aircraft_machines_revs_wind_and_touchdown() {
    let mut w = heli_world(); let mut d = Director::new();
    w.aircraft.as_mut().unwrap().status = Status::Landed;
    let (p, _) = d.update(&w, 0.0, true);
    assert_eq!(p.machine, Machine::Heli); assert!((p.rpm - 0.45).abs() < 1e-3, "{}", p.rpm);
    { let a = w.aircraft.as_mut().unwrap(); a.status = Status::Flying; a.pos.y += 80.0; a.vel = v3(0., 0., 25.); }
    let (p2, ev) = d.update(&w, 0.0, true);
    assert!(p2.rpm > 0.6 && p2.wind > p.wind + 0.2 && ev.is_empty(), "{p2:?}");
    { let a = w.aircraft.as_mut().unwrap(); a.vel = v3(0., -2.5, 0.); } d.update(&w, 0.0, true);
    w.aircraft.as_mut().unwrap().status = Status::Landed;
    assert!(matches!(d.update(&w, 0.0, true).1.as_slice(), [Event::Touchdown(s)] if *s > 0.5));
    let dw = World::for_mission(air_mission(MissionKind::Recon, AircraftKind::Drone, vec![v3(0., 40., 100.)]));
    assert_eq!(Director::new().update(&dw, 0.0, true).0.machine, Machine::Drone);
    w.aircraft.as_mut().unwrap().status = Status::Crashed;
    assert!(d.update(&w, 0.0, true).1.contains(&Event::Crash));
}
#[test] fn mission_progress_gives_checkpoint_board_and_one_result_event() {
    let mut w = heli_world(); let mut d = Director::new();
    w.step_with(&Input::default(), &crate::game::flight::FlightInput { yaw: 0.2, ..Default::default() }, 1.0 / 60.0);
    d.update(&w, 0.0, true);
    // fly through ring 1
    let mut crossed = false; let mut ev_all = Vec::new();
    for (z0, z1) in [(99.0, 101.0), (199.0, 201.0)] {
        for z in [z0, z1] { w.aircraft.as_mut().unwrap().pos = v3(0., 30., z); w.step_with(&Input::default(), &crate::game::flight::FlightInput { yaw: 0.2, ..Default::default() }, 1e-4); ev_all.extend(d.update(&w, 0.0, true).1); }
        crossed = true;
    }
    assert!(crossed);
    assert_eq!(ev_all.iter().filter(|e| **e == Event::Checkpoint).count(), 1, "{ev_all:?}");
    assert_eq!(ev_all.iter().filter(|e| **e == Event::Success).count(), 1, "{ev_all:?}");
    assert!(d.update(&w, 0.0, true).1.is_empty(), "the result is reported once");
    let mut w = heli_world(); let mut d = Director::new(); d.update(&w, 0.0, true);
    w.abandon_mission();
    assert_eq!(d.update(&w, 0.0, true).1, vec![Event::Fail]);
}
#[test] fn paused_is_inactive_without_events_and_a_new_world_is_silent() {
    let mut w = settled(); let mut d = Director::new();
    d.update(&w, 0.0, true);
    w.vehicle.contacts = 0; w.vehicle.vel.y = -9.0;
    let (p, ev) = d.update(&w, 0.0, false);
    assert!(!p.active && ev.is_empty());
    w.vehicle.contacts = 4;
    assert!(d.update(&w, 0.0, false).1.is_empty());
    let mut w2 = World::new(99); for _ in 0..30 { w2.step(&Input::default(), 1.0 / 60.0); }
    w2.vehicle.vel = v3(0., 0., 20.);
    let ev = d.update(&w2, 0.0, true).1;
    assert!(ev.is_empty(), "first update of another world must not emit events: {ev:?}");
}

#[test] fn a_reset_while_driving_is_not_a_crash() {
    let mut w = settled(); let mut d = Director::new();
    w.vehicle.vel = v3(0., 0., 18.); d.update(&w, 1.0, true);
    w.vehicle.reset_upright(&w.terrain);        // what the Reset button does: velocity zeroed, car lifted
    w.time += 1.0 / 60.0;
    assert!(!d.update(&w, 0.0, true).1.contains(&Event::Crash));
}
#[test] fn a_one_frame_contact_flicker_on_a_slope_is_not_a_touchdown() {
    let mut w = settled(); let mut d = Director::new();
    w.vehicle.vel = v3(0., -8.5, 20.); d.update(&w, 1.0, true);
    w.time += 1.0 / 60.0; w.vehicle.contacts = 0; d.update(&w, 1.0, true);
    w.time += 1.0 / 60.0; w.vehicle.contacts = 4;
    assert!(d.update(&w, 1.0, true).1.is_empty(), "no real jump, no thump");
}
#[test] fn a_real_jump_still_thumps_and_the_engine_stops_after_the_result() {
    let mut w = settled(); let mut d = Director::new();
    d.update(&w, 0.0, true);
    w.time += 0.016; w.vehicle.contacts = 0; w.vehicle.vel.y = -7.0; d.update(&w, 0.0, true);
    w.time += 0.4; d.update(&w, 0.0, true);
    w.time += 0.016; w.vehicle.contacts = 4;
    assert!(matches!(d.update(&w, 0.0, true).1.as_slice(), [Event::Touchdown(_)]));
    let mut w = heli_world(); let mut d = Director::new(); d.update(&w, 0.0, true);
    w.abandon_mission();
    let (p, ev) = d.update(&w, 0.0, true);
    assert_eq!((p.machine, ev), (Machine::None, vec![Event::Fail]));
}
