use super::*;
use crate::game::city::{HALF, STREET};
use crate::game::math::v3;
use crate::game::roads::cfg::cfg;

fn site() -> CitySite { CitySite { centre: v3(-500.0, 3.0, 900.0), half: HALF } }
fn on_street(s: &CitySite, p: V3) -> bool {
    let off = |v: f32, c: f32| { let r = (v - (c - s.half)).rem_euclid(PITCH); r.min(PITCH - r) };
    off(p.x, s.centre.x) <= STREET / 2.0 - 0.4 || off(p.z, s.centre.z) <= STREET / 2.0 - 0.4
}
/// Do two segments (xz) properly cross?
fn cross(a: V3, b: V3, c: V3, d: V3) -> bool {
    let o = |p: V3, q: V3, r: V3| (q.x - p.x) * (r.z - p.z) - (q.z - p.z) * (r.x - p.x);
    o(a, b, c) * o(a, b, d) < 0.0 && o(c, d, a) * o(c, d, b) < 0.0
}

#[test] fn the_loop_is_closed_on_the_streets_and_does_not_cross_itself() {
    let s = site();
    for seed in 1..=14u64 {
        let l = build(&s, seed, &cfg().city_loop);
        assert!((l.pts[0] - *l.pts.last().unwrap()).len() < 0.5, "seed {seed}: not closed");
        assert!(l.length > 900.0 && l.length < 2200.0, "seed {seed}: {} m", l.length);
        assert!(l.pts.iter().all(|p| on_street(&s, *p)), "seed {seed}: a point is off the streets");
        for w in l.pts.windows(2) { let d = (w[1] - w[0]).len(); assert!(d > 0.05 && d < 5.0, "seed {seed}: spacing {d}"); }
        let worst = l.pts.windows(3).map(|w| crate::game::roads::shape::radius((w[0].x, w[0].z), (w[1].x, w[1].z), (w[2].x, w[2].z))).fold(f32::INFINITY, f32::min);
        assert!(worst >= 5.5, "seed {seed}: corner radius {worst}");
        // no crossing between segments that are far apart along the loop
        let step: Vec<V3> = l.pts.iter().step_by(3).cloned().collect();
        for i in 0..step.len() - 1 { for j in i + 6..step.len() - 1 {
            if i == 0 && j + 1 >= step.len() - 1 { continue; }
            assert!(!cross(step[i], step[i + 1], step[j], step[j + 1]), "seed {seed}: the loop crosses itself at {i}/{j}");
        } }
    }
}
#[test] fn the_lines_stand_on_the_loop_and_the_first_is_the_start() {
    let s = site();
    let l = build(&s, 5, &cfg().city_loop);
    assert!(l.lines[0].kind == LineKind::Start && l.lines.iter().skip(1).all(|x| x.kind == LineKind::Check));
    assert!(l.lines.len() >= 2 && l.lines.len() as f32 <= l.length / cfg().city_loop.check_every + 2.0);
    for x in &l.lines {
        assert!(l.pts.iter().any(|p| (*p - x.pos).len() < 1.0));
        assert!((x.dir.len() - 1.0).abs() < 1e-3 && x.dir.y == 0.0);
    }
}
#[test] fn the_loop_is_deterministic_and_differs_between_seeds() {
    let s = site();
    let (a, b, c) = (build(&s, 3, &cfg().city_loop), build(&s, 3, &cfg().city_loop), build(&s, 4, &cfg().city_loop));
    assert_eq!(a.pts.len(), b.pts.len());
    assert!(a.pts.iter().zip(&b.pts).all(|(p, q)| *p == *q));
    assert!(a.pts.len() != c.pts.len() || a.pts[50] != c.pts[50]);
}
