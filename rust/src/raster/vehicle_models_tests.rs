use super::*;
use crate::game::{flight::{Aircraft, AircraftKind, AircraftSpec}, input::Input, math::{v3, Q}, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};
fn finite_unit(m: &Mesh3, name: &str) {
    assert!(!m.verts.is_empty() && m.idx.len() % 3 == 0, "{name}");
    assert!(m.idx.iter().all(|&i| (i as usize) < m.verts.len()), "{name} indices");
    for v in &m.verts { assert!(v.pos.iter().all(|c| c.is_finite()), "{name}"); let l = (v.nrm[0].powi(2) + v.nrm[1].powi(2) + v.nrm[2].powi(2)).sqrt(); assert!((l - 1.0).abs() < 1e-3, "{name} normal {l}"); assert!(v.col[3] <= MAT_EMISSIVE, "{name} material"); }
}
fn outward(m: &Mesh3, name: &str) { outward_min(m, name, 0.2) }
fn outward_min(m: &Mesh3, name: &str, min: f32) {
    let (lo, hi) = m.bounds(); let c = (lo + hi) * 0.5;
    let s: f32 = m.verts.iter().map(|v| { let d = v3(v.pos[0] - c.x, v.pos[1] - c.y, v.pos[2] - c.z).norm(); d.x * v.nrm[0] + d.y * v.nrm[1] + d.z * v.nrm[2] }).sum::<f32>() / m.verts.len() as f32;
    assert!(s > min, "{name}: normals do not look outward ({s})");
}
#[test] fn the_car_model_is_valid_sized_like_the_spec_and_within_the_triangle_limit() {
    let spec = VehicleSpec::offroader(); let m = build_car(&spec);
    finite_unit(&m.body, "car body"); finite_unit(&m.wheel, "wheel"); outward(&m.body, "car body");
    assert!(m.body.tris() + 4 * m.wheel.tris() <= 4000, "{}", m.body.tris() + 4 * m.wheel.tris());
    let (lo, hi) = m.body.bounds();
    let (w, l) = (hi.x - lo.x, hi.z - lo.z);
    assert!((w - spec.size.x).abs() <= spec.size.x * 0.12 + spec.look.wheel_width && (l - spec.size.z).abs() <= spec.size.z * 0.05, "body {w} x {l} vs {:?}", spec.size);
    assert!(hi.y - lo.y <= spec.size.y * 1.4);
    let mats: std::collections::HashSet<u8> = m.body.verts.iter().map(|v| v.col[3]).collect();
    for want in [MAT_PAINT, MAT_GLASS, MAT_PLASTIC, MAT_CHROME, MAT_EMISSIVE] { assert!(mats.contains(&want), "missing material {want}"); }
    assert!(m.wheel.verts.iter().any(|v| v.col[3] == MAT_RUBBER) && m.wheel.verts.iter().any(|v| v.col[3] == MAT_CHROME));
    let (wlo, whi) = m.wheel.bounds();
    assert!(((whi.y - wlo.y) / 2.0 - spec.radius).abs() < spec.radius * 0.08, "wheel radius {}", (whi.y - wlo.y) / 2.0);
    assert!((wlo.x + whi.x).abs() < 0.02 && (wlo.y + whi.y).abs() < 0.02, "the wheel is centred on its axle");
}
#[test] fn the_helicopter_and_drone_models_are_valid_and_small_enough() {
    let heli = build_heli(&AircraftSpec::of(AircraftKind::Heli)); let drone = build_drone(&AircraftSpec::of(AircraftKind::Drone));
    for (m, n) in [(&heli.body, "heli body"), (&heli.rotor, "rotor"), (&heli.tail_rotor, "tail rotor"), (&drone.body, "drone body"), (&drone.prop, "prop")] { finite_unit(m, n); }
    outward(&heli.body, "heli body"); outward_min(&drone.body, "drone body", 0.1);   // arms and legs make the drone less compact
    assert!(heli.body.tris() + heli.rotor.tris() + heli.tail_rotor.tris() <= 2000);
    assert!(drone.body.tris() + 4 * drone.prop.tris() <= 1000);
    let (lo, hi) = heli.rotor.bounds(); assert!(hi.x - lo.x > 9.0 && hi.y - lo.y < 0.6, "the rotor is a wide flat disc of blades");
    assert!(((lo.x + hi.x) / 2.0).abs() < 0.05 && ((lo.z + hi.z) / 2.0).abs() < 0.05, "centred on the hub");
}
#[test] fn wheel_poses_sit_at_the_mounts_and_front_wheels_steer() {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    let p = car_wheel_poses(&v, 0.0);
    for w in 0..4 { let mount = v.pos + v.rot.rotate(v.spec.wheels[w]); assert!((p[w].1.x - mount.x).abs() < 0.01 && (p[w].1.z - mount.z).abs() < 0.01, "wheel {w}"); assert!(p[w].1.y < mount.y + 1e-3); }
    let mut s = v; s.steer = 0.4;
    let ps = car_wheel_poses(&s, 0.0);
    let axle = |q: Q| q.rotate(v3(1., 0., 0.));
    assert!((axle(ps[0].0).z).abs() > 0.2 && (axle(ps[2].0).z).abs() < 0.05, "front wheels turn, rear wheels do not");
    let spun = car_wheel_poses(&v, 1.0);
    assert!((spun[0].0.rotate(v3(0., 1., 0.)) - p[0].0.rotate(v3(0., 1., 0.))).len() > 0.5, "spin rotates the wheel about its axle");
}
#[test] fn rotors_spin_about_their_hubs_and_follow_the_aircraft() {
    let a = Aircraft::new(AircraftKind::Heli, v3(10., 20., 30.), 0.5);
    let (p0, p1) = (heli_poses(&a, 0.0), heli_poses(&a, 0.05));
    assert!((p0.rotor.1 - p1.rotor.1).len() < 1e-4, "the hub does not move");
    assert!((p0.rotor.0.rotate(v3(1., 0., 0.)) - p1.rotor.0.rotate(v3(1., 0., 0.))).len() > 0.1, "the blades turn");
    assert!((p0.rotor.1 - a.pos).len() < 3.0);
    let d = Aircraft::new(AircraftKind::Drone, v3(0., 10., 0.), 0.0);
    let (body, props) = drone_poses(&d, 0.0);
    assert!((body.1 - d.pos).len() < 0.3);
    for pr in props { assert!((pr.1 - d.pos).len() < 1.2 && pr.1.y > d.pos.y - 0.2); }
}
#[test] fn drone_arms_run_along_the_diagonals_to_the_motors() {
    let m = build_drone(&AircraftSpec::of(AircraftKind::Drone));
    for (sx, sz) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let near = m.body.verts.iter().any(|v| (v.pos[1].abs() - 0.02).abs() < 1e-3 && (v.pos[0] - sx * 0.62).abs() < 0.08 && (v.pos[2] - sz * 0.62).abs() < 0.08);
        assert!(near, "no arm reaches the motor at ({sx},{sz})");
    }
}
