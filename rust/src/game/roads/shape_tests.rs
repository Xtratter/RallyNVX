use super::*;
use crate::game::roads::RoadKind;

fn zigzag() -> Vec<(f32, f32)> {   // an octilinear staircase like the A* output
    let mut p = vec![(0.0, 0.0)];
    for k in 0..30 { let (x, z) = *p.last().unwrap(); p.push(if k % 3 == 0 { (x + 16.0, z + 16.0) } else if k % 3 == 1 { (x + 16.0, z) } else { (x, z + 16.0) }); }
    p
}

#[test] fn resampling_keeps_both_ends_and_spaces_points_evenly() {
    let p = resample_uniform(&zigzag(), 4.0);
    assert_eq!((p[0], *p.last().unwrap()), ((0.0, 0.0), *zigzag().last().unwrap()));
    for w in p.windows(2) { let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1); assert!(d > 2.5 && d < 4.6, "{d}"); }
}
#[test] fn the_smoothed_curve_has_no_tight_bends_and_keeps_its_ends() {
    let s = smooth_curve(&zigzag());
    assert_eq!((s[0], *s.last().unwrap()), ((0.0, 0.0), *zigzag().last().unwrap()));
    let worst = s.windows(3).map(|w| radius(w[0], w[1], w[2])).fold(f32::INFINITY, f32::min);
    assert!(worst >= MIN_RADIUS * 0.75, "tightest radius {worst}");
    for w in s.windows(2) { let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1); assert!(d > 2.5 && d < 4.6); }
}
#[test] fn a_hairpin_input_is_opened_up_to_the_minimum_radius() {
    let hairpin = vec![(0.0, 0.0), (60.0, 0.0), (60.0, 6.0), (0.0, 6.0), (0.0, 12.0), (60.0, 12.0)];
    let s = smooth_curve(&hairpin);
    let worst = s.windows(3).map(|w| radius(w[0], w[1], w[2])).fold(f32::INFINITY, f32::min);
    assert!(worst >= MIN_RADIUS * 0.75, "{worst}");
}
#[test] fn radius_of_a_known_circle() {
    let r = 50.0f32; let a = |t: f32| (r * t.cos(), r * t.sin());
    assert!((radius(a(0.0), a(0.2), a(0.4)) - r).abs() < 0.01);
    assert!(radius((0.0, 0.0), (1.0, 0.0), (2.0, 0.0)).is_infinite());
}
#[test] fn the_profile_keeps_the_end_heights_and_the_grade_limit() {
    let mut raw: Vec<f32> = (0..250).map(|i| ((i as f32) * 0.37).sin() * 6.0 + ((i as f32) * 0.05).cos() * 14.0).collect();
    raw[0] = 3.0; let last = raw.len() - 1; raw[last] = -4.0;
    let y = profile(&raw, 4.0);
    assert_eq!((y[0], y[last]), (3.0, -4.0));
    for w in y.windows(2) { assert!((w[1] - w[0]).abs() <= MAX_GRADE * 4.0 + 1e-3, "{}", (w[1] - w[0]).abs() / 4.0); }
}
#[test] fn ends_too_far_apart_in_height_raise_the_grade_limit_but_stay_below_a_quarter() {
    let raw = vec![0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 60.0];
    let y = profile(&raw, 4.0);
    assert_eq!((y[0], y[10]), (0.0, 60.0));
    for w in y.windows(2) { assert!((w[1] - w[0]).abs() / 4.0 <= 1.6 + 1e-3); }   // 60 m over 40 m: infeasible, the limit gives way
    let long = vec![0.0f32; 400]; let mut raw = long.clone(); raw[399] = 70.0;
    let y = profile(&raw, 4.0);
    assert!(y.windows(2).all(|w| (w[1] - w[0]).abs() / 4.0 <= 0.25), "a feasible case never needs more than 25 %");
}
#[test] fn the_profile_follows_gentle_terrain_closely() {
    let raw: Vec<f32> = (0..200).map(|i| i as f32 * 0.2).collect();   // a 5 % ramp
    let y = profile(&raw, 4.0);
    assert!(y.iter().zip(&raw).all(|(a, b)| (a - b).abs() < 0.6));
}
#[test] fn widths_stay_in_range_and_the_liaison_is_constant() {
    let p = smooth_curve(&zigzag());
    let l = widths(RoadKind::Liaison, &p, 4.0, 1);
    assert!(l.iter().all(|w| *w == 8.0) && l.len() == p.len());
    let long: Vec<(f32, f32)> = (0..400).map(|i| (i as f32 * 4.0, 0.0)).collect();
    let s = widths(RoadKind::Stage, &long, 4.0, 7);
    assert!(s.iter().all(|w| (4.6..=9.0).contains(w)), "{:?}", s.iter().cloned().fold((9.0f32, 0.0f32), |m, w| (m.0.min(w), m.1.max(w))));
    assert!(s.iter().cloned().fold(0.0f32, f32::max) - s.iter().cloned().fold(9.0f32, f32::min) > 1.5, "the width varies along the road");
    for w in s.windows(2) { assert!((w[1] - w[0]).abs() < 0.35, "width jumps"); }
}

fn line(n: usize) -> Vec<(f32, f32)> { (0..n).map(|i| (i as f32 * 4.0, 0.0)).collect() }
fn crest_cfg() -> crate::game::roads::cfg::CrestCfg { crate::game::roads::cfg::cfg().crest }
fn bumps(y: &[f32]) -> usize { (1..y.len() - 1).filter(|&i| y[i] > 0.5 && y[i] >= y[i - 1] && y[i] > y[i + 1]).count() }

#[test] fn crests_keep_the_ends_the_slope_cap_and_the_end_margin() {
    let xz = line(750); let mut y = vec![0.0f32; 750];
    add_crests(&mut y, &xz, 4.0, f32::MIN, 3, &crest_cfg());
    let c = crest_cfg();
    assert!(bumps(&y) >= 4, "{}", bumps(&y));
    assert_eq!((y[0], y[749]), (0.0, 0.0));
    for w in y.windows(2) { assert!((w[1] - w[0]).abs() / 4.0 <= c.max_slope + 1e-3, "{}", (w[1] - w[0]).abs() / 4.0); }
    let m = (c.keep_ends / 4.0) as usize;
    assert!(y[..m].iter().chain(&y[750 - m..]).all(|v| v.abs() < 1e-4), "a crest inside the end margin");
}
#[test] fn crests_are_deterministic_per_seed() {
    let xz = line(750);
    let run = |seed| { let mut y = vec![0.0f32; 750]; add_crests(&mut y, &xz, 4.0, f32::MIN, seed, &crest_cfg()); y };
    assert_eq!(run(5), run(5));
    assert_ne!(run(5), run(6));
}
#[test] fn no_crest_in_a_tight_bend_or_on_a_causeway() {
    // a half circle of radius 40 m in the middle of a long line
    let mut xz = line(300);
    let (cx, cz) = (xz[299].0, 40.0);
    for k in 1..=40 { let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * k as f32 / 40.0; xz.push((cx + 40.0 * a.cos(), cz + 40.0 * a.sin())); }
    let n = xz.len(); let mut y = vec![0.0f32; n];
    add_crests(&mut y, &xz, 4.0, f32::MIN, 9, &crest_cfg());
    let bend = 299..n.min(340);
    assert!(y[bend].iter().all(|v| v.abs() < 1e-4), "crest in the bend");
    // causeway: the floor is only 1 m below the road, crests are refused everywhere
    let xz = line(750); let mut y = vec![1.0f32; 750];
    add_crests(&mut y, &xz, 4.0, 0.0, 3, &crest_cfg());
    assert!(y.iter().all(|v| *v == 1.0));
}
#[test] fn tiny_inputs_are_left_alone() {
    for n in 0..4 { let xz = line(n); let mut y = vec![2.0f32; n]; add_crests(&mut y, &xz, 4.0, f32::MIN, 1, &crest_cfg()); assert!(y.iter().all(|v| *v == 2.0)); }
}
