use super::*;
use crate::game::terrain::{Terrain, TILE_SIZE};
#[test] fn flat_terrain_has_no_water_mesh() { assert!(water_mesh(&Terrain::flat(), 0, 0).is_none()); }
#[test] fn a_flooded_tile_is_one_flat_grid_with_depth() {
    let m = water_mesh(&Terrain::flooded(1.5), 2, -1).expect("flooded tile");
    assert_eq!(m.verts.len(), (CELLS + 1) * (CELLS + 1));
    assert_eq!(m.idx.len(), CELLS * CELLS * 6);
    for v in &m.verts { assert!((v[1] - 1.5).abs() < 1e-6 && (v[3] - 1.5).abs() < 1e-6 && v[5] == 1.0); }
    let (x0, x1) = (2.0 * TILE_SIZE, 3.0 * TILE_SIZE);
    assert!(m.verts.iter().all(|v| v[0] >= x0 - 1e-3 && v[0] <= x1 + 1e-3));
}
#[test] fn real_levels_emit_only_cells_near_the_water_and_stay_valid() {
    let (mut with, mut without) = (0, 0);
    for seed in 1..=12u64 { let t = Terrain::new(seed); let level = t.water_level().unwrap();
        for tz in -6..6 { for tx in -6..6 {
            match water_mesh(&t, tx, tz) {
                None => { without += 1;
                    for j in 0..=CELLS { for i in 0..=CELLS { let (x, z) = (tx as f32 * TILE_SIZE + i as f32 * TILE_SIZE / CELLS as f32, tz as f32 * TILE_SIZE + j as f32 * TILE_SIZE / CELLS as f32);
                        assert!(level - t.height(x, z) <= -0.5 + 1e-3, "dry tile has a wet corner"); } } }
                Some(m) => { with += 1;
                    assert!(m.idx.len() % 6 == 0 && !m.idx.is_empty() && m.idx.iter().all(|&i| (i as usize) < m.verts.len()));
                    for v in &m.verts { assert!((v[1] - level).abs() < 1e-5); }
                    assert!(m.verts.iter().any(|v| v[3] > -0.5), "emitted a tile with no wet corner"); }
            } } } }
    assert!(with > 0 && without > 0, "with {with} without {without}");
}
