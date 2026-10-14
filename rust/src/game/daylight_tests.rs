use super::*;

fn lum(c: [f32; 3]) -> f32 { 0.3 * c[0] + 0.6 * c[1] + 0.1 * c[2] }

#[test] fn elevation_swings_from_midnight_to_noon() {
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for i in 0..=1000 { let e = elevation_deg(i as f32 / 1000.0); lo = lo.min(e); hi = hi.max(e); }
    assert!((lo + 43.0).abs() < 0.1 && (hi - 67.0).abs() < 0.1, "{lo} {hi}");
    assert!((elevation_deg(0.5) - 67.0).abs() < 1e-3 && (elevation_deg(0.0) + 43.0).abs() < 1e-3);
}
#[test] fn the_sun_is_up_about_56_percent_and_deep_night_about_a_third() {
    let n = 2000;
    let up = (0..n).filter(|i| elevation_deg(*i as f32 / n as f32) > 0.0).count() as f32 / n as f32;
    let deep = (0..n).filter(|i| elevation_deg(*i as f32 / n as f32) < -12.0).count() as f32 / n as f32;
    assert!((0.52..0.62).contains(&up), "up {up}");
    assert!((0.30..0.40).contains(&deep), "deep {deep}");
}
#[test] fn a_level_never_starts_at_night() {
    for seed in 0..1000u64 {
        let e = elevation_deg(start_phase(seed));
        assert!(e >= START_MIN_DEG - 0.05 && e <= START_MAX_DEG + 0.05, "seed {seed}: {e}");
        assert!(daylight(seed, 0.0).night < 0.01, "seed {seed}");
    }
}
#[test] fn the_start_phase_depends_on_the_seed_and_covers_both_sides_of_noon() {
    assert!((1..40u64).any(|s| (start_phase(s) - start_phase(s + 1)).abs() > 1e-3));
    assert!((0..200u64).any(|s| start_phase(s) < 0.5) && (0..200u64).any(|s| start_phase(s) > 0.5));
}
#[test] fn everything_is_finite_in_range_and_the_cycle_repeats() {
    for t in [0.0, 100.0, 360.0, 719.9, 5000.0, 1.0e6] {
        let d = daylight(3, t);
        for v in [d.sun, d.sun_disc, d.moon] { assert!((v.len() - 1.0).abs() < 1e-3, "{v:?}"); }
        assert!(d.sun_col.iter().chain(&d.sky_top).chain(&d.sky_hor).all(|c| c.is_finite() && *c >= 0.0 && *c <= 4.0), "t {t}");
        assert!((0.0..=1.0).contains(&d.night));
    }
    assert!((daylight(3, 100.0).sun_disc - daylight(3, 100.0 + DAY_SECONDS).sun_disc).len() < 1e-3);
}
#[test] fn the_light_never_jumps_and_never_points_below_the_horizon() {
    let n = 1440;
    let mut prev = daylight_at_phase(0.0);
    for i in 1..=n {
        let d = daylight_at_phase(i as f32 / n as f32);
        assert!(d.sun.y > -0.08, "phase {i}: light y {}", d.sun.y);
        assert!(((lum(d.sun_col) - lum(prev.sun_col)).abs()) < 0.06, "light colour steps at {i}");
        if lum(d.sun_col) > 0.02 || lum(prev.sun_col) > 0.02 { assert!((d.sun - prev.sun).len() < 0.02, "light direction jumps at {i}"); }
        prev = d;
    }
}
#[test] fn the_moon_is_opposite_the_sun_turned_by_the_level_offset() {
    for (phase, off) in [(0.1f32, 0.0f32), (0.3, 0.6), (0.8, -1.0), (0.0, 1.2)] {
        let d = daylight_at(phase, 0.9, off);
        assert!((d.moon.y + d.sun_disc.y).abs() < 1e-4, "heights are mirrored");
        let (hs, hm) = ((d.sun_disc.x, d.sun_disc.z), (d.moon.x, d.moon.z));
        let (ls, lm) = ((hs.0 * hs.0 + hs.1 * hs.1).sqrt(), (hm.0 * hm.0 + hm.1 * hm.1).sqrt());
        if ls > 0.2 { let cos = -(hs.0 * hm.0 + hs.1 * hm.1) / (ls * lm); assert!((cos.clamp(-1.0, 1.0).acos() - off.abs()).abs() < 1e-3, "offset {off}: {cos}"); }
    }
}
#[test] fn the_night_factor_and_the_sky_follow_the_sun() {
    let noon = daylight_at_phase(0.5); let mid = daylight_at_phase(0.0);
    assert!(noon.night < 1e-3 && mid.night > 0.99);
    assert!(lum(mid.sky_hor) < 0.15 * lum(noon.sky_hor) && lum(mid.sky_top) < 0.02, "night sky is dark");
    assert!(lum(mid.sun_col) > 0.1 && lum(mid.sun_col) < 0.4, "moonlight is dim but there: {}", lum(mid.sun_col));
    assert!(mid.sun.y > 0.0, "the light at night is the moon above the horizon");
    assert!((mid.sun - mid.moon).len() < 1e-4);
}
#[test] fn low_sun_is_warmer_than_noon_and_twilight_is_not_black() {
    let low = daylight_at(phase_at_elevation(8.0, true), 0.9, 0.0); let noon = daylight_at_phase(0.5);
    assert!(low.sun_col[2] / low.sun_col[0] < noon.sun_col[2] / noon.sun_col[0]);
    assert!(low.sky_hor[0] > low.sky_hor[2]);
    let twi = daylight_at(phase_at_elevation(-6.0, false), 0.9, 0.0);
    assert!(lum(twi.sky_hor) > 0.2 && lum(twi.sky_hor) < lum(low.sky_hor), "{}", lum(twi.sky_hor));
}
