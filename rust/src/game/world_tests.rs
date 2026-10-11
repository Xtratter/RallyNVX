use super::*;
use crate::game::math::Q;
#[test] fn world_spawns_upright_on_flat_ground_and_runs() {
    let mut w = World::new(42);
    assert!(w.vehicle.up().y > 0.99);
    for _ in 0..120 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.vehicle.pos.is_finite() && w.vehicle.speed() > 3.0);
}
#[test] fn same_seed_same_world() {
    let (mut a, mut b) = (World::new(5), World::new(5));
    for _ in 0..300 { let i = Input { throttle: 1.0, steer: 0.3, ..Default::default() }; a.step(&i, 1.0 / 60.0); b.step(&i, 1.0 / 60.0); }
    assert_eq!(a.vehicle.pos, b.vehicle.pos);
}
#[test] fn world_wall_keeps_vehicle_inside() {
    let mut w = World::new(1);
    w.vehicle.pos = v3(WORLD_LIMIT - 5.0, 40.0, 0.0);
    w.vehicle.vel = v3(30.0, 0.0, 0.0);
    for _ in 0..600 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.vehicle.pos.x.abs() <= WORLD_LIMIT + 1.0 && w.vehicle.pos.z.abs() <= WORLD_LIMIT + 1.0);
}
#[test] fn reset_input_uprights_a_flipped_car() {
    let mut w = World::new(3);
    w.vehicle.rot = w.vehicle.rot.mul(Q::axis_angle(w.vehicle.forward(), 3.0));
    w.step(&Input { reset: true, ..Default::default() }, 1.0 / 60.0);
    assert!(w.vehicle.up().y > 0.9);
}
#[test] fn props_load_around_a_tile_and_are_dropped_when_far() {
    let mut w = World::new(9);
    w.ensure_props(10, 10, 1);
    assert!(w.props.len() == 9);
    w.ensure_props(-10, -10, 1);
    assert!(w.props.len() == 9);
}
#[test] fn sun_dir_is_unit_and_above_horizon() {
    let w = World::new(1); let s = w.sun_dir();
    assert!((s.len() - 1.0).abs() < 1e-4 && s.y > 0.3);
}
#[test] fn world_view_lists_props_and_vehicle() {
    let mut w = World::new(17);
    w.ensure_props(0, 0, 3);
    let v = w.view(3);
    assert!(!v.props.is_empty());
    assert!(v.sun.len() > 0.99);
}
use crate::game::missions::{Mission, MissionKind};
fn mk(kind: MissionKind, pts: Vec<V3>) -> Mission {
    let goal = *pts.last().unwrap();
    Mission { kind, seed: 42, difficulty: 1, start: v3(0., 0., 0.), goal, checkpoints: pts, length_m: 300.0, time_limit: 100.0, reward_base: 100.0, aircraft: None }
}
#[test] fn for_mission_uses_the_mission_seed_and_starts_ready() {
    let w = World::for_mission(mk(MissionKind::Cargo, vec![v3(0., 0., 300.)]));
    assert_eq!(w.terrain.seed, 42);
    assert!(w.vehicle.up().y > 0.99);
    assert_eq!(w.mission.as_ref().unwrap().phase(), crate::game::run::Phase::Ready);
}
#[test] fn world_step_feeds_the_run() {
    let mut w = World::for_mission(mk(MissionKind::Cargo, vec![v3(0., 0., 300.)]));
    for _ in 0..30 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.mission.as_ref().unwrap().time() > 0.3);
}
#[test] fn cargo_markers_are_one_beacon() {
    let w = World::for_mission(mk(MissionKind::Cargo, vec![v3(10., 2., 300.)]));
    let m = w.markers();
    assert_eq!(m.len(), 1);
    assert_eq!((m[0].kind, m[0].pos), (MarkerKind::Beacon, v3(10., 2., 300.)));
}
#[test] fn race_markers_highlight_the_next_checkpoint_and_drop_passed_ones() {
    let mut w = World::for_mission(mk(MissionKind::Race, vec![v3(0., 0., 100.), v3(0., 0., 200.), v3(0., 0., 300.)]));
    let kinds = |w: &World| w.markers().iter().map(|m| m.kind).collect::<Vec<_>>();
    assert_eq!(kinds(&w), vec![MarkerKind::Next, MarkerKind::Ring, MarkerKind::Ring, MarkerKind::Beacon]);
    w.vehicle.pos = v3(0., 0.9, 100.0);
    w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0);
    assert_eq!(kinds(&w), vec![MarkerKind::Next, MarkerKind::Ring, MarkerKind::Beacon]);
}
#[test] fn no_markers_without_a_mission_or_when_done() {
    assert!(World::new(1).markers().is_empty());
    let mut w = World::for_mission(mk(MissionKind::Cargo, vec![v3(0., 0., 300.)]));
    w.mission.as_mut().unwrap().abandon();
    assert!(w.markers().is_empty());
}
#[test] fn view_carries_the_markers() {
    let w = World::for_mission(mk(MissionKind::Cargo, vec![v3(0., 0., 300.)]));
    assert_eq!(w.view(1).markers.len(), 1);
}
#[test] fn mission_starts_facing_the_first_target_with_the_camera_behind() {
    let w = World::for_mission(mk(MissionKind::Race, vec![v3(300., 0., -200.), v3(600., 0., -400.)]));
    let want = v3(300., 0., -200.).norm();
    let f = w.vehicle.forward();
    assert!(f.x * want.x + f.z * want.z > 0.999, "forward {f:?}");
    assert!(w.vehicle.up().y > 0.99);
    let to_cam = (w.cam.pos - w.vehicle.pos).norm();
    assert!(to_cam.x * f.x + to_cam.z * f.z < -0.9, "camera must start behind the car");
}
fn air(kind: MissionKind, ac: crate::game::flight::AircraftKind, pts: Vec<V3>) -> Mission {
    let goal = *pts.last().unwrap();
    Mission { kind, seed: 42, difficulty: 1, start: v3(0., 0., 0.), goal, checkpoints: pts, length_m: 300.0, time_limit: 100.0, reward_base: 100.0, aircraft: Some(ac) }
}
use crate::game::flight::{AircraftKind, FlightInput};
#[test] fn air_mission_starts_with_the_aircraft_3_m_up_facing_the_first_target() {
    let w = World::for_mission(air(MissionKind::Rings, AircraftKind::Heli, vec![v3(300., 30., -200.), v3(600., 40., -400.)]));
    let a = w.aircraft.as_ref().unwrap();
    assert!((a.pos.y - (w.terrain.height(0., 0.) + 3.0)).abs() < 0.01);
    let want = v3(300., 0., -200.).norm(); let f = a.forward();
    assert!(f.x * want.x + f.z * want.z > 0.999);
    assert!(w.air.is_some() && w.mission.is_none());
    assert_eq!(w.player_pos(), a.pos);
}
#[test] fn air_steps_feed_the_aircraft_and_the_run() {
    let mut w = World::for_mission(air(MissionKind::Rings, AircraftKind::Heli, vec![v3(0., 30., 300.)]));
    for _ in 0..60 { w.step_with(&Input::default(), &FlightInput { pitch: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.aircraft.as_ref().unwrap().vel.len() > 1.0);
    assert!(w.air.as_ref().unwrap().time() > 0.9);
    assert!(w.run_info().is_some());
}
#[test] fn the_car_does_not_move_in_an_air_mission() {
    let mut w = World::for_mission(air(MissionKind::Rings, AircraftKind::Heli, vec![v3(0., 30., 300.)]));
    let p = w.vehicle.pos;
    for _ in 0..120 { w.step_with(&Input { throttle: 1.0, ..Default::default() }, &FlightInput::default(), 1.0 / 60.0); }
    assert_eq!(w.vehicle.pos, p);
}
#[test] fn sling_missions_spawn_the_cargo_on_pad_a_and_attach_it_after_the_hover() {
    let mut w = World::for_mission(air(MissionKind::Sling, AircraftKind::Heli, vec![v3(0., 0., 60.), v3(0., 0., 400.)]));
    assert!(w.sling.as_ref().map_or(false, |s| !s.attached));
    w.aircraft.as_mut().unwrap().pos = v3(0., w.terrain.height(0., 60.) + 2.5, 60.);
    for _ in 0..140 { w.step_with(&Input::default(), &FlightInput { yaw: 0.2, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.sling.as_ref().unwrap().attached);
}
#[test] fn air_markers_and_run_info() {
    let mut w = World::for_mission(air(MissionKind::Rings, AircraftKind::Heli, vec![v3(0., 30., 100.), v3(0., 30., 200.)]));
    let k = |w: &World| w.markers().iter().map(|m| m.kind).collect::<Vec<_>>();
    assert_eq!(k(&w), vec![MarkerKind::AirNext, MarkerKind::AirRing]);
    assert!((w.markers()[0].normal.len() - 1.0).abs() < 1e-3);
    let ri = w.run_info().unwrap();
    assert_eq!((ri.total, ri.next, ri.phase), (2, 0, crate::game::run::Phase::Ready));
    let w2 = World::for_mission(air(MissionKind::Sling, AircraftKind::Heli, vec![v3(0., 0., 60.), v3(0., 0., 400.)]));
    assert_eq!(w2.markers().iter().filter(|m| m.kind == MarkerKind::Pad).count(), 2);
    let w3 = World::for_mission(air(MissionKind::Recon, AircraftKind::Drone, vec![v3(0., 40., 100.), v3(100., 50., 100.)]));
    assert_eq!(w3.markers().iter().filter(|m| m.kind == MarkerKind::Zone).count(), 2);
    w.abandon_mission();
    assert!(w.markers().is_empty());
}
#[test] fn the_chase_camera_stays_above_the_terrain_for_an_aircraft_on_the_ground() {
    let mut w = World::for_mission(air(MissionKind::Rings, AircraftKind::Drone, vec![v3(0., 30., 300.)]));
    for _ in 0..600 { w.step_with(&Input::default(), &FlightInput { lift: -0.2, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.cam.pos.y >= w.terrain.height(w.cam.pos.x, w.cam.pos.z) + 0.89);
}
use crate::game::city::{Building, City, CitySite};
fn walled_world() -> World {
    let mut w = World::new(1);
    let g = w.terrain.height(0., 40.);
    w.cities = vec![City::from_buildings(CitySite { centre: v3(0., g, 40.), half: 264.0 }, vec![Building { min: v3(-20., g, 30.), max: v3(20., g + 30.0, 50.), color: 0 }])];
    w
}
#[test] fn worlds_have_cities_matching_the_terrain_sites() {
    let w = World::new(7);
    assert_eq!(w.cities.len(), w.terrain.sites().len());
    assert!(w.cities.iter().all(|c| c.buildings.len() > 60));
    assert!(World::new(7).cities[0].buildings.len() == w.cities[0].buildings.len());
}
#[test] fn the_car_cannot_drive_through_a_building() {
    let mut w = walled_world();
    for _ in 0..(8.0 / (1.0 / 60.0)) as usize { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.vehicle.pos.z <= 30.0 - 0.9, "the car went through the wall: z {}", w.vehicle.pos.z);
    assert!(w.vehicle.pos.is_finite());
}
#[test] fn an_aircraft_that_hits_a_wall_crashes_but_one_above_the_roof_does_not() {
    use crate::game::flight::{AircraftKind, FlightInput, Status};
    let m = air(MissionKind::Rings, AircraftKind::Heli, vec![v3(0., 30., 100.)]);
    let mut w2 = World::for_mission(m.clone());
    w2.cities = walled_world().cities;
    let g = w2.terrain.height(0., 0.);
    w2.aircraft.as_mut().unwrap().pos = v3(0., g + 10.0, 0.0);
    for _ in 0..(6.0 / (1.0 / 60.0)) as usize { w2.step_with(&Input::default(), &FlightInput { pitch: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert_eq!(w2.aircraft.as_ref().unwrap().status, Status::Crashed, "flying into the wall");
    let mut w3 = World::for_mission(m);
    w3.cities = walled_world().cities;
    let top = w3.cities[0].buildings[0].max.y;
    w3.aircraft.as_mut().unwrap().pos = v3(0., top + 12.0, 0.0);
    for _ in 0..(4.0 / (1.0 / 60.0)) as usize { w3.step_with(&Input::default(), &FlightInput { pitch: 0.4, ..Default::default() }, 1.0 / 60.0); }
    assert_ne!(w3.aircraft.as_ref().unwrap().status, Status::Crashed);
}
#[test] fn cargo_is_pushed_out_of_walls() {
    use crate::game::flight::{AircraftKind, FlightInput};
    let mut w = World::for_mission(air(MissionKind::Sling, AircraftKind::Heli, vec![v3(0., 0., 200.), v3(0., 0., 400.)]));
    w.cities = walled_world().cities;
    let base = w.cities[0].buildings[0].min.y;
    w.sling.as_mut().unwrap().pos = v3(0.0, base + 10.0, 40.0);
    w.step_with(&Input::default(), &FlightInput::default(), 1.0 / 60.0);
    let p = w.sling.as_ref().unwrap().pos;
    assert!(!(p.x.abs() < 20.0 && p.z > 30.0 && p.z < 50.0 && p.y < base + 30.0), "cargo still inside the building at {p:?}");
}
#[test] fn place_near_city_puts_the_player_outside_the_first_city_facing_it() {
    let mut w = World::new(7);
    assert!(w.place_near_city());
    let s = w.cities[0].site;
    let (dx, dz) = (s.centre.x - w.vehicle.pos.x, s.centre.z - w.vehicle.pos.z);
    let f = w.vehicle.forward();
    assert!((f.x * dx + f.z * dz) / dx.hypot(dz) > 0.99, "facing the centre");
    assert!(!w.cities[0].inside(w.vehicle.pos.x, w.vehicle.pos.z));
    assert!(dx.hypot(dz) < s.half * 1.5 + 100.0);
    let mut flat = World::new(1); flat.terrain = crate::game::terrain::Terrain::flat(); flat.cities.clear();
    assert!(!flat.place_near_city());
}
#[test] fn view_carries_the_cities() { let w = World::new(7); assert_eq!(w.view(1).cities.len(), w.cities.len()); }
#[test] fn glancing_wall_hits_at_speed_do_not_roll_the_car() {
    for deg in [20.0f32, 28.0, 35.0] {
        let mut w = World::new(1);
        let g = w.terrain.height(0., 40.);
        w.cities = vec![City::from_buildings(CitySite { centre: v3(0., g, 20.), half: 264.0 }, vec![Building { min: v3(-200., g, 10.), max: v3(200., g + 30.0, 30.), color: 0 }])];
        let yaw = (deg).to_radians();                          // yaw 0 = +z (towards the wall); 20-35 deg off perpendicular
        // the whole run stays within 25 m of the origin, where no props are placed
        w.vehicle = crate::game::vehicle::Vehicle::new(crate::game::vehicle::VehicleSpec::offroader(), v3(0.0, g + 1.1, -12.0), -yaw);
        w.vehicle.vel = w.vehicle.forward() * 30.0;
        for _ in 0..240 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
        assert!(w.vehicle.up().y > 0.5, "the car rolled over at {deg} deg: up.y {}", w.vehicle.up().y);
        assert!(w.vehicle.pos.z <= 10.0 - 0.9 || w.vehicle.pos.z > 30.0 + 0.9 && false, "z {}", w.vehicle.pos.z);
    }
}
