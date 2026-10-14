use super::*;
use crate::game::lines::{Line, LineKind};
use crate::game::math::v3;

const DT: f32 = 0.05;
fn line(z: f32, kind: LineKind) -> Line { Line { pos: v3(0.0, 0.0, z), dir: v3(0.0, 0.0, 1.0), width: 8.0, kind } }
fn run() -> Run { Run::new(vec![line(100.0, LineKind::Check), line(200.0, LineKind::Check), line(300.0, LineKind::Finish)]) }
/// Drives straight along z (x offset `x`) from z0 to z1 at 20 m/s.
fn drive(r: &mut Run, x: f32, z0: f32, z1: f32) { let mut z = z0; while z < z1 { r.update(v3(x, 0.0, z), DT, None); z += 20.0 * DT; } }
fn go(r: &mut Run) { for _ in 0..80 { r.update(v3(0.0, 0.0, 0.0), DT, None); } assert_eq!(r.phase, Phase::Running); }

#[test] fn the_countdown_counts_3_2_1_then_go_and_the_clock_starts_at_go() {
    let mut r = run();
    assert_eq!((r.phase, r.countdown_digit()), (Phase::Countdown, 3));
    let mut digits = vec![r.countdown_digit()];
    for _ in 0..200 { r.update(v3(0.0, 0.0, 0.0), DT, None); if *digits.last().unwrap() != r.countdown_digit() { digits.push(r.countdown_digit()); } if r.phase == Phase::Running { break; } }
    assert_eq!(digits, vec![3, 2, 1, 0]);
    assert!(r.time < 0.1 && r.drain_events().contains(&Event::Go));
    for _ in 0..20 { r.update(v3(0.0, 0.0, 0.0), DT, None); }
    assert!((r.time - 1.0).abs() < 0.12, "{}", r.time);
}
#[test] fn lines_in_order_give_splits_and_the_finish_ends_the_run() {
    let mut r = run(); go(&mut r); r.drain_events();
    drive(&mut r, 0.0, -5.0, 310.0);
    assert_eq!(r.phase, Phase::Finished);
    assert_eq!(r.splits.len(), 2);
    assert!(r.splits[0] < r.splits[1] && r.penalty == 0.0);
    let ev = r.drain_events();
    assert!(matches!(ev[0], Event::Split { index: 0, .. }) && matches!(ev[1], Event::Split { index: 1, .. }) && matches!(ev[2], Event::Finish { .. }), "{ev:?}");
    let t = r.time; drive(&mut r, 0.0, 310.0, 330.0); assert_eq!(r.time, t, "the clock stops at the finish");
}
#[test] fn the_wrong_side_and_the_wrong_place_do_not_count() {
    let mut r = run(); go(&mut r);
    r.teleported();
    r.update(v3(0.0, 0.0, 105.0), DT, None); r.update(v3(0.0, 0.0, 95.0), DT, None);   // backwards over line 0
    assert_eq!(r.next_line(), 0);
    r.update(v3(0.0, 0.0, 95.0), DT, None); r.update(v3(30.0, 0.0, 105.0), DT, None);   // far outside the width
    assert_eq!(r.next_line(), 0);
}
#[test] fn a_skipped_line_costs_thirty_seconds_each() {
    let mut r = run(); go(&mut r);
    r.teleported();
    drive(&mut r, 0.0, 150.0, 210.0);   // starts past line 0: crosses line 1 first
    assert_eq!(r.next_line(), 2);
    assert_eq!(r.penalty, MISSED_PENALTY);
    assert!(r.drain_events().iter().any(|e| matches!(e, Event::Missed { lines: 1 })));
    let mut r = run(); go(&mut r);
    r.teleported();
    drive(&mut r, 0.0, 250.0, 310.0);   // crosses only the finish: two lines skipped
    assert_eq!((r.phase, r.penalty), (Phase::Finished, 2.0 * MISSED_PENALTY));
}
#[test] fn a_reset_costs_ten_seconds_and_a_teleport_is_not_a_crossing() {
    let mut r = run(); go(&mut r);
    r.update(v3(0.0, 0.0, 90.0), DT, None);
    r.reset_penalty();
    r.update(v3(0.0, 0.0, 110.0), DT, None);   // the car was put beyond line 0: no crossing from the teleport
    assert_eq!((r.penalty, r.next_line()), (RESET_PENALTY, 0));
    // a reset during the countdown costs nothing
    let mut r = run(); r.reset_penalty(); assert_eq!(r.penalty, 0.0);
}
#[test] fn leaving_the_road_for_three_seconds_costs_five_once_and_rearms() {
    let mut r = run(); go(&mut r);
    let off = |r: &mut Run, secs: f32, e: Option<f32>| { for _ in 0..(secs / DT) as usize { r.update(v3(0.0, 0.0, 10.0), DT, e); } };
    off(&mut r, 2.0, Some(2.0)); assert_eq!(r.penalty, 0.0);
    off(&mut r, 1.5, Some(2.0)); assert_eq!(r.penalty, OFFROAD_PENALTY);
    off(&mut r, 6.0, Some(2.0)); assert_eq!(r.penalty, OFFROAD_PENALTY, "once per excursion");
    off(&mut r, 3.0, Some(-1.0)); off(&mut r, 3.5, Some(2.0));
    assert_eq!(r.penalty, 2.0 * OFFROAD_PENALTY, "armed again after the car was back on the road");
    let mut r = run(); go(&mut r); off(&mut r, 10.0, None); assert_eq!(r.penalty, 0.0, "the city stage has no off-road rule");
}
