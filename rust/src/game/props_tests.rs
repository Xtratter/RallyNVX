use super::*;
use crate::game::terrain::TILE_SIZE;
#[test] fn props_are_reproducible() {
    let t = Terrain::new(8);
    let (a, b) = (tile_props(&t, 2, -3), tile_props(&t, 2, -3));
    assert_eq!(a.len(), b.len()); assert!(a.iter().zip(&b).all(|(p, q)| p.pos == q.pos && p.radius == q.radius));
}
#[test] fn none_near_spawn_and_none_on_steep_slopes() {
    let t = Terrain::new(21);
    for tx in -2..2 { for tz in -2..2 { for p in tile_props(&t, tx, tz) {
        assert!(p.pos.x.hypot(p.pos.z) >= MIN_PROP_DIST);
        assert!(t.normal(p.pos.x, p.pos.z).y >= 0.85);
        assert!((p.pos.y - t.height(p.pos.x, p.pos.z)).abs() < 1e-3);
    } } }
}
#[test] fn props_stay_inside_their_tile_and_exist() {
    let t = Terrain::new(4);
    let mut total = 0;
    for tx in 3..6 { for tz in 3..6 { let ps = tile_props(&t, tx, tz); total += ps.len();
        for p in ps { assert!(p.pos.x >= tx as f32 * TILE_SIZE && p.pos.x < (tx + 1) as f32 * TILE_SIZE); assert!(p.radius > 0.0 && p.height > 0.0); } } }
    assert!(total > 20, "{total}");
}
#[test] fn no_props_inside_cities_or_their_rim() {
    let t = Terrain::new(7);
    let s = t.sites()[0];
    let (c0x, c0z) = (crate::game::world::tile_of(s.centre.x - s.half - 120.0), crate::game::world::tile_of(s.centre.z - s.half - 120.0));
    let (c1x, c1z) = (crate::game::world::tile_of(s.centre.x + s.half + 120.0), crate::game::world::tile_of(s.centre.z + s.half + 120.0));
    let mut outside = 0;
    for tx in c0x..=c1x { for tz in c0z..=c1z { for p in tile_props(&t, tx, tz) {
        assert!(!t.in_city(p.pos.x, p.pos.z, 100.0), "prop inside the city rim at {:?}", p.pos);
        outside += 1;
    } } }
    assert!(outside > 0, "props still exist around the city");
}
#[test] fn no_prop_stands_in_a_lake() {
    let mut checked = 0;
    for seed in 1..=60u64 { let t = Terrain::new(seed); if t.water_level().is_none() { continue; }
        for tz in -4..4 { for tx in -4..4 { for p in tile_props(&t, tx, tz) { checked += 1; assert!(!t.is_water(p.pos.x, p.pos.z, 1.0), "seed {seed}"); } } } }
    assert!(checked > 1000);
}

#[test] fn the_stage_has_a_rocky_edge_four_to_eight_metres_out() {
    let base = Terrain::new(21);
    let net = crate::game::roads::gen::generate_with(21, &base, crate::game::roads::gen::GenCfg::coarse());
    let t = base.with_net(net);
    let stage = &t.roads().unwrap().roads[1];
    let (mut near, mut closer) = (0, 0);
    let mut tiles = std::collections::HashSet::new();
    for p in stage.pts.iter().step_by(8) { tiles.insert((crate::game::world::tile_of(p.pos.x), crate::game::world::tile_of(p.pos.z))); }
    for &(tx, tz) in &tiles { for p in tile_props(&t, tx, tz) {
        assert!(!t.near_road(p.pos.x, p.pos.z, 3.9), "a prop closer than 4 m to a road at {:?}", p.pos);
        if p.kind == PropKind::Rock { if let Some(h) = t.road_hit(p.pos.x, p.pos.z) { let e = h.dist - h.half; if (4.0..=8.0).contains(&e) { near += 1; } else { closer += 1; } } }
    } }
    let per_km = near as f32 / (stage.length() / 1000.0).max(1.0);
    assert!(per_km > 10.0, "only {near} rocks beside the stage ({per_km}/km), {closer} elsewhere");
    let again: usize = tiles.iter().map(|&(tx, tz)| tile_props(&t, tx, tz).len()).sum();
    let twice: usize = tiles.iter().map(|&(tx, tz)| tile_props(&t, tx, tz).len()).sum();
    assert_eq!(again, twice);
}
