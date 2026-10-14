//! Road ribbons: one quad per road segment, in the terrain tile that holds the segment's midpoint. Vertex
//! `[x, y, z, across (-1 left .. 1 right), s (m along the road), kind * 100 + width]`.
use crate::game::roads::{Road, RoadKind};
use crate::game::terrain::{Mesh, Terrain, TILE_SIZE};

/// The ribbon floats this far above the road height (the depth bias does the rest on coarse terrain LODs).
const LIFT: f32 = 0.07;

fn side(road: &Road, i: usize) -> (f32, f32) {
    let (a, b) = (road.pts[i.saturating_sub(1)].pos, road.pts[(i + 1).min(road.pts.len() - 1)].pos);
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let l = dx.hypot(dz).max(1e-6);
    (-dz / l, dx / l)
}

pub fn road_mesh(t: &Terrain, tx: i32, tz: i32) -> Option<Mesh> {
    let net = t.roads()?;
    let (x0, z0) = (tx as f32 * TILE_SIZE, tz as f32 * TILE_SIZE);
    let (mut verts, mut idx) = (Vec::new(), Vec::new());
    for road in &net.roads {
        let code = match road.kind { RoadKind::Liaison => 100.0, RoadKind::Stage | RoadKind::Side => 0.0 };
        let lift = if road.kind == RoadKind::Side { LIFT + 0.01 } else { LIFT };   // a side road's mouth lies over the stage's ribbon
        for i in 0..road.pts.len().saturating_sub(1) {
            let (a, b) = (road.pts[i], road.pts[i + 1]);
            let (mx, mz) = ((a.pos.x + b.pos.x) * 0.5, (a.pos.z + b.pos.z) * 0.5);
            if mx < x0 || mx >= x0 + TILE_SIZE || mz < z0 || mz >= z0 + TILE_SIZE { continue; }
            let base = verts.len() as u32;
            for (p, k) in [(a, i), (b, i + 1)] {
                let (sx, sz) = side(road, k);
                let h = p.width * 0.5;
                verts.push([p.pos.x + sx * h, p.pos.y + lift, p.pos.z + sz * h, 1.0, p.s, code + p.width]);
                verts.push([p.pos.x - sx * h, p.pos.y + lift, p.pos.z - sz * h, -1.0, p.s, code + p.width]);
            }
            idx.extend_from_slice(&[base, base + 2, base + 1, base + 1, base + 2, base + 3]);
        }
    }
    // the lines of the city loop: a checkered start/finish line (code 200) and plain check lines (300), 3 m deep, across the 12 m street
    if let Some(l) = t.city_loop() {
        for line in &l.lines {
            let (p, d) = (line.pos, line.dir);
            if p.x < x0 || p.x >= x0 + TILE_SIZE || p.z < z0 || p.z >= z0 + TILE_SIZE { continue; }
            let (rx, rz) = (d.z, -d.x);
            let code = if line.start { 200.0 } else { 300.0 };
            let base = verts.len() as u32;
            for (s, along) in [(0.0f32, -1.5f32), (3.0, 1.5)] {
                for across in [1.0f32, -1.0] {
                    verts.push([p.x + d.x * along + rx * 6.0 * across, p.y + LIFT - 0.02, p.z + d.z * along + rz * 6.0 * across, across, s, code + 12.0]);
                }
            }
            idx.extend_from_slice(&[base, base + 2, base + 1, base + 1, base + 2, base + 3]);
        }
    }
    if idx.is_empty() { None } else { Some(Mesh { verts, idx }) }
}

#[cfg(test)]
#[path = "roadgen_tests.rs"]
mod tests;
