//! Generated plants and rocks (no files). Unit conventions: a tree has a trunk of height 1.0 and a total height of
//! about 1.8-1.9 (scale it by the prop's trunk height); a rock fits a unit sphere with semi-axes (1, 0.7, 1) (scale it by
//! the prop's radius); bushes and grass tufts are used at their natural size.
use super::vehicle_models::{add_cylinder, add_sphere, Axis, Mesh3, MAT_FOLIAGE, MAT_PLASTIC};
use crate::game::math::{v3, V3};
use crate::game::terrain::hash01;

pub struct NatureModels { pub trees: Vec<Mesh3>, pub bushes: Vec<Mesh3>, pub rocks: Vec<Mesh3>, pub grass: Mesh3 }

const TRUNK: [f32; 3] = [0.30, 0.20, 0.12];

/// Cone with a flat bottom: smooth side normals, one apex vertex per segment.
fn add_cone(m: &mut Mesh3, base: V3, radius: f32, height: f32, segs: usize, col: [f32; 3], mat: u8) {
    let slope = radius / height;
    let apex = base + v3(0.0, height, 0.0);
    let mut ring = Vec::new();
    for k in 0..=segs {
        let a = std::f32::consts::TAU * k as f32 / segs as f32;
        ring.push((a, base + v3(a.cos() * radius, 0.0, a.sin() * radius)));
    }
    for k in 0..segs {
        let (a0, p0) = ring[k]; let (a1, p1) = ring[k + 1];
        let n0 = v3(a0.cos(), slope, a0.sin()).norm(); let n1 = v3(a1.cos(), slope, a1.sin()).norm();
        let am = (a0 + a1) * 0.5; let nm = v3(am.cos(), slope, am.sin()).norm();
        let (i0, i1, ia) = (m.vert(p0, n0, col, mat), m.vert(p1, n1, col, mat), m.vert(apex, nm, col, mat));
        m.idx.extend_from_slice(&[i0, ia, i1]);
    }
    let c = m.vert(base, v3(0., -1., 0.), col, mat);
    let rim: Vec<u32> = ring.iter().map(|(_, p)| m.vert(*p, v3(0., -1., 0.), col, mat)).collect();
    for k in 0..segs { m.idx.extend_from_slice(&[c, rim[k], rim[k + 1]]); }
}

fn conifer(cones: &[(f32, f32, f32)], col: [f32; 3]) -> Mesh3 {
    let mut m = Mesh3::default();
    add_cylinder(&mut m, v3(0.0, 0.5, 0.0), Axis::Y, 0.06, 1.0, 6, TRUNK, MAT_PLASTIC);
    for &(y, r, h) in cones { add_cone(&mut m, v3(0.0, y, 0.0), r, h, 8, col, MAT_FOLIAGE); }
    m
}

fn broadleaf(crowns: &[(V3, V3)], col: [f32; 3], rings: usize, segs: usize) -> Mesh3 {
    let mut m = Mesh3::default();
    add_cylinder(&mut m, v3(0.0, 0.5, 0.0), Axis::Y, 0.07, 1.0, 6, TRUNK, MAT_PLASTIC);
    for &(c, r) in crowns { add_sphere(&mut m, c, r, rings, segs, col, MAT_FOLIAGE); }
    m
}

/// Flat-shaded copy: every triangle gets its own vertices and the outward face normal (about the origin).
fn facet(src: &Mesh3) -> Mesh3 {
    let mut m = Mesh3::default();
    for t in src.idx.chunks(3) {
        let p: Vec<V3> = t.iter().map(|&i| { let q = src.verts[i as usize].pos; v3(q[0], q[1], q[2]) }).collect();
        let mut n = (p[1] - p[0]).cross(p[2] - p[0]);
        if n.len() < 1e-9 { continue; }
        let ctr = (p[0] + p[1] + p[2]) * (1.0 / 3.0);
        if n.dot(ctr) < 0.0 { n = -n; }
        let n = n.norm();
        let col = src.verts[t[0] as usize].col;
        let base = m.verts.len() as u32;
        for q in &p { m.verts.push(super::vehicle_models::Vert { pos: [q.x, q.y, q.z], nrm: [n.x, n.y, n.z], col }); }
        m.idx.extend_from_slice(&[base, base + 1, base + 2]);
    }
    m
}

fn rock(v: u32) -> Mesh3 {
    let mut s = Mesh3::default();
    let g = 0.50 + 0.04 * v as f32;
    add_sphere(&mut s, V3::ZERO, v3(1.0, 0.7, 1.0), 6, 8, [g, g * 0.98, g * 0.94], MAT_PLASTIC);
    for vert in &mut s.verts {
        let q = vert.pos;
        let h = hash01(0x40C + v as u64, (q[0] * 20.0).round() as i32, ((q[1] * 20.0).round() as i32).wrapping_mul(131).wrapping_add((q[2] * 20.0).round() as i32));
        let k = 0.85 + 0.15 * h;
        vert.pos = [q[0] * k, q[1] * k, q[2] * k];
    }
    facet(&s)
}

fn bush(v: u32) -> Mesh3 {
    let mut m = Mesh3::default();
    let col = if v == 0 { [0.16, 0.34, 0.10] } else { [0.24, 0.36, 0.12] };
    add_sphere(&mut m, v3(0.0, 0.38, 0.0), v3(0.5, 0.4, 0.5), 4, 6, col, MAT_FOLIAGE);
    add_sphere(&mut m, v3(0.3, 0.28, 0.1), v3(0.4, 0.3, 0.4), 4, 6, col, MAT_FOLIAGE);
    add_sphere(&mut m, v3(-0.25, 0.26, -0.15), v3(0.38, 0.28, 0.38), 4, 6, col, MAT_FOLIAGE);
    m
}

fn grass_tuft() -> Mesh3 {
    let mut m = Mesh3::default();
    let (base, top) = ([0.10, 0.22, 0.06], [0.38, 0.58, 0.20]);
    for k in 0..3 {
        let a = std::f32::consts::PI / 3.0 * k as f32;
        let d = v3(a.cos(), 0.0, a.sin());
        let up = v3(0.0, 1.0, 0.0);
        let ids = [(-0.17f32, 0.0f32, base), (0.17, 0.0, base), (0.04, 0.6, top), (-0.04, 0.6, top)]
            .map(|(x, y, c)| m.vert(d * x + up * y, up, c, MAT_FOLIAGE));
        m.idx.extend_from_slice(&[ids[0], ids[1], ids[2], ids[0], ids[2], ids[3]]);
    }
    m
}

pub fn build_all() -> NatureModels {
    let trees = vec![
        conifer(&[(0.45, 0.55, 0.70), (0.85, 0.42, 0.65), (1.20, 0.28, 0.60)], [0.08, 0.26, 0.10]),
        conifer(&[(0.40, 0.48, 0.55), (0.70, 0.40, 0.55), (1.00, 0.32, 0.50), (1.30, 0.22, 0.60)], [0.10, 0.30, 0.13]),
        broadleaf(&[(v3(0.0, 1.25, 0.0), v3(0.55, 0.45, 0.55)), (v3(0.15, 1.6, 0.1), v3(0.4, 0.35, 0.4))], [0.20, 0.40, 0.10], 7, 8),
        broadleaf(&[(v3(0.0, 1.2, 0.0), v3(0.5, 0.4, 0.5)), (v3(0.3, 1.45, 0.0), v3(0.35, 0.3, 0.35)), (v3(-0.3, 1.5, 0.1), v3(0.35, 0.3, 0.35))], [0.28, 0.42, 0.12], 5, 7),
    ];
    NatureModels { trees, bushes: vec![bush(0), bush(1)], rocks: vec![rock(0), rock(1), rock(2)], grass: grass_tuft() }
}

/// Model variant for a prop of `kind` (0 tree, 1 bush, 2 rock) at `pos`; stable for the same position.
pub fn variant(kind: u8, pos: V3) -> usize {
    let n = match kind { 0 => 4, 1 => 2, _ => 3 };
    (hash01(0xAA + kind as u64, (pos.x * 3.0).floor() as i32, (pos.z * 3.0).floor() as i32) * n as f32) as usize % n
}

/// Colour multiplier per instance.
pub fn tint(pos: V3) -> [f32; 3] {
    let k = 0.85 + 0.3 * hash01(0xBB, (pos.x * 2.0).floor() as i32, (pos.z * 2.0).floor() as i32);
    [k, k, k]
}

#[cfg(test)]
#[path = "nature_models_tests.rs"]
mod tests;
