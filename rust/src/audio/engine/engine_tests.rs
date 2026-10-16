use super::*;

const DT: f64 = 1.0 / 88200.0;
fn run(e: &mut Engine, secs: f64) { for _ in 0..(secs / DT) as usize { e.step(DT); } }
/// Cranks the engine on the starter until it runs by itself.
fn started() -> Engine {
    let mut e = Engine::new(EngineSpec::rally_four());
    e.controls = Controls { starter: true, ..Default::default() };
    run(&mut e, 1.5);
    e.controls.starter = false;
    run(&mut e, 2.0);
    e
}

#[test] fn the_volume_of_a_cylinder_follows_the_crank_and_the_compression_ratio_is_right() {
    let s = EngineSpec::rally_four();
    let a = std::f64::consts::PI * s.bore * s.bore / 4.0;
    let vc = s.displacement() / 4.0 / (s.compression - 1.0);
    let (top, bottom) = (vc + a * r_x(&s, 0.0), vc + a * r_x(&s, std::f64::consts::PI));
    assert!((bottom / top - s.compression).abs() < 1e-6, "{}", bottom / top);
    assert!((s.displacement() - 0.001997).abs() < 2e-5, "a two-litre: {}", s.displacement());
    // the piston's speed is the derivative of its place
    let h = 1e-6; let a0 = 1.0;
    assert!((piston_dx(&s, a0) - (r_x(&s, a0 + h) - r_x(&s, a0 - h)) / (2.0 * h)).abs() < 1e-6);
}
#[test] fn the_valves_open_and_close_at_their_angles() {
    let s = EngineSpec::rally_four();
    assert_eq!(Engine::lift(s.intake_open, s.intake_close, 100.0), 0.0);
    assert!((Engine::lift(s.intake_open, s.intake_close, 470.0) - 1.0).abs() < 1e-9, "the peak in the middle of the duration");
    assert!(Engine::lift(s.exhaust_open, s.exhaust_close, 250.0) > 0.9 && Engine::lift(s.exhaust_open, s.exhaust_close, 450.0) == 0.0);
    assert!(Engine::lift(s.intake_open, s.intake_close, 360.0) > 0.0 && Engine::lift(s.exhaust_open, s.exhaust_close, 360.0) > 0.0, "overlap at the top");
    assert!(crossed(718.0, 2.0, 0.5) && !crossed(10.0, 12.0, 0.5) && crossed(100.0, 101.0, 100.5));
}
#[test] fn the_starter_cranks_it_and_it_idles_by_itself_near_the_idle_speed() {
    let mut e = Engine::new(EngineSpec::rally_four());
    e.controls = Controls { starter: true, ..Default::default() };
    run(&mut e, 0.08);
    assert!(e.rpm() > 60.0 && e.rpm() < 900.0, "cranking: {} rpm", e.rpm());
    run(&mut e, 1.4); e.controls.starter = false; run(&mut e, 2.5);
    let mut lo = f64::MAX; let mut hi = 0.0f64;
    for _ in 0..20 { run(&mut e, 0.05); lo = lo.min(e.rpm()); hi = hi.max(e.rpm()); }
    assert!(lo > 700.0 && hi < 1300.0, "it idles: {lo}..{hi} rpm");
}
#[test] fn full_throttle_revs_it_up_to_the_limiter_and_the_fuel_cut_holds_it_there() {
    let mut e = started();
    e.controls.throttle = 1.0;
    run(&mut e, 2.5);
    let mut top = 0.0f64; let mut cut = false;
    for _ in 0..40 { run(&mut e, 0.02); top = top.max(e.rpm()); cut |= e.fuel_cut(); }
    assert!(top > 7000.0 && top < 8400.0 && cut, "the limiter: top {top} rpm, cut {cut}");
}
#[test] fn the_torque_is_that_of_a_two_litre_and_grows_with_the_throttle() {
    // held at a speed by the clutch, the torque the clutch takes is the engine's net torque
    let measure = |rpm: f64, thr: f64| {
        let mut e = Engine::new(EngineSpec::rally_four());
        e.controls = Controls { throttle: thr, clutch: 1.0, drive_rpm: rpm, ..Default::default() };
        e.omega = rpm * TAU / 60.0;
        run(&mut e, 0.6);
        let mut sum = 0.0; let n = (0.2 / DT) as usize;
        for _ in 0..n { e.step(DT); sum += e.clutch_torque; }
        sum / n as f64
    };
    let (t4, t_half, t_idle) = (measure(4000.0, 1.0), measure(4000.0, 0.4), measure(4000.0, 0.0));
    assert!(t4 > 110.0 && t4 < 300.0, "torque at 4000 rpm, full throttle: {t4} N m");
    assert!(t4 > t_half + 20.0 && t_half > t_idle + 10.0, "more throttle, more torque: {t4} {t_half} {t_idle}");
    assert!(t_idle < 0.0, "a closed throttle brakes (engine braking): {t_idle}");
    let hp6 = measure(6000.0, 1.0) * 6000.0 * TAU / 60.0 / 745.7;
    assert!(hp6 > 100.0 && hp6 < 260.0, "power at 6000 rpm: {hp6} hp");
}
#[test] fn the_ignition_off_stops_the_fire_and_the_engine_winds_down() {
    let mut e = started();
    e.controls.ignition = false; e.controls.throttle = 0.3;
    run(&mut e, 2.0);
    assert!(e.rpm() < 300.0, "{} rpm", e.rpm());
}
#[test] fn the_state_stays_finite_and_physical_through_abuse() {
    let mut e = Engine::new(EngineSpec::rally_four());
    e.controls = Controls { starter: true, throttle: 1.0, ..Default::default() };
    for i in 0..(6.0 / DT) as usize {
        if i % 20000 == 0 { e.controls.throttle = if (i / 20000) % 2 == 0 { 1.0 } else { 0.0 }; e.controls.clutch = ((i / 7000) % 3) as f64 * 0.5; e.controls.drive_rpm = ((i / 9000) % 5) as f64 * 1800.0; }
        e.step(DT);
        if i % 997 == 0 { assert!(e.omega.is_finite() && e.exhaust_pressure().is_finite() && e.intake_pressure().is_finite() && e.collector.pressure() > 0.0 && e.plenum.pressure() > 0.0, "step {i}"); }
    }
    assert!(e.cyl.iter().all(|c| c.gas.pressure() < 1.5e7 && c.gas.temperature() < 4000.0), "no absurd cylinders");
}

#[test] #[ignore] fn time_one_second() {
    let mut e = started();
    e.controls.throttle = 0.7;
    let t0 = std::time::Instant::now();
    run(&mut e, 1.0);
    eprintln!("TIME one second of engine: {:?} (rpm {:.0})", t0.elapsed(), e.rpm());
}
