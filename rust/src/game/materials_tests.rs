use super::*;
use crate::game::{math::v3, terrain::{Terrain, TILE_CELLS}};
fn sum(w: [u8; 4]) -> u32 { w.iter().map(|v| *v as u32).sum() }
#[test] fn weights_always_sum_to_255() {
    for i in 0..2000u32 {
        let (x, z) = (i as f32 * 13.7 - 9000.0, i as f32 * -7.3 + 2000.0);
        let h = (i as f32 * 0.37).sin() * 30.0 - 5.0;
        let n = v3((i as f32 * 0.11).sin() * 0.6, 1.0, (i as f32 * 0.07).cos() * 0.6).norm();
        assert_eq!(sum(terrain_weights(h, n, x, z)), 255, "i {i}");
    }
}
#[test] fn flat_low_ground_is_grass_and_steep_ground_is_rock() {
    let up = v3(0.0, 1.0, 0.0);
    let g = terrain_weights(3.0, up, 100.0, 100.0);
    assert!(g[0] as u32 + g[1] as u32 >= 200, "flat mid ground is grass/dirt: {g:?}");
    assert!(g[3] < 30, "no rock on flat ground: {g:?}");
    let steep = v3(0.7, 0.55, 0.0).norm();
    assert!(terrain_weights(3.0, steep, 100.0, 100.0)[3] > 180, "steep ground is rock");
    assert!(rock_dominant(3.0, steep, 100.0, 100.0) && !rock_dominant(3.0, up, 100.0, 100.0));
}
#[test] fn the_lowest_ground_is_sandy() {
    let up = v3(0.0, 1.0, 0.0);
    assert!(terrain_weights(-9.0, up, 50.0, 50.0)[2] > 150);
    assert!(terrain_weights(2.0, up, 50.0, 50.0)[2] < 20);
}
#[test] fn weights_change_smoothly_and_deterministically() {
    let up = v3(0.0, 1.0, 0.0);
    assert_eq!(terrain_weights(1.0, up, 10.0, 20.0), terrain_weights(1.0, up, 10.0, 20.0));
    let mut prev = terrain_weights(1.0, up, 0.0, 0.0);
    for i in 1..400 { let w = terrain_weights(1.0, up, i as f32 * 0.5, 0.0);
        for c in 0..4 { assert!((w[c] as i32 - prev[c] as i32).abs() <= 40, "jump at {i}: {prev:?} -> {w:?}"); } prev = w; }
    let mut prev = terrain_weights(30.0, v3(0.9, 0.44, 0.0).norm(), 0.0, 0.0);
    for i in 1..100 { let n = v3(0.9 - i as f32 * 0.008, 0.44 + i as f32 * 0.005, 0.0).norm(); let w = terrain_weights(30.0, n, 0.0, 0.0);
        for c in 0..4 { assert!((w[c] as i32 - prev[c] as i32).abs() <= 40, "slope jump at {i}"); } prev = w; }
}
#[test] fn tile_weights_match_the_mesh_for_every_lod_and_skirts_copy_their_parents() {
    let t = Terrain::new(5);
    for lod in 0..=2u32 {
        let (m, w) = t.tile_mesh_with_weights(-2, 3, lod);
        assert_eq!(w.len(), m.verts.len());
        assert!(w.iter().all(|x| sum(*x) == 255));
        let n = (TILE_CELLS >> lod) + 1;
        assert_eq!(m.verts.len(), t.tile_mesh(-2, 3, lod).verts.len());
        for i in 0..n { assert_eq!(w[n * n + i], w[i], "skirt vertex {i} lod {lod}"); }
    }
}
#[test] fn neighbouring_tiles_agree_on_their_shared_edge_weights() {
    let t = Terrain::new(11);
    let (_a, wa) = t.tile_mesh_with_weights(0, 0, 0); let (_b, wb) = t.tile_mesh_with_weights(1, 0, 0);
    let n = TILE_CELLS + 1;
    for j in 0..n { assert_eq!(wa[j * n + TILE_CELLS], wb[j * n], "row {j}"); }
}
