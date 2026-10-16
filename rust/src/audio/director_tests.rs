use super::*;
use crate::game::{input::Input, math::v3, terrain::Terrain, world::World};

fn settled() -> World { let mut w = World::new(7); for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); } w }
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
#[test] fn a_real_jump_still_thumps_and_the_engine_keeps_running() {
    let mut w = settled(); let mut d = Director::new();
    d.update(&w, 0.0, true);
    w.time += 0.016; w.vehicle.contacts = 0; w.vehicle.vel.y = -7.0; d.update(&w, 0.0, true);
    w.time += 0.4; d.update(&w, 0.0, true);
    w.time += 0.016; w.vehicle.contacts = 4;
    assert!(matches!(d.update(&w, 0.0, true).1.as_slice(), [Event::Touchdown(_)]));
}

#[test] fn the_wind_you_hear_is_the_apparent_wind() {
    let mut w = settled(); let mut d = Director::new();
    let (p0, _) = d.update(&w, 0.0, true);
    assert!(p0.wind > 0.01 && p0.wind < 0.1, "a parked car hears the level's wind: {}", p0.wind);
    let wd = w.wind();
    let along = if wd.len() > 0.1 { wd.norm() } else { v3(1.0, 0.0, 0.0) };
    w.time += 1.0 / 60.0;
    w.vehicle.vel = along * 25.0;
    let tail = d.update(&w, 1.0, true).0.wind;
    w.time += 1.0 / 60.0;
    w.vehicle.vel = along * -25.0;
    let head = d.update(&w, 1.0, true).0.wind;
    assert!(head > tail * 1.15, "driving into the wind is noisier than with it: head {head} tail {tail}");
}

#[test] fn the_scene_puts_the_listener_at_the_camera_and_the_sources_on_the_car() {
    use super::{SRC_BAY, SRC_BODY, SRC_EXHAUST, SRC_WHEEL};
    let mut w = settled(); let d = Director::new();
    for _ in 0..30 { w.cam.update(&w.vehicle, &w.terrain, 1.0 / 60.0); }
    let sc = d.scene(&w);
    assert_eq!(sc.listener, w.cam.pos);
    assert!((sc.fwd.len() - 1.0).abs() < 1e-3);
    let c = w.vehicle.pos; let fwd = w.vehicle.forward();
    let along = |p: crate::game::math::V3| (p - c).dot(fwd);
    assert!(along(sc.src[SRC_EXHAUST]) < -1.0 && along(sc.src[SRC_BAY]) > 0.5, "the exhaust is at the tail, the engine bay at the nose");
    assert!((sc.src[SRC_BODY] - c).len() < 1e-4);
    for k in 0..4 { assert!((sc.src[SRC_WHEEL + k] - c).len() < 3.0, "tyre {k}"); assert!(sc.wheel_on[k] == 1.0, "settled on the ground"); }
    assert!(sc.src.iter().all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite()));
}

#[test] fn the_cockpit_hears_the_world_through_the_body_and_the_chase_camera_does_not() {
    use super::{SRC_BAY, SRC_EXHAUST};
    let mut w = World::new_with(7, crate::game::vehicle::VehicleSpec::by_id("vulpes").unwrap()); for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); }
    let d = Director::new();
    for _ in 0..60 { w.cam.update(&w.vehicle, &w.terrain, 1.0 / 60.0); }
    let out = d.scene(&w);
    assert!(out.wind_gain == 1.0 && out.extra[SRC_EXHAUST].gain > 0.9, "the chase camera is outside: {:?}", out.extra[SRC_EXHAUST]);
    w.cam.set_view(5, 1.0, 0.0);
    for _ in 0..60 { w.cam.update(&w.vehicle, &w.terrain, 1.0 / 60.0); }
    let inside = d.scene(&w);
    assert!(Director::cabin(&w).contains(inside.listener), "the driver's eyes are in the cabin: {:?}", inside.listener);
    assert!(inside.wind_gain < 0.6 && inside.wind_hz < 3000.0, "the wind is muffled");
    let ex = inside.extra[SRC_EXHAUST];
    assert!(ex.gain < out.extra[SRC_EXHAUST].gain * 0.5 && ex.hz < 4000.0 && ex.echo.is_none(), "the exhaust is heard through the body: {ex:?}");
    assert!(inside.extra[SRC_BAY].gain > ex.gain, "the engine bay is nearer and louder than the tail pipe");
    assert!(out.extra[SRC_EXHAUST].echo.is_some(), "outside, the ground echoes the exhaust");
}
