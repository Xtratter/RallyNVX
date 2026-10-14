use super::*;
use crate::game::{input::Input, math::{v3, Q}, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};

fn cars() -> Vec<VehicleSpec> { VehicleSpec::all().into_iter().map(|(_, s)| s).collect() }
fn settled(spec: VehicleSpec) -> Vehicle {
    let t = Terrain::flat(); let mut v = Vehicle::new(spec, v3(0., 1.2, 0.), 0.0);
    for _ in 0..180 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v
}

#[test] fn lamps_sit_at_the_right_ends_symmetric_and_at_a_plausible_height() {
    for spec in cars() {
        let v = settled(spec); let l = lamps(&v); let f = v.forward(); let r = v.right();
        for p in l.head { assert!((p - v.pos).dot(f) > spec.size.z * 0.4, "head lamp not at the front"); assert!(p.y > 0.4 && p.y < 1.25, "height {}", p.y); }
        for p in l.tail.iter().chain(&l.rev) { assert!((*p - v.pos).dot(f) < -spec.size.z * 0.4, "tail lamp not at the rear"); }
        for pair in [l.head, l.tail, l.rev] { assert!(((pair[0] - v.pos).dot(r) + (pair[1] - v.pos).dot(r)).abs() < 1e-3, "lamps are symmetric"); assert!((pair[0] - pair[1]).len() > 0.3); }
        assert!(l.head_dir.dot(f) > 0.99 && l.head_dir.y < 0.0, "the beam points ahead and slightly down");
    }
}
#[test] fn lamps_follow_the_rotation_of_the_body() {
    let spec = VehicleSpec::offroader();
    let mut v = Vehicle::new(spec, v3(10., 2., -5.), 0.0);
    v.rot = Q::axis_angle(v3(0., 1., 0.), 1.2).mul(Q::axis_angle(v3(1., 0., 0.), 0.3));
    let l = lamps(&v);
    for p in l.head { assert!((p - v.pos).dot(v.forward()) > spec.size.z * 0.4); }
    assert!((l.head_dir.len() - 1.0).abs() < 1e-4);
}
#[test] fn the_headlights_follow_the_night_and_the_other_lamps_follow_the_driver() {
    let mut v = settled(VehicleSpec::offroader());
    assert_eq!(state_with(u8::MAX, &v, 0.0), CarLights { head: false, brake: false, reverse: false });
    assert!(state_with(u8::MAX, &v, 0.3).head && !state_with(u8::MAX, &v, 0.2).head);
    v.braking = true;
    assert!(state_with(u8::MAX, &v, 0.0).brake && !state_with(u8::MAX, &v, 0.0).head, "braking in daylight: tail lights only");
    v.braking = false; v.reversing = true;
    assert!(state_with(u8::MAX, &v, 0.0).reverse);
}
#[test] fn the_debug_modes_force_the_lamps() {
    let v = settled(VehicleSpec::offroader());
    assert_eq!(state_with(1, &v, 0.0), CarLights { head: true, brake: true, reverse: false });
    assert_eq!(state_with(2, &v, 0.0), CarLights { head: true, brake: false, reverse: true });
    assert_eq!(state_with(0, &v, 0.0), CarLights { head: true, brake: false, reverse: false });
}
#[test] fn the_vehicle_remembers_brake_and_reverse() {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.2, 0.), 0.0);
    v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 60.0); assert!(v.braking && !v.reversing);
    v.step(&t, &[], &Input { handbrake: true, ..Default::default() }, 1.0 / 60.0); assert!(v.braking);
    v.step(&t, &[], &Input::default(), 1.0 / 60.0); assert!(!v.braking);
    v.step(&t, &[], &Input { throttle: -1.0, ..Default::default() }, 1.0 / 60.0); assert!(v.reversing, "throttle asks for reverse");
    for _ in 0..200 { v.step(&t, &[], &Input { throttle: -1.0, ..Default::default() }, 1.0 / 60.0); }
    v.step(&t, &[], &Input::default(), 1.0 / 60.0);
    assert!(v.speed() < -0.5 && v.reversing, "rolling backwards: reverse lamps stay on, speed {}", v.speed());
    v.vel = v3(0., 0., 0.3); v.step(&t, &[], &Input::default(), 1.0 / 60.0); assert!(!v.reversing, "creeping forward is not reversing");
}
