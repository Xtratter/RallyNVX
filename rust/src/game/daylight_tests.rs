use super::*;
#[test] fn elevation_swings_between_dawn_and_noon_and_is_continuous() {
    let mut prev = daylight(7, 0.0).sun.y; let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for i in 0..=1440 { let d = daylight(7, i as f32 * 0.5); assert!((d.sun.y - prev).abs() < 0.01); prev = d.sun.y; lo = lo.min(d.sun.y); hi = hi.max(d.sun.y); }
    assert!((lo - 10f32.to_radians().sin()).abs() < 0.01 && (hi - 60f32.to_radians().sin()).abs() < 0.01, "{lo} {hi}");
}
#[test] fn everything_is_finite_unit_and_the_cycle_repeats() {
    for t in [0.0, 100.0, 360.0, 719.9, 5000.0] {
        let d = daylight(3, t);
        assert!((d.sun.len() - 1.0).abs() < 1e-4 && d.sun.y > 0.1);
        assert!(d.sun_col.iter().chain(&d.sky_top).chain(&d.sky_hor).all(|c| c.is_finite() && *c >= 0.0 && *c <= 4.0));
    }
    assert!((daylight(3, 100.0).sun - daylight(3, 100.0 + DAY_SECONDS).sun).len() < 1e-3);
}
#[test] fn low_sun_is_warmer_than_noon() {
    let dawn = daylight_at_phase(0.0); let noon = daylight_at_phase(0.5);
    assert!(dawn.sun_col[2] / dawn.sun_col[0] < noon.sun_col[2] / noon.sun_col[0]);
    assert!(dawn.sky_hor[0] > dawn.sky_hor[2]);
}
#[test] fn the_start_phase_depends_on_the_seed() {
    assert!((1..40u64).any(|s| (daylight(s, 0.0).sun - daylight(s + 1, 0.0).sun).len() > 1e-3));
}
