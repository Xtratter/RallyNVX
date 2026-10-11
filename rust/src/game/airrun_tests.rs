use super::*;
use crate::game::{flight::{Aircraft, AircraftKind, Status}, math::v3, medals::Medal, missions::{path_length, Mission, MissionKind}, run::{Failure, Outcome, Phase}, sling::Sling};
const DT: f32 = 1.0 / 60.0;
fn mission(kind: MissionKind, ac: AircraftKind, pts: Vec<V3>, limit: f32) -> Mission {
    let goal = *pts.last().unwrap();
    Mission { kind, seed: 1, difficulty: 1, start: v3(0., 0., 0.), goal, length_m: path_length(v3(0., 0., 0.), &pts), checkpoints: pts, time_limit: limit, reward_base: 100.0, aircraft: Some(ac), urban: false, start_yaw: None, normals: vec![] }
}
fn heli_at(p: V3) -> Aircraft { Aircraft::new(AircraftKind::Heli, p, 0.0) }
fn rings() -> AirRun { AirRun::new(mission(MissionKind::Rings, AircraftKind::Heli, vec![v3(0., 30., 100.), v3(0., 30., 200.), v3(0., 30., 300.)], 100.0)) }
fn pass(r: &mut AirRun, a: &mut Aircraft, from: V3, to: V3) { r.prev = None; a.pos = from; r.update(a, None, None, true, DT); a.pos = to; r.update(a, None, None, true, DT); }

#[test] fn clock_starts_with_the_first_input() {
    let mut r = rings(); let a = heli_at(v3(0., 3., 0.));
    for _ in 0..120 { r.update(&a, None, None, false, DT); }
    assert_eq!((r.phase(), r.time()), (Phase::Ready, 0.0));
    r.update(&a, None, None, true, DT);
    assert_eq!(r.phase(), Phase::Running);
}
#[test] fn rings_are_taken_in_order_by_crossing_the_plane_inside_the_radius() {
    let mut r = rings(); let mut a = heli_at(v3(0., 30., 0.));
    r.update(&a, None, None, true, DT);
    pass(&mut r, &mut a, v3(0., 30., 199.0), v3(0., 30., 201.0));
    assert_eq!(r.next_index(), 0);
    pass(&mut r, &mut a, v3(0., 30., 99.0), v3(0., 30., 101.0));
    assert_eq!(r.next_index(), 1);
    pass(&mut r, &mut a, v3(0., 30., 199.0), v3(0., 30., 201.0));
    pass(&mut r, &mut a, v3(0., 30., 299.0), v3(0., 30., 301.0));
    assert!(matches!(r.outcome(), Some(Outcome::Passed { .. })));
}
#[test] fn backwards_crossing_outside_radius_and_grazing_do_not_count() {
    let mut r = rings(); let mut a = heli_at(v3(0., 30., 0.));
    r.update(&a, None, None, true, DT);
    pass(&mut r, &mut a, v3(0., 30., 101.0), v3(0., 30., 99.0));
    pass(&mut r, &mut a, v3(9.0, 30., 99.0), v3(9.0, 30., 101.0));
    assert_eq!(r.next_index(), 0);
    pass(&mut r, &mut a, v3(7.5, 30., 99.0), v3(7.5, 30., 101.0));
    assert_eq!(r.next_index(), 1);
}
#[test] fn one_huge_step_across_the_plane_counts_once_and_tiny_steps_do_not_double_count() {
    let mut r = rings(); let mut a = heli_at(v3(0., 30., 0.));
    r.update(&a, None, None, true, DT);
    pass(&mut r, &mut a, v3(0., 30., 50.0), v3(0., 30., 150.0));
    assert_eq!(r.next_index(), 1);
    for k in 0..200 { a.pos = v3(0., 30., 150.0 + k as f32 * 0.5); r.update(&a, None, None, true, DT); }
    assert_eq!(r.next_index(), 2, "ring 2 (z=200) is crossed exactly once by the creeping loop");
}
#[test] fn the_drone_has_a_smaller_ring_radius() {
    let mut r = AirRun::new(mission(MissionKind::Rings, AircraftKind::Drone, vec![v3(0., 30., 100.), v3(0., 30., 200.)], 100.0));
    let mut a = Aircraft::new(AircraftKind::Drone, v3(0., 30., 0.), 0.0);
    r.update(&a, None, None, true, DT);
    pass(&mut r, &mut a, v3(7.0, 30., 99.0), v3(7.0, 30., 101.0));
    assert_eq!(r.next_index(), 0);
    pass(&mut r, &mut a, v3(5.0, 30., 99.0), v3(5.0, 30., 101.0));
    assert_eq!(r.next_index(), 1);
}
#[test] fn crash_and_time_up_fail() {
    let mut r = rings(); let mut a = heli_at(v3(0., 30., 0.));
    r.update(&a, None, None, true, DT);
    a.status = Status::Crashed; r.update(&a, None, None, true, DT);
    assert_eq!(r.outcome(), Some(Outcome::Failed(Failure::Crashed)));
    let mut r = rings(); let a = heli_at(v3(0., 30., 0.));
    for _ in 0..(101.0 / DT) as usize { r.update(&a, None, None, true, DT); }
    assert_eq!(r.outcome(), Some(Outcome::Failed(Failure::TimeUp)));
}
#[test] fn rings_pass_grades_by_time_only() {
    let mut r = rings(); let mut a = heli_at(v3(0., 30., 0.));
    r.update(&a, None, None, true, DT);
    for z in [100.0, 200.0, 300.0] { pass(&mut r, &mut a, v3(0., 30., z - 1.0), v3(0., 30., z + 1.0)); }
    assert!(matches!(r.outcome(), Some(Outcome::Passed { medal: Medal::Gold, .. })));
}
fn recon() -> AirRun { AirRun::new(mission(MissionKind::Recon, AircraftKind::Drone, vec![v3(0., 40., 100.), v3(200., 60., 100.), v3(400., 30., 100.)], 200.0)) }
#[test] fn recon_needs_two_seconds_inside_a_zone_and_any_order() {
    let mut r = recon(); let mut a = Aircraft::new(AircraftKind::Drone, v3(0., 40., 0.), 0.0);
    r.update(&a, None, None, true, DT);
    a.pos = v3(200., 60., 100.);
    for _ in 0..100 { r.update(&a, None, None, true, DT); }
    assert_eq!(r.next_index(), 0);
    for _ in 0..30 { r.update(&a, None, None, true, DT); }
    assert_eq!(r.next_index(), 1);
    a.pos = v3(0., 40., 100.); for _ in 0..130 { r.update(&a, None, None, true, DT); }
    a.pos = v3(400., 30., 100.); for _ in 0..130 { r.update(&a, None, None, true, DT); }
    assert!(matches!(r.outcome(), Some(Outcome::Passed { .. })));
}
#[test] fn leaving_a_zone_resets_the_dwell() {
    let mut r = recon(); let mut a = Aircraft::new(AircraftKind::Drone, v3(0., 40., 0.), 0.0);
    r.update(&a, None, None, true, DT);
    a.pos = v3(200., 60., 100.); for _ in 0..90 { r.update(&a, None, None, true, DT); }
    a.pos = v3(200., 60., 400.); r.update(&a, None, None, true, DT);
    a.pos = v3(200., 60., 100.); for _ in 0..90 { r.update(&a, None, None, true, DT); }
    assert_eq!(r.next_index(), 0);
}
fn sling_run() -> AirRun { AirRun::new(mission(MissionKind::Sling, AircraftKind::Heli, vec![v3(100., 0., 0.), v3(500., 0., 0.)], 300.0)) }
#[test] fn sling_pick_up_needs_a_low_hover_for_1_5_seconds_then_attaches_once() {
    let mut r = sling_run(); let mut a = heli_at(v3(100., 3., 0.));
    let mut s = Sling::on_ground(v3(100., 0.3, 0.));
    r.update(&a, Some(&s), None, true, DT);
    let mut attach = 0;
    for _ in 0..80 { if r.update(&a, Some(&s), None, true, DT) == Some(AirAction::AttachCargo) { attach += 1; s.attach(); } }
    assert_eq!(attach, 0, "1.33 s is not enough");
    for _ in 0..20 { if r.update(&a, Some(&s), None, true, DT) == Some(AirAction::AttachCargo) { attach += 1; s.attach(); } }
    assert_eq!(attach, 1);
    for _ in 0..60 { if r.update(&a, Some(&s), None, true, DT) == Some(AirAction::AttachCargo) { attach += 1; } }
    assert_eq!(attach, 1);
    assert_eq!(r.next_index(), 1);
    a.pos = v3(100., 6.0, 0.);
    r.update(&a, Some(&s), None, true, DT);
    assert_eq!(r.next_index(), 1);
}
#[test] fn hovering_too_high_or_too_far_does_not_pick_up() {
    let mut r = sling_run(); let mut a = heli_at(v3(100., 5.0, 0.));
    let s = Sling::on_ground(v3(100., 0.3, 0.));
    r.update(&a, Some(&s), None, true, DT);
    for _ in 0..200 { assert_eq!(r.update(&a, Some(&s), None, true, DT), None); }
    a.pos = v3(110., 3.0, 0.);
    for _ in 0..200 { assert_eq!(r.update(&a, Some(&s), None, true, DT), None); }
}
fn attached_run() -> (AirRun, Aircraft, Sling) {
    let mut r = sling_run(); let a = heli_at(v3(100., 3., 0.));
    let mut s = Sling::on_ground(v3(100., 0.3, 0.));
    r.update(&a, Some(&s), None, true, DT);
    for _ in 0..100 { if r.update(&a, Some(&s), None, true, DT).is_some() { s.attach(); } }
    (r, a, s)
}
#[test] fn delivery_needs_the_cargo_in_the_circle_low_and_slow() {
    let (mut r, mut a, mut s) = attached_run();
    a.pos = v3(500., 12., 0.); s.pos = v3(500., 4.0, 0.); s.vel = V3::ZERO;
    r.update(&a, Some(&s), None, true, DT);
    assert_eq!(r.outcome(), None, "cargo still 4 m up");
    s.pos = v3(500., 0.4, 0.); s.vel = v3(0., 0., 3.0);
    r.update(&a, Some(&s), None, true, DT);
    assert_eq!(r.outcome(), None, "too fast");
    s.vel = v3(0., 0., 1.0); s.pos = v3(508., 0.4, 0.);
    r.update(&a, Some(&s), None, true, DT);
    assert_eq!(r.outcome(), None, "outside the 6 m circle");
    s.pos = v3(504., 0.4, 0.);
    r.update(&a, Some(&s), None, true, DT);
    assert!(matches!(r.outcome(), Some(Outcome::Passed { .. })));
}
#[test] fn impact_damage_swing_damage_and_destruction() {
    let (mut r, a, s) = attached_run();
    r.update(&a, Some(&s), Some(8.0), true, DT);
    assert!((r.integrity() - 95.0).abs() < 1e-3, "{}", r.integrity());
    r.update(&a, Some(&s), Some(5.9), true, DT);
    assert!((r.integrity() - 95.0).abs() < 1e-3);
    let mut fast = s; fast.vel = v3(0., 0., 20.0);
    for _ in 0..60 { r.update(&a, Some(&fast), None, true, DT); }
    assert!((r.integrity() - 91.0).abs() < 0.2, "swing 4/s for 1 s, got {}", r.integrity());
    r.update(&a, Some(&s), Some(60.0), true, DT);
    assert_eq!(r.outcome(), Some(Outcome::Failed(Failure::CargoDestroyed)));
}
#[test] fn no_damage_before_the_pick_up_and_nothing_changes_after_done() {
    let mut r = sling_run(); let a = heli_at(v3(0., 30., 0.));
    let s = Sling::on_ground(v3(100., 0.3, 0.));
    r.update(&a, Some(&s), None, true, DT);
    r.update(&a, Some(&s), Some(50.0), true, DT);
    assert_eq!(r.integrity(), 100.0);
    r.abandon();
    let (t, o) = (r.time(), r.outcome());
    r.update(&a, Some(&s), Some(50.0), true, DT);
    assert_eq!((r.time(), r.outcome()), (t, o));
}
#[test] fn targets_for_the_hud_arrow() {
    let r = rings(); assert_eq!(r.target(v3(0., 0., 0.)), Some(v3(0., 30., 100.)));
    let rc = recon(); assert_eq!(rc.target(v3(390., 30., 100.)), Some(v3(400., 30., 100.)));
    let s = sling_run(); assert_eq!(s.target(V3::ZERO), Some(v3(100., 0., 0.)));
    let (r, ..) = attached_run(); assert_eq!(r.target(V3::ZERO), Some(v3(500., 0., 0.)));
    assert_eq!((rings().total(), recon().total(), sling_run().total()), (3, 3, 2));
}
