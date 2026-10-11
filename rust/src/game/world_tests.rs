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
    Mission { kind, seed: 42, difficulty: 1, start: v3(0., 0., 0.), goal, checkpoints: pts, length_m: 300.0, time_limit: 100.0, reward_base: 100.0 }
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
