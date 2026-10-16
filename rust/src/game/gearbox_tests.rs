use super::*;

fn drive(g: &mut Gearbox, speed: f32, thr: f32, secs: f32) -> Drive { let mut d = Drive { rpm: 0.0, clutch: 0.0, gear: 1, cut: false, blip: 0.0 }; for _ in 0..(secs * 60.0) as usize { d = g.update(1.0 / 60.0, speed, thr, 0.326, false); } d }

#[test] fn the_revs_follow_the_speed_and_the_gear() {
    let r = 0.326;
    // 100 km/h in 4th: 27.8 m/s, 1.31 x 5.0 over a 0.326 m wheel
    assert!((rpm_in_gear(27.8, r, 4) - 27.8 / r * 1.31 * 5.0 * 9.549).abs() < 5.0);
    assert!(rpm_in_gear(20.0, r, 2) > rpm_in_gear(20.0, r, 3) && rpm_in_gear(20.0, r, 3) > rpm_in_gear(20.0, r, 6));
    assert!(rpm_in_gear(55.0, r, 6) < 7800.0, "the top gear does not pass the limiter at the top speed: {}", rpm_in_gear(55.0, r, 6));
}
#[test] fn accelerating_shifts_up_through_the_gears_at_the_top_of_the_revs_and_never_over_the_limiter() {
    let mut g = Gearbox::default(); let mut speed = 0.0f32; let mut gears = vec![1]; let mut top = 0.0f32;
    for _ in 0..60 * 40 {
        let d = g.update(1.0 / 60.0, speed, 1.0, 0.326, false);
        speed += 7.0 / 60.0 * (1.0 - speed / 52.0);
        top = top.max(d.rpm);
        if gears.last() != Some(&d.gear) { gears.push(d.gear); }
    }
    assert_eq!(&gears[..6], &[1, 2, 3, 4, 5, 6], "{gears:?}");
    assert!(top < 7700.0, "the shift comes before the limiter: {top}");
}
#[test] fn a_light_foot_shifts_earlier_and_slowing_down_shifts_back() {
    let (mut hard, mut light) = (Gearbox::default(), Gearbox::default());
    let (a, b) = (drive(&mut hard, 12.0, 1.0, 1.0), drive(&mut light, 12.0, 0.2, 1.0));
    assert!(b.gear >= a.gear, "light {} hard {}", b.gear, a.gear);
    let mut g = Gearbox::default(); drive(&mut g, 40.0, 0.5, 2.0);
    let high = g.gear; drive(&mut g, 8.0, 0.0, 3.0);
    assert!(g.gear < high && g.gear >= 1, "{} then {}", high, g.gear);
}
#[test] fn the_clutch_opens_for_a_shift_in_the_air_and_at_rest_and_slips_when_pulling_away() {
    let mut g = Gearbox::default();
    assert!(drive(&mut g, 0.0, 0.0, 1.0).clutch < 0.05, "idling at rest");
    let slip = drive(&mut g, 2.0, 1.0, 1.0); assert!(slip.clutch > 0.3 && slip.clutch < 0.9, "pulling away slips: {}", slip.clutch);
    assert!(drive(&mut g, 14.0, 0.6, 1.0).clutch > 0.95, "locked at speed");
    let mut a = Gearbox::default(); for _ in 0..60 { a.update(1.0 / 60.0, 20.0, 0.5, 0.326, false); }
    for _ in 0..30 { a.update(1.0 / 60.0, 20.0, 0.5, 0.326, true); }
    assert!(a.clutch < 0.1, "the wheels are in the air: {}", a.clutch);
    let rev = drive(&mut Gearbox::default(), -3.0, -1.0, 1.0); assert_eq!(rev.gear, -1, "reversing");
    // a shift: the clutch opens for a moment at the up-shift point
    let mut s = Gearbox::default(); let mut min_clutch = 1.0f32; let mut speed = 0.0;
    for _ in 0..60 * 8 { let d = s.update(1.0 / 60.0, speed, 1.0, 0.326, false); speed += 0.12; if speed > 8.0 { min_clutch = min_clutch.min(d.clutch); } }
    assert!(min_clutch < 0.6, "a shift opens the clutch: {min_clutch}");
}

#[test] fn a_hard_upshift_cuts_the_spark_and_a_downshift_blips_the_throttle() {
    let mut g = Gearbox::default(); let (mut cut, mut speed) = (false, 0.0f32);
    for _ in 0..60 * 8 { let d = g.update(1.0 / 60.0, speed, 1.0, 0.326, false); speed += 0.12; cut |= d.cut; }
    assert!(cut, "the flat shift");
    let mut soft = Gearbox::default(); let mut speed = 0.0f32; let mut soft_cut = false;
    for _ in 0..60 * 12 { let d = soft.update(1.0 / 60.0, speed, 0.3, 0.326, false); speed += 0.08; soft_cut |= d.cut; }
    assert!(!soft_cut, "a light foot shifts without the cut");
    let mut g = Gearbox::default(); drive(&mut g, 40.0, 0.5, 2.0);
    let mut blip = 0.0f32; for _ in 0..180 { blip = blip.max(g.update(1.0 / 60.0, 8.0, 0.0, 0.326, false).blip); }
    assert!(blip > 0.3, "the blip on the way down: {blip}");
}
