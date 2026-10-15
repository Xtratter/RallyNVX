//! A signed distance field of a triangle mesh on a voxel grid, built once per car model for the dust (the exact-shape option): the triangles are sampled into
//! voxels, the voxels reachable from outside are the outside, a chamfer distance transform gives the distance to the surface. Negative inside.

pub struct Sdf { pub origin: [f32; 3], pub cell: f32, pub n: [usize; 3], pub d: Vec<f32> }

const FAR: f32 = 1.0e3;

fn idx(n: [usize; 3], x: usize, y: usize, z: usize) -> usize { (z * n[1] + y) * n[0] + x }

/// The field of the mesh (`verts`, `tris` as vertex-index triples flattened): voxels of `cell` metres in the mesh's box grown by `margin` on every side. An empty mesh gives an
/// all-outside field.
pub fn build(verts: &[[f32; 3]], tris: &[u32], cell: f32, margin: f32) -> Sdf {
    let cell = cell.max(0.02);
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    let finite = |v: &[f32; 3]| v.iter().all(|x| x.is_finite());
    for v in verts.iter().filter(|v| finite(v)) { for a in 0..3 { lo[a] = lo[a].min(v[a]); hi[a] = hi[a].max(v[a]); } }
    if lo[0] > hi[0] { return Sdf { origin: [0.0; 3], cell, n: [1, 1, 1], d: vec![FAR] }; }   // no finite vertex at all
    let origin = [lo[0] - margin, lo[1] - margin, lo[2] - margin];
    let n = [0, 1, 2].map(|a| (((hi[a] - lo[a]) + 2.0 * margin) / cell).ceil().max(1.0) as usize + 1);
    let total = n[0] * n[1] * n[2];
    // 1. the voxels the surface passes through: samples spread over every triangle, spaced half a cell
    let mut surf = vec![false; total];
    let mark = |p: [f32; 3], surf: &mut Vec<bool>| {
        let c = [0, 1, 2].map(|a| (((p[a] - origin[a]) / cell).floor().max(0.0) as usize).min(n[a] - 1));
        surf[idx(n, c[0], c[1], c[2])] = true;
    };
    for t in tris.chunks_exact(3) {
        if t.iter().any(|&i| i as usize >= verts.len()) { continue; }
        let (a, b, c) = (verts[t[0] as usize], verts[t[1] as usize], verts[t[2] as usize]);
        if !finite(&a) || !finite(&b) || !finite(&c) { continue; }
        let len = |p: [f32; 3], q: [f32; 3]| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
        let k = ((len(a, b).max(len(b, c)).max(len(c, a))) / (0.5 * cell)).ceil().clamp(1.0, 400.0) as usize;
        for i in 0..=k { for j in 0..=(k - i) {
            let (u, v) = (i as f32 / k as f32, j as f32 / k as f32);
            let w = 1.0 - u - v;
            mark([0, 1, 2].map(|q| a[q] * w + b[q] * u + c[q] * v).try_into().unwrap(), &mut surf);
        } }
    }
    // 2. the outside: a flood fill from the box's faces through the voxels the surface does not occupy; the surface is thickened by a voxel first so the small
    // holes of a model (gaps between panels) do not let the fill into the body
    let mut thick = surf.clone();
    for z in 0..n[2] { for y in 0..n[1] { for x in 0..n[0] {
        if !surf[idx(n, x, y, z)] { continue; }
        for dz in -1i32..=1 { for dy in -1i32..=1 { for dx in -1i32..=1 {
            let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);
            if nx >= 0 && ny >= 0 && nz >= 0 && nx < n[0] as i32 && ny < n[1] as i32 && nz < n[2] as i32 { thick[idx(n, nx as usize, ny as usize, nz as usize)] = true; }
        } } }
    } } }
    let mut outside = vec![false; total];
    let mut stack: Vec<[usize; 3]> = Vec::new();
    for z in 0..n[2] { for y in 0..n[1] { for x in 0..n[0] {
        if (x == 0 || y == 0 || z == 0 || x == n[0] - 1 || y == n[1] - 1 || z == n[2] - 1) && !thick[idx(n, x, y, z)] && !outside[idx(n, x, y, z)] { outside[idx(n, x, y, z)] = true; stack.push([x, y, z]); }
    } } }
    while let Some([x, y, z]) = stack.pop() {
        for (dx, dy, dz) in [(1i32, 0i32, 0i32), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
            let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);
            if nx < 0 || ny < 0 || nz < 0 || nx >= n[0] as i32 || ny >= n[1] as i32 || nz >= n[2] as i32 { continue; }
            let i = idx(n, nx as usize, ny as usize, nz as usize);
            if !thick[i] && !outside[i] { outside[i] = true; stack.push([nx as usize, ny as usize, nz as usize]); }
        }
    }
    // 3. the distance to the nearest surface voxel: a 3 x 3 x 3 chamfer transform, forward and backward
    let mut dist = vec![FAR; total];
    for i in 0..total { if surf[i] { dist[i] = 0.0; } }
    let w = |dx: i32, dy: i32, dz: i32| cell * ((dx * dx + dy * dy + dz * dz) as f32).sqrt();
    let pass = |dist: &mut Vec<f32>, forward: bool| {
        let range = |m: usize| -> Vec<usize> { if forward { (0..m).collect() } else { (0..m).rev().collect() } };
        for &z in &range(n[2]) { for &y in &range(n[1]) { for &x in &range(n[0]) {
            let mut best = dist[idx(n, x, y, z)];
            for dz in -1i32..=1 { for dy in -1i32..=1 { for dx in -1i32..=1 {
                if (dx, dy, dz) == (0, 0, 0) { continue; }
                // only the neighbours already visited in this sweep direction
                let before = if forward { (dz, dy, dx) < (0, 0, 0) } else { (dz, dy, dx) > (0, 0, 0) };
                if !before { continue; }
                let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);
                if nx < 0 || ny < 0 || nz < 0 || nx >= n[0] as i32 || ny >= n[1] as i32 || nz >= n[2] as i32 { continue; }
                best = best.min(dist[idx(n, nx as usize, ny as usize, nz as usize)] + w(dx, dy, dz));
            } } }
            dist[idx(n, x, y, z)] = best;
        } } }
    };
    pass(&mut dist, true); pass(&mut dist, false);
    // 4. signed: the surface voxels count as inside (distance 0)
    let d: Vec<f32> = (0..total).map(|i| if outside[i] { dist[i] } else { -dist[i] }).collect();
    Sdf { origin, cell, n, d }
}

impl Sdf {
    fn at_cell(&self, c: [i32; 3]) -> f32 {
        let cl = [0, 1, 2].map(|a| c[a].clamp(0, self.n[a] as i32 - 1) as usize);
        self.d[idx(self.n, cl[0], cl[1], cl[2])]
    }
    /// The distance at `p` (trilinear between voxel centres); far outside the grid it is the distance to the grid's box plus the nearest edge value.
    pub fn at(&self, p: [f32; 3]) -> f32 {
        let g = [0, 1, 2].map(|a| (p[a] - self.origin[a]) / self.cell - 0.5);
        let i = g.map(|x| x.floor());
        let f = [g[0] - i[0], g[1] - i[1], g[2] - i[2]];
        let c0 = [i[0] as i32, i[1] as i32, i[2] as i32];
        let v = |dx: i32, dy: i32, dz: i32| self.at_cell([c0[0] + dx, c0[1] + dy, c0[2] + dz]);
        let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let x00 = l(v(0, 0, 0), v(1, 0, 0), f[0]); let x10 = l(v(0, 1, 0), v(1, 1, 0), f[0]);
        let x01 = l(v(0, 0, 1), v(1, 0, 1), f[0]); let x11 = l(v(0, 1, 1), v(1, 1, 1), f[0]);
        l(l(x00, x10, f[1]), l(x01, x11, f[1]), f[2])
    }
    /// The direction away from the body at `p` (the unit gradient of the field; zero where it is flat).
    pub fn gradient(&self, p: [f32; 3]) -> [f32; 3] {
        let e = 0.5 * self.cell;
        let d = |a: usize| { let (mut q, mut m) = (p, p); q[a] += e; m[a] -= e; self.at(q) - self.at(m) };
        let g = [d(0), d(1), d(2)];
        let l = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
        if l < 1e-6 { [0.0; 3] } else { [g[0] / l, g[1] / l, g[2] / l] }
    }
}

#[cfg(test)]
#[path = "sdf_tests.rs"]
mod tests;
