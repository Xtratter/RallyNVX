use crate::game::{flight::AircraftKind, missions::{path_length, ring_normal, roll, MissionKind}, terrain::Terrain, world::WORLD_LIMIT};

#[test] fn air_missions_carry_the_right_aircraft_and_valid_points() {
    for ps in 0..200u64 { for r in 0..2u32 { for m in roll(ps * 7919, r, 0).into_iter().filter(|m| m.is_air()) {
        let t = Terrain::new(m.seed);
        assert!(m.aircraft.is_some());
        match m.kind { MissionKind::Sling => assert_eq!(m.aircraft, Some(AircraftKind::Heli)), MissionKind::Recon => assert_eq!(m.aircraft, Some(AircraftKind::Drone)), _ => {} }
        for p in &m.checkpoints {
            assert!(p.x.abs() <= WORLD_LIMIT - 40.0 && p.z.abs() <= WORLD_LIMIT - 40.0, "wall, seed {ps}");
            let agl = p.y - t.height(p.x, p.z);
            match m.kind {
                MissionKind::Sling => assert!(agl.abs() < 1e-3 && t.normal(p.x, p.z).y >= 0.94, "pad on gentle ground, seed {ps}"),
                _ => assert!(agl >= 14.99, "air point at {agl} m, seed {ps}"),
            }
        }
        match m.kind {
            MissionKind::Rings => { let hmax = (if m.aircraft == Some(AircraftKind::Drone) { 120.0 } else { 90.0f32 }).max(112.0);   // rings are lifted above tall buildings
                assert!(m.checkpoints.iter().all(|p| p.y - t.height(p.x, p.z) <= hmax + 0.01)); assert!(m.length_m >= 450.0 && m.length_m <= 1700.0, "{}", m.length_m); }
            MissionKind::Sling => { let d = (m.checkpoints[1] - m.checkpoints[0]).len(); assert!(d >= 350.0 && d <= 1350.0, "{d}"); }
            MissionKind::Recon => { assert!(m.checkpoints.iter().all(|p| p.y - t.height(p.x, p.z) <= 100.01)); }
            _ => unreachable!(),
        }
        assert_eq!(*m.checkpoints.last().unwrap(), m.goal);
        assert!((m.length_m - path_length(m.start, &m.checkpoints)).abs() < 1e-2);
        let want = match m.kind {
            MissionKind::Rings => { let v = if m.aircraft == Some(AircraftKind::Drone) { 15.0 } else { 18.0 }; 1.5 * m.length_m / v + 20.0 }
            MissionKind::Sling => 1.6 * m.length_m / 12.0 + 35.0,
            _ => 1.4 * m.length_m / 15.0 + 25.0,
        };
        assert!((m.time_limit - want).abs() < 1e-2, "{:?} {} vs {}", m.kind, m.time_limit, want);
    } } }
}
#[test] fn ring_normals_follow_the_path() {
    let m = roll(11, 0, 0).into_iter().find(|m| m.kind == MissionKind::Rings).unwrap();
    for i in 0..m.checkpoints.len() {
        let n = ring_normal(&m, i);
        assert!((n.len() - 1.0).abs() < 1e-3);
        let prev = if i == 0 { m.start } else { m.checkpoints[i - 1] };
        assert!(n.dot((m.checkpoints[i] - prev).norm()) > 0.999);
    }
}
#[test] fn sling_and_recon_alternate_by_round_parity() {
    for r in 0..10u32 {
        assert_eq!(roll(5, r, 0)[5].kind, if r % 2 == 0 { MissionKind::Sling } else { MissionKind::Recon }, "round {r}");
    }
}
#[test] fn rings_aircraft_varies_with_the_seed() {
    let kinds: std::collections::HashSet<_> = (0..40u64).map(|ps| roll(ps * 31, 0, 0)[4].aircraft.unwrap() as u8).collect();
    assert_eq!(kinds.len(), 2, "both heli and drone rings should occur");
}
use crate::game::city::City;
#[test] fn rings_never_intersect_buildings() {
    let mut checked = 0;
    for ps in 0..300u64 { for m in roll(ps * 7919, (ps % 3) as u32, 0).into_iter().filter(|m| m.kind == MissionKind::Rings) {
        let t = Terrain::new(m.seed);
        let cities: Vec<City> = t.sites().iter().map(|s| City::build(*s, m.seed)).collect();
        let r = if m.aircraft == Some(AircraftKind::Drone) { 6.0 } else { 8.0 };
        for p in &m.checkpoints { for c in &cities { checked += 1; assert!(c.push_out(*p, r).is_none(), "ring at {p:?} touches a building, seed {ps}"); } }
    } }
    assert!(checked > 500, "{checked}");
}
