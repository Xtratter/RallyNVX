use super::*;
use crate::game::autopilot::cfg::cfg;

fn lim() -> Limits { Limits { mu: 0.7, v_max: 35.0, accel: 5.0, stop_at_end: false } }
fn straight(n: usize) -> Vec<V3> { (0..n).map(|i| v3(0.0, 0.0, i as f32 * 4.0)).collect() }
fn arc(radius: f32, deg: f32) -> Vec<V3> {   // a circular arc sampled every ~4 m, starting at the origin heading +z, turning left
    let steps = (radius * deg.to_radians() / 4.0) as usize;
    (0..=steps).map(|i| { let a = i as f32 * 4.0 / radius; v3(radius * (1.0 - a.cos()), 0.0, radius * a.sin()) }).collect()
}
fn max_kappa(r: &Route) -> f32 { r.kappa.iter().cloned().fold(0.0, f32::max) }

#[test] fn a_straight_gets_the_car_maximum_and_a_circle_its_grip_limit() {
    let c = cfg();
    let r = Route::build(&straight(300), &vec![8.0; 300], lim(), &c);
    assert!(r.speed[150] > 34.9, "{}", r.speed[150]);
    // a long circle of 100 m radius with a width of 5 m (no room to cut): v = sqrt(lat * mu * g * R)
    let pts = arc(100.0, 160.0);
    let r = Route::build(&pts, &vec![5.0; pts.len()], lim(), &c);
    let want = (c.lat_factor * 0.7 * 9.81 * 100.0f32).sqrt();
    let mid = r.speed[pts.len() / 2];
    assert!((mid - want).abs() < want * 0.12, "mid {mid} want {want}");
}
#[test] fn the_car_brakes_before_a_hairpin_and_accelerates_after_it() {
    let c = cfg();
    let mut pts = straight(80);   // 320 m
    let last = *pts.last().unwrap();
    for q in arc(25.0, 180.0).into_iter().skip(1) { pts.push(v3(last.x - q.x, 0.0, last.z + q.z)); }   // a left... mirrored hairpin
    let w = vec![6.0; pts.len()];
    let r = Route::build(&pts, &w, lim(), &c);
    let hp = r.speed.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(hp < 14.0, "hairpin speed {hp}");
    let i0 = r.speed.iter().position(|v| *v == hp).unwrap();
    // 60 m before the hairpin's slowest point the speed is at most what braking allows
    let j = (0..i0).rev().find(|&j| r.s[i0] - r.s[j] >= 60.0).unwrap();
    let a = c.brake_factor * 0.7 * 9.81;
    assert!(r.speed[j] <= (hp * hp + 2.0 * a * 60.0).sqrt() + 0.5, "{} vs {}", r.speed[j], (hp * hp + 2.0 * a * 60.0).sqrt());
    assert!(r.speed[0] <= r.speed[40] + 1e-3, "it accelerates from the start");
}
#[test] fn cutting_corners_stays_inside_the_corridor_and_barely_lengthens_the_route() {
    let c = cfg();
    let mut pts = straight(40);
    let last = *pts.last().unwrap();
    for q in arc(40.0, 90.0).into_iter().skip(1) { pts.push(v3(last.x + q.x, 0.0, last.z + q.z)); }
    let tail = *pts.last().unwrap();
    for i in 1..40 { pts.push(v3(tail.x + i as f32 * 4.0, 0.0, tail.z)); }
    let w = vec![8.0; pts.len()];
    let r = Route::build(&pts, &w, lim(), &c);
    let centre_len: f32 = pts.windows(2).map(|p| (p[1] - p[0]).len()).sum();
    assert!(r.length() <= centre_len * 1.02, "{} vs {centre_len}", r.length());
    let allowed = c.offset_max * (8.0 * 0.5 - 1.4) + 1e-3;
    for i in 0..pts.len() {
        let near = pts.iter().map(|q| (q.x - r.p[i].x).hypot(q.z - r.p[i].z)).fold(f32::INFINITY, f32::min);
        assert!(near <= allowed + 0.3, "point {i} is {near} m off the centre line (limit {allowed})");
    }
    let base = Route::build(&pts, &w, lim(), &AutoCfg { offset_max: 0.0, ..c });
    assert!(max_kappa(&r) < max_kappa(&base), "no flatter than the centre line: {} vs {}", max_kappa(&r), max_kappa(&base));
    assert!(r.p[0] == pts[0] || (r.p[0] - pts[0]).len() < 1e-3);
}
#[test] fn a_crest_slows_the_car_down_before_it() {
    let c = cfg();
    let pts: Vec<V3> = (0..250).map(|i| { let z = i as f32 * 4.0; let d = z - 500.0; let h = if d.abs() < 20.0 { 1.6 * 0.5 * (1.0 + (std::f32::consts::PI * d / 20.0).cos()) } else { 0.0 }; v3(0.0, h, z) }).collect();
    let r = Route::build(&pts, &vec![8.0; 250], lim(), &c);
    let top = r.speed[125];
    assert!(top <= c.crest_speed + 0.01, "{top}");
    assert!(r.speed[115] < 34.0 && r.speed[115] > top, "braking starts before the crest: {} {top}", r.speed[115]);
    assert!(r.speed[200] > top, "and the car speeds up again after it");
}
#[test] fn nearest_and_ahead() {
    let c = cfg();
    let r = Route::build(&straight(100), &vec![8.0; 100], lim(), &c);
    let (i, d) = r.nearest(1.0, 100.0);
    assert!((r.p[i].z - 100.0).abs() < 4.0 && (d - 1.0).abs() < 0.5);
    let (j, p) = r.ahead(10, 20.0);
    assert!((p.z - 60.0).abs() < 4.5 && j > 10);
    let (k, _) = r.ahead(95, 500.0);
    assert_eq!(k, 99);
}
#[test] fn stopping_at_the_end_brakes_in_time() {
    let c = cfg();
    let r = Route::build(&straight(200), &vec![8.0; 200], Limits { stop_at_end: true, ..lim() }, &c);
    assert_eq!(*r.speed.last().unwrap(), 0.0);
    let a = c.brake_factor * 0.7 * 9.81;
    for i in 0..199 { assert!(r.speed[i] * r.speed[i] <= 2.0 * a * (r.length() - r.s[i]) * 1.05 + 1.0, "point {i}"); }
}
