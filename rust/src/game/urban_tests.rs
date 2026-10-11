use super::*;
use crate::game::{city::{City, CitySite, HALF, PITCH}, math::v3, terrain::Terrain};
fn city(seed: u64) -> City { let t = Terrain::new(seed); City::build(t.sites()[0], seed) }
fn on_street(c: &City, p: V3) -> bool {
    let (u, w) = ((p.x - (c.site.centre.x - HALF)) / PITCH, (p.z - (c.site.centre.z - HALF)) / PITCH);
    let off = |v: f32| (v - v.round()).abs() * PITCH;
    (p.x - c.site.centre.x).abs() <= HALF + 0.01 && (p.z - c.site.centre.z).abs() <= HALF + 0.01 && (off(u) <= 6.0 || off(w) <= 6.0)
}
fn manhattan(a: V3, b: V3) -> f32 { (a.x - b.x).abs() + (a.z - b.z).abs() }

#[test] fn start_rows_are_inside_the_grid_and_the_start_is_a_free_west_intersection() {
    for key in 0..200u64 {
        let row = start_row(key); assert!((1..=10).contains(&row));
        let c = city(key % 7 + 1);
        for u in [cargo(&c, key), race(&c, key), rings(&c, key), sling(&c, key)] {
            assert_eq!(u.start, c.west_edge(row), "urban starts are the west edge intersection of the row");
            assert!(c.roof_at(u.start.x, u.start.z).is_none());
        }
    }
}
#[test] fn urban_cargo_goal_is_a_far_intersection_in_range() {
    for key in 0..300u64 {
        let c = city(key % 11 + 1); let u = cargo(&c, key);
        assert_eq!(u.points.len(), 1); assert_eq!(u.yaw, Some(std::f32::consts::FRAC_PI_2));
        let g = u.points[0];
        let d = manhattan(u.start, g);
        assert!(d >= 300.0 - 1e-3 && d <= 600.0 + 1e-3, "Manhattan {d}, key {key}");
        let (a, b) = ((g.x - (c.site.centre.x - HALF)) / PITCH, (g.z - (c.site.centre.z - HALF)) / PITCH);
        assert!((a - a.round()).abs() < 1e-3 && (b - b.round()).abs() < 1e-3, "an intersection");
        assert!(c.roof_at(g.x, g.z).is_none());
    }
}
#[test] fn urban_race_checkpoints_follow_straight_street_legs() {
    for key in 0..300u64 {
        let c = city(key % 11 + 1); let u = race(&c, key);
        assert!((6..=10).contains(&u.points.len()), "{} points, key {key}", u.points.len());
        let mut prev = u.start;
        for p in &u.points {
            let (dx, dz) = ((p.x - prev.x).abs(), (p.z - prev.z).abs());
            assert!((dx < 1e-3) != (dz < 1e-3), "each leg runs along one street: {prev:?} -> {p:?}");
            let len = dx.max(dz); assert!((len / PITCH - (len / PITCH).round()).abs() < 1e-3 && len >= PITCH - 1e-3, "leg length {len}");
            assert!(on_street(&c, *p) && c.roof_at(p.x, p.z).is_none());
            prev = *p;
        }
        assert!(u.points[0].x > u.start.x && (u.points[0].z - u.start.z).abs() < 1e-3, "the first leg runs east");
    }
}
#[test] fn urban_rings_hang_over_the_street_between_the_houses() {
    for key in 0..300u64 {
        let c = city(key % 11 + 1); let u = rings(&c, key);
        assert!((5..=8).contains(&u.points.len()), "{} rings, key {key}", u.points.len());
        assert_eq!(u.yaw, None);
        for p in &u.points {
            assert!(on_street(&c, *p), "ring centre over a street line: {p:?}");
            let h = p.y - c.site.centre.y; assert!(h >= 18.0 - 1e-3 && h <= 40.0 + 1e-3, "height {h}");
        }
        let first = u.points[0];
        assert!(first.x > u.start.x && (first.z - u.start.z).abs() < 1e-3, "ring 0 is the midpoint of the first (east) segment: {first:?}");
        assert!((first.x - (u.start.x + PITCH / 2.0)).abs() < 1e-3);
    }
}
#[test] fn sling_pads_are_on_roofs_within_the_distance_bands() {
    let mut exact = 0;
    for key in 0..300u64 {
        let c = city(key % 11 + 1); let u = sling(&c, key);
        assert_eq!(u.points.len(), 2); assert_eq!(u.yaw, None);
        for p in &u.points {
            let b = c.buildings.iter().find(|b| p.x >= b.min.x && p.x <= b.max.x && p.z >= b.min.z && p.z <= b.max.z).expect("a pad must be on a building");
            assert!((p.y - b.max.y).abs() < 1e-3 && b.max.y - c.site.centre.y >= 20.0 - 1e-3 && b.size().x >= 14.0 - 1e-3 && b.size().z >= 14.0 - 1e-3);
        }
        assert_ne!(u.points[0], u.points[1]);
        let (da, ab) = ((u.points[0].x - u.start.x).hypot(u.points[0].z - u.start.z), (u.points[1].x - u.points[0].x).hypot(u.points[1].z - u.points[0].z));
        if da <= 300.0 && ab >= 200.0 && ab <= 600.0 { exact += 1; }
    }
    assert!(exact > 270, "the distance bands should hold almost always: {exact}/300");
}
#[test] fn generators_are_deterministic_and_never_panic_for_extreme_keys() {
    let c = city(7);
    for key in [0u64, 1, u64::MAX, 0xFF_FFFF] {
        assert_eq!(cargo(&c, key).points, cargo(&c, key).points);
        assert_eq!(race(&c, key).points, race(&c, key).points);
        let _ = (rings(&c, key), sling(&c, key));
    }
    let tiny = City::from_buildings(CitySite { centre: v3(0., 0., 0.), half: HALF }, vec![]);
    let _ = (cargo(&tiny, 1), race(&tiny, 1), rings(&tiny, 1));
    assert_eq!(sling(&tiny, 1).points.len(), 2);
}
