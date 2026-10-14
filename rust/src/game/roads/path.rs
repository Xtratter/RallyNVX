//! The cost grid and the A* search that finds where a road goes (see the design: nothing is forbidden except the world
//! edge, steep ground, lakes and cities are expensive, a random low-frequency field makes the road meander).
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::game::terrain::{hash01, Terrain};

fn smooth(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }
/// Smooth value noise in [0, 1) with a unit lattice.
pub fn value_noise(seed: u64, x: f32, z: f32) -> f32 {
    let (fx, fz) = (x.floor(), z.floor());
    let (ix, iz) = (fx as i32, fz as i32);
    let (u, v) = (smooth(x - fx), smooth(z - fz));
    let (a, b) = (hash01(seed, ix, iz), hash01(seed, ix + 1, iz));
    let (c, d) = (hash01(seed, ix, iz + 1), hash01(seed, ix + 1, iz + 1));
    let top = a + (b - a) * u; let bot = c + (d - c) * u;
    top + (bot - top) * v
}

pub struct Grid { pub cell: f32, pub half: f32, pub n: usize, h: Vec<f32>, wet: Vec<bool>, city: Vec<bool>, noise: Vec<f32> }

#[derive(Copy, Clone)]
struct Node { f: f32, i: u32 }
impl PartialEq for Node { fn eq(&self, o: &Self) -> bool { self.f == o.f } }
impl Eq for Node {}
impl PartialOrd for Node { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }
impl Ord for Node { fn cmp(&self, o: &Self) -> Ordering { o.f.total_cmp(&self.f) } }   // a min-heap on f

const NEIGHBOURS: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)];

impl Grid {
    /// Samples the (roadless) terrain on a square grid of `cell` metres covering [-half, half] in x and z.
    pub fn build(t: &Terrain, cell: f32, half: f32) -> Grid {
        let n = (2.0 * half / cell) as usize + 1;
        let (mut h, mut wet, mut city, mut noise) = (Vec::with_capacity(n * n), Vec::with_capacity(n * n), Vec::with_capacity(n * n), Vec::with_capacity(n * n));
        for j in 0..n { for i in 0..n {
            let (x, z) = (-half + i as f32 * cell, -half + j as f32 * cell);
            h.push(t.height(x, z));
            wet.push(t.is_water(x, z, 0.5));
            city.push(t.in_city(x, z, 40.0));
            noise.push(value_noise(t.seed ^ 0x40AD, x / 320.0, z / 320.0));
        } }
        Grid { cell, half, n, h, wet, city, noise }
    }

    fn cell_index(&self, x: f32, z: f32) -> (usize, usize) {
        let f = |v: f32| (((v + self.half) / self.cell).round().max(0.0) as usize).min(self.n - 1);
        (f(x), f(z))
    }
    fn world(&self, i: usize) -> (f32, f32) { (-self.half + (i % self.n) as f32 * self.cell, -self.half + (i / self.n) as f32 * self.cell) }
    pub fn is_wet(&self, x: f32, z: f32) -> bool { let (i, j) = self.cell_index(x, z); self.wet[j * self.n + i] }

    fn step_cost(&self, a: usize, b: usize, diag: bool, avoid_city: bool) -> f32 {
        let d = self.cell * if diag { std::f32::consts::SQRT_2 } else { 1.0 };
        let slope = (self.h[b] - self.h[a]).abs() / d;
        let rough = (slope / 0.10).powi(2).min(40.0);
        let mut c = d * (1.0 + 0.9 * rough) * (1.0 + 0.8 * self.noise[b]);
        if self.wet[b] { c += 25.0 * d; }
        if avoid_city && self.city[b] { c += 20.0 * d; }
        c
    }
    fn heur(&self, a: usize, b: usize) -> f32 { let (p, q) = (self.world(a), self.world(b)); (p.0 - q.0).hypot(p.1 - q.1) }

    /// The cheapest 8-neighbour path from `from` to `to` (world x, z): the first and last points are exactly `from`/`to`.
    pub fn find(&self, from: (f32, f32), to: (f32, f32), avoid_city: bool) -> Vec<(f32, f32)> {
        let n = self.n;
        let (si, sj) = self.cell_index(from.0, from.1); let (ti, tj) = self.cell_index(to.0, to.1);
        let (start, goal) = (sj * n + si, tj * n + ti);
        let mut g = vec![f32::INFINITY; n * n];
        let mut came = vec![u32::MAX; n * n];
        let mut closed = vec![false; n * n];
        let mut heap = BinaryHeap::new();
        g[start] = 0.0; heap.push(Node { f: self.heur(start, goal), i: start as u32 });
        while let Some(Node { i, .. }) = heap.pop() {
            let i = i as usize;
            if closed[i] { continue; }
            closed[i] = true;
            if i == goal { break; }
            let (ci, cj) = ((i % n) as i32, (i / n) as i32);
            for (di, dj) in NEIGHBOURS {
                let (ni, nj) = (ci + di, cj + dj);
                if ni < 0 || nj < 0 || ni >= n as i32 || nj >= n as i32 { continue; }
                let k = nj as usize * n + ni as usize;
                if closed[k] { continue; }
                let c = g[i] + self.step_cost(i, k, di != 0 && dj != 0, avoid_city);
                if c < g[k] { g[k] = c; came[k] = i as u32; heap.push(Node { f: c + self.heur(k, goal), i: k as u32 }); }
            }
        }
        let mut cells = vec![goal];
        let mut cur = goal;
        while cur != start && came[cur] != u32::MAX { cur = came[cur] as usize; cells.push(cur); }
        cells.reverse();
        let mut out: Vec<(f32, f32)> = cells.iter().map(|&c| self.world(c)).collect();
        if out.len() < 2 { out = vec![from, to]; }
        out[0] = from; let last = out.len() - 1; out[last] = to;
        out
    }
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;
