use super::*;
use crate::game::{city::HALF, roads::RoadKind, terrain::Terrain};

fn on_square_edge(t: &Terrain, p: crate::game::math::V3) -> bool {
    t.sites().iter().any(|s| {
        let (dx, dz) = ((p.x - s.centre.x).abs(), (p.z - s.centre.z).abs());
        ((dx - HALF).abs() < 2.0 && dz <= HALF + 2.0) || ((dz - HALF).abs() < 2.0 && dx <= HALF + 2.0)
    })
}
fn net(seed: u64) -> (Terrain, RoadNet) { let t = Terrain::new(seed); let n = generate_with(seed, &t, GenCfg::coarse()); (t, n) }

#[test] fn every_seed_gets_a_liaison_and_a_stage_with_sound_geometry() {
    let mut in_target = 0;
    for seed in 1..=14u64 {
        let (t, n) = net(seed);
        assert_eq!(n.roads.len(), 2, "seed {seed}");
        let (lia, stage) = (&n.roads[0], &n.roads[1]);
        assert_eq!((lia.kind, stage.kind), (RoadKind::Liaison, RoadKind::Stage));
        for r in &n.roads {
            for w in r.pts.windows(2) {
                let d = (w[1].pos - w[0].pos);
                let flat = d.x.hypot(d.z);
                assert!(flat > 3.0 && flat < 5.0, "seed {seed} spacing {flat}");
                assert!((w[1].s - w[0].s - flat).abs() < 1e-2, "s is arc length");
                assert!(d.y.abs() / flat <= 0.25 + 1e-3, "seed {seed} grade {}", d.y.abs() / flat);
            }
            assert!(r.pts.iter().all(|p| p.pos.x.abs() <= 3900.0 && p.pos.z.abs() <= 3900.0 && p.pos.is_finite()), "seed {seed} bounds");
            let radii: Vec<f32> = r.pts.windows(3).map(|w| super::super::shape::radius((w[0].pos.x, w[0].pos.z), (w[1].pos.x, w[1].pos.z), (w[2].pos.x, w[2].pos.z))).collect();
            assert!(radii.iter().cloned().fold(f32::INFINITY, f32::min) >= 10.0, "seed {seed} tight bend {}", radii.iter().cloned().fold(f32::INFINITY, f32::min));
            let wet = r.pts.iter().filter(|p| t.is_water(p.pos.x, p.pos.z, 0.0)).count();
            assert!(wet * 12 <= r.pts.len(), "seed {seed}: {wet}/{} road points over a lake", r.pts.len());
            if let Some(l) = t.water_level() { assert!(r.pts.iter().all(|p| p.pos.y >= l + 0.2), "seed {seed}: a road point under the water"); }
        }
        assert!(lia.pts[0].pos.x.hypot(lia.pts[0].pos.z) < 1.0, "the liaison starts at the spawn");
        assert!(on_square_edge(&t, lia.pts.last().unwrap().pos), "seed {seed}: the liaison ends on the city boundary");
        assert!(on_square_edge(&t, stage.pts[0].pos), "seed {seed}: the stage starts on the city boundary");
        let (la, sa) = (lia.pts.last().unwrap().pos, stage.pts[0].pos);
        assert!((la - sa).len() >= 40.0, "seed {seed}: the gates differ");
        assert!(stage.length() > 3000.0 && stage.length() < 14_000.0, "seed {seed}: stage {} m", stage.length());
        if stage.length() > 5500.0 && stage.length() < 11_000.0 { in_target += 1; }
        // no doubling back: over 40 m steps (10 points) the heading never turns by more than 150 degrees
        for r in &n.roads { for i in 10..r.pts.len() - 10 {
            let (a, b, c) = (r.pts[i - 10].pos, r.pts[i].pos, r.pts[i + 10].pos);
            let (u, v) = ((b.x - a.x, b.z - a.z), (c.x - b.x, c.z - b.z));
            let cos = (u.0 * v.0 + u.1 * v.1) / (u.0.hypot(u.1) * v.0.hypot(v.1)).max(1e-3);
            assert!(cos > -0.87, "seed {seed}: the road turns back at point {i} (cos {cos})");
        } }
        assert!(stage.pts.iter().all(|p| (4.5..=9.1).contains(&p.width)) && lia.pts.iter().all(|p| p.width == 8.0));
    }
    assert!(in_target >= 10, "only {in_target}/14 stages are 5.5-11 km");
}
#[test] fn generation_is_deterministic_and_seeds_differ() {
    let (_, a) = net(7); let (_, b) = net(7); let (_, c) = net(8);
    assert_eq!(a.roads[1].pts.len(), b.roads[1].pts.len());
    assert!(a.roads[1].pts.iter().zip(&b.roads[1].pts).all(|(p, q)| p.pos == q.pos && p.width == q.width));
    assert!(a.roads[1].pts.len() != c.roads[1].pts.len() || (a.roads[1].pts[100].pos - c.roads[1].pts[100].pos).len() > 1.0);
}
#[test] fn the_stage_is_not_a_straight_line() {
    for seed in [3u64, 9, 21] {
        let (_, n) = net(seed); let s = &n.roads[1];
        let chord = (s.pts.last().unwrap().pos - s.pts[0].pos).len();
        assert!(s.length() > chord * 1.15, "seed {seed}: length {} chord {chord}", s.length());
    }
}
#[test] fn the_real_resolution_works_too() {
    for seed in [4u64, 12] {
        let t = Terrain::new(seed);
        let n = generate(seed, &t);
        assert_eq!(n.roads.len(), 2);
        assert!(n.roads[1].length() > 3000.0 && n.roads[1].pts.windows(2).all(|w| (w[1].pos - w[0].pos).len() < 5.5));
    }
}
#[test] fn a_flat_test_terrain_has_no_roads() {
    assert!(generate_with(1, &Terrain::flat(), GenCfg::coarse()).is_empty());
}
