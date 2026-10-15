use super::*;

#[test]
fn embedded_config_parses() {
    let c = cfg();
    assert_eq!(c.n, [48, 24, 96]); assert!((c.cell - 0.25).abs() < 1e-6); assert!(!c.debug);
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
