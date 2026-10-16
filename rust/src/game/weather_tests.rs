use super::*;
use crate::game::daylight::daylight_at_phase;

#[test] fn the_modes_ask_for_the_right_weather() {
    assert_eq!(target(1, 5), Weather::default());
    assert!(target(2, 5).cloud > 0.5 && target(2, 5).rain == 0.0);
    assert!(target(3, 5).rain > 0.3 && target(3, 5).rain < target(4, 5).rain);
    assert!(target(5, 5).fog > 0.8 && target(5, 5).rain == 0.0);
    assert_eq!(target(99, 5), target(5, 5), "a wild mode is the last one");
}
#[test] fn auto_weather_follows_the_seed_and_is_mostly_fine() {
    let (mut clear, mut cloudy, mut rainy, mut fog) = (0, 0, 0, 0);
    for seed in 0..2000u64 { let w = target(0, seed); assert_eq!(w, target(0, seed), "deterministic"); if w.fog > 0.5 { fog += 1 } else if w.rain > 0.0 { rainy += 1 } else if w.cloud > 0.0 { cloudy += 1 } else { clear += 1 } }
    assert!(clear > 850 && clear < 1150, "clear {clear}"); assert!(cloudy > 300 && cloudy < 600, "cloudy {cloudy}");
    assert!(rainy > 300 && rainy < 480, "rain {rainy}"); assert!(fog > 40 && fog < 200, "fog {fog}");
}
#[test] fn the_weather_moves_smoothly_the_ground_gets_wet_fast_and_dries_slowly() {
    let mut c = Climate::with_mode(1, 1);
    assert_eq!(c.weather(), Weather::default());
    for _ in 0..(10 * 60) { c.step_with(1.0 / 60.0, 4); }   // ten seconds of heavy rain
    let w = c.weather();
    assert!(w.rain > 0.9 && w.wet > 0.9 && w.cloud > 0.9, "{w:?}");
    let mid = { let mut c2 = Climate::with_mode(1, 1); for _ in 0..60 { c2.step_with(1.0 / 60.0, 4); } c2.weather() };
    assert!(mid.rain > 0.1 && mid.rain < 0.4, "one second in: {mid:?}");
    for _ in 0..(30 * 60) { c.step_with(1.0 / 60.0, 1); }   // thirty seconds of clear weather
    let d = c.weather();
    assert!(d.rain == 0.0 && d.wet > 0.4 && d.wet < 0.9, "still wet after the rain: {d:?}");
    for _ in 0..(200 * 60) { c.step_with(1.0 / 60.0, 1); }
    assert!(c.weather().wet < 0.01, "dry again");
}
#[test] fn a_rainy_level_starts_wet_and_garbage_steps_change_nothing() {
    let c = Climate::with_mode(3, 4);
    assert!(c.weather().rain > 0.9 && c.weather().wet > 0.9);
    let mut c2 = c; c2.step_with(f32::NAN, 1); c2.step_with(-1.0, 1); c2.step_with(f32::INFINITY, 1);
    assert_eq!(c2.weather(), c.weather());
    let mut c3 = Climate::with_mode(3, 1); c3.step_with(1.0 / 60.0, 200);
    assert!(c3.weather().fog > 0.0 && c3.weather().rain == 0.0, "a wild mode is the last one (fog), no panic: {:?}", c3.weather());
}
#[test] fn clouds_and_rain_dim_and_grey_the_sky_and_clear_weather_changes_nothing() {
    let dl = daylight_at_phase(0.5);
    let same = apply(&dl, Weather::default());
    assert_eq!((same.sky_top, same.sky_hor, same.sun_col), (dl.sky_top, dl.sky_hor, dl.sun_col));
    let wet = apply(&dl, Weather { rain: 1.0, cloud: 1.0, fog: 0.3, wet: 1.0 });
    let lum = |c: [f32; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    assert!(lum(wet.sky_top) < lum(dl.sky_top) && lum(wet.sun_col) < lum(dl.sun_col) * 0.6, "darker");
    let spread = |c: [f32; 3]| c.iter().cloned().fold(f32::MIN, f32::max) - c.iter().cloned().fold(f32::MAX, f32::min);
    assert!(spread(wet.sky_top) < spread(dl.sky_top), "greyer");
    let night = daylight_at_phase(0.0);
    let nw = apply(&night, Weather { rain: 1.0, cloud: 1.0, fog: 0.0, wet: 1.0 });
    assert!(lum(nw.sky_top) <= lum(night.sky_top) + 1e-6, "the night does not get brighter");
    assert!(nw.sky_top.iter().chain(nw.sky_hor.iter()).chain(nw.sun_col.iter()).all(|x| x.is_finite() && *x >= 0.0));
}
#[test] fn the_fog_and_the_discs_follow_the_weather() {
    assert!((fog_k(Weather::default()) - 1.0 / 330.0).abs() < 1e-9);
    assert!(fog_k(Weather { rain: 1.0, ..Default::default() }) > fog_k(Weather::default()) * 3.5);
    assert!(fog_k(Weather { fog: 1.0, ..Default::default() }) > fog_k(Weather::default()) * 9.5);
    assert_eq!(disc_visibility(Weather::default()), 1.0);
    assert_eq!(disc_visibility(Weather { cloud: 1.0, ..Default::default() }), 0.0);
    assert!(disc_visibility(Weather { cloud: 0.3, ..Default::default() }) > 0.5);
}
#[test] fn the_player_mode_is_clamped() { set_mode(200); assert_eq!(mode(), MODES - 1); set_mode(0); assert_eq!(mode(), 0); }
