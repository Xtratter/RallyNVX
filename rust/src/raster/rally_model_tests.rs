use super::*;
use crate::game::{input::Input, math::v3, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};
use crate::raster::vehicle_models::{build_car, car_body_pose, car_wheel_poses, Mesh3, MAT_CARLAMP, MAT_CHROME, MAT_GLASS, MAT_PAINT, MAT_PLASTIC, MAT_RUBBER};

/// Triangles of the rally car in total (body plus four wheels): lowered step by step after phone measurements.
const CAR_TRI_BUDGET: usize = 30_000;

fn peugeot() -> VehicleSpec { VehicleSpec::by_id("vulpes").unwrap() }
fn check(m: &Mesh3, name: &str) {
    assert!(!m.verts.is_empty() && m.idx.len() % 3 == 0 && m.idx.iter().all(|&i| (i as usize) < m.verts.len()), "{name}");
    for v in &m.verts { assert!(v.pos.iter().all(|c| c.is_finite()), "{name}"); let l = (v.nrm[0].powi(2) + v.nrm[1].powi(2) + v.nrm[2].powi(2)).sqrt(); assert!((l - 1.0).abs() < 1e-3, "{name} normal {l}"); }
}

#[test] fn the_rally_body_is_valid_sized_like_the_spec_and_within_the_triangle_budget() {
    let s = peugeot(); let m = build_car(&s);
    check(&m.body, "rally body"); check(&m.wheel, "rally wheel");
    assert!(m.body.tris() + 4 * m.wheel.tris() <= CAR_TRI_BUDGET, "{} triangles", m.body.tris() + 4 * m.wheel.tris());
    let (lo, hi) = m.body.bounds();
    assert!(hi.x.max(-lo.x) <= s.size.x / 2.0 * 1.08 && hi.x.max(-lo.x) >= s.size.x / 2.0 * 0.97, "half width {}", hi.x.max(-lo.x));
    assert!(((hi.z - lo.z) - s.size.z).abs() <= s.size.z * 0.06, "length {} vs {}", hi.z - lo.z, s.size.z);
    assert!(lo.y >= -0.01 && hi.y <= s.size.y * 1.08 && hi.y >= s.size.y * 0.95, "height {}..{} vs {}", lo.y, hi.y, s.size.y);
    let mats: std::collections::HashSet<u8> = m.body.verts.iter().map(|v| v.col[3]).collect();
    for want in [MAT_PAINT, MAT_GLASS, MAT_PLASTIC, MAT_CHROME, MAT_CARLAMP] { assert!(mats.contains(&want), "missing material {want}"); }
    assert!(m.wheel.verts.iter().any(|v| v.col[3] == MAT_RUBBER) && m.wheel.verts.iter().any(|v| v.col[3] == MAT_CHROME));
    let (wlo, whi) = m.wheel.bounds();
    assert!(((whi.y - wlo.y) / 2.0 - s.radius).abs() < s.radius * 0.08, "wheel radius {}", (whi.y - wlo.y) / 2.0);
}
#[test] fn the_asset_keeps_the_rear_wing_the_lamps_and_the_arches() {
    let s = peugeot(); let m = build_car(&s);
    let wing = m.body.verts.iter().filter(|v| v.pos[1] > 1.1 && v.pos[2] < s.wheels[2].z - 0.2).count();
    assert!(wing >= 30, "a rear wing above 1.1 m behind the rear axle: {wing} vertices");
    let front_lamps = m.body.verts.iter().filter(|v| v.col[3] == MAT_CARLAMP && v.pos[2] > 1.4).count();
    let rear_lamps = m.body.verts.iter().filter(|v| v.col[3] == MAT_CARLAMP && v.pos[2] < -1.5).count();
    assert!(front_lamps >= 20 && rear_lamps >= 6, "lamps: {front_lamps} front, {rear_lamps} rear vertices");
    // both sides are modelled (mirrors reach beyond the doors on each side)
    let (lo, hi) = m.body.bounds(); assert!(hi.x > 0.85 && lo.x < -0.85, "{lo:?} {hi:?}");
}
#[test] fn wheels_stand_on_the_ground_under_the_body_frame_and_at_the_mounts() {
    let s = peugeot();
    let mut v = Vehicle::new(s, v3(0., 1.0, 0.), 0.0); let t = Terrain::flat();
    for _ in 0..240 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    let (bq, bp) = car_body_pose(&v);
    let ground_y = bp.y;                                           // the rally body frame starts at the ground
    assert!(ground_y.abs() < 0.04, "the body frame origin is {ground_y} m from the ground");
    for (i, (q, p)) in car_wheel_poses(&v, 0.0).iter().enumerate() {
        let _ = q;
        assert!((p.y - s.radius).abs() < 0.04, "wheel {i} centre height {}", p.y);
        assert!((p.x - s.wheels[i].x).abs() < 0.02 && (p.z - s.wheels[i].z).abs() < 0.02, "wheel {i} at ({}, {})", p.x, p.z);
    }
    let _ = bq;
}
#[test] fn the_suv_model_is_untouched_by_the_rally_dispatch() {
    let o = VehicleSpec::offroader(); let m = build_car(&o);
    assert!(m.body.tris() > 0 && m.body.tris() + 4 * m.wheel.tris() <= 4000);
}
