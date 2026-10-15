use super::*;
use crate::game::{input::Input, math::{v3, V3, Q}, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};

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

#[test] fn the_rally_lamps_sit_on_the_lamp_vertices_of_the_asset() {
    // the lamp vertices (material 5) of the embedded Peugeot asset: the headlight units are light cream, the tail units red
    let d = include_bytes!("../../assets/rally_car.rcar");
    let nv = u32::from_le_bytes([d[4], d[5], d[6], d[7]]) as usize;
    let f = |at: usize| f32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]);
    let (mut head, mut n) = ([0.0f32; 3], 0.0f32);
    for i in 0..nv {
        let a = 12 + i * 28;
        if d[a + 24..a + 28] == [255, 242, 204, 5] && f(a) > 0.45 && f(a + 8) > 1.5 { for k in 0..3 { head[k] += f(a + 4 * k); } n += 1.0; }
    }
    assert!(n > 20.0, "the asset has headlight vertices");
    let spec = VehicleSpec::by_id("vulpes").unwrap();
    let v = Vehicle::new(spec, v3(0.0, 0.0, 0.0), 0.0);   // at rest on no ground: the body pose is fixed by the spec
    let (rot, origin) = crate::raster::vehicle_models::car_body_pose(&v);
    let k = [spec.size.x / 1.91, spec.size.y / 1.50, spec.size.z / 3.97];
    let centre = origin + rot.rotate(v3(head[0] / n * k[0], head[1] / n * k[1], head[2] / n * k[2]));
    let right_lamp = lamps(&v).head.iter().copied().min_by(|a, b| (*a - centre).len().total_cmp(&(*b - centre).len())).unwrap();
    assert!((right_lamp - centre).len() < 0.05, "the lamp is {:.3} m from the asset's headlight", (right_lamp - centre).len());
}

fn flat_lamp() -> (V3, V3, V3) { (v3(0.6, 0.7, 0.0), v3(0.0, -0.05, 1.0).norm(), v3(1.0, 0.0, 0.0)) }
fn vulpes() -> VehicleSpec { VehicleSpec::by_id("vulpes").expect("vulpes") }

#[test] fn the_beam_is_strongest_on_the_axis_and_falls_with_distance() {
    let (l, f, r) = flat_lamp();
    let near = beam_gain(l, f, r, l + f * 8.0 + v3(0.0, -0.4, 0.0));
    let far = beam_gain(l, f, r, l + f * 60.0 + v3(0.0, -3.0, 0.0));
    assert!(near > 0.5 && far < near * 0.5 && far > 0.0, "{near} {far}");
}
#[test] fn the_beam_has_a_sharp_upper_cut_off_that_is_higher_on_the_right() {
    let (l, f, r) = flat_lamp();
    let below = beam_gain(l, f, r, l + f * 20.0 + v3(0.0, -0.5, 0.0));
    let above = beam_gain(l, f, r, l + f * 20.0 + v3(0.0, 3.0, 0.0));
    assert!(below > 0.3 && above < 0.02, "{below} {above}");
    let cut = |x: f32| beam_gain(l, f, r, l + f * 20.0 + r * x + v3(0.0, 0.55, 0.0));
    assert!(cut(5.0) > cut(-5.0) * 2.0, "right {} left {}", cut(5.0), cut(-5.0));
}
#[test] fn the_beam_ignores_what_is_behind_or_far_to_the_side_and_never_gives_nan() {
    let (l, f, r) = flat_lamp();
    assert_eq!(beam_gain(l, f, r, l - f * 5.0), 0.0);
    assert!(beam_gain(l, f, r, l + r * 20.0 + f * 2.0) < 0.01);
    for p in [l, l + f * 1e-6] { assert!(beam_gain(l, f, r, p).is_finite()); }
}
#[test] fn the_beam_follows_a_turned_and_pitched_body() {
    let (l, f, r) = flat_lamp();
    let q = Q::axis_angle(v3(0., 1., 0.), 1.1).mul(Q::axis_angle(v3(1., 0., 0.), 0.2));
    let (l2, f2, r2) = (q.rotate(l), q.rotate(f), q.rotate(r));
    let a = beam_gain(l, f, r, l + f * 12.0 + v3(0.0, -0.4, 0.0));
    let b = beam_gain(l2, f2, r2, l2 + q.rotate(f * 12.0 + v3(0.0, -0.4, 0.0)));
    assert!((a - b).abs() < 1e-4, "{a} {b}");
}
#[test] fn the_glow_needs_a_view_along_the_lamp() {
    let axis = v3(0.0, 0.0, 1.0);
    assert!(glow_factor(axis, v3(0.0, 0.0, 1.0)) > 0.95);
    assert!(glow_factor(axis, v3(1.0, 0.0, 0.2).norm()) < 0.25, "from the side");
    assert_eq!(glow_factor(axis, v3(0.0, 0.0, -1.0)), 0.0, "from behind the lamp");
}
#[test] fn rally_cars_have_position_lamps_beside_the_headlights_at_the_front() {
    let v = settled(vulpes());
    let l = lamps(&v);
    for p in l.pos { assert!((p - v.pos).dot(v.forward()) > v.spec.size.z * 0.35); }
    assert!(((l.pos[0] - v.pos).dot(v.right()) + (l.pos[1] - v.pos).dot(v.right())).abs() < 1e-3);
}
