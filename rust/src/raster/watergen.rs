//! Lake surfaces: a flat grid per terrain tile at the water level, vertex `[x, level, z, depth, 0, 1]` where depth is
//! `level - ground` (negative on the shore side, so the fragment shader can fade the water out where it meets land).
use crate::game::terrain::{Mesh, Terrain, TILE_SIZE};

pub const CELLS: usize = 32;
/// Cells whose every corner is more than this far above the water are not emitted.
const SHORE_MARGIN: f32 = 0.5;

pub fn water_mesh(t: &Terrain, tx: i32, tz: i32) -> Option<Mesh> {
    let level = t.water_level()?;
    let step = TILE_SIZE / CELLS as f32;
    let (ox, oz) = (tx as f32 * TILE_SIZE, tz as f32 * TILE_SIZE);
    let w = CELLS + 1;
    let mut depth = vec![0.0f32; w * w];
    let mut verts = Vec::with_capacity(w * w);
    for j in 0..w { for i in 0..w {
        let (x, z) = (ox + i as f32 * step, oz + j as f32 * step);
        let d = level - t.height(x, z);
        depth[j * w + i] = d;
        verts.push([x, level, z, d, 0.0, 1.0]);
    } }
    let mut idx = Vec::new();
    for j in 0..CELLS { for i in 0..CELLS {
        let (a, b, c, d) = ((j * w + i) as u32, (j * w + i + 1) as u32, ((j + 1) * w + i) as u32, ((j + 1) * w + i + 1) as u32);
        let wet = [a, b, c, d].iter().any(|&k| depth[k as usize] > -SHORE_MARGIN);
        if wet { idx.extend_from_slice(&[a, c, b, b, c, d]); }
    } }
    if idx.is_empty() { None } else { Some(Mesh { verts, idx }) }
}

#[cfg(test)]
#[path = "watergen_tests.rs"]
mod tests;
