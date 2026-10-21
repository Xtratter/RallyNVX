use super::*;

fn lsd() -> Diff { Diff { kind: Kind::Lsd, split: 0.5, preload: 40.0, power: 0.35, coast: 0.2, visc: 0.0, cap: 900.0 } }
fn open() -> Diff { Diff { kind: Kind::Open, ..lsd() } }
fn visc() -> Diff { Diff { kind: Kind::Viscous, visc: 25.0, cap: 500.0, ..lsd() } }

#[test] fn an_open_diff_splits_by_its_share_whatever_the_speeds() {
    let (a, b) = split(&open(), 1000.0, 200.0, 5.0);
    assert!((a - 500.0).abs() < 1e-3 && (b - 500.0).abs() < 1e-3);
    let c = Diff { split: 0.4, ..open() };
    let (a, b) = split(&c, 1000.0, 0.0, 50.0);
    assert!((a - 400.0).abs() < 1e-3 && (b - 600.0).abs() < 1e-3);
}
#[test] fn an_lsd_moves_torque_from_the_faster_side_to_the_slower_up_to_its_lock() {
    let d = lsd();
    // a small speed difference: proportional (60 N m per rad/s)
    let (a, b) = split(&d, 1000.0, 10.5, 10.0);
    assert!((a - (500.0 - 30.0)).abs() < 1e-3 && (b - (500.0 + 30.0)).abs() < 1e-3, "{a} {b}");
    // a big one: saturates at the preload + power x torque = 40 + 350
    let (a, b) = split(&d, 1000.0, 80.0, 10.0);
    assert!((a - (500.0 - 390.0)).abs() < 1e-3 && (b - (500.0 + 390.0)).abs() < 1e-3, "{a} {b}");
    // the slower side is A: the other way round
    let (a, _) = split(&d, 1000.0, 10.0, 80.0);
    assert!(a > 500.0);
}
#[test] fn the_preload_locks_a_little_even_without_torque() {
    let (a, b) = split(&lsd(), 0.0, 20.0, 0.0);
    assert!((a + 40.0).abs() < 1e-3 && (b - 40.0).abs() < 1e-3, "{a} {b}");
}
#[test] fn coast_uses_the_coast_ramp_and_keeps_the_sum() {
    let d = lsd();
    let (pa, pb) = split(&d, 1000.0, 80.0, 10.0); let (ca, cb) = split(&d, -1000.0, 80.0, 10.0);
    assert!(((pa - 500.0).abs() - (ca + 500.0).abs()).abs() > 100.0, "the two ramps differ: power moves {} coast {}", pa - 500.0, ca + 500.0);
    assert!((ca + cb + 1000.0).abs() < 1e-3 && (pa + pb - 1000.0).abs() < 1e-3);
    assert!((ca + 500.0).abs() < 40.0 + 200.0 + 1e-3, "40 + 0.2 x 1000");
}
#[test] fn a_viscous_diff_moves_torque_in_proportion_to_the_speed_difference_up_to_its_cap() {
    let d = visc();
    let (a, _) = split(&d, 800.0, 12.0, 10.0);
    assert!((a - (400.0 - 50.0)).abs() < 1e-3, "{a}");
    let (a, b) = split(&d, 800.0, 100.0, 0.0);
    assert!((a - (400.0 - 500.0)).abs() < 1e-3 && (b - (400.0 + 500.0)).abs() < 1e-3);
}
#[test] fn the_input_speed_is_power_consistent_for_an_open_diff() {
    let d = Diff { split: 0.4, ..open() };
    let (wa, wb, t) = (30.0f32, 50.0f32, 700.0f32);
    let (ta, tb) = split(&d, t, wa, wb);
    assert!(((ta * wa + tb * wb) - t * input_speed(&d, wa, wb)).abs() < 1e-2);
}
#[test] fn abuse_keeps_the_sum_and_stays_finite() {
    let mut seed = 5u32; let mut r = || { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); (seed >> 8) as f32 / 16_777_216.0 };
    for _ in 0..20000 {
        let k = [Kind::Open, Kind::Lsd, Kind::Viscous][(r() * 3.0) as usize % 3];
        let d = Diff { kind: k, split: 0.2 + r() * 0.6, preload: r() * 200.0, power: r(), coast: r(), visc: r() * 100.0, cap: r() * 2000.0 };
        let (t, wa, wb) = ((r() - 0.5) * 8000.0, (r() - 0.5) * 2000.0, (r() - 0.5) * 2000.0);
        let (a, b) = split(&d, t, wa, wb);
        assert!(a.is_finite() && b.is_finite() && (a + b - t).abs() < 1e-2 * (1.0 + t.abs() * 1e-3), "{d:?} {t} {wa} {wb} -> {a} {b}");
        assert!((moved(&d, t, wa - wb)).abs() <= d.cap.max(d.preload + t.abs() * d.power.max(d.coast)) + 1e-3);
    }
}
