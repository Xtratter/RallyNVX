//! Raster renderer (render mode C): tiled terrain, instanced props and vehicle, sky, sun shadow map.
pub mod citygen;
pub mod meshes;
pub mod models;
pub mod texgen;

/// Terrain tiles around the camera tile: which to build (nearest first) and which cached meshes to draw
/// (the wanted LOD, else any cached LOD of the same tile so there are never holes while a tile is rebuilt).
pub fn tile_plan(cx: i32, cz: i32, have: &std::collections::HashSet<(i32, i32, u32)>, radius: i32) -> (Vec<(i32, i32, u32)>, Vec<(i32, i32, u32)>) {
    let mut want: Vec<(i32, i32, u32, i32)> = Vec::new();
    for dz in -radius..=radius { for dx in -radius..=radius {
        let ring = dx.abs().max(dz.abs());
        want.push((cx + dx, cz + dz, lod_for_ring(ring), ring));
    } }
    want.sort_by_key(|w| w.3);
    let build = want.iter().filter(|w| !have.contains(&(w.0, w.1, w.2))).map(|w| (w.0, w.1, w.2)).collect();
    let visible = want.iter().filter_map(|w| {
        if have.contains(&(w.0, w.1, w.2)) { return Some((w.0, w.1, w.2)); }
        (0..=2u32).find(|&l| have.contains(&(w.0, w.1, l))).map(|l| (w.0, w.1, l))
    }).collect();
    (build, visible)
}

/// Terrain LOD by distance (in tiles) from the camera tile.
pub fn lod_for_ring(ring: i32) -> u32 { if ring <= 1 { 0 } else if ring == 2 { 1 } else { 2 } }

#[cfg(test)]
mod plan_tests {
    use super::*;
    use std::collections::HashSet;
    #[test] fn nearest_tiles_are_built_first() {
        let (build, visible) = tile_plan(5, -2, &HashSet::new(), 3);
        assert_eq!(build.len(), 49);
        assert_eq!((build[0].0, build[0].1), (5, -2));
        let ring = |t: &(i32, i32, u32)| (t.0 - 5).abs().max((t.1 + 2).abs());
        assert!(build.windows(2).all(|w| ring(&w[0]) <= ring(&w[1])));
        assert!(visible.is_empty());
    }
    #[test] fn old_lod_stays_visible_until_the_new_one_exists() {
        // tile (6,-2) is ring 1 (lod 0) now; only its old lod 1 mesh is cached
        let mut have = HashSet::new();
        have.insert((6, -2, 1));
        let (build, visible) = tile_plan(5, -2, &have, 3);
        assert!(build.contains(&(6, -2, 0)));
        assert!(visible.contains(&(6, -2, 1)) && !visible.contains(&(6, -2, 0)));
        have.insert((6, -2, 0));
        let (_, visible) = tile_plan(5, -2, &have, 3);
        assert!(visible.contains(&(6, -2, 0)) && !visible.contains(&(6, -2, 1)));
    }
    #[test] fn nothing_to_build_when_everything_is_cached() {
        let (all, _) = tile_plan(0, 0, &HashSet::new(), 3);
        let have: HashSet<_> = all.iter().copied().collect();
        let (build, visible) = tile_plan(0, 0, &have, 3);
        assert!(build.is_empty() && visible.len() == 49);
    }
}


#[cfg(any(target_os = "android", feature = "desktop"))]
mod gpu;
#[cfg(any(target_os = "android", feature = "desktop"))]
pub use gpu::Raster;
