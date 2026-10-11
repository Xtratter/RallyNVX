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
