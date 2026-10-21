use super::*;

const R: f32 = 0.326;
/// rpm per unit of ratio per m/s at the Peugeot's wheel radius.
fn sit(gear: i32, speed: f32, gas: f32) -> Situation {
    Situation { gear, rpm: rpm_in_gear(speed, R, gear), gas, rate: 0.0, brake: false, speed, wheel_r: R, gearing: 1.0, lat_g: 0.0, slope: 0.0 }
}
fn mem(load: f32) -> Memory { Memory { load, ..Memory::default() } }

#[test] fn a_lift_never_upshifts() {
    let s = sit(3, 25.0, 0.0);                        // 5900 rpm, the pedal up
    assert_eq!(choose(Profile::Sport, &mem(0.0), &s), Choice::Hold);
}
#[test] fn the_upshift_point_grows_with_the_remembered_pedal() {
    // gear 2 at about 5000 rpm: a light pedal shifts, a floored one holds for the power
    let v = 5000.0 / (rpm_in_gear(1.0, R, 2));
    assert_eq!(choose(Profile::Sport, &mem(0.3), &sit(2, v, 0.3)), Choice::Up(3));
    assert_eq!(choose(Profile::Sport, &mem(1.0), &sit(2, v, 1.0)), Choice::Hold);
}
#[test] fn the_profiles_differ_in_when_they_climb() {
    let v = 5000.0 / rpm_in_gear(1.0, R, 2);
    let s = sit(2, v, 0.3);
    assert_eq!(choose(Profile::Comfort, &mem(0.3), &s), Choice::Up(3));
    assert_eq!(choose(Profile::Sport, &mem(0.3), &s), Choice::Up(3));
    assert_eq!(choose(Profile::Race, &mem(0.3), &s), Choice::Hold, "race holds the gear for the power");
}
#[test] fn a_floored_pedal_in_a_high_gear_skips_to_the_gear_with_the_most_force() {
    let s = sit(6, 22.0, 1.0);                        // about 3000 rpm in sixth
    assert_eq!(choose(Profile::Sport, &mem(1.0), &s), Choice::Down(3));
}
#[test] fn a_kick_down_does_not_pick_a_gear_that_overreves() {
    let s = sit(6, 24.0, 1.0);                        // 3200 rpm in sixth: second would be 7500 rpm, over the limit, so it stops at third
    match choose(Profile::Sport, &mem(1.0), &s) { Choice::Down(g) => { assert_eq!(g, 3); assert!(rpm_in_gear(24.0, R, g) < 6600.0); }, c => panic!("{c:?}") }
}
#[test] fn braking_steps_down_to_the_band_of_the_engine_brake_and_never_up() {
    let mut s = sit(6, 23.0, 0.0); s.brake = true;    // 3100 rpm
    assert_eq!(choose(Profile::Sport, &mem(0.0), &s), Choice::Down(4));
    let mut up = sit(2, 14.0, 0.0); up.brake = true;  // high revs, braking: no up-shift
    assert_eq!(choose(Profile::Sport, &mem(0.0), &up), Choice::Hold);
}
#[test] fn cornering_holds_the_gear() {
    let v = 5000.0 / rpm_in_gear(1.0, R, 2);
    let mut s = sit(2, v, 0.3); s.lat_g = 0.5;
    assert_eq!(choose(Profile::Sport, &mem(0.3), &s), Choice::Hold);
}
#[test] fn an_uphill_grade_raises_the_floor() {
    let v = 13.55;                                    // fourth at about 2600 rpm
    let mut s = sit(4, v, 0.3);
    assert_eq!(choose(Profile::Sport, &mem(0.3), &s), Choice::Hold);
    s.slope = 0.15;
    assert_eq!(choose(Profile::Sport, &mem(0.3), &s), Choice::Down(3));
}
#[test] fn a_downhill_with_the_pedal_nearly_up_does_not_climb() {
    let v = 5000.0 / rpm_in_gear(1.0, R, 2);
    let mut s = sit(2, v, 0.2); s.slope = -0.1;
    assert_eq!(choose(Profile::Sport, &mem(0.2), &s), Choice::Hold);
}
#[test] fn the_dwell_stops_the_box_from_hunting() {
    let v = 5000.0 / rpm_in_gear(1.0, R, 2);
    let mut m = mem(0.3); m.since_shift = 0.2;
    assert_eq!(choose(Profile::Sport, &m, &sit(2, v, 0.3)), Choice::Hold);
    m.since_shift = 1.0;
    assert_eq!(choose(Profile::Sport, &m, &sit(2, v, 0.3)), Choice::Up(3));
}
#[test] fn lugging_below_the_floor_steps_down_even_inside_the_dwell() {
    let mut m = mem(0.0); m.since_shift = 0.1;
    let s = Situation { rpm: 1200.0, ..sit(3, 6.0, 0.1) };
    assert_eq!(choose(Profile::Sport, &m, &s), Choice::Down(2));
}
#[test] fn memory_remembers_a_press_at_once_and_forgets_a_lift_slowly() {
    let mut m = Memory::default();
    m.update(1.0 / 60.0, 1.0, false, 0.0);
    assert!(m.load > 0.99 && m.aggr > 0.5, "{m:?}");
    for _ in 0..60 { m.update(1.0 / 60.0, 0.0, false, 0.0); }
    assert!(m.load > 0.3 && m.load < 0.6, "after a second: {}", m.load);
    for _ in 0..60 * 30 { m.update(1.0 / 60.0, 0.0, false, 0.0); }
    assert!(m.load < 0.01 && m.aggr < 0.05, "{m:?}");
}
#[test] fn a_stab_counts_more_than_a_slow_press() {
    let mut stab = Memory::default(); let mut slow = Memory::default();
    stab.update(1.0 / 60.0, 0.6, false, 0.0); stab.update(1.0 / 60.0, 0.6, false, 0.0);
    for k in 0..60 { slow.update(1.0 / 60.0, 0.6 * k as f32 / 60.0, false, 0.0); }
    assert!(stab.aggr > slow.aggr + 0.1, "stab {} slow {}", stab.aggr, slow.aggr);
}
#[test] fn garbage_in_holds_the_gear() {
    let m = Memory::default();
    for &(rpm, speed, gas) in &[(f32::NAN, 10.0, 0.5), (3000.0, f32::INFINITY, 0.5), (-5.0, -20.0, 2.0), (1e9, 1e9, f32::NAN), (0.0, 0.0, 0.0)] {
        for g in -1..=7 {
            let s = Situation { gear: g, rpm, gas, rate: f32::NAN, brake: true, speed, wheel_r: R, gearing: 1.0, lat_g: f32::NAN, slope: f32::NAN };
            match choose(Profile::Sport, &m, &s) { Choice::Hold => {}, Choice::Up(t) | Choice::Down(t) => assert!((1..=6).contains(&t) && (t - g).abs() >= 1, "{g} -> {t} for {rpm} {speed} {gas}") }
        }
    }
}
