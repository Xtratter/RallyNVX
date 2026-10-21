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
        let o = a.step(1.0 / 60.0, &DriveIn { throttle: r * 2.0 - 1.0, speed: (r - 0.3) * 80.0, wheel_r: R, airborne: k % 500 < 20, brake: k % 7 == 0, lat_g: r - 0.5, slope: r - 0.5, tc_slip: r * 3.0, esc_cut: r });
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


#[test] fn traction_control_cuts_the_engine_when_a_wheel_spins_and_gives_it_back() {
    let go = |tc_on: bool, slip: f32| { let mut d = Drivetrain::new(); d.opts.tc = tc_on; let mut o = d.last;
        for _ in 0..180 { o = d.step(1.0 / 60.0, &DriveIn { throttle: 1.0, speed: 8.0, wheel_r: R, tc_slip: slip, ..Default::default() }); } o.wheel_torque };
    let (calm, spin_tc, spin_free) = (go(true, 0.5), go(true, 2.5), go(false, 2.5));
    assert!(spin_tc < calm * 0.5, "cut: {spin_tc} against {calm}");
    assert!(spin_free > calm * 0.9, "TC off does not cut: {spin_free} against {calm}");
}
#[test] fn the_wheel_torque_is_the_force_times_the_radius() {
    let mut c = Car::new(); let o = c.run(1.0, 2.0);
    assert!((o.wheel_torque - o.force * R).abs() < 1.0, "{} against {}", o.wheel_torque, o.force * R);
}
#[test] fn reverse_traction_control_cuts_spin_too() {
    let run = |slip: f32| { let mut d = Drivetrain::new(); d.reverse_sel = true; let mut o = d.last;
        for _ in 0..180 { o = d.step(1.0 / 60.0, &DriveIn { throttle: -1.0, speed: -3.0, wheel_r: R, tc_slip: slip, ..Default::default() }); } o.wheel_torque };
    let (spin, calm) = (run(2.5), run(0.0));
    assert!(spin < 0.0 && calm < 0.0 && spin.abs() < calm.abs() * 0.5, "{spin} against {calm}");
}
/// Wheels as a plain inertia with a given tyre torque against them, driven by the coupled substeps for `secs`; returns (the wheel speeds' history, the torques).
fn spin_up(d: &mut Drivetrain, i_wheels: f32, tyre_of: impl Fn(f32) -> f32, secs: f32, w0: f32) -> (Vec<f32>, Vec<f32>) {
    let h = 1.0 / 240.0; let (mut w, mut ws, mut ts) = (w0, Vec::new(), Vec::new());
    for k in 0..(secs / h) as usize {
        if k % 4 == 0 { d.begin(4.0 * h, &DriveIn { throttle: 1.0, speed: w * R, wheel_r: R, ..Default::default() }); }
        let t = d.sub_coupled(h, w, i_wheels, tyre_of(w));
        w += h * (t - tyre_of(w)) / i_wheels;
        ws.push(w); ts.push(t);
        if k % 4 == 3 { d.finish(); }
    }
    (ws, ts)
}
#[test] fn a_wheel_that_spins_freely_does_not_make_the_clutch_chatter() {
    // no tyre torque at all (ice): the engine and the wheels accelerate together; the torque must be smooth, not alternate between its limits
    let mut d = Drivetrain::new();
    let (ws, ts) = spin_up(&mut d, 5.2, |_| 0.0, 3.0, 8.0);
    assert!(ws.iter().all(|w| w.is_finite()) && ws.last().unwrap() > &ws[0], "{:?}", &ws[ws.len() - 3..]);
    // chatter is a saw-tooth: big steps of the torque that alternate in sign; a re-engagement after a shift or a change of gear is a step followed by a smooth run
    let dt_: Vec<f32> = ts.windows(2).skip(240).map(|p| p[1] - p[0]).collect();
    let flips = dt_.windows(2).filter(|p| p[0].abs() > 100.0 && p[1].abs() > 100.0 && p[0] * p[1] < 0.0).count();
    assert_eq!(flips, 0, "the torque saws {flips} times");
}
#[test] fn heavy_wheels_make_the_coupled_engine_agree_with_the_kinematic_one() {
    let w = 20.0 / R;                                   // 20 m/s in third: about 4700 rpm
    let mut a = Drivetrain::new(); a.gear = 3; let mut b = a.clone();
    let mut oa = a.last;
    for _ in 0..90 { oa = a.step(1.0 / 60.0, &DriveIn { throttle: 0.6, speed: 20.0, wheel_r: R, ..Default::default() }); }
    for _ in 0..90 { b.begin(1.0 / 60.0, &DriveIn { throttle: 0.6, speed: 20.0, wheel_r: R, ..Default::default() }); for _ in 0..4 { b.sub_coupled(1.0 / 240.0, w, 1e7, 0.0); } b.finish(); }
    assert!((a.rpm() / b.rpm() - 1.0).abs() < 0.03 && oa.gear == b.last.gear, "{} against {} rpm", a.rpm(), b.rpm());
}

#[test] fn launch_control_holds_the_revs_with_the_brake_and_the_gas_and_launches_when_the_brake_is_released() {
    let go = |on: bool| {
        let mut c = Car::new(); c.dt.opts.launch = on;
        c.run(0.0, 1.0);
        c.brake = true;
        let held = c.run(1.0, 2.0).rpm;                  // full gas with the brake down at a standstill
        c.brake = false;
        let after = c.run(1.0, 3.0);
        (held, after.rpm, c.v)
    };
    let (held, _, v_on) = go(true);
    let (free, _, _) = go(false);
    assert!(held > 4200.0 && held < 5400.0, "held at the launch revs: {held}");
    assert!(free > 6500.0 || free < 1500.0 || (free - held).abs() > 300.0, "without it the revs are not held there: {free}");
    assert!(v_on > 12.0, "the launch pulls away: {v_on} m/s after 3 s");
}
#[test] fn launch_control_is_for_the_standstill_with_the_brake_and_the_gas_only() {
    // moving, or without the brake, or with a light pedal: the throttle goes through as it is
    let mut c = Car::new(); c.dt.opts.launch = true; c.run(1.0, 8.0);
    assert!(c.dt.last.rpm > 5500.0 || c.v > 15.0, "a normal launch is not held: {} rpm {} m/s", c.dt.last.rpm, c.v);
    let mut d = Car::new(); d.dt.opts.launch = true; d.brake = true; d.run(0.4, 2.0);
    assert!(d.dt.last.throttle >= 0.35, "a light pedal with the brake is not limited to the launch revs: {}", d.dt.last.throttle);
}

#[test] fn a_clutch_feathered_for_long_in_the_h_pattern_heats_and_loses_grip_but_the_automatic_does_not_burn() {
    let mut c = Car::h_manual(); c.run(0.0, 1.5); c.dt.set_clutch(1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.brake = true; c.dt.set_clutch(0.7);             // the clutch nearly out (it carries about 100 N m), the brake holding the car, the gas floored: it slips
    c.run(1.0, 10.0);
    assert!(c.dt.clutch.temp > 150.0, "10 s of slip: {}", c.dt.clutch.temp);
    let held = c.dt.last.grip;
    assert!(held < 1.0, "the grip fades: {held}");
    let mut a = Car::new(); for _ in 0..6 { a.brake = false; a.run(1.0, 2.0); a.brake = true; a.run(0.0, 2.0); }
    assert!(a.dt.clutch.temp < 60.0, "the automatic's converter does not burn the clutch: {}", a.dt.clutch.temp);
}
#[test] fn the_coolant_heats_when_the_engine_is_revved_for_long_without_the_air_and_an_overheated_engine_loses_torque() {
    let after = |gas: f32| { let mut c = Car::h_manual(); c.run(0.0, 1.0); c.run(gas, 300.0); c.dt.coolant.temp };   // neutral, the car standing: no air through the radiator
    let (idle, revved) = (after(0.0), after(1.0));
    assert!(revved > idle + 5.0, "5 minutes at the limiter against idling: {revved} against {idle}");
    let torque = |t: f32| { let mut e = Drivetrain::new(); e.gear = 3; e.coolant.temp = t; let mut o = e.last; for _ in 0..60 { o = e.step(1.0 / 60.0, &DriveIn { throttle: 1.0, speed: 15.0, wheel_r: R, ..Default::default() }); } o.torque };
    assert!(torque(128.0) < torque(90.0) * 0.7, "{} against {}", torque(128.0), torque(90.0));
}
#[test] fn the_damage_of_the_engine_takes_power_through_the_power_factor() {
    let torque = |k: f32| { let mut e = Drivetrain::new(); e.gear = 3; e.power_k = k; let mut o = e.last; for _ in 0..60 { o = e.step(1.0 / 60.0, &DriveIn { throttle: 1.0, speed: 15.0, wheel_r: R, ..Default::default() }); } o.torque };
    assert!((torque(0.6) / torque(1.0) - 0.6).abs() < 0.05, "{} against {}", torque(0.6), torque(1.0));
}

#[test] fn the_engine_burns_fuel_by_its_work_and_an_empty_tank_stops_it_for_good() {
    let burnt = |gas: f32| { let mut c = Car::new(); let l0 = c.dt.fuel.litres; c.run(gas, 20.0); l0 - c.dt.fuel.litres };
    let (idle, hard) = (burnt(0.0), burnt(1.0));
    assert!(idle > 0.0 && idle < 0.05 && hard > idle * 8.0, "idle {idle} l, 20 s of full throttle {hard} l");
    let mut c = Car::new(); c.dt.fuel = crate::game::fuel::Fuel::with(0.0); c.run(0.5, 6.0);
    assert!(c.dt.last.stalled && c.dt.rpm() < 300.0, "an empty tank: {} rpm", c.dt.rpm());
    c.run(0.0, 6.0); assert!(c.dt.last.stalled, "it does not restart without fuel");
    c.dt.fuel = crate::game::fuel::Fuel::with(10.0); c.run(0.0, 6.0);
    assert!(!c.dt.last.stalled && c.dt.rpm() > 600.0, "with fuel it starts again: {}", c.dt.rpm());
}
