use super::*;

/// A windscreen leaning back 27 degrees from the horizontal: gravity's component along the glass is sin(27) = 0.46 down the slope (v negative).
const SLOPE: [f32; 2] = [0.0, -0.46];

#[test] fn a_still_drop_runs_down_only_when_it_is_big_enough() {
    let r = critical_radius_mm(SLOPE, [0.0, 0.0]);
    assert!((2.0..3.2).contains(&r), "gravity alone on a 27 degree screen: about 2.5 mm, got {r}");
    assert!(critical_radius_mm([0.0, -1.0], [0.0, 0.0]) < r, "a steeper pane sheds smaller drops");
    assert!(critical_radius_mm([0.0, 0.0], [0.0, 0.0]) >= 6.0, "a flat horizontal pane never sheds");
}
#[test] fn the_wind_makes_smaller_drops_run_and_pushes_them_up_the_screen() {
    let air = [0.0, 25.0];   // 90 km/h along the glass, up the slope
    let rc = critical_radius_mm(SLOPE, air);
    assert!(rc < critical_radius_mm(SLOPE, [0.0, 0.0]), "{rc}");
    let f = drive_force(rc + 0.3, SLOPE, air);
    assert!(f[1] > 0.0, "a drop just above critical is blown up the screen at speed: {f:?}");
    let side = critical_radius_mm([0.0, 0.0], [-25.0, 0.0]);
    assert!(side < 6.0, "on a vertical-free side pane the wind alone sheds drops: {side}");
}
#[test] fn pinning_grows_with_r_and_the_drive_with_r_squared_and_cubed() {
    assert!(pinning_force(2.0) > pinning_force(1.0) * 1.9 && pinning_force(2.0) < pinning_force(1.0) * 2.1);
    let (f1, f2) = (drive_force(1.0, [0.0, 0.0], [0.0, 20.0]), drive_force(2.0, [0.0, 0.0], [0.0, 20.0]));
    assert!((f2[1] / f1[1] - 4.0).abs() < 0.05, "drag goes with r squared");
    let (g1, g2) = (drive_force(1.0, SLOPE, [0.0, 0.0]), drive_force(2.0, SLOPE, [0.0, 0.0]));
    assert!((g2[1] / g1[1] - 8.0).abs() < 0.1, "weight goes with r cubed");
}
#[test] fn a_runner_speeds_up_with_excess_force_and_never_runs_away() {
    let rc = critical_radius_mm(SLOPE, [0.0, 0.0]);
    assert_eq!(slide_speed(rc * 0.5, drive_force(rc * 0.5, SLOPE, [0.0, 0.0])), 0.0, "pinned");
    let (a, b) = (slide_speed(rc + 0.5, drive_force(rc + 0.5, SLOPE, [0.0, 0.0])), slide_speed(rc + 1.5, drive_force(rc + 1.5, SLOPE, [0.0, 0.0])));
    assert!(a > 0.0 && b > a && b <= MAX_SLIDE, "{a} {b}");
    assert!(a > 0.002 && a < 0.3, "a few millimetres a second to a few centimetres: {a}");
}
#[test] fn the_air_dries_faster_when_dry_moving_and_sunny_and_is_saturated_after_rain() {
    assert!(humidity(0.0) > 0.95 && humidity(300.0) < 0.6 && humidity(10.0) > humidity(60.0));
    assert!(evaporation_rate(0.55, 0.0, 0.0) > evaporation_rate(0.95, 0.0, 0.0) * 5.0, "damp air takes little vapour");
    assert!(evaporation_rate(0.6, 25.0, 0.0) > evaporation_rate(0.6, 0.0, 0.0) && evaporation_rate(0.6, 0.0, 1.0) > evaporation_rate(0.6, 0.0, 0.0));
    assert_eq!(evaporation_rate(1.0, 20.0, 1.0), 0.0, "saturated air takes nothing");
}
#[test] fn a_one_millimetre_drop_dries_in_game_seconds_not_instantly() {
    // r^2 = 1 mm^2 at 55 % humidity at rest: a handful of seconds
    let t = 1.0 / evaporation_rate(0.55, 0.0, 0.0);
    assert!((5.0..40.0).contains(&t), "{t} s");
}
#[test] fn a_cells_drop_arrives_with_the_water_grows_dries_and_is_capped() {
    assert_eq!(static_drop_radius(1.5, 0.5, 0.2, 0.0, 9.0), 0.0, "not hit yet");
    let (small, big) = (static_drop_radius(1.5, 0.5, 0.6, 0.0, 9.0), static_drop_radius(1.5, 0.5, 1.2, 0.0, 9.0));
    assert!(small > 0.0 && small < big && (big - 1.5).abs() < 1e-4, "{small} {big}");
    assert!(static_drop_radius(1.5, 0.5, 1.2, 1.0, 9.0) < big, "evaporation shrinks it");
    assert_eq!(static_drop_radius(1.5, 0.5, 1.2, 2.5, 9.0), 0.0, "and then it is gone (r^2 = 2.25 mm^2)");
    assert!((static_drop_radius(1.5, 0.5, 1.2, 0.0, 1.0) - 1.0).abs() < 1e-6, "the cap");
    // smaller drops die first
    assert_eq!(static_drop_radius(0.6, 0.0, 1.0, 0.5, 9.0), 0.0);
    assert!(static_drop_radius(1.5, 0.0, 1.0, 0.5, 9.0) > 1.0);
}

fn screen() -> Windscreen { Windscreen { origin: [0.0, 1.1, 0.7], u: [1.0, 0.0, 0.0], v: [0.0, 0.46, -0.89], n: [0.0, 0.89, 0.46], u_range: [-0.6, 0.6], v_range: [-0.3, 0.3], hull: [[0.0; 2]; 24], hull_n: 0 } }
fn nowhere(_: f32, _: f32) -> bool { false }

#[test] fn the_film_wets_in_the_rain_dries_after_it_and_a_new_rain_wets_it_again() {
    let mut f = Film::default();
    let s = screen();
    let wet = Weather { rain: 1.0, air_speed: 0.0, sun: 0.0 };
    for _ in 0..200 { f.advance(0.05, wet, Some(&s), SLOPE, [0.0, 0.0], &nowhere); }
    assert!(f.water > 1.0 && f.evap == 0.0, "10 s of heavy rain: {} {}", f.water, f.evap);
    let dry = Weather { rain: 0.0, air_speed: 0.0, sun: 0.0 };
    let mut e = 0.0;
    for k in 0..2000 { f.advance(0.05, dry, Some(&s), SLOPE, [0.0, 0.0], &nowhere); if k == 200 { e = f.evap; } }
    assert!(e > 0.0 && f.evap > e, "it keeps drying: {e} {}", f.evap);
    let before = f.evap;
    for _ in 0..40 { f.advance(0.05, wet, Some(&s), SLOPE, [0.0, 0.0], &nowhere); }
    assert!(f.evap < before && f.dry == 0.0, "rain wets it again");
}
#[test] fn the_film_without_rain_eventually_resets_and_never_goes_wrong() {
    let mut f = Film::default();
    f.water = 2.0;
    let dry = Weather { rain: 0.0, air_speed: 20.0, sun: 1.0 };
    for _ in 0..40000 { f.advance(0.05, dry, None, SLOPE, [0.0, 0.0], &nowhere); }
    assert!(f.water == 0.0 || f.evap <= 40.0, "{} {}", f.water, f.evap);
    for w in [f32::NAN, -1.0, 7.0] { f.advance(f32::NAN, Weather { rain: w, air_speed: f32::NAN, sun: 2.0 }, None, SLOPE, [0.0, 0.0], &nowhere); }
    assert!(f.water.is_finite() && f.evap.is_finite());
}
#[test] fn rain_spawns_runners_that_run_up_at_speed_leave_the_glass_and_are_wiped() {
    let s = screen();
    let w = Weather { rain: 1.0, air_speed: 25.0, sun: 0.0 };
    let air = [0.0, 25.0];
    let mut f = Film::default();
    let mut seen = 0usize;
    for _ in 0..400 { f.advance(0.05, w, Some(&s), SLOPE, air, &nowhere); seen = seen.max(f.sliders.len()); assert!(f.sliders.len() <= MAX_SLIDERS); }
    assert!(seen > 0, "heavy rain makes runners");
    assert!(f.sliders.iter().all(|d| s.contains(d.a, d.b, 0.0) && d.r_mm.is_finite() && d.r_mm <= 5.0));
    // a wiper over the whole screen sweeps them all away
    let mut g = f.clone();
    g.advance(0.05, w, Some(&s), SLOPE, air, &|_, _| true);
    assert!(g.sliders.is_empty(), "wiped");
    // no screen, no runners
    let mut h = f.clone();
    h.advance(0.05, w, None, SLOPE, air, &nowhere);
    assert!(h.sliders.is_empty());
}
#[test] fn a_runner_moves_along_the_force_and_grows_on_a_wet_screen() {
    let s = screen();
    let mut f = Film::default();
    f.water = 1.0;
    f.sliders.push(Slider { a: 0.0, b: 0.2, r_mm: 3.5, dir: [0.0, 1.0], speed: 0.0 });
    let w = Weather { rain: 0.0, air_speed: 0.0, sun: 0.0 };
    f.advance(0.1, w, Some(&s), SLOPE, [0.0, 0.0], &nowhere);
    let d = f.sliders[0];
    assert!(d.b < 0.2 && d.speed > 0.0 && d.dir[1] < -0.9, "it runs down the slope: {d:?}");
    assert!(d.r_mm > 3.5 - 1e-4, "and gathers water");
}
#[test] fn the_windscreen_is_found_in_the_cars_glass() {
    // a tilted quad facing forward and up, plus a rear window and a side pane that must be ignored
    let verts = [[-0.5, 0.9, 1.0], [0.5, 0.9, 1.0], [0.5, 1.3, 0.1], [-0.5, 1.3, 0.1], [-0.5, 1.3, -0.7], [0.5, 1.3, -0.7], [0.5, 0.9, -1.4], [-0.5, 0.9, -1.4]];
    let idx = [0u32, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    let s = Windscreen::from_glass(&verts, &idx).expect("a windscreen");
    assert!(s.n[1] > 0.7 && s.n[2] > 0.3, "{:?}", s.n);
    assert!(s.v[1] > 0.0 && s.v[2] < 0.0, "v runs up and back along the slope: {:?}", s.v);
    assert!(s.u_range[1] - s.u_range[0] > 0.9 && s.v_range[1] - s.v_range[0] > 0.3);
    assert!(Windscreen::from_glass(&verts[4..], &[0, 1, 2, 0, 2, 3]).is_none(), "the rear window is no windscreen");
    let p = s.at(0.0, 0.0);
    assert!((p[1] - s.origin[1]).abs() < 1e-5);
}

#[test] fn the_hull_of_a_trapezoid_is_its_four_corners_and_contains_its_inside() {
    let h = convex_hull(&[[0.0, 0.0], [1.0, 0.0], [0.8, 0.5], [0.2, 0.5], [0.5, 0.2], [0.5, 0.1]]);
    assert_eq!(h.len(), 4, "{h:?}");
    let mut s = screen();
    s.hull_n = h.len();
    for (k, p) in h.iter().enumerate() { s.hull[k] = *p; }
    assert!(s.inside(0.5, 0.25, 0.0) && !s.inside(0.95, 0.45, 0.0) && !s.inside(-0.1, 0.1, 0.0), "the slanted sides cut the corners off");
    assert!(!s.inside(0.5, 0.01, 0.05), "a margin keeps off the edge");
}
