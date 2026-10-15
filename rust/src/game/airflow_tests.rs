use super::*;

#[test]
fn embedded_config_parses() {
    let c = cfg();
    assert_eq!(c.n, [40, 20, 80]); assert!((c.cell - 0.3).abs() < 1e-6); assert!(!c.debug);
}
#[test]
fn bad_or_missing_block_is_a_clear_error() {
    assert!(AirflowCfg::from_json("{}").unwrap_err().contains("airflow"));
    let bad = r#"{"airflow":{"cell":0.01,"nx":48,"ny":24,"nz":96,"iterations":8,"substeps":1,"confinement":0.5}}"#;
    assert!(AirflowCfg::from_json(bad).unwrap_err().contains("cell"));
}

#[test]
fn empty_box_stays_uniform_and_finite() {
    let mut a = Airflow::new([8, 8, 16], 0.5);
    a.set_inflow([0.0, 0.0, -20.0]);
    for _ in 0..30 { a.step(1.0 / 60.0, 20, 0.0); }
    for v in &a.vel { assert!(v.iter().all(|x| x.is_finite())); }
    let mid = a.vel[a.idx([4, 4, 8])];
    assert!((mid[2] + 20.0).abs() < 0.5 && mid[0].abs() < 0.2 && mid[1].abs() < 0.2, "{mid:?}");
}
#[test]
fn standing_car_makes_no_wind() {
    let mut a = Airflow::new([8, 8, 16], 0.5);
    a.set_solid_box([2, 0, 6], [5, 3, 10]);
    for _ in 0..30 { a.step(1.0 / 60.0, 20, 0.5); }
    assert!(a.vel.iter().all(|v| v.iter().all(|x| x.is_finite() && x.abs() < 1e-3)));
}
#[test]
fn a_cube_leaves_a_slow_wake_and_speeds_up_beside_it() {
    let mut a = Airflow::new([16, 12, 40], 0.5);
    a.set_inflow([0.0, 0.0, -20.0]);
    a.set_solid_box([6, 0, 14], [10, 5, 18]);   // the flow runs towards -z, so the wake is at z < 14
    for _ in 0..90 { a.step(1.0 / 60.0, 20, 0.3); }
    let (wake, far, beside) = (a.speed_at([8, 2, 11]), a.speed_at([8, 2, 4]), a.speed_at([12, 2, 16]));
    println!("wake {wake} far {far} beside {beside}");
    assert!(wake < 10.0, "wake {wake}");
    assert!(beside > 20.0, "beside {beside}");
    assert!(far > wake, "far {far} wake {wake}");
}
#[test]
fn what_flows_in_flows_out() {
    let mut a = Airflow::new([16, 12, 40], 0.5);
    a.set_inflow([0.0, 0.0, -20.0]);
    a.set_solid_box([6, 0, 14], [10, 5, 18]);
    for _ in 0..40 { a.step(1.0 / 60.0, 30, 0.0); }
    let flux = |z: i32| -> f32 { (0..12).flat_map(|y| (0..16).map(move |x| (x, y))).map(|(x, y)| -a.vel[a.idx([x, y, z])][2]).sum() };
    let (up, down) = (flux(30), flux(4));
    assert!((up - down).abs() < 0.1 * up, "up {up} down {down}");
}
#[test]
fn huge_dt_does_not_blow_up() {
    let mut a = Airflow::new([8, 8, 16], 0.5);
    a.set_inflow([0.0, 0.0, -60.0]);
    a.set_solid_box([2, 0, 6], [5, 3, 10]);
    for _ in 0..10 { a.step(0.5, 20, 0.5); }
    assert!(a.vel.iter().all(|v| v.iter().all(|x| x.is_finite())));
}

fn count(af: &Airflow) -> usize { af.solid.iter().filter(|s| s.is_some()).count() }
fn vulpes() -> VehicleSpec { VehicleSpec::by_id("vulpes").expect("vulpes") }
#[test]
fn body_fills_cells_and_each_part_adds_some() {
    let mut a = Airflow::new([48, 24, 96], 0.25); voxelize(&mut a, &vulpes(), 12.0);
    let n0 = count(&a);
    assert!(n0 > 100, "{n0}");
    for part in 0..3 {
        let mut s = vulpes();
        match part { 0 => s.kit.splitter = 3, 1 => s.kit.wing = 3, _ => s.kit.underbody = 3 }
        let mut b = Airflow::new([48, 24, 96], 0.25); voxelize(&mut b, &s, 12.0);
        assert!(count(&b) > n0, "part {part}");
    }
}
#[test]
fn body_stays_inside_the_box_with_all_parts() {
    for id in ["offroader", "vulpes"] {
        let mut s = VehicleSpec::by_id(id).unwrap(); s.kit = crate::game::aero::AeroKit { splitter: 3, wing: 3, underbody: 3, wing_angle: 2 };
        for (lo, hi) in body_boxes(&s, 0.15, 12.0) { assert!(lo[0] >= -6.0 && hi[0] <= 6.0 && lo[1] >= 0.0 && hi[1] <= 6.0 && lo[2] >= 0.0 && hi[2] <= 24.0, "{id}: {lo:?} {hi:?}"); }
    }
}
#[test]
fn the_car_in_the_wind_makes_a_wake() {
    let mut a = Airflow::new([48, 24, 96], 0.25);
    a.set_inflow([0.0, 0.0, -30.0]);
    voxelize(&mut a, &vulpes(), 12.0);
    for _ in 0..12 { a.step(1.0 / 60.0, 12, 0.3); }
    assert!(a.vel.iter().all(|v| v.iter().all(|x| x.is_finite())));
    let behind = a.speed_at([24, 5, 36]);   // ~1 m behind the tail, at mid height
    assert!(behind < 20.0, "{behind}");
}

#[test]
fn inflow_is_against_the_cars_motion() {
    use crate::game::math::v3;
    let mut v = Vehicle::new(vulpes(), v3(0.0, 1.0, 0.0), 0.0);
    v.vel = v.forward() * 30.0;
    let f = flow_in(&v, v3(0.0, 0.0, 0.0), &cfg());
    assert!((f.inflow[2] + 30.0).abs() < 1e-3 && f.inflow[0].abs() < 1e-3 && f.inflow[1].abs() < 1e-3, "{:?}", f.inflow);
    let tail = flow_in(&v, v.forward() * 10.0, &cfg());   // a tailwind of 10 m/s
    assert!((tail.inflow[2] + 20.0).abs() < 1e-3, "{:?}", tail.inflow);
}
#[test]
fn starting_from_rest_stays_bounded() {
    let mut a = Airflow::new([12, 8, 48], 0.5);
    a.inflow = [0.0, 0.0, -17.0];   // the air at rest, only the front face moves
    a.set_solid_box([4, 0, 20], [8, 3, 26]);
    for _ in 0..180 { a.step(1.0 / 60.0, 20, 0.3); }
    let max = a.vel.iter().map(|v| v[0].abs().max(v[1].abs()).max(v[2].abs())).fold(0.0, f32::max);
    assert!(max < 25.0, "{max}");   // it blew up (to 220) once the BFECC result was not limited
}
#[test]
fn inflow_is_smoothed_and_tilts_little() {
    let a = smooth_inflow([0.0, 0.0, -17.0], [0.0, 0.0, -5.0], 1.0 / 60.0);
    assert!(a[2] < -16.0 && a[2] > -17.0, "{a:?}");   // moves towards the target, slowly (time constant 0.3 s)
    let mut x = [0.0, 0.0, -17.0];
    for _ in 0..600 { x = smooth_inflow(x, [0.0, 0.0, -5.0], 1.0 / 60.0); }
    assert!((x[2] + 5.0).abs() < 0.01, "{x:?}");
    let mut v = Vehicle::new(vulpes(), crate::game::math::v3(0.0, 1.0, 0.0), 0.0);
    v.vel = v.forward() * 10.0 + crate::game::math::v3(0.0, -8.0, 0.0);   // a hard landing
    let f = flow_in(&v, crate::game::math::v3(0.0, 0.0, 0.0), &cfg());
    assert!(f.inflow[1].abs() <= 0.25 * f.inflow[2].abs() + 1e-3, "{:?}", f.inflow);
}
#[test]
fn speed_limiter_keeps_the_air_near_the_inflow() {
    let mut a = Airflow::new([8, 8, 16], 0.5);
    a.set_inflow([0.0, 0.0, -10.0]);
    let i = a.idx([4, 4, 8]); a.vel[i] = [90.0, 90.0, 90.0];
    a.step(1.0 / 60.0, 10, 0.0);
    let m = a.vel.iter().map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()).fold(0.0, f32::max);
    assert!(m <= SPEED_LIMIT * 10.0 + 1.0 + 1e-3, "{m}");
}
