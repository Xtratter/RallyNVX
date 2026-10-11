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
#[test] fn mesh_indices_are_valid_and_counts_match() {
    let t = Terrain::new(5);
    for lod in 0..=2u32 {
        let m = t.tile_mesh(-2, 3, lod);
        let n = (TILE_CELLS >> lod) + 1;
        assert!(m.verts.len() >= n * n);
        assert_eq!(m.idx.len() % 3, 0);
        assert!(m.idx.iter().all(|&i| (i as usize) < m.verts.len()));
        assert!(m.verts.iter().all(|v| v.iter().all(|c| c.is_finite())));
    }
}
#[test] fn neighbouring_tiles_share_edge_heights() {
    let t = Terrain::new(11);
    let a = t.tile_mesh(0, 0, 0); let b = t.tile_mesh(1, 0, 0);
    let n = TILE_CELLS + 1;
    for j in 0..n { assert_eq!(a.verts[j * n + TILE_CELLS][1], b.verts[j * n][1], "row {j}"); }
}
#[test] fn mesh_normals_are_unit_and_point_up() {
    let m = Terrain::new(2).tile_mesh(4, -1, 1);
    let n = (TILE_CELLS >> 1) + 1;
    for v in &m.verts[..n * n] { let l = (v[3] * v[3] + v[4] * v[4] + v[5] * v[5]).sqrt(); assert!((l - 1.0).abs() < 1e-3 && v[4] > 0.0); }
}
#[test] fn plateau_is_flat_inside_a_city_and_blends_out() {
    let t = Terrain::new(7);
    let s = t.sites()[0];
    for (dx, dz) in [(0.0, 0.0), (100.0, 80.0), (-200.0, 150.0), (250.0, -250.0), (-260.0, -260.0)] {
        let h = t.height(s.centre.x + dx, s.centre.z + dz);
        assert!((h - s.centre.y).abs() < 1e-3, "inside the city: {h} vs {}", s.centre.y);
    }
    let (x, z) = (s.centre.x + s.half + 101.0, s.centre.z);
    assert_eq!(t.height(x, z), t.raw_height(x, z));
    let mut prev = t.height(s.centre.x + s.half - 20.0, s.centre.z);
    for i in 1..400 {
        let h = t.height(s.centre.x + s.half - 20.0 + i as f32 * 0.5, s.centre.z);
        assert!(h.is_finite() && (h - prev).abs() < 6.0, "step {i}: {prev} -> {h}");
        prev = h;
    }
}
#[test] fn flat_terrain_has_no_cities_and_terrain_stays_copy() {
    assert!(Terrain::flat().sites().is_empty());
    let t = Terrain::new(3); let u = t; assert_eq!(t.height(10.0, 10.0), u.height(10.0, 10.0));
    assert_eq!(Terrain::flat().height(500.0, 500.0), 0.0);
}
#[test] fn the_spawn_plateau_is_still_flat_with_cities() {
    for seed in [1u64, 7, 99, 12345] { let t = Terrain::new(seed); for (x, z) in [(0.0, 0.0), (5.0, -4.0), (-8.0, 8.0)] { assert!(t.height(x, z).abs() < 0.5); } }
}
