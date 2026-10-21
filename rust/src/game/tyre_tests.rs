use super::*;

#[test] fn a_warm_tyre_of_each_set_grips_fully_at_its_default_pressure() {
    for set in 0..3u8 {
        let t = Tyre::warm(set);
        let g = grip(set, t.temp, 0.0, cold_default(set));
        assert!(g > 0.99 && g <= 1.0, "set {set}: {g}");
    }
}
#[test] fn the_thermal_window_costs_grip_when_cold_or_overheated_but_never_below_the_floor() {
    let (opt, _) = window(0);
    let (cold, ok, hot, boiling) = (grip(0, 15.0, 0.0, 1.69), grip(0, opt, 0.0, 1.69), grip(0, 130.0, 0.0, 1.69), grip(0, 400.0, 0.0, 1.69));
    assert!(cold < 0.8 && cold > 0.55, "a cold tarmac tyre: {cold}");
    assert!(ok > hot && hot > 0.6, "an overheated one: {hot}");
    assert!(boiling >= 0.55 * 0.7 - 1e-3, "the floor of the window and of the pressure: {boiling}");
    // the snow set likes the cold, the tarmac set does not
    assert!(grip(2, 5.0, 0.0, 1.53) > grip(0, 5.0, 0.0, 1.69) + 0.15);
}
#[test] fn wear_lowers_the_grip_and_the_last_fifth_costs_more() {
    let g = |w: f32| grip(0, 85.0, w, 1.69);
    assert!(g(0.0) > g(0.3) && g(0.3) > g(0.6) && g(0.6) > g(0.8) && g(0.8) > g(1.0));
    assert!((g(0.8) - g(1.0)) > (g(0.6) - g(0.8)) * 1.3, "the last stretch is steeper");
    assert!(g(1.0) > 0.55, "{}", g(1.0));
}
#[test] fn the_pressure_has_an_optimum_and_the_hot_pressure_follows_the_temperature() {
    let at = |bar: f32| grip(0, 85.0, 0.0, bar);
    assert!(at(1.69) > at(1.3) && at(1.69) > at(2.3), "{} {} {}", at(1.3), at(1.69), at(2.3));
    assert!(at(0.2) >= 0.7 - 1e-3 && at(9.0) >= 0.7 - 1e-3, "the pressure factor floors at 0.7");
    assert!(hot_pressure(1.69, 85.0) > hot_pressure(1.69, 15.0) * 1.2);
    assert!((hot_pressure(1.69, 85.0) - 2.1).abs() < 0.05, "{}", hot_pressure(1.69, 85.0));
}
#[test] fn a_tyre_heats_with_work_and_cools_in_the_air() {
    let mut t = Tyre { temp: 20.0, wear: 0.0 };
    for _ in 0..240 * 10 { t.step(1.0 / 240.0, 15000.0, 20.0, 3000.0, 15.0, 1.0); }
    assert!(t.temp > 30.0, "10 s of hard work: {}", t.temp);
    let mut h = Tyre { temp: 120.0, wear: 0.0 };
    for _ in 0..240 * 10 { h.step(1.0 / 240.0, 0.0, 25.0, 3000.0, 15.0, 1.0); }
    assert!(h.temp < 112.0 && h.temp > 15.0, "10 s of cruising cools a hot tyre: 120 -> {}", h.temp);
}
#[test] fn a_standing_tyre_only_cools() {
    let mut t = Tyre { temp: 90.0, wear: 0.2 };
    for _ in 0..240 * 30 { t.step(1.0 / 240.0, 0.0, 0.0, 3000.0, 15.0, 1.0); }
    assert!(t.temp < 90.0 && t.temp > 15.0 && t.wear == 0.2, "{t:?}");
}
#[test] fn the_balance_of_a_hard_steady_drive_is_a_hot_but_sane_tyre() {
    let mut t = Tyre { temp: 15.0, wear: 0.0 };
    for _ in 0..240 * 400 { t.step(1.0 / 240.0, 8000.0, 30.0, 3000.0, 15.0, 1.0); }
    assert!(t.temp > 70.0 && t.temp < 120.0, "{}", t.temp);
}
#[test] fn wear_grows_with_work_and_the_surface_and_stays_below_one() {
    let wear = |work: f32, ab: f32, secs: f32| { let mut t = Tyre { temp: 85.0, wear: 0.0 }; for _ in 0..(secs * 240.0) as usize { t.step(1.0 / 240.0, work, 25.0, 3000.0, 15.0, ab); } t.wear };
    assert!(wear(10000.0, 1.0, 60.0) > wear(5000.0, 1.0, 60.0) * 1.8);
    assert!(wear(8000.0, 2.0, 60.0) > wear(8000.0, 1.0, 60.0) * 1.8);
    let stage = wear(8000.0, 1.0, 400.0);
    assert!(stage > 0.05 && stage < 0.4, "a stage of hard driving on tarmac wears {stage}");
    assert!(wear(2.0e5, 3.0, 4000.0) <= 1.0);
}
#[test] fn the_abrasiveness_follows_the_surface_and_the_wrong_set() {
    use crate::game::surface::Surface;
    assert!(abrasiveness(Surface::Gravel, 1.0) > abrasiveness(Surface::Asphalt, 1.0) * 1.4);
    assert!(abrasiveness(Surface::Asphalt, 0.5) > abrasiveness(Surface::Asphalt, 1.0) * 1.5, "the wrong set wears faster");
    assert!(abrasiveness(Surface::Water, 1.0) < abrasiveness(Surface::Asphalt, 1.0));
}
#[test] fn abuse_keeps_the_state_in_range() {
    let mut seed = 11u32; let mut r = || { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); (seed >> 8) as f32 / 16_777_216.0 };
    let mut t = Tyre::warm(0);
    for _ in 0..20000 {
        let (h, work, v, fz, air) = (0.001 + r() * 0.05, [f32::NAN, f32::INFINITY, -5.0, 1e9, r() * 5e4][(r() * 5.0) as usize % 5], (r() - 0.5) * 300.0, (r() - 0.2) * 1e5, -40.0 + r() * 90.0);
        t.step(h, work, v, fz, air, r() * 4.0);
        assert!(t.temp.is_finite() && t.temp >= -60.0 && t.temp <= 250.0 && t.wear.is_finite() && (0.0..=1.0).contains(&t.wear), "{t:?}");
        let g = grip(((r() * 3.0) as u8) % 3, t.temp, t.wear, r() * 5.0);
        assert!(g.is_finite() && g > 0.2 && g <= 1.0, "{g}");   // the worst case: cold or hot, a wrong pressure, a tyre worn out
    }
}
