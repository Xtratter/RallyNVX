use super::*;

const R: f32 = 0.326;
/// A bare car: the drivetrain's force against the drag and the rolling resistance of a 1230 kg car; returns the speed.
struct Car { dt: Drivetrain, v: f32, t: f32, brake: bool }
impl Car {
    fn new() -> Car { Car { dt: Drivetrain::new(), v: 0.0, t: 0.0, brake: false } }
    fn h_manual() -> Car { let mut c = Car::new(); c.dt.box_type = BoxType::HPattern; c.dt.set_mode(Mode::Manual); c.dt.reset(); c }
    fn step(&mut self, thr: f32) -> DriveOut {
        let o = self.dt.step(1.0 / 60.0, &DriveIn { throttle: thr, speed: self.v, wheel_r: R, brake: self.brake, ..Default::default() });
        if self.brake && self.v > 0.0 { self.v = (self.v - 8.0 / 60.0).max(0.0); }   // the brakes: 8 m/s^2
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
    let mut c = Car::h_manual();
    assert_eq!(c.dt.gear, 0);
    let o = c.run(1.0, 3.0);
    assert!(o.rpm > LIMITER_RPM - 400.0 && o.rpm < LIMITER_RPM + 300.0 && o.force.abs() < 1.0, "{} rpm, {} N", o.rpm, o.force);
    assert!(c.run(1.0, 1.0).cut || c.dt.last.rpm < LIMITER_RPM + 300.0, "the limiter holds it");
}
#[test] fn the_manual_stalls_when_the_clutch_is_let_go_in_gear_at_a_standstill_and_restarts() {
    let mut c = Car::h_manual();
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
    let mut c = Car::h_manual();
    c.run(0.0, 1.5); c.dt.set_clutch(1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.dt.set_clutch(0.55);
    let o = c.run(0.5, 3.0);
    assert!(c.v > 3.0 && !o.stalled, "pulling away: {} m/s, {} rpm", c.v, o.rpm);
    c.dt.set_clutch(0.0); let o = c.run(0.5, 2.0);
    assert!(c.v > 6.0 && !o.stalled && o.grip > 0.5, "clutch up: {} m/s", c.v);
}
#[test] fn manual_shifts_go_through_neutral_to_the_reverse_cut_the_spark_and_take_a_moment() {
    let mut c = Car::h_manual();
    c.dt.shift(-1); assert_eq!(c.dt.gear, -1); c.run(0.0, 0.25);
    c.dt.shift(1); c.run(0.0, 0.25); assert_eq!(c.dt.gear, 0); c.run(0.0, 0.25);
    for k in 1..=6 { c.dt.shift(1); c.run(0.0, 0.25); assert_eq!(c.dt.gear, k.min(6)); }
    c.dt.shift(1); assert_eq!(c.dt.gear, 6, "no seventh gear");
    // a request in the middle of a shift is ignored
    c.dt.shift(-1); c.dt.shift(-1); assert_eq!(c.dt.gear, 5);
    // a hard up-shift cuts the spark
    let mut d = Car::h_manual(); d.dt.shift(1); d.run(0.0, 0.25); d.dt.shift(1); d.run(0.0, 0.25); d.dt.set_clutch(0.5);
    d.run(1.0, 0.5); d.dt.shift(1); let o = d.step(1.0); assert!(o.cut, "the flat shift");
}
#[test] fn the_reverse_goes_backwards_in_the_automatic_and_stays_finite_through_abuse() {
    let mut c = Car::new(); c.dt.auto_reverse = true;   // the autopilot asks for the reverse with a negative throttle
    c.run(0.0, 1.0); c.run(-1.0, 3.0);
    assert!(c.v < -1.0 && c.dt.gear == -1, "backing: {} m/s gear {}", c.v, c.dt.gear);
    let mut a = Drivetrain::new(); let mut seed = 3u32;
    for k in 0..6000 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); let r = (seed >> 8) as f32 / 16_777_216.0;
        if k % 97 == 0 { a.shift(if r > 0.5 { 1 } else { -1 }); a.set_clutch(r); if k % 300 == 0 { a.set_mode(if r > 0.5 { Mode::Auto } else { Mode::Manual }); } }
        let o = a.step(1.0 / 60.0, &DriveIn { throttle: r * 2.0 - 1.0, speed: (r - 0.3) * 80.0, wheel_r: R, airborne: k % 500 < 20, brake: k % 7 == 0, lat_g: r - 0.5, slope: r - 0.5 });
        assert!(o.force.is_finite() && o.rpm.is_finite() && o.rpm >= 0.0 && o.rpm < 12000.0 && o.torque.is_finite(), "step {k}: {o:?}");
    }
}

#[test] fn the_automatic_has_a_reverse_selector_and_a_car_rolling_back_pulls_forward_when_the_gas_is_pressed() {
    // R chosen: the gas drives backwards from a standstill
    let mut c = Car::new(); c.run(0.0, 1.0); c.dt.reverse_sel = true;
    c.run(0.6, 4.0);
    assert!(c.v < -2.0 && c.dt.gear == -1, "R: backing with the gas: {} m/s, gear {}", c.v, c.dt.gear);
    // D chosen while the car rolls backwards: the gas takes the first gear at once and the force is forward
    let mut d = Car::new(); d.dt.reverse_sel = false; d.v = -2.5; d.run(0.0, 0.1);
    let o = d.run(0.8, 0.3);
    assert!(o.gear >= 1 && o.force > 500.0, "D while rolling back: gear {} force {} N", o.gear, o.force);
    assert!(d.v > -2.5, "the car slows its roll back: {}", d.v);
    // D and no gas while rolling back: it stays in reverse until it stops (no lurch), and the negative throttle still reverses at rest
    let mut e = Car::new(); e.dt.auto_reverse = true; e.run(0.0, 0.5); e.run(-1.0, 3.0); assert_eq!(e.dt.gear, -1);
    // the driver in D never gets the reverse by himself: a negative throttle at a standstill does nothing
    let mut f = Car::new(); f.run(0.0, 0.5); f.run(-1.0, 3.0);
    assert!(f.dt.gear >= 1 && f.v >= -0.1, "D stays in drive: gear {} v {}", f.dt.gear, f.v);
}

#[test] fn a_lift_in_the_automatic_keeps_the_gear_and_a_floored_pedal_kicks_down() {
    // a lift at speed must not shift up, the box holds the gear it was in
    let mut c = Car::new(); c.run(1.0, 6.0);
    c.run(0.0, 0.3);
    let (g, rpm) = (c.dt.gear, c.dt.last.rpm);
    assert!(g >= 3 && rpm > 3500.0, "gear {g} at {rpm} rpm");
    c.run(0.0, 2.0); assert!(c.dt.gear <= g, "a lift upshifted: {g} -> {}", c.dt.gear);
    // a cruise at light gas in a high gear, then the floor: it drops a gear within half a second
    let mut d = Car::new(); d.v = 22.0; d.dt.gear = 6; d.run(0.12, 6.0);
    let g = d.dt.gear; assert!(g >= 4 && d.dt.last.rpm < 3300.0, "cruising in gear {g} at {} rpm", d.dt.last.rpm);
    d.run(1.0, 0.5); assert!(d.dt.gear < g, "kick-down: gear {g} -> {}", d.dt.gear);
}
#[test] fn the_automatic_does_not_hunt_under_a_wandering_pedal() {
    let mut c = Car::new(); c.run(1.0, 4.0);
    let (mut last_gear, mut last_shift, mut shifts, mut seed) = (c.dt.gear, c.t, 0, 7u32);
    let mut pedal = 1.0f32;
    for k in 0..60 * 90 {
        if k % 240 == 0 { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); pedal = [0.0, 0.3, 0.6, 1.0][(seed >> 24) as usize % 4]; c.brake = pedal == 0.0 && (seed >> 16) % 2 == 0; }
        let thr = (pedal + if k % 5 == 0 { 0.08 } else { -0.05 }).clamp(0.0, 1.0);
        let o = c.step(thr);
        if o.gear != last_gear {
            assert!(c.t - last_shift > 0.3, "two shifts {} s apart (gear {last_gear} -> {})", c.t - last_shift, o.gear);
            last_gear = o.gear; last_shift = c.t; shifts += 1;
        }
    }
    assert!(shifts > 3 && shifts < 70, "{shifts} shifts in 90 s");
}
#[test] fn braking_takes_the_automatic_down_through_the_gears() {
    let mut c = Car::new(); c.run(1.0, 25.0); c.run(0.0, 0.3);
    let g = c.dt.gear; assert!(g >= 5 && c.v > 30.0, "gear {g} at {} m/s", c.v);
    c.brake = true; c.run(0.0, 3.5);
    assert!(c.dt.gear < g - 1, "the engine brake: gear {g} -> {} at {} m/s", c.dt.gear, c.v);
    assert!(c.dt.last.rpm < 6700.0, "{}", c.dt.last.rpm);
}
#[test] fn a_shift_of_the_automatic_meets_the_new_gear_with_matching_revs() {
    let mut c = Car::new(); let mut g0 = c.dt.gear; let mut n = 0;
    for _ in 0..60 * 20 {
        let before = c.dt.last.rpm; c.step(1.0);
        if c.dt.gear > g0 {
            let from = c.dt.gear; for _ in 0..21 { c.step(1.0); }
            let want = rpm_in_gear(c.v, R, from); let got = c.dt.last.rpm;
            assert!((got - want).abs() < 450.0 || c.dt.gear != from, "after the shift to {from} (from {before} rpm): {got} rpm, the wheels ask {want}");
            n += 1;
        }
        g0 = c.dt.gear;
    }
    assert!(n >= 4, "{n} shifts seen");
}

#[test] fn a_stab_in_a_high_gear_kicks_down_several_gears() {
    let mut d = Car::new(); d.v = 22.0; d.dt.gear = 6; d.run(0.12, 6.0);
    let g = d.dt.gear; assert!(g >= 5, "cruising in {g}");
    d.run(1.0, 0.6);
    assert!(d.dt.gear <= g - 2, "kick-down skipped to {} from {g}", d.dt.gear);
}
#[test] fn race_holds_its_gears_longer_than_comfort() {
    let run = |p: Profile| { let mut c = Car::new(); c.dt.profile = p; c.run(1.0, 3.0); (c.dt.gear, c.dt.last.rpm) };
    let (comfort, race) = (run(Profile::Comfort), run(Profile::Race));
    assert!(comfort.0 >= race.0, "comfort {comfort:?} race {race:?}");
}
#[test] fn brake_taps_do_not_hunt() {
    let mut c = Car::new(); c.run(1.0, 20.0); let mut shifts = 0; let mut last = c.dt.gear;
    for k in 0..60 * 12 {
        c.brake = (k / 20) % 2 == 0;                    // 1/3 s on, 1/3 s off
        let o = c.step(if c.brake { 0.0 } else { 0.4 });
        if o.gear != last { shifts += 1; last = o.gear; }
    }
    assert!(shifts <= 8, "{shifts} shifts in 12 s of brake taps");
}
#[test] fn reverse_and_the_selector_are_untouched() {
    let mut c = Car::new(); c.dt.reverse_sel = true; c.run(0.6, 3.0);
    assert_eq!(c.dt.gear, -1);
    c.dt.reverse_sel = false; c.run(0.0, 1.0);
    assert!(c.dt.gear >= 1, "back in drive: {}", c.dt.gear);
}

fn seq_manual() -> Car { let mut c = Car::new(); c.dt.set_mode(Mode::Manual); c.dt.reset(); c }

#[test] fn the_sequential_launches_without_a_clutch_pedal_and_does_not_stall() {
    let mut c = seq_manual(); assert_eq!(c.dt.box_type, BoxType::Sequential);
    c.run(0.0, 1.5); c.dt.shift(1); c.run(0.0, 0.3);
    assert_eq!(c.dt.gear, 1);
    let o = c.run(1.0, 3.0);
    assert!(c.v > 10.0 && !o.stalled, "{} m/s, stalled {}", c.v, o.stalled);
}
#[test] fn the_sequential_idles_in_gear_without_stalling() {
    let mut c = seq_manual(); c.run(0.0, 1.5); c.dt.shift(1); c.run(0.0, 0.3);
    let o = c.run(0.0, 6.0);
    assert!(!o.stalled && o.rpm > 600.0, "idle in gear: {} rpm", o.rpm);
    assert!(c.v < 3.0, "creeps at most: {}", c.v);
}
#[test] fn the_sequential_upshift_cuts_the_spark_briefly_and_is_quick() {
    let mut c = seq_manual(); c.run(0.0, 1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.run(1.0, 2.0);
    c.dt.shift(1); let o = c.step(1.0);
    assert!(o.cut, "flat shift: the spark is cut");
    let mut n = 0; while c.step(1.0).cut && n < 30 { n += 1; }
    assert!(n >= 2 && n <= 8, "the cut lasts {} steps (about 0.08 s)", n);
    assert_eq!(c.dt.gear, 2);
}
#[test] fn the_sequential_downshift_blips_the_throttle() {
    let mut c = seq_manual(); c.run(0.0, 1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.run(1.0, 2.5); c.dt.shift(1); c.run(1.0, 0.3);
    c.run(0.0, 0.5);
    c.dt.shift(-1); let o = c.step(0.0);
    assert!(o.throttle >= 0.55, "the blip: {}", o.throttle);
}
#[test] fn the_h_pattern_shift_takes_twenty_hundredths_and_ignores_a_second_request() {
    let mut c = Car::h_manual(); c.dt.set_clutch(1.0);
    c.dt.shift(1); assert_eq!(c.dt.gear, 1);
    c.run(0.0, 0.1); c.dt.shift(1); assert_eq!(c.dt.gear, 1, "still shifting");
    c.run(0.0, 0.15); c.dt.shift(1); assert_eq!(c.dt.gear, 2);
}
#[test] fn the_clutch_pedal_still_overrides_the_sequential() {
    let mut c = seq_manual(); c.run(0.0, 1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.dt.set_clutch(1.0);                              // the pedal down: the clutch is open whatever the revs
    let o = c.run(1.0, 1.5);
    assert!(c.v < 0.5 && o.rpm > 5000.0, "free revs with the pedal down: {} m/s {} rpm", c.v, o.rpm);
}
#[test] fn switching_the_box_during_a_shift_is_safe() {
    let mut c = seq_manual(); c.run(0.0, 1.0); c.dt.shift(1); c.run(0.0, 0.1);
    for k in 0..600 {
        if k % 17 == 0 { c.dt.box_type = if (k / 17) % 2 == 0 { BoxType::HPattern } else { BoxType::Sequential }; c.dt.shift(if k % 3 == 0 { -1 } else { 1 }); }
        let o = c.step(if k % 5 < 3 { 1.0 } else { 0.0 });
        assert!(o.force.is_finite() && o.rpm.is_finite() && o.rpm >= 0.0 && o.rpm < 12000.0, "step {k}: {o:?}");
    }
}
