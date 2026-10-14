use super::*;
use crate::game::city::{BLOCK, HALF, STREET};

fn site() -> CitySite { CitySite { centre: v3(1000.0, 5.0, -400.0), half: HALF } }
/// Is the point on a street (within half a street width of a grid line)?
fn on_street(s: &CitySite, p: V3) -> bool {
    let off = |v: f32, c: f32| { let r = (v - (c - s.half)).rem_euclid(PITCH); r.min(PITCH - r) };
    off(p.x, s.centre.x) <= STREET / 2.0 - 0.5 || off(p.z, s.centre.z) <= STREET / 2.0 - 0.5
}
fn gate(s: &CitySite, side: usize, k: i32) -> V3 {
    let along = -s.half + k as f32 * PITCH;
    match side { 0 => v3(s.centre.x - s.half, s.centre.y, s.centre.z + along), 1 => v3(s.centre.x + s.half, s.centre.y, s.centre.z + along),
                 2 => v3(s.centre.x + along, s.centre.y, s.centre.z - s.half), _ => v3(s.centre.x + along, s.centre.y, s.centre.z + s.half) }
}

#[test] fn every_pair_of_gates_gets_a_path_on_the_streets_with_gentle_corners() {
    let s = site();
    for sa in 0..4 { for sb in 0..4 { for (ka, kb) in [(3, 8), (5, 5), (8, 3)] {
        if sa == sb && ka == kb { continue; }
        let (a, b) = (gate(&s, sa, ka), gate(&s, sb, kb));
        let path = city_path(&s, a, b);
        assert!((path[0] - a).len() < 0.5 && (*path.last().unwrap() - b).len() < 0.5, "ends: sides {sa}->{sb}");
        for w in path.windows(2) { let d = (w[1] - w[0]).len(); assert!(d > 0.05 && d < 5.0, "spacing {d} sides {sa}->{sb}"); }
        assert!(path.iter().all(|p| on_street(&s, *p)), "sides {sa}->{sb} keys {ka}->{kb}: a point leaves the streets");
        let worst = path.windows(3).map(|w| crate::game::roads::shape::radius((w[0].x, w[0].z), (w[1].x, w[1].z), (w[2].x, w[2].z))).fold(f32::INFINITY, f32::min);
        assert!(worst >= CORNER * 0.7, "corner radius {worst} sides {sa}->{sb}");
        let _ = BLOCK;
    } } }
}
#[test] fn a_straight_crossing_is_straight() {
    let s = site();
    let path = city_path(&s, gate(&s, 0, 5), gate(&s, 1, 5));
    assert!(path.iter().all(|p| (p.z - path[0].z).abs() < 0.01));
    assert!((path.last().unwrap().x - path[0].x - 2.0 * HALF).abs() < 1.0);
}
