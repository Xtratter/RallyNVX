use super::*;

const R: f32 = 0.326;
const H: f32 = 1.0 / 240.0;
fn base(w: f32, vf: f32) -> In { In { w, vf, radius: R, fmax: 3600.0, kp: 0.07, slide: 0.85, drive: 0.0, brake: 0.0, abs: false, h: H, inertia: INERTIA } }
fn run(mut i: In, secs: f32) -> Out {
    let mut o = step(&i);
    for _ in 1..(secs / H) as usize { i.w = o.w; o = step(&i); }
    o
}

#[test] fn the_curve_rises_to_the_peak_falls_to_the_slide_and_is_odd() {
    assert_eq!(curve(0.0, 0.07, 0.85), 0.0);
    assert!((curve(0.07, 0.07, 0.85) - 1.0).abs() < 1e-5, "the peak at the peak slip");
    assert!(curve(0.03, 0.07, 0.85) > 0.0 && curve(0.03, 0.07, 0.85) < 1.0);
    assert!((curve(0.9, 0.07, 0.85) - 0.85).abs() < 0.01, "sliding keeps the share");
    assert!((curve(-0.05, 0.07, 0.85) + curve(0.05, 0.07, 0.85)).abs() < 1e-6);
    assert!(slope(0.5, 0.07, 0.85) > 0.0 && slope(0.07, 0.07, 0.85) > 0.0, "the slope never vanishes (it damps the implicit step)");
}
#[test] fn a_free_wheel_keeps_rolling_with_the_ground() {
    let o = run(base(10.0 / R, 10.0), 1.0);
    assert!((o.w - 10.0 / R).abs() < 0.05 && o.fx.abs() < 20.0, "w {} fx {}", o.w, o.fx);
}
#[test] fn a_drive_torque_settles_at_a_slip_below_the_peak_where_the_force_balances_it() {
    let mut i = base(10.0 / R, 10.0); i.drive = 800.0;
    let o = run(i, 1.0);
    assert!((R * o.fx - 800.0).abs() < 15.0, "the tyre carries the torque: {} N m", R * o.fx);
    assert!(o.slip > 0.0 && o.slip < 0.07, "slip {}", o.slip);
}
#[test] fn too_much_torque_spins_the_wheel_past_the_peak() {
    let mut i = base(10.0 / R, 10.0); i.drive = 2500.0;
    let o = run(i, 0.5);
    assert!(o.slip > 0.07 * 1.5, "spinning: {}", o.slip);
    assert!(o.fx > 0.8 * 3600.0 && o.fx < 3600.0, "the force falls to the slide: {}", o.fx);
}
#[test] fn the_brake_without_abs_locks_the_wheel_and_it_slides_at_the_reduced_force() {
    let mut i = base(20.0 / R, 20.0); i.brake = 3000.0;
    let o = run(i, 0.4);
    assert!(o.w.abs() < 0.01, "locked: {}", o.w);
    assert!((o.fx + 0.85 * 3600.0).abs() < 120.0, "sliding force {}", o.fx);
    assert!(o.brake_used > 0.0 || o.w == 0.0);
}
#[test] fn abs_holds_the_slip_near_the_peak_and_the_force_near_the_maximum() {
    let mut i = base(20.0 / R, 20.0); i.brake = 3000.0; i.abs = true;
    let o = run(i, 0.5);
    assert!(o.slip < -0.04 && o.slip > -0.12, "slip {}", o.slip);
    assert!(-o.fx > 0.93 * 3600.0, "force {}", o.fx);
}
#[test] fn a_locked_wheel_at_a_standstill_stays_put_without_chatter() {
    let mut i = base(0.0, 0.0); i.brake = 500.0;
    let mut o = step(&i);
    for _ in 0..2000 { i.w = o.w; o = step(&i); assert!(o.w.abs() < 1e-4 && o.fx.abs() < 1.0, "{o:?}"); }
}
#[test] fn an_airborne_wheel_spins_up_freely_and_the_brake_stops_it() {
    let mut i = base(0.0, 0.0); i.fmax = 0.0; i.drive = 100.0;
    let o = run(i, 1.0);
    assert!((o.w - 100.0 / INERTIA).abs() < 1.0, "w {}", o.w);
    let mut b = base(o.w, 0.0); b.fmax = 0.0; b.brake = 400.0;
    assert!(run(b, 1.0).w.abs() < 1e-3);
}
#[test] fn reverse_is_the_mirror_image() {
    let mut f = base(10.0 / R, 10.0); f.drive = 800.0;
    let mut r = base(-10.0 / R, -10.0); r.drive = -800.0;
    let (a, b) = (run(f, 1.0), run(r, 1.0));
    assert!((a.w + b.w).abs() < 1e-3 && (a.fx + b.fx).abs() < 1.0 && (a.slip + b.slip).abs() < 1e-4);
}
#[test] fn abuse_stays_finite_and_bounded() {
    let mut seed = 9u32; let mut r = || { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); (seed >> 8) as f32 / 16_777_216.0 };
    for _ in 0..20000 {
        let i = In { w: (r() - 0.5) * 600.0, vf: (r() - 0.5) * 120.0, radius: 0.3, fmax: r() * 8000.0, kp: 0.05 + r() * 0.12, slide: 0.6 + r() * 0.35, drive: (r() - 0.5) * 6000.0, brake: r() * 6000.0, abs: r() > 0.5, h: 1.0 / 240.0, inertia: 1.3 };
        let o = step(&i);
        assert!(o.w.is_finite() && o.fx.is_finite() && o.slip.is_finite() && o.w.abs() < 3000.0 && o.fx.abs() <= i.fmax + 1.0, "{i:?} -> {o:?}");
    }
}
#[test] fn a_brake_torque_below_the_grip_is_carried_by_the_tyre_one_to_one() {
    // the wheel rolls at 20 m/s and a brake of 400 N m (well under what the tyre can hold): once it settles, the tyre's torque is that brake's, not a multiple of it
    let mut i = base(20.0 / R, 20.0); i.brake = 400.0; i.abs = false;
    let o = run(i, 0.6);
    assert!((R * o.fx + 400.0).abs() < 25.0, "the tyre carries {} N m for a brake of 400", -R * o.fx);
}
#[test] fn a_soft_brake_and_a_hard_brake_differ_in_proportion() {
    let go = |b: f32| { let mut i = base(20.0 / R, 20.0); i.brake = b; i.abs = true; -R * run(i, 0.6).fx };
    let (soft, hard) = (go(300.0), go(900.0));
    assert!(hard > soft * 2.5 && hard < soft * 3.5, "300 N m -> {soft}, 900 N m -> {hard}");
}
