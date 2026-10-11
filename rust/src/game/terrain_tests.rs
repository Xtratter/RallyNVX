use super::*;
#[test] fn same_seed_same_heights() {
    let (a, b) = (Terrain::new(7), Terrain::new(7));
    for i in 0..50 { let (x, z) = (i as f32 * 37.1 - 900.0, i as f32 * -23.7 + 400.0); assert_eq!(a.height(x, z), b.height(x, z)); }
}
#[test] fn different_seeds_differ() {
    let (a, b) = (Terrain::new(1), Terrain::new(2));
    let diff = (0..50).filter(|i| { let x = 300.0 + *i as f32 * 11.0; (a.height(x, 250.0) - b.height(x, 250.0)).abs() > 0.01 }).count();
    assert!(diff > 25);
}
#[test] fn spawn_area_is_flat() {
    let t = Terrain::new(99);
    for (x, z) in [(0.0, 0.0), (5.0, -4.0), (-8.0, 8.0)] { assert!(t.height(x, z).abs() < 0.5, "{x} {z} {}", t.height(x, z)); }
}
#[test] fn heights_stay_in_bounds() {
    let t = Terrain::new(3);
    for i in 0..400 { let h = t.height(i as f32 * 53.0 - 10000.0, i as f32 * 29.0); assert!(h.abs() <= 52.0, "{h}"); }
}
#[test] fn negative_tiles_and_extreme_seeds_are_finite() {
    for seed in [0u64, 1, u64::MAX] {
        let t = Terrain::new(seed);
        for (x, z) in [(-5.0 * TILE_SIZE, -9.0 * TILE_SIZE), (-1.0, -1.0), (3999.0, -3999.0)] {
            assert!(t.height(x, z).is_finite()); assert!(t.normal(x, z).len() > 0.99);
        }
        assert!((0.0..1.0).contains(&hash01(seed, -7, -3)));
    }
}
#[test] fn flat_terrain_is_zero_with_up_normal() {
    let t = Terrain::flat();
    assert_eq!(t.height(123.0, -45.0), 0.0);
    assert!((t.normal(1.0, 1.0).y - 1.0).abs() < 1e-6);
}
