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
    assert_eq!(a.len(), 6);
    assert_eq!(a.iter().map(|m| m.kind).collect::<Vec<_>>(), vec![MissionKind::Cargo, MissionKind::Race, MissionKind::Cargo, MissionKind::Race, MissionKind::Rings, MissionKind::Sling]);
    assert_eq!(roll(77, 1, 0)[5].kind, MissionKind::Recon);
    for (x, y) in a.iter().zip(&b) { assert_eq!((x.seed, x.goal, x.checkpoints.len()), (y.seed, y.goal, y.checkpoints.len())); }
    let c = roll(77, 1, 0);
    assert!(a.iter().zip(&c).any(|(x, y)| x.seed != y.seed), "refresh must change the list");
}
#[test] fn cargo_has_one_point_race_has_five_to_eight() {
    for r in 0..20 { for m in roll(5, r, 0) {
        match m.kind {
            MissionKind::Cargo => assert_eq!(m.checkpoints, vec![m.goal]),
            MissionKind::Race if m.urban => assert!((6..=10).contains(&m.checkpoints.len()), "{}", m.checkpoints.len()),
            MissionKind::Race | MissionKind::Rings => assert!((5..=8).contains(&m.checkpoints.len()), "{}", m.checkpoints.len()),
            MissionKind::Sling => assert_eq!(m.checkpoints.len(), 2),
            MissionKind::Recon => assert!((3..=5).contains(&m.checkpoints.len())),
        }
        assert_eq!(*m.checkpoints.last().unwrap(), m.goal);
        assert!(m.seed <= 0xFF_FFFF);
    } }
}
#[test] fn generated_points_obey_the_rules_over_many_seeds() {
    for ps in 0..200u64 { for m in roll(ps * 7919, (ps % 5) as u32, 0).into_iter().filter(|m| !m.is_air() && !m.urban) {
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
    for (ps, r) in [(0u64, 0u32), (u64::MAX, u32::MAX), (1, u32::MAX)] { assert_eq!(roll(ps, r, 99).len(), 6); }
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
#[test] fn ground_mission_points_stay_out_of_cities() {
    for ps in 0..200u64 { for m in roll(ps * 7919, (ps % 5) as u32, 0).into_iter().filter(|m| !m.is_air() && !m.urban) {
        let t = Terrain::new(m.seed);
        for p in &m.checkpoints { assert!(!t.in_city(p.x, p.z, 20.0), "point {p:?} inside a city, seed {ps}"); }
    } }
}
#[test] fn the_list_has_the_urban_slots_and_open_missions_are_unchanged() {
    for ps in 0..100u64 {
        let l = roll(ps * 7919, (ps % 4) as u32, 0);
        assert_eq!(l.iter().map(|m| m.urban).collect::<Vec<_>>()[..4], [false, false, true, true], "slots 0-3");
        assert!(l[5].urban == (l[5].kind == MissionKind::Sling), "sling is urban, recon is open");
        for m in l.iter().filter(|m| m.urban) {
            let t = Terrain::new(m.seed); let c = crate::game::city::City::build(t.sites()[0], m.seed);
            assert!((m.start.x - c.site.centre.x).abs() <= c.site.half + 0.01 && c.roof_at(m.start.x, m.start.z).is_none(), "urban start in the city, seed {ps}");
            assert!(m.start_yaw.is_some() == matches!(m.kind, MissionKind::Cargo | MissionKind::Race));
        }
        for m in l.iter().filter(|m| !m.urban) { assert!(m.start_yaw.is_none() && m.start.x == 0.0 && m.start.z == 0.0); }
    }
    let rings_urban = (0..200u64).filter(|ps| roll(ps * 31, 0, 0)[4].urban).count();
    assert!(rings_urban > 60 && rings_urban < 140, "about half of the rings are urban: {rings_urban}");
}
#[test] fn urban_time_limits_follow_the_formulas() {
    for ps in 0..100u64 { for m in roll(ps * 7919, (ps % 2) as u32, 0).into_iter().filter(|m| m.urban) {
        let want = match m.kind {
            MissionKind::Cargo | MissionKind::Race => 1.6 * m.length_m / 8.0 + 20.0,
            MissionKind::Rings => 1.5 * (24.0 + 48.0 * (m.checkpoints.len() as f32 - 1.0)) / 10.0 + 20.0,   // the street length, not the diagonal
            _ => 1.6 * m.length_m / 12.0 + 35.0 + 0.25 * (m.checkpoints.iter().map(|p| p.y).fold(f32::MIN, f32::max) - m.start.y),   // plus the climb to the roofs
        };
        assert!((m.time_limit - want).abs() < 1e-2, "{:?} {} vs {}", m.kind, m.time_limit, want);
        assert!((m.length_m - path_length(m.start, &m.checkpoints)).abs() < 1e-2);
    } }
}
#[test] fn urban_missions_never_panic_for_extreme_arguments() {
    for (ps, r, g) in [(0u64, 0u32, 0u32), (u64::MAX, u32::MAX, u32::MAX), (1, 1, 99)] { assert_eq!(roll(ps, r, g).len(), 6); }
}
#[test] fn urban_ring_normals_are_flat_and_the_urban_seed_is_salted() {
    for ps in 0..100u64 { for m in roll(ps * 31, 0, 0).into_iter().filter(|m| m.urban && m.kind == MissionKind::Rings) {
        for i in 0..m.checkpoints.len() { let n = ring_normal(&m, i); assert!(n.y == 0.0 && (n.len() - 1.0).abs() < 1e-5, "{n:?}"); }
    } }
    for k in 0..50u64 { for d in 1..=3u8 { assert_ne!(mission_seed(k * 977, d, true), mission_seed(k * 977, d, false)); } }
}
#[test] fn ground_mission_points_are_never_in_a_lake() {
    let mut wet_levels = 0;
    for ps in 1..=40u64 { for m in roll(ps, 0, 0) {
        if m.is_air() { continue; }
        let t = Terrain::new(m.seed);
        if t.water_level().is_some() { wet_levels += 1; }
        for p in std::iter::once(&m.start).chain(m.checkpoints.iter()) { assert!(!t.is_water(p.x, p.z, 1.5), "profile {ps} mission seed {}: point in water", m.seed); }
    } }
    assert!(wet_levels > 0);
}
#[test] fn a_wet_point_is_not_ok() {
    for seed in 1..=60u64 { let t = Terrain::new(seed); let Some(l) = t.water_level() else { continue };
        for i in -20..20 { for j in -20..20 { let (x, z) = (i as f32 * 60.0, j as f32 * 60.0);
            if t.height(x, z) < l - 0.5 { assert!(!point_ok(&t, v3(x, t.height(x, z), z))); return; } } } }
    panic!("no wet point found");
}
