use super::*;
use crate::game::{math::v3, props::{Prop, PropKind}, terrain::Terrain, world::WORLD_LIMIT};
const DT: f32 = 1.0 / 60.0;
fn fly(a: &mut Aircraft, t: &Terrain, i: FlightInput, secs: f32) { for _ in 0..(secs / DT) as usize { a.step(t, &[], &i, DT); } }
fn heli(y: f32) -> (Aircraft, Terrain) { (Aircraft::new(AircraftKind::Heli, v3(0., y, 0.), 0.0), Terrain::flat()) }

#[test] fn neutral_sticks_hold_position_and_altitude() {
    let (mut a, t) = heli(50.0);
    fly(&mut a, &t, FlightInput::default(), 10.0);
    assert!((a.pos.y - 50.0).abs() < 0.01 && a.vel.len() < 0.01 && a.status == Status::Flying);
}
#[test] fn pitch_input_accelerates_forward_within_the_speed_cap() {
    let (mut a, t) = heli(50.0);
    fly(&mut a, &t, FlightInput { pitch: 1.0, ..Default::default() }, 20.0);
    let v = a.vel.x.hypot(a.vel.z);
    assert!(v > 25.0 && v <= 35.0 + 0.01, "speed {v}");
    assert!(a.forward().x * a.vel.x + a.forward().z * a.vel.z > 0.99 * v, "moves along the heading");
    assert!(a.pitch > 0.3 && a.pitch < 0.45, "25 deg nose down, got {}", a.pitch);
}
#[test] fn roll_input_moves_to_the_right_hand_side() {
    let (mut a, t) = heli(50.0);
    fly(&mut a, &t, FlightInput { roll: 1.0, ..Default::default() }, 3.0);
    assert!(a.vel.dot(a.right()) > 3.0, "{:?}", a.vel);
}
#[test] fn positive_yaw_turns_right() {
    let (mut a, t) = heli(50.0);
    fly(&mut a, &t, FlightInput { yaw: 1.0, ..Default::default() }, 1.0);
    assert!(a.forward().x < -0.3, "forward {:?}", a.forward());
    assert!(a.yaw_rate.abs() <= 1.6 + 1e-3);
}
#[test] fn vertical_speed_follows_lift_and_is_capped() {
    let (mut a, t) = heli(50.0);
    fly(&mut a, &t, FlightInput { lift: 1.0, ..Default::default() }, 5.0);
    assert!((a.vel.y - 6.0).abs() < 0.05, "{}", a.vel.y);
    let mut d = Aircraft::new(AircraftKind::Drone, v3(0., 50., 0.), 0.0);
    fly(&mut d, &t, FlightInput { lift: 1.0, ..Default::default() }, 3.0);
    assert!((d.vel.y - 10.0).abs() < 0.05);
}
#[test] fn the_ceiling_stops_the_climb() {
    let (mut a, t) = heli(CEILING - 1.0);
    fly(&mut a, &t, FlightInput { lift: 1.0, ..Default::default() }, 10.0);
    assert!(a.pos.y <= CEILING + 0.5, "{}", a.pos.y);
}
#[test] fn gentle_descent_lands_softly() {
    let (mut a, t) = heli(10.0);
    fly(&mut a, &t, FlightInput { lift: -0.4, ..Default::default() }, 8.0);
    assert_eq!(a.status, Status::Landed, "{:?}", a.status);
    assert!(a.pos.y >= 0.29 && a.pos.y < 0.6);
}
#[test] fn fast_descent_crashes() {
    let (mut a, t) = heli(10.0);
    fly(&mut a, &t, FlightInput { lift: -1.0, ..Default::default() }, 8.0);
    assert_eq!(a.status, Status::Crashed);
}
#[test] fn steep_tilt_at_contact_crashes_even_when_slow() {
    let mut a = Aircraft::new(AircraftKind::Drone, v3(0., 0.2, 0.), 0.0);
    a.pitch = 0.7;
    a.step(&Terrain::flat(), &[], &FlightInput { pitch: 1.0, ..Default::default() }, DT);
    assert_eq!(a.status, Status::Crashed);
}
#[test] fn can_take_off_again_after_landing() {
    let (mut a, t) = heli(1.0);
    fly(&mut a, &t, FlightInput { lift: -0.3, ..Default::default() }, 3.0);
    assert_eq!(a.status, Status::Landed);
    fly(&mut a, &t, FlightInput { lift: 1.0, ..Default::default() }, 2.0);
    assert!(a.status == Status::Flying && a.pos.y > 3.0);
}
#[test] fn hitting_a_tree_or_rock_crashes() {
    let (mut a, t) = heli(5.0);
    let tree = Prop { kind: PropKind::Tree, pos: v3(0., 0., 20.), radius: 0.35, height: 8.0 };
    for _ in 0..(10.0 / DT) as usize { a.step(&t, &[tree], &FlightInput { pitch: 1.0, ..Default::default() }, DT); }
    assert_eq!(a.status, Status::Crashed);
    let (mut b, t) = heli(2.0);
    let rock = Prop { kind: PropKind::Rock, pos: v3(0., 0., 15.), radius: 2.0, height: 2.8 };
    for _ in 0..(10.0 / DT) as usize { b.step(&t, &[rock], &FlightInput { pitch: 1.0, ..Default::default() }, DT); }
    assert_eq!(b.status, Status::Crashed);
}
#[test] fn flying_over_a_tree_is_fine() {
    let (mut a, t) = heli(30.0);
    let tree = Prop { kind: PropKind::Tree, pos: v3(0., 0., 20.), radius: 0.35, height: 8.0 };
    for _ in 0..(10.0 / DT) as usize { a.step(&t, &[tree], &FlightInput { pitch: 1.0, ..Default::default() }, DT); }
    assert_eq!(a.status, Status::Flying);
}
#[test] fn the_world_wall_stops_outward_flight() {
    let (mut a, t) = heli(50.0);
    a.pos.x = WORLD_LIMIT - 30.0;
    a.yaw = std::f32::consts::FRAC_PI_2;
    fly(&mut a, &t, FlightInput { pitch: 1.0, ..Default::default() }, 20.0);
    assert!(a.pos.x <= WORLD_LIMIT - 20.0 + 0.1, "{}", a.pos.x);
}
#[test] fn big_steps_and_garbage_input_stay_finite() {
    let (mut a, t) = heli(50.0);
    for _ in 0..300 { a.step(&t, &[], &FlightInput { yaw: f32::NAN, lift: 9.0, pitch: f32::INFINITY, roll: -9.0 }, 0.1); }
    assert!(a.pos.is_finite() && a.vel.is_finite() && a.yaw.is_finite() && a.pitch.is_finite());
    let i = FlightInput { yaw: f32::NAN, lift: 5.0, pitch: -5.0, roll: f32::NEG_INFINITY }.sanitized();
    assert_eq!((i.yaw, i.lift, i.pitch, i.roll), (0.0, 1.0, -1.0, 0.0));
}
#[test] fn crashed_aircraft_stays_crashed_and_still() {
    let (mut a, t) = heli(10.0);
    fly(&mut a, &t, FlightInput { lift: -1.0, ..Default::default() }, 8.0);
    let p = a.pos;
    fly(&mut a, &t, FlightInput { lift: 1.0, pitch: 1.0, ..Default::default() }, 2.0);
    assert_eq!((a.status, a.pos), (Status::Crashed, p));
}
#[test] fn rotation_matches_heading_and_tilt() {
    let mut a = Aircraft::new(AircraftKind::Heli, v3(0., 5., 0.), 0.7);
    let f = a.rotation().rotate(v3(0., 0., 1.));
    assert!((f.x - 0.7f32.sin()).abs() < 1e-4 && (f.z - 0.7f32.cos()).abs() < 1e-4);
    a.pitch = 0.3;
    assert!(a.rotation().rotate(v3(0., 0., 1.)).y < -0.2);
}
