use super::*;
use crate::game::{math::v3, props::tile_props, terrain::Terrain, world::{tile_of, WORLD_LIMIT}};

#[test] fn formulas() {
    assert!((time_limit_for(1200.0) - (1.6 * 1200.0 / 12.0 + 20.0)).abs() < 1e-3);
    assert!((reward_base_for(1000.0, 1) - 250.0).abs() < 1e-3);
    assert!((reward_base_for(1000.0, 2) - 350.0).abs() < 1e-3);
    assert!((reward_base_for(1000.0, 3) - 475.0).abs() < 1e-3);
    let s = v3(0., 0., 0.);
    assert!((path_length(s, &[v3(3., 0., 4.), v3(3., 0., 14.)]) - 15.0).abs() < 1e-4);
}
#[test] fn roll_is_deterministic_and_shaped() {
    let a = roll(77, 0, 0); let b = roll(77, 0, 0);
    assert_eq!(a.len(), 4);
    assert_eq!(a.iter().map(|m| m.kind).collect::<Vec<_>>(), vec![MissionKind::Cargo, MissionKind::Race, MissionKind::Cargo, MissionKind::Race]);
    for (x, y) in a.iter().zip(&b) { assert_eq!((x.seed, x.goal, x.checkpoints.len()), (y.seed, y.goal, y.checkpoints.len())); }
    let c = roll(77, 1, 0);
    assert!(a.iter().zip(&c).any(|(x, y)| x.seed != y.seed), "refresh must change the list");
}
#[test] fn cargo_has_one_point_race_has_five_to_eight() {
    for r in 0..20 { for m in roll(5, r, 0) {
        match m.kind {
            MissionKind::Cargo => assert_eq!(m.checkpoints, vec![m.goal]),
            MissionKind::Race => assert!((5..=8).contains(&m.checkpoints.len()), "{}", m.checkpoints.len()),
        }
        assert_eq!(*m.checkpoints.last().unwrap(), m.goal);
        assert!(m.seed <= 0xFF_FFFF);
    } }
}
#[test] fn generated_points_obey_the_rules_over_many_seeds() {
    for ps in 0..200u64 { for m in roll(ps * 7919, (ps % 5) as u32, 0) {
        let t = Terrain::new(m.seed);
        for p in &m.checkpoints {
            assert!(t.normal(p.x, p.z).y >= 0.94, "slope at {p:?} seed {ps}");
            assert!((p.y - t.height(p.x, p.z)).abs() < 1e-3);
            assert!(p.x.abs() <= WORLD_LIMIT - 40.0 && p.z.abs() <= WORLD_LIMIT - 40.0);
            let (tx, tz) = (tile_of(p.x), tile_of(p.z));
            for dz in -1..=1 { for dx in -1..=1 { for pr in tile_props(&t, tx + dx, tz + dz) {
                assert!((pr.pos.x - p.x).hypot(pr.pos.z - p.z) >= 8.0, "prop near point, seed {ps}");
            } } }
        }
        assert!((m.length_m - path_length(m.start, &m.checkpoints)).abs() < 1e-2);
        assert!((m.time_limit - time_limit_for(m.length_m)).abs() < 1e-3);
        assert!((m.reward_base - reward_base_for(m.length_m, m.difficulty)).abs() < 1e-3);
        let d = m.goal.x.hypot(m.goal.z);
        assert!(d >= 380.0 && d <= 1520.0, "goal distance {d}");
    } }
}
#[test] fn difficulty_is_gated_by_gold_medals() {
    for r in 0..200 { for m in roll(3, r, 0) { assert!(m.difficulty <= 2); } }
    let hard = (0..200).flat_map(|r| roll(3, r, 3)).filter(|m| m.difficulty == 3).count();
    assert!(hard > 0, "3-star missions should appear after 3 golds");
}
#[test] fn extreme_inputs_do_not_panic() {
    for (ps, r) in [(0u64, 0u32), (u64::MAX, u32::MAX), (1, u32::MAX)] { assert_eq!(roll(ps, r, 99).len(), 4); }
}
#[test] fn point_and_line_checks_accept_flat_ground() {
    let t = Terrain::flat();
    assert!(point_ok(&t, v3(100., 0., 100.)));
    assert!(line_ok(&t, v3(0., 0., 0.), v3(500., 0., 500.)));
    assert!(!point_ok(&t, v3(5000., 0., 0.)));
}
#[test] fn changing_difficulty_changes_the_mission_identity() {
    // when a gold-medal count flips a slot's difficulty, the level seed must change too, so results
    // recorded for the easier mission are not attached to the harder one
    let mut flipped = 0;
    for r in 0..60 {
        let (a, b) = (roll(3, r, 2), roll(3, r, 3));
        for (x, y) in a.iter().zip(&b) {
            if x.difficulty != y.difficulty { flipped += 1; assert_ne!(x.seed, y.seed, "round {r}"); }
        }
    }
    assert!(flipped > 0, "the probe must actually flip some difficulties");
}
