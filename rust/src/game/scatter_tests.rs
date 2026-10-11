use super::*;
use crate::game::{materials::terrain_weights, terrain::Terrain};
fn seeds() -> impl Iterator<Item = u64> { 1..=25u64 }
#[test] fn bushes_are_deterministic_and_stay_on_gentle_dry_ground() {
    let mut total = 0;
    for seed in seeds() { let t = Terrain::new(seed);
        for tz in -3..3 { for tx in -3..3 {
            let a = tile_bushes(&t, tx, tz); let b = tile_bushes(&t, tx, tz);
            assert_eq!(a.len(), b.len()); assert!(a.len() <= 6);
            for (x, y) in a.iter().zip(&b) { assert_eq!((x.pos.x, x.pos.z, x.scale), (y.pos.x, y.pos.z, y.scale)); }
            for bush in &a { total += 1;
                assert!(!t.in_city(bush.pos.x, bush.pos.z, 0.0) && !t.is_water(bush.pos.x, bush.pos.z, 0.5));
                assert!(t.normal(bush.pos.x, bush.pos.z).y >= 0.85 && bush.scale >= 0.7 && bush.scale <= 1.6);
                assert!((bush.pos.y - t.height(bush.pos.x, bush.pos.z)).abs() < 1e-3);
            } } } }
    assert!(total > 100, "{total}");
}
#[test] fn grass_spots_follow_the_rules_and_the_cap() {
    let mut any = false;
    for seed in seeds() { let t = Terrain::new(seed); let c = v3(40.0, 0.0, -30.0);
        let spots = grass_spots(&t, c, 40.0, 3000);
        assert!(spots.len() <= 3000);
        assert_eq!(spots.len(), grass_spots(&t, c, 40.0, 3000).len());
        for (p, s) in &spots { any = true;
            assert!((p.x - c.x).hypot(p.z - c.z) <= 40.5 && *s >= 0.8 && *s <= 1.4);
            assert!(!t.in_city(p.x, p.z, 0.0) && !t.is_water(p.x, p.z, 0.3));
            let n = t.normal(p.x, p.z); assert!(n.y >= 0.8);
            assert!(terrain_weights(t.height(p.x, p.z), n, p.x, p.z)[0] >= 128);
        }
        assert!(grass_spots(&t, c, 40.0, 100).len() <= 100);
        assert!(grass_spots(&t, c, 40.0, 0).is_empty());
    }
    assert!(any, "no grass anywhere");
}
