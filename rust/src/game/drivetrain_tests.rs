use super::*;

const R: f32 = 0.326;
/// A bare car: the drivetrain's force against the drag and the rolling resistance of a 1230 kg car; returns the speed.
struct Car { dt: Drivetrain, v: f32, t: f32 }
impl Car {
    fn new() -> Car { Car { dt: Drivetrain::new(), v: 0.0, t: 0.0 } }
    fn step(&mut self, thr: f32) -> DriveOut {
        let o = self.dt.step(1.0 / 60.0, &DriveIn { throttle: thr, speed: self.v, wheel_r: R, airborne: false });
        let drag = 0.5 * 1.2 * 0.62 * 2.1 * self.v * self.v.abs() + 160.0 * self.v.signum().min(1.0).max(-1.0) * (self.v.abs() / (self.v.abs() + 0.3));
        let grip = 1.2 * 1230.0 * 9.81;
        self.v += (o.force.clamp(-grip, grip) - drag) / 1230.0 / 60.0; self.t += 1.0 / 60.0;
        o
    }
    fn run(&mut self, thr: f32, secs: f32) -> DriveOut { let mut o = self.dt.last; for _ in 0..(secs * 60.0) as usize { o = self.step(thr); } o }
}

#[test] fn the_torque_map_is_a_turbocharged_two_litre() {
    assert!(engine_torque(5000.0, 1.0) > 300.0 && engine_torque(5000.0, 1.0) < 380.0, "{}", engine_torque(5000.0, 1.0));
    assert!(engine_torque(1500.0, 1.0) < engine_torque(4500.0, 1.0), "the torque builds with the boost");
    assert!(engine_torque(3000.0, 0.0) < -10.0, "the engine brakes with the throttle shut: {}", engine_torque(3000.0, 0.0));
    assert!(engine_torque(4000.0, 0.25) < engine_torque(4000.0, 0.5) && engine_torque(4000.0, 0.5) < engine_torque(4000.0, 1.0));
    let hp = engine_torque(6000.0, 1.0) * 6000.0 * std::f32::consts::TAU / 60.0 / 745.7; assert!(hp > 250.0 && hp < 450.0, "the power: {hp} hp");
    assert!(engine_torque(0.0, 1.0).abs() < 1.0, "nothing at a standstill");
}
#[test] fn the_ratios_and_the_revs_match_the_gearbox_of_the_sound() {
    assert!((rpm_in_gear(27.8, R, 4) - 27.8 / R * 1.31 * 5.0 * 9.549).abs() < 5.0);
    assert!(rpm_in_gear(55.0, R, 6) < LIMITER_RPM + 100.0);
}
#[test] fn the_engine_idles_and_the_automatic_creeps_gently() {
    let mut c = Car::new();
    let o = c.run(0.0, 5.0);
    assert!((o.rpm - IDLE_RPM).abs() < 250.0 && !o.stalled, "idle {}", o.rpm);
    assert!(c.v >= 0.0 && c.v < 2.5, "a gentle creep: {} m/s", c.v);
}
#[test] fn the_automatic_launches_shifts_up_through_the_gears_and_stays_under_the_limiter() {
    let mut c = Car::new(); let (mut gears, mut top, mut t100) = (vec![1], 0.0f32, None);
    for _ in 0..60 * 30 {
        let o = c.step(1.0);
        top = top.max(o.rpm);
        if gears.last() != Some(&o.gear) { gears.push(o.gear); }
        if t100.is_none() && c.v * 3.6 >= 100.0 { t100 = Some(c.t); }
    }
    assert_eq!(&gears[..6], &[1, 2, 3, 4, 5, 6], "{gears:?}");
    let t = t100.expect("it reaches 100 km/h"); assert!(t > 3.0 && t < 8.0, "0-100 km/h in {t} s");
    assert!(top < LIMITER_RPM + 250.0, "{top}");
    assert!(c.v * 3.6 > 170.0 && c.v * 3.6 < 260.0, "the top speed {} km/h", c.v * 3.6);
}
#[test] fn the_launch_force_is_that_of_a_rally_car_and_a_lift_brakes_with_the_engine() {
    let mut c = Car::new(); let mut peak = 0.0f32;
    for _ in 0..180 { peak = peak.max(c.step(1.0).force); }
    assert!(peak > 9000.0 && peak < 26000.0, "the force at the launch: {peak} N");
    let mut c = Car::new(); c.run(1.0, 12.0);
    let rpm = c.dt.last.rpm; let f = c.run(0.0, 0.6).force;
    assert!(rpm > 3000.0 && f < -300.0, "a lift at {rpm} rpm: the engine brakes with {f} N");
}
#[test] fn the_manual_has_neutral_and_the_engine_revs_free_to_the_limiter() {
    let mut c = Car::new(); c.dt.set_mode(Mode::Manual); c.dt.reset();
    assert_eq!(c.dt.gear, 0);
    let o = c.run(1.0, 3.0);
    assert!(o.rpm > LIMITER_RPM - 400.0 && o.rpm < LIMITER_RPM + 300.0 && o.force.abs() < 1.0, "{} rpm, {} N", o.rpm, o.force);
    assert!(c.run(1.0, 1.0).cut || c.dt.last.rpm < LIMITER_RPM + 300.0, "the limiter holds it");
}
#[test] fn the_manual_stalls_when_the_clutch_is_let_go_in_gear_at_a_standstill_and_restarts() {
    let mut c = Car::new(); c.dt.set_mode(Mode::Manual); c.dt.reset();
    c.run(0.0, 2.0);
    c.dt.shift(1); c.run(0.0, 0.3); assert_eq!(c.dt.gear, 1);
    c.dt.set_clutch(0.0);              // the pedal up: the clutch closed, the wheels do not turn
    let o = c.run(0.0, 2.0);
    assert!(o.stalled && o.rpm < 300.0, "stalled: {} rpm", o.rpm);
    c.dt.set_clutch(1.0);              // the pedal down: the engine can start
    let o = c.run(0.0, 4.0);
    assert!(!o.stalled && o.rpm > 700.0, "restarted: {} rpm, stalled {}", o.rpm, o.stalled);
}
#[test] fn a_slipping_clutch_pulls_away_in_the_manual_and_a_full_release_follows() {
    let mut c = Car::new(); c.dt.set_mode(Mode::Manual); c.dt.reset();
    c.run(0.0, 1.5); c.dt.set_clutch(1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.dt.set_clutch(0.55);
    let o = c.run(0.5, 3.0);
    assert!(c.v > 3.0 && !o.stalled, "pulling away: {} m/s, {} rpm", c.v, o.rpm);
    c.dt.set_clutch(0.0); let o = c.run(0.5, 2.0);
    assert!(c.v > 6.0 && !o.stalled && o.grip > 0.5, "clutch up: {} m/s", c.v);
}
#[test] fn manual_shifts_go_through_neutral_to_the_reverse_cut_the_spark_and_take_a_moment() {
    let mut c = Car::new(); c.dt.set_mode(Mode::Manual); c.dt.reset();
    c.dt.shift(-1); assert_eq!(c.dt.gear, -1); c.run(0.0, 0.2);
    c.dt.shift(1); c.run(0.0, 0.2); assert_eq!(c.dt.gear, 0); c.run(0.0, 0.2);
    for k in 1..=6 { c.dt.shift(1); c.run(0.0, 0.2); assert_eq!(c.dt.gear, k.min(6)); }
    c.dt.shift(1); assert_eq!(c.dt.gear, 6, "no seventh gear");
    // a request in the middle of a shift is ignored
    c.dt.shift(-1); c.dt.shift(-1); assert_eq!(c.dt.gear, 5);
    // a hard up-shift cuts the spark
    let mut d = Car::new(); d.dt.set_mode(Mode::Manual); d.dt.reset(); d.dt.shift(1); d.run(0.0, 0.2); d.dt.shift(1); d.run(0.0, 0.2); d.dt.set_clutch(0.5);
    d.run(1.0, 0.5); d.dt.shift(1); let o = d.step(1.0); assert!(o.cut, "the flat shift");
}
#[test] fn the_reverse_goes_backwards_in_the_automatic_and_stays_finite_through_abuse() {
    let mut c = Car::new();
    c.run(0.0, 1.0); c.run(-1.0, 3.0);
    assert!(c.v < -1.0 && c.dt.gear == -1, "backing: {} m/s gear {}", c.v, c.dt.gear);
    let mut a = Drivetrain::new(); let mut seed = 3u32;
    for k in 0..6000 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); let r = (seed >> 8) as f32 / 16_777_216.0;
        if k % 97 == 0 { a.shift(if r > 0.5 { 1 } else { -1 }); a.set_clutch(r); if k % 300 == 0 { a.set_mode(if r > 0.5 { Mode::Auto } else { Mode::Manual }); } }
        let o = a.step(1.0 / 60.0, &DriveIn { throttle: r * 2.0 - 1.0, speed: (r - 0.3) * 80.0, wheel_r: R, airborne: k % 500 < 20 });
        assert!(o.force.is_finite() && o.rpm.is_finite() && o.rpm >= 0.0 && o.rpm < 12000.0 && o.torque.is_finite(), "step {k}: {o:?}");
    }
}
