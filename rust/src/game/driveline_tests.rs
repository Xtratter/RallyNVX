use super::*;
use crate::game::{drivetrain::{DriveIn, Drivetrain}, wheel};

const R: f32 = 0.326;
fn calm() -> Ctl { Ctl { throttle: 1.0, brake: false, steer: 0.0, speed: 10.0 } }
fn sum(a: [f32; 4]) -> f32 { a.iter().sum() }

#[test] fn the_peugeot_is_four_wheel_drive_with_a_nominal_half_and_half_split() {
    let d = Driveline::peugeot();
    assert!((d.centre.split - 0.5).abs() < 1e-6 && d.front.split == 0.5 && d.rear.split == 0.5 && d.active);
    let t = d.torques(1200.0, [30.0; 4], &calm());
    for x in t { assert!((x - 300.0).abs() < 1e-3, "{t:?}"); }
}
#[test] fn the_wheel_torques_always_add_up_to_the_input_forward_coast_and_reverse() {
    let d = Driveline::peugeot();
    for &t in &[2500.0f32, 0.0, -300.0, -1800.0] {
        for w in [[30.0, 31.0, 29.0, 30.5], [80.0, 5.0, 30.0, 30.0], [0.0, 0.0, 120.0, 0.0], [-20.0, -21.0, -19.0, -20.0]] {
            let q = d.torques(t, w, &calm());
            assert!((sum(q) - t).abs() < 1e-2, "{t} {w:?} -> {q:?}");
        }
    }
}
#[test] fn reverse_and_coast_keep_the_sum() {
    let d = Driveline::peugeot();
    let q = d.torques(-900.0, [-10.0, -12.0, -8.0, -9.0], &Ctl { throttle: 0.0, ..calm() });
    assert!((sum(q) + 900.0).abs() < 1e-2, "{q:?}");
}
#[test] fn a_spinning_rear_wheel_loses_torque_to_the_other_side_through_the_lsd() {
    let d = Driveline::peugeot();
    let even = d.torques(1600.0, [30.0; 4], &calm());
    // the rear axle turns as fast as the front one on the whole (the centre diff stays quiet), but its left wheel spins and the right one is held
    let spin = d.torques(1600.0, [30.0, 30.0, 50.0, 10.0], &calm());
    assert!(spin[2] < even[2] - 100.0 && spin[3] > even[3] + 100.0, "{even:?} -> {spin:?}");
}
#[test] fn the_centre_diff_sends_torque_to_the_slower_axle() {
    let d = Driveline::peugeot();
    let q = d.torques(1600.0, [90.0, 90.0, 30.0, 30.0], &calm());   // the front axle spins
    assert!(q[0] + q[1] < q[2] + q[3] - 100.0, "{q:?}");
}
#[test] fn the_input_speed_of_equal_wheels_is_their_speed_and_a_split_shifts_the_weight() {
    let d = Driveline::peugeot();
    assert!((d.input_speed([42.0; 4]) - 42.0).abs() < 1e-4);
    let front = Driveline { centre: Diff { split: 0.7, ..d.centre }, ..d };
    assert!(front.input_speed([60.0, 60.0, 20.0, 20.0]) > d.input_speed([60.0, 60.0, 20.0, 20.0]));
}
#[test] fn tuning_scales_the_locks_and_moves_the_split_inside_thirty_to_seventy_percent() {
    let base = Driveline::peugeot();
    let t = base.tuned(&DiffTuning { front: 0.0, rear: 2.0, centre: 1.0, split: 1.4 });
    assert!(t.front.preload == 0.0 && t.front.power == 0.0 && (t.rear.preload - 2.0 * base.rear.preload).abs() < 1e-4);
    assert!((t.centre.split - 0.7).abs() < 1e-4);
    assert!((base.tuned(&DiffTuning { split: 0.6, ..Default::default() }).centre.split - 0.3).abs() < 1e-4);
}
#[test] fn extreme_tuning_stays_sane() {
    for &(a, b, c, s) in &[(0.0, 0.0, 0.0, 0.0), (2.0, 2.0, 2.0, 2.0), (-5.0, 9.0, f32::NAN, f32::INFINITY), (1.0, 1.0, 1.0, -3.0)] {
        let d = Driveline::peugeot().tuned(&DiffTuning { front: a, rear: b, centre: c, split: s });
        assert!(d.centre.split >= 0.3 - 1e-4 && d.centre.split <= 0.7 + 1e-4, "{s} -> {}", d.centre.split);
        let q = d.torques(2000.0, [30.0, 50.0, 10.0, 70.0], &calm());
        assert!(q.iter().all(|x| x.is_finite()) && (sum(q) - 2000.0).abs() < 1e-2, "{q:?}");
    }
}
#[test] fn the_active_lock_is_continuous() {
    let mut worst = 0.0f32; let mut last = active_lock(&Ctl { throttle: 0.0, brake: false, steer: 1.0, speed: 0.0 });   // the sweep's own first point
    for k in 1..=2000 {
        let x = k as f32 / 2000.0;
        let l = active_lock(&Ctl { throttle: x, brake: false, steer: 1.0 - x, speed: 50.0 * x });
        worst = worst.max((l - last).abs()); last = l;
    }
    assert!(worst < 0.02, "the largest jump of the lock per step: {worst}");
}
#[test] fn the_active_lock_opens_for_turn_in_and_closes_on_the_power_and_at_speed() {
    let power = active_lock(&Ctl { throttle: 1.0, brake: false, steer: 0.0, speed: 10.0 });
    let turn_in = active_lock(&Ctl { throttle: 0.0, brake: false, steer: 0.8, speed: 10.0 });
    let braking = active_lock(&Ctl { throttle: 0.0, brake: true, steer: 0.0, speed: 10.0 });
    let fast = active_lock(&Ctl { throttle: 0.0, brake: false, steer: 0.0, speed: 45.0 });
    let slow = active_lock(&Ctl { throttle: 0.0, brake: false, steer: 0.0, speed: 5.0 });
    assert!(power > turn_in + 0.4 && power > braking + 0.3 && fast > slow + 0.1, "{power} {turn_in} {braking} {fast} {slow}");
}

/// A bench for the whole loop: the real engine and clutch (`Drivetrain`), a diff and two driven wheels on the same axle, one on grip and one on ice, pulling a car; returns the car's speed.
fn mu_split_launch(axle: Diff, secs: f32) -> (f32, f32) {
    let mut d = Drivetrain::new(); d.opts.tc = false;
    let (h, mass, grip, ice) = (1.0 / 240.0, 1230.0f32, 3600.0f32, 450.0f32);
    let (mut wl, mut wr, mut v) = (0.0f32, 0.0f32, 0.0f32);
    let (mut tyre, mut spin_var) = (0.0f32, 0.0f32);
    let mut prev = 0.0f32;
    for k in 0..(secs / h) as usize {
        if k % 4 == 0 { d.begin(4.0 * h, &DriveIn { throttle: 1.0, speed: v, wheel_r: R, ..Default::default() }); }
        let t = d.sub_coupled(h, crate::game::diff::input_speed(&axle, wl, wr), 2.0 * wheel::INERTIA, tyre);
        let (tl, tr) = crate::game::diff::split(&axle, t, wl, wr);
        let ol = wheel::step(&wheel::In { w: wl, vf: v, radius: R, fmax: grip, kp: 0.07, slide: 0.85, drive: tl, brake: 0.0, abs: false, h, inertia: wheel::INERTIA });
        let or = wheel::step(&wheel::In { w: wr, vf: v, radius: R, fmax: ice, kp: 0.07, slide: 0.85, drive: tr, brake: 0.0, abs: false, h, inertia: wheel::INERTIA });
        wl = ol.w; wr = or.w; tyre = R * (ol.fx + or.fx);
        v += (ol.fx + or.fx - 0.5 * 1.2 * 0.62 * 2.1 * v * v) / mass * h;
        if k % 4 == 3 { d.finish(); }
        if k as f32 * h > secs * 0.5 { spin_var += (wr - prev).abs(); }
        prev = wr;
    }
    (v, spin_var)
}
#[test] fn an_lsd_pulls_away_where_an_open_diff_spins() {
    let open = Diff { kind: Kind::Open, ..Driveline::peugeot().rear };
    let lsd = Driveline::peugeot().rear;
    let (v_open, _) = mu_split_launch(open, 3.0);
    let (v_lsd, wobble) = mu_split_launch(lsd, 3.0);
    assert!(v_open < 4.0, "an open diff on a split surface barely moves: {v_open} m/s");
    assert!(v_lsd > v_open * 2.0, "the LSD pulls: {v_lsd} against {v_open} m/s");
    assert!(wobble.is_finite() && wobble < 2000.0, "the ice wheel's speed does not saw: {wobble}");
}
