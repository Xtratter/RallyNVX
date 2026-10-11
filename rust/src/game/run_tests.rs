use super::*;
use crate::game::{math::{v3, Q}, missions::MissionKind, vehicle::{Vehicle, VehicleSpec}};

const DT: f32 = 1.0 / 60.0;
fn mission(kind: MissionKind, pts: Vec<V3>) -> Mission {
    let goal = *pts.last().unwrap();
    let len = crate::game::missions::path_length(v3(0., 0., 0.), &pts);
    Mission { kind, seed: 1, difficulty: 1, start: v3(0., 0., 0.), goal, checkpoints: pts, length_m: len, time_limit: 100.0, reward_base: 100.0, aircraft: None }
}
fn car(x: f32, z: f32) -> Vehicle { let mut v = Vehicle::new(VehicleSpec::offroader(), v3(x, 0.9, z), 0.0); v.contacts = 4; v }
fn gas() -> Input { Input { throttle: 1.0, ..Default::default() } }
fn cargo() -> MissionRun { MissionRun::new(mission(MissionKind::Cargo, vec![v3(0., 0., 300.)])) }

#[test] fn timer_starts_on_first_throttle() {
    let mut r = cargo(); let v = car(0., 0.);
    for _ in 0..120 { r.update(&v, &Input::default(), false, DT); }
    assert_eq!((r.phase(), r.time()), (Phase::Ready, 0.0));
    r.update(&v, &gas(), false, DT);
    assert_eq!(r.phase(), Phase::Running);
    r.update(&v, &gas(), false, DT);
    assert!((r.time() - DT).abs() < 1e-5 || (r.time() - 2.0 * DT).abs() < 1e-5);
}
#[test] fn time_up_fails() {
    let mut r = cargo(); let v = car(0., 0.);
    for _ in 0..(101.0 / DT) as usize { r.update(&v, &gas(), false, DT); }
    assert_eq!(r.outcome(), Some(Outcome::Failed(Failure::TimeUp)));
}
#[test] fn reaching_the_goal_passes_with_a_medal() {
    let mut r = cargo(); let mut v = car(0., 0.);
    for _ in 0..600 { r.update(&v, &gas(), false, DT); }
    v.pos = v3(0., 0.9, 300.0 - 3.0);
    r.update(&v, &gas(), false, DT);
    match r.outcome() { Some(Outcome::Passed { medal, integrity, .. }) => { assert_eq!(medal, Medal::Gold); assert_eq!(integrity, 100.0); } o => panic!("{o:?}") }
}
#[test] fn goal_radius_is_6_metres() {
    let mut r = cargo(); let mut v = car(0., 0.);
    r.update(&v, &gas(), false, DT);
    v.pos = v3(0., 0.9, 300.0 - 6.5); r.update(&v, &gas(), false, DT);
    assert_eq!(r.outcome(), None);
    v.pos = v3(0., 0.9, 300.0 - 5.5); r.update(&v, &gas(), false, DT);
    assert!(matches!(r.outcome(), Some(Outcome::Passed { .. })));
}
#[test] fn checkpoints_must_be_taken_in_order() {
    let mut r = MissionRun::new(mission(MissionKind::Race, vec![v3(0., 0., 100.), v3(0., 0., 200.), v3(0., 0., 300.)]));
    let mut v = car(0., 0.);
    r.update(&v, &gas(), false, DT);
    v.pos = v3(0., 0.9, 200.0); r.update(&v, &gas(), false, DT);
    assert_eq!((r.next_index(), r.outcome()), (0, None));
    v.pos = v3(0., 0.9, 100.0); r.update(&v, &gas(), false, DT);
    assert_eq!(r.next_index(), 1);
    v.pos = v3(0., 0.9, 200.0); r.update(&v, &gas(), false, DT);
    v.pos = v3(0., 0.9, 300.0); r.update(&v, &gas(), false, DT);
    assert!(matches!(r.outcome(), Some(Outcome::Passed { .. })));
}
#[test] fn impact_damage_uses_horizontal_speed_change_over_6() {
    let mut r = cargo(); let mut v = car(0., 0.);
    v.vel = v3(0., 0., 20.); r.update(&v, &gas(), false, DT);
    v.vel = v3(0., 0., 20.0 - 5.9); r.update(&v, &gas(), false, DT);
    assert_eq!(r.integrity(), 100.0);
    v.vel = v3(0., 0., 20.0 - 5.9 - 8.0); r.update(&v, &gas(), false, DT);
    assert!((r.integrity() - (100.0 - 2.0 * 2.5)).abs() < 1e-3, "{}", r.integrity());
}
#[test] fn vertical_speed_change_alone_is_not_an_impact() {
    let mut r = cargo(); let mut v = car(0., 0.);
    v.vel = v3(0., -3.0, 10.0); r.update(&v, &gas(), false, DT);
    v.vel = v3(0., 0.0, 10.0); r.update(&v, &gas(), false, DT);
    assert_eq!(r.integrity(), 100.0);
}
#[test] fn hard_landing_after_air_time_hurts() {
    let mut r = cargo(); let mut v = car(0., 0.);
    v.contacts = 0; v.vel = v3(0., -10.0, 10.0);
    for _ in 0..30 { r.update(&v, &gas(), false, DT); }
    v.contacts = 4; v.vel = v3(0., 0.0, 10.0);
    r.update(&v, &gas(), false, DT);
    assert!((r.integrity() - (100.0 - 3.0 * 3.0)).abs() < 1e-3, "{}", r.integrity());
}
#[test] fn short_hop_does_not_hurt() {
    let mut r = cargo(); let mut v = car(0., 0.);
    v.contacts = 0; v.vel = v3(0., -10.0, 10.0);
    for _ in 0..10 { r.update(&v, &gas(), false, DT); }
    v.contacts = 4; v.vel = v3(0., 0.0, 10.0); r.update(&v, &gas(), false, DT);
    assert_eq!(r.integrity(), 100.0);
}
#[test] fn flipped_drains_8_per_second_and_destroys_the_cargo() {
    let mut r = cargo(); let mut v = car(0., 0.);
    v.rot = v.rot.mul(Q::axis_angle(v.forward(), std::f32::consts::PI));
    r.update(&v, &gas(), false, DT);
    for _ in 0..59 { r.update(&v, &gas(), false, DT); }
    assert!((r.integrity() - 92.0).abs() < 0.2, "{}", r.integrity());
    for _ in 0..(12.0 / DT) as usize { r.update(&v, &gas(), false, DT); }
    assert_eq!(r.outcome(), Some(Outcome::Failed(Failure::CargoDestroyed)));
}
#[test] fn reset_costs_5_and_spam_destroys_the_cargo() {
    let mut r = cargo(); let v = car(0., 0.);
    r.update(&v, &gas(), true, DT);
    assert!((r.integrity() - 95.0).abs() < 1e-4);
    for _ in 0..25 { r.update(&v, &gas(), true, DT); }
    assert_eq!(r.outcome(), Some(Outcome::Failed(Failure::CargoDestroyed)));
}
#[test] fn race_takes_no_cargo_damage() {
    let mut r = MissionRun::new(mission(MissionKind::Race, vec![v3(0., 0., 300.)]));
    let mut v = car(0., 0.);
    v.vel = v3(0., 0., 30.); r.update(&v, &gas(), false, DT);
    v.vel = v3(0., 0., 0.); r.update(&v, &gas(), true, DT);
    assert_eq!((r.integrity(), r.outcome()), (100.0, None));
}
#[test] fn nothing_changes_after_the_run_is_done() {
    let mut r = cargo(); let mut v = car(0., 0.);
    r.update(&v, &gas(), false, DT);
    v.pos = v3(0., 0.9, 299.0); r.update(&v, &gas(), false, DT);
    let done = r.outcome(); let (t, i) = (r.time(), r.integrity());
    v.vel = v3(0., 0., 50.); r.update(&v, &gas(), true, DT); v.vel = V3::ZERO; r.update(&v, &gas(), true, DT);
    assert_eq!((r.outcome(), r.time(), r.integrity()), (done, t, i));
}
#[test] fn goal_wins_when_reached_as_the_clock_runs_out() {
    let mut r = cargo(); let mut v = car(0., 0.);
    for _ in 0..(99.9 / DT) as usize { r.update(&v, &gas(), false, DT); }
    v.pos = v3(0., 0.9, 299.0);
    for _ in 0..30 { r.update(&v, &gas(), false, DT); if r.outcome().is_some() { break; } }
    assert!(matches!(r.outcome(), Some(Outcome::Passed { .. })));
}
#[test] fn abandon_fails_once() {
    let mut r = cargo(); r.abandon();
    assert_eq!(r.outcome(), Some(Outcome::Failed(Failure::Abandoned)));
    r.abandon(); assert_eq!(r.phase(), Phase::Done);
}
#[test] fn bearing_is_zero_ahead_and_positive_on_the_right() {
    let b = |to: V3| bearing(0.0, 1.0, v3(0., 0., 0.), to);
    assert!(b(v3(0., 0., 10.)).abs() < 1e-5);
    assert!((b(v3(-10., 0., 0.)) - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    assert!((b(v3(10., 0., 0.)) + std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    assert!((b(v3(0., 0., -10.)).abs() - std::f32::consts::PI).abs() < 1e-4);
}
#[test] fn reset_costs_only_5_even_at_speed_and_in_the_air() {
    let mut r = cargo(); let mut v = car(0., 0.);
    v.vel = v3(0., 0., 30.); r.update(&v, &gas(), false, DT);
    v.vel = V3::ZERO; r.update(&v, &gas(), true, DT);              // reset zeroes the velocity at 30 m/s
    assert!((r.integrity() - 95.0).abs() < 1e-3, "{}", r.integrity());
    // reset while airborne after a fall: the landing rule must not fire on the zeroed velocity
    let mut r = cargo(); let mut v = car(0., 0.);
    v.contacts = 0; v.vel = v3(0., -15.0, 10.0);
    for _ in 0..40 { r.update(&v, &gas(), false, DT); }
    let before = r.integrity();
    v.contacts = 4; v.vel = V3::ZERO; r.update(&v, &gas(), true, DT);
    assert!((r.integrity() - (before - 5.0)).abs() < 1e-3, "{} vs {}", r.integrity(), before);
}
