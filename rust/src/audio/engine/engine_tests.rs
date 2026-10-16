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
    assert!(t4 > 200.0 && t4 < 420.0, "torque at 4000 rpm, full throttle: {t4} N m (turbocharged two-litre)");
    assert!(t4 > t_half + 20.0 && t_half > t_idle + 10.0, "more throttle, more torque: {t4} {t_half} {t_idle}");
    assert!(t_idle < 0.0, "a closed throttle brakes (engine braking): {t_idle}");
    let hp6 = measure(6000.0, 1.0) * 6000.0 * TAU / 60.0 / 745.7;
    assert!(hp6 > 180.0 && hp6 < 450.0, "power at 6000 rpm: {hp6} hp");
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


#[test] #[ignore] fn debug_turbo() {
    let mut e = Engine::new(EngineSpec::rally_four());
    e.controls = Controls { throttle: 0.0, clutch: 1.0, drive_rpm: 4500.0, ..Default::default() };
    e.omega = 4500.0 * TAU / 60.0;
    run(&mut e, 1.0);
    eprintln!("DBG closed: turbo {:.0} rpm boost {:.2} bar", e.turbo_omega * 60.0 / TAU, e.boost_bar());
    e.controls.throttle = 1.0;
    for i in 0..12 { run(&mut e, 0.25); eprintln!("DBG wot t={:.2}: turbo {:.0} rpm boost {:.2} bar wastegate {:.2} torque {:.0}", (i + 1) as f64 * 0.25, e.turbo_omega * 60.0 / TAU, e.boost_bar(), e.wastegate, e.clutch_torque); }
    e.controls.throttle = 0.0;
    let mut bov = 0.0f64; let mut peak = 0.0f64;
    for i in 0..60 { run(&mut e, 0.02); bov = bov.max(e.bov_flow); peak = peak.max(e.boost_bar()); if i % 10 == 0 { eprintln!("DBG lift t={:.2}: turbo {:.0} boost {:.2} bov {:.2} mol/s plenum {:.2}", (i + 1) as f64 * 0.02, e.turbo_omega * 60.0 / TAU, e.boost_bar(), e.bov_flow, e.plenum.pressure() / 1e5); } }
    eprintln!("DBG bov max {bov:.2} mol/s");
}


fn held(rpm: f64, thr: f64, als: bool) -> Engine {
    let mut e = Engine::new(EngineSpec::rally_four());
    e.controls = Controls { throttle: thr, clutch: 1.0, drive_rpm: rpm, als, ..Default::default() };
    e.omega = rpm * TAU / 60.0;
    e
}
#[test] fn the_turbo_spools_up_with_a_lag_to_the_wastegate_limit_and_idles_without_boost() {
    let mut e = held(4500.0, 0.0, false);
    run(&mut e, 1.0);
    assert!(e.boost_bar() < 0.12, "no boost with the throttle shut: {}", e.boost_bar());
    let idle_rpm = e.turbo_omega * 60.0 / TAU;
    e.controls.throttle = 1.0;
    run(&mut e, 0.15); let early = e.boost_bar();
    run(&mut e, 2.0);
    let late = e.boost_bar();
    assert!(early < 0.6 * late && late > 0.3 && late < 0.9, "lag: {early} bar after 0.15 s, {late} bar after 2 s");
    assert!(e.turbo_omega * 60.0 / TAU > 2.0 * idle_rpm, "the turbo spools up");
    run(&mut e, 4.0);
    assert!(e.boost_bar() < 0.75, "the wastegate holds the boost at its limit: {} bar (open {})", e.boost_bar(), e.wastegate);
}
#[test] fn lifting_off_a_boosted_engine_blows_off_and_the_boost_falls() {
    let mut e = held(4500.0, 1.0, false);
    run(&mut e, 2.5);
    let boosted = e.boost_bar(); assert!(boosted > 0.35, "{boosted}");
    e.controls.throttle = 0.0;
    let mut bov = 0.0f64; for _ in 0..50 { run(&mut e, 0.01); bov = bov.max(e.bov_flow); }
    assert!(bov > 1.0, "the blow-off valve opens: {bov} mol/s");
    run(&mut e, 0.6);
    assert!(e.boost_bar() < 0.5 * boosted, "the boost falls: {} from {boosted}", e.boost_bar());
}
#[test] fn the_anti_lag_keeps_the_turbo_spinning_and_bangs_in_the_exhaust_on_a_lift() {
    let lift = |als: bool| {
        let mut e = held(5000.0, 1.0, als);
        run(&mut e, 2.0);
        e.controls.throttle = 0.0;
        let mut peak = 0.0f64; for _ in 0..80 { run(&mut e, 0.01); peak = peak.max(e.exhaust_pressure()); }
        (e.turbo_omega, peak)
    };
    let (w0, p0) = lift(false); let (w1, p1) = lift(true);
    assert!(w1 > 1.15 * w0, "the turbo keeps its speed with anti-lag: {w1} vs {w0}");
    assert!(p1 > 1.5 * p0.max(5e3), "bangs: collector peak {p1} Pa vs {p0}");
}
#[test] fn a_spark_cut_sends_fuel_to_the_exhaust_that_burns_there() {
    let burned = |cut: bool| {
        let mut e = held(5500.0, 1.0, false);
        run(&mut e, 1.5);
        let before = e.exhaust_burned;
        e.controls.cut = cut;
        run(&mut e, 0.15);
        e.exhaust_burned - before
    };
    let (normal, cut) = (burned(false), burned(true));
    assert!(cut > 20.0 * normal.max(1e-9), "a cut pops: {cut} mol of fuel burnt in the exhaust vs {normal}");
}
