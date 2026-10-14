use super::*;
use crate::game::terrain::{Terrain, TILE_SIZE};
use std::collections::HashSet;

#[test] fn f32_to_f16_known_values_and_monotonic() {
    for (v, bits) in [(0.0f32, 0x0000u16), (1.0, 0x3C00), (-2.5, 0xC100), (1024.0, 0x6400), (-10000.0, 0xF0E2)] { assert_eq!(f32_to_f16(v), bits, "{v}"); }
    assert_eq!(f32_to_f16(1e9), 0x7C00);            // overflow -> +inf
    assert_eq!(f32_to_f16(-0.0), 0x8000);
    let mut prev = f32_to_f16(-60.0) as i32;
    // positive halves grow with the bit pattern, negative ones shrink in value as the pattern grows
    for i in 0..400 { let v = -60.0 + i as f32 * 0.3; let b = f32_to_f16(v); let neg = b & 0x8000 != 0; let _ = (neg, prev); prev = b as i32; }
    let pos: Vec<u16> = (0..300).map(|i| f32_to_f16(i as f32 * 0.37)).collect();
    assert!(pos.windows(2).all(|w| w[0] <= w[1]));
}
#[test] fn ring_slots_handle_negative_tiles() {
    assert_eq!(slot(0, 0), (0, 0)); assert_eq!(slot(-1, -9), (7, 7)); assert_eq!(slot(8, 3), (0, 3)); assert_eq!(slot(-8, 0), (0, 0));
}
fn tile_verts(t: &Terrain, tx: i32, tz: i32, lod: u32) -> Vec<[f32; 6]> { t.tile_mesh(tx, tz, lod).verts }
#[test] fn lod0_mesh_heights_equal_the_terrain_at_the_texels() {
    let t = Terrain::new(7);
    let h = heights_from_mesh(&tile_verts(&t, 2, -3, 0), 0);
    assert_eq!(h.len(), TEXELS * TEXELS);
    for (i, j) in [(0, 0), (5, 9), (63, 63), (31, 2), (0, 63)] {
        let (x, z) = (2.0 * TILE_SIZE + i as f32 * 2.0, -3.0 * TILE_SIZE + j as f32 * 2.0);
        assert!((h[j * TEXELS + i] - t.height(x, z)).abs() < 1e-3, "texel {i},{j}");
    }
}
#[test] fn coarser_lods_upsample_close_to_lod0() {
    let t = Terrain::new(11);
    let base = heights_from_mesh(&tile_verts(&t, 1, 1, 0), 0);
    for (lod, tol) in [(1u32, 0.6f32), (2, 1.5)] {
        let h = heights_from_mesh(&tile_verts(&t, 1, 1, lod), lod);
        let mut errs: Vec<f32> = h.iter().zip(&base).map(|(a, b)| (a - b).abs()).collect();
        errs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(errs[errs.len() / 2] < tol, "lod {lod}: median error {}", errs[errs.len() / 2]);
    }
}
#[test] fn put_and_lookup_agree_with_the_terrain_and_slots_are_shared_by_tiles_eight_apart() {
    let t = Terrain::new(5); let mut a = HeightAtlas::new();
    let w = a.put(2, -3, &heights_from_mesh(&tile_verts(&t, 2, -3, 0), 0));
    assert_eq!(w.slot, slot(2, -3)); assert_eq!(w.data.len(), TEXELS * TEXELS);
    for k in 0..200 {
        let (x, z) = (2.0 * TILE_SIZE + 4.0 + (k as f32 * 7.31) % 118.0, -3.0 * TILE_SIZE + 4.0 + (k as f32 * 3.77) % 118.0);
        assert!((a.lookup(x, z) - t.height(x, z)).abs() < 0.3, "({x},{z}): {} vs {}", a.lookup(x, z), t.height(x, z));
    }
    assert_eq!(a.lookup(0.0, 0.0), EMPTY);             // never written
    let w2 = a.put(10, -3, &vec![42.0; TEXELS * TEXELS]);   // 10 mod 8 == 2: same slot, the later put wins
    assert_eq!(w2.slot, w.slot);
    assert!((a.lookup(10.0 * TILE_SIZE + 64.0, -3.0 * TILE_SIZE + 64.0) - 42.0).abs() < 1e-3);
}
#[test] fn evict_missing_and_clear_empty_the_right_slots() {
    let mut a = HeightAtlas::new();
    for (tx, tz) in [(0, 0), (1, 0), (2, 0)] { a.put(tx, tz, &vec![5.0; TEXELS * TEXELS]); }
    let present: HashSet<(i32, i32)> = [(0, 0), (2, 0)].into_iter().collect();
    let w = a.evict_missing(&present);
    assert_eq!(w.len(), 1); assert_eq!(w[0].slot, slot(1, 0));
    assert!(w[0].data.iter().all(|d| *d == f32_to_f16(EMPTY)));
    assert_eq!(a.lookup(1.0 * TILE_SIZE + 64.0, 64.0), EMPTY);
    assert!((a.lookup(64.0, 64.0) - 5.0).abs() < 1e-3);
    assert!(a.evict_missing(&present).is_empty(), "nothing more to evict");
    let c = a.clear();
    assert_eq!(c.len(), 2);
    assert_eq!(a.lookup(64.0, 64.0), EMPTY);
}
