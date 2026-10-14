use super::*;

#[test] fn the_wind_is_deterministic_horizontal_and_bounded() {
    assert_eq!(wind_now(3, 10.0), wind_now(3, 10.0));
    assert!(wind_now(3, 10.0) != wind_now(4, 10.0));
    for k in 0..400 {
        let w = wind_now(9, k as f32 * 3.7);
        assert!(w.y == 0.0 && w.is_finite(), "{w:?}");
        let mean = wind_at(9, k as f32 * 3.7).len();
        assert!((1.45..=2.5 + 1e-3).contains(&mean), "mean wind {mean}");
        assert!(w.len() <= 2.5 * 1.35 + 1e-3 && w.len() >= 1.45 * 0.65 - 1e-3, "{w:?}");
    }
}
#[test] fn the_direction_wanders_over_minutes_and_the_gusts_over_seconds() {
    assert!((wind_at(9, 0.0) - wind_at(9, 400.0)).len() > 0.1);
    let g: Vec<f32> = (0..200).map(|k| gust(5, k as f32 * 0.5)).collect();
    let (lo, hi) = (g.iter().cloned().fold(f32::MAX, f32::min), g.iter().cloned().fold(f32::MIN, f32::max));
    assert!(lo >= 1.0 - 0.35 - 1e-3 && hi <= 1.0 + 0.35 + 1e-3 && hi - lo > 0.2, "gust factor {lo}..{hi}");
    assert!((gust(5, 10.0) - gust(5, 10.0)).abs() == 0.0);
    for t in [f32::NAN, f32::INFINITY, -50.0, 1.0e9] { assert!(wind_now(1, t).is_finite() && gust(1, t).is_finite()); }
}
