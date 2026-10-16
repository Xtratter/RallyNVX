use super::*;
use crate::game::{input::Input, math::{v3, Q}, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};
fn finite_unit(m: &Mesh3, name: &str) {
    assert!(!m.verts.is_empty() && m.idx.len() % 3 == 0, "{name}");
    assert!(m.idx.iter().all(|&i| (i as usize) < m.verts.len()), "{name} indices");
    for v in &m.verts { assert!(v.pos.iter().all(|c| c.is_finite()), "{name}"); let l = (v.nrm[0].powi(2) + v.nrm[1].powi(2) + v.nrm[2].powi(2)).sqrt(); assert!((l - 1.0).abs() < 1e-3, "{name} normal {l}"); assert!(v.col[3] <= MAT_FOLIAGE, "{name} material"); }
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

#[test] fn the_caliper_does_not_spin_but_follows_steer_and_suspension() {
    use crate::game::{math::v3, vehicle::{Vehicle, VehicleSpec}};
    let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0.0, 1.0, 0.0), 0.0); v.steer = 0.3; v.comp = [0.05, 0.0, 0.1, 0.0];
    let (wheel, cal) = (car_wheel_poses(&v, 1.2), car_caliper_poses(&v));
    for w in 0..4 { assert!((wheel[w].1 - cal[w].1).len() < 1e-5, "same hub position"); }
    let spun = car_wheel_poses(&v, 0.0);
    for w in 0..4 { assert!((cal[w].0.rotate(v3(0., 1., 0.)) - spun[w].0.rotate(v3(0., 1., 0.))).len() < 1e-5, "the caliper's orientation is the wheel's with no spin"); }
    assert!((wheel[0].0.rotate(v3(0., 0., 1.)) - cal[0].0.rotate(v3(0., 0., 1.))).len() > 0.1, "the wheel itself has turned");
}

#[test] fn the_suspension_offsets_are_zero_at_rest_map_to_the_right_corner_and_stay_bounded() {
    use crate::game::{math::v3, vehicle::{Vehicle, VehicleSpec}};
    let spec = VehicleSpec::by_id("vulpes").unwrap();
    let mut v = Vehicle::new(spec, v3(0.0, 1.0, 0.0), 0.0);
    let c0 = (spec.mass * 9.81 / 4.0 / spec.spring).min(spec.rest);
    v.comp = [c0; 4];
    assert!(susp_offsets(&v).iter().all(|o| o.abs() < 1e-6), "the static compression is no offset");
    for w in 0..4 {
        let mut c = [c0; 4]; c[w] = c0 + 0.1; v.comp = c;
        let o = susp_offsets(&v);
        let corner = (if spec.wheels[w].z > 0.0 { 0 } else { 2 }) + (if spec.wheels[w].x > 0.0 { 0 } else { 1 });
        assert!((o[corner] - 0.1).abs() < 1e-6 && o.iter().enumerate().all(|(k, x)| k == corner || x.abs() < 1e-6), "wheel {w} -> corner {corner}: {o:?}");
    }
    let mut all = std::collections::HashSet::new();
    for w in 0..4 { all.insert((if spec.wheels[w].z > 0.0 { 0 } else { 2 }) + (if spec.wheels[w].x > 0.0 { 0 } else { 1 })); }
    assert_eq!(all.len(), 4, "every wheel has its own corner");
    v.comp = [9.0, -9.0, f32::NAN, 0.0];
    assert!(susp_offsets(&v).iter().all(|o| o.is_finite() && o.abs() <= 0.25), "clamped and finite");
}

#[test] fn the_glass_is_see_through_near_solid_far_and_covers_are_clearer() {
    let near = glass_alpha(3.0, 0.0, 0.0);
    assert!(near > 0.15 && near < 0.3, "{near}");
    assert!(glass_alpha(3.0, 1.0, 0.0) > near, "fresnel makes it more opaque");
    let far = glass_alpha(12.0, 0.0, 0.0);
    assert!(far > 0.9 && far <= 1.0, "{far}");
    assert!(glass_alpha(3.0, 0.3, 0.0) <= glass_alpha(6.0, 0.3, 0.0) && glass_alpha(6.0, 0.3, 0.0) <= glass_alpha(12.0, 0.3, 0.0), "rises with distance");
    assert!(glass_alpha(3.0, 0.3, 1.0) < glass_alpha(3.0, 0.3, 0.0) * 0.5, "a lamp cover is clear");
    for d in [0.0, 5.0, 9.0, 12.0, 16.0, 100.0] { for f in [0.0, 0.5, 1.0] { for c in [0.0, 1.0] { let a = glass_alpha(d, f, c); assert!((0.0..=1.0).contains(&a)); } } }
}
#[test] fn the_interior_is_drawn_only_near_the_car() {
    assert!(draw_interior(0.0) && draw_interior(7.9) && !draw_interior(8.0) && !draw_interior(500.0));
}

#[test] fn the_dust_sdf_input_includes_the_glass_so_window_holes_do_not_let_dust_in() {
    let tri = |x: f32| { let v = |p: [f32; 3]| Vert { pos: p, nrm: [0.0, 1.0, 0.0], col: [0; 4], uv: NO_UV }; Mesh3 { verts: vec![v([x, 0.0, 0.0]), v([x + 1.0, 0.0, 0.0]), v([x, 1.0, 0.0])], idx: vec![0, 1, 2] } };
    let mut car = CarModel::new(tri(0.0), tri(0.0));
    let (v0, i0) = sdf_input(&car);
    assert_eq!((v0.len(), i0.len()), (3, 3), "no glass: the body only");
    car.glass = Some(tri(5.0));
    let (v1, i1) = sdf_input(&car);
    assert_eq!((v1.len(), i1.len()), (6, 6));
    assert_eq!(&i1[3..], &[3, 4, 5], "the glass indices are shifted past the body's vertices");
    assert_eq!(v1[3][0], 5.0);
}

#[test] fn the_near_plane_is_close_inside_the_car_and_normal_outside() {
    assert_eq!(near_for(0.0), 0.08); assert_eq!(near_for(2.49), 0.08); assert_eq!(near_for(2.5), 0.3); assert_eq!(near_for(100.0), 0.3);
}

#[test] fn the_steering_wheel_follows_the_steering_within_its_lock_and_the_driver_leaves_the_cockpit_view() {
    assert_eq!(steering_wheel_angle(0.0), 0.0);
    assert!((steering_wheel_angle(0.2) + steering_wheel_angle(-0.2)).abs() < 1e-6, "odd");
    assert!(steering_wheel_angle(0.3).abs() > steering_wheel_angle(0.1).abs(), "more steer, more turn");
    assert!(steering_wheel_angle(10.0).abs() <= 3.8 + 1e-6 && steering_wheel_angle(-10.0).abs() <= 3.8 + 1e-6, "bounded");
    assert_eq!(steering_wheel_angle(f32::NAN), 0.0);
    assert!(draw_driver(5.0) && draw_driver(1.2) && !draw_driver(0.9) && !draw_driver(0.0), "no driver with the camera inside the car");
}
