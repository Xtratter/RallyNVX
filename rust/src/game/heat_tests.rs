use super::*;

#[test] fn a_slipping_clutch_heats_and_a_cool_one_grips_fully() {
    let mut c = Clutch::new(20.0);
    assert_eq!(c.fade(), 1.0);
    for _ in 0..240 * 3 { c.step(1.0 / 240.0, 40000.0, 20.0); }   // three seconds of a 40 kW slip
    assert!(c.temp > 200.0, "{}", c.temp);
    assert!(c.fade() < 1.0, "it loses its grip: {}", c.fade());
    for _ in 0..240 * 120 { c.step(1.0 / 240.0, 0.0, 20.0); }
    assert!(c.temp < 100.0 && c.fade() == 1.0, "it cools in two minutes: {} {}", c.temp, c.fade());
}
#[test] fn the_fade_is_smooth_and_falls_to_a_floor() {
    let f = |t: f32| Clutch { temp: t }.fade();
    assert_eq!(f(100.0), 1.0); assert_eq!(f(250.0), 1.0);
    let mut last = 1.0; for t in (250..=600).step_by(10) { let v = f(t as f32); assert!(v <= last + 1e-6 && (last - v) < 0.1, "{t}: {v}"); last = v; }
    assert!((f(900.0) - 0.4).abs() < 1e-5, "the floor of a burnt clutch: {}", f(900.0));
}
#[test] fn a_launch_warms_the_clutch_a_little_and_a_hill_start_on_the_slip_a_lot() {
    let launch = { let mut c = Clutch::new(20.0); for _ in 0..240 { c.step(1.0 / 240.0, 30000.0, 20.0); } c.temp };   // one second of 30 kW
    let hill = { let mut c = Clutch::new(20.0); for _ in 0..240 * 8 { c.step(1.0 / 240.0, 9000.0, 20.0); } c.temp };
    assert!(launch > 40.0 && launch < 120.0, "a launch: {launch}");
    assert!(hill > 200.0, "feathering the clutch for 8 s: {hill}");
}
#[test] fn the_coolant_holds_its_temperature_at_a_cruise_and_overheats_when_pushed_slowly() {
    let run = |kw: f32, speed: f32, secs: f32| { let mut c = Coolant::new(70.0); for _ in 0..(secs * 60.0) as usize { c.step(1.0 / 60.0, kw * 1000.0, speed, 20.0); } c.temp };
    let cruise = run(40.0, 30.0, 600.0);
    assert!(cruise > 70.0 && cruise < 100.0, "a cruise: {cruise}");
    let idle = run(0.0, 0.0, 600.0);
    assert!(idle > 60.0 && idle < 100.0, "idle: {idle}");
    let hard_fast = run(150.0, 35.0, 600.0);
    assert!(hard_fast < 105.0, "flat out at speed is cooled by the air: {hard_fast}");
    let hard_slow = run(150.0, 4.0, 600.0);
    assert!(hard_slow > 110.0, "flat out at a crawl overheats: {hard_slow}");
}
#[test] fn an_overheated_engine_loses_power_and_a_cold_one_does_not() {
    let p = |t: f32| Coolant { temp: t }.power();
    assert_eq!(p(20.0), 1.0); assert_eq!(p(95.0), 1.0); assert_eq!(p(104.0), 1.0);
    assert!(p(118.0) < 1.0 && p(118.0) > 0.7);
    assert!((p(200.0) - 0.5).abs() < 1e-5, "the floor: {}", p(200.0));
}
#[test] fn the_state_stays_sane_under_garbage() {
    let mut c = Clutch::new(f32::NAN); let mut k = Coolant::new(f32::INFINITY);
    for x in [f32::NAN, f32::INFINITY, -1e9, 1e12, 0.0] {
        c.step(0.01, x, x); k.step(0.01, x, x, x);
        assert!(c.temp.is_finite() && c.temp > -60.0 && c.temp < 1500.0 && k.temp.is_finite() && k.temp > -60.0 && k.temp < 300.0, "{c:?} {k:?}");
        assert!(c.fade() >= 0.4 && k.power() >= 0.5);
    }
}
