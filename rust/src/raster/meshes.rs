//! Unit primitives (extent -0.5..0.5 on every axis) for instanced drawing. Winding is irrelevant (culling is off).
use crate::game::terrain::Mesh;

use std::f32::consts::TAU;

pub fn cube() -> Mesh {
    let mut verts = Vec::new(); let mut idx = Vec::new();
    // (normal, u axis, v axis)
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1., 0., 0.], [0., 1., 0.], [0., 0., 1.]), ([-1., 0., 0.], [0., 0., 1.], [0., 1., 0.]),
        ([0., 1., 0.], [0., 0., 1.], [1., 0., 0.]), ([0., -1., 0.], [1., 0., 0.], [0., 0., 1.]),
        ([0., 0., 1.], [1., 0., 0.], [0., 1., 0.]), ([0., 0., -1.], [0., 1., 0.], [1., 0., 0.]),
    ];
    for (n, u, v) in faces {
        let base = verts.len() as u32;
        for (su, sv) in [(-1.0f32, -1.0f32), (1., -1.), (1., 1.), (-1., 1.)] {
            let p = [0, 1, 2].map(|k| n[k] * 0.5 + u[k] * 0.5 * su + v[k] * 0.5 * sv);
            verts.push([p[0], p[1], p[2], n[0], n[1], n[2]]);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh { verts, idx }
}

pub fn cylinder() -> Mesh {
    const N: u32 = 12;
    let mut verts = Vec::new(); let mut idx = Vec::new();
    for i in 0..=N {
        let a = i as f32 / N as f32 * TAU; let (s, c) = a.sin_cos();
        verts.push([c * 0.5, -0.5, s * 0.5, c, 0., s]);
        verts.push([c * 0.5, 0.5, s * 0.5, c, 0., s]);
    }
    for i in 0..N { let b = i * 2; idx.extend_from_slice(&[b, b + 1, b + 2, b + 1, b + 3, b + 2]); }
    for (y, ny) in [(-0.5f32, -1.0f32), (0.5, 1.0)] {
        let centre = verts.len() as u32;
        verts.push([0., y, 0., 0., ny, 0.]);
        for i in 0..=N {
            let a = i as f32 / N as f32 * TAU; let (s, c) = a.sin_cos();
            verts.push([c * 0.5, y, s * 0.5, 0., ny, 0.]);
        }
        for i in 0..N { idx.extend_from_slice(&[centre, centre + 1 + i, centre + 2 + i]); }
    }
    Mesh { verts, idx }
}

pub fn cone() -> Mesh {
    const N: u32 = 10;
    let mut verts = Vec::new(); let mut idx = Vec::new();
    // side: slope normal for radius 0.5, height 1.0
    let k = 1.0 / (1.0f32 + 0.25).sqrt();
    for i in 0..=N {
        let a = i as f32 / N as f32 * TAU; let (s, c) = a.sin_cos();
        verts.push([c * 0.5, -0.5, s * 0.5, c * k, 0.5 * k, s * k]);
        verts.push([0., 0.5, 0., c * k, 0.5 * k, s * k]);
    }
    for i in 0..N { let b = i * 2; idx.extend_from_slice(&[b, b + 1, b + 2]); }
    let centre = verts.len() as u32;
    verts.push([0., -0.5, 0., 0., -1., 0.]);
    for i in 0..=N {
        let a = i as f32 / N as f32 * TAU; let (s, c) = a.sin_cos();
        verts.push([c * 0.5, -0.5, s * 0.5, 0., -1., 0.]);
    }
    for i in 0..N { idx.extend_from_slice(&[centre, centre + 1 + i, centre + 2 + i]); }
    Mesh { verts, idx }
}

pub fn sphere() -> Mesh {
    const R: u32 = 8; const S: u32 = 12;
    let mut verts = Vec::new(); let mut idx = Vec::new();
    for r in 0..=R {
        let t = r as f32 / R as f32 * std::f32::consts::PI;
        for s in 0..=S {
            let a = s as f32 / S as f32 * TAU;
            let n = [t.sin() * a.cos(), t.cos(), t.sin() * a.sin()];
            verts.push([n[0] * 0.5, n[1] * 0.5, n[2] * 0.5, n[0], n[1], n[2]]);
        }
    }
    let w = S + 1;
    for r in 0..R { for s in 0..S {
        let a = r * w + s;
        idx.extend_from_slice(&[a, a + w, a + 1, a + 1, a + w, a + w + 1]);
    } }
    Mesh { verts, idx }
}

#[cfg(test)]
#[path = "meshes_tests.rs"]
mod tests;
