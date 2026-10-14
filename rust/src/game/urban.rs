//! Mission points inside a city: starts, goals, race checkpoints.
use super::city::{City, BLOCKS, GRID};
use super::math::V3;
use super::terrain::hash01;

#[derive(Clone, Debug)]
pub struct Urban {
    pub start: V3, pub yaw: Option<f32>, pub points: Vec<V3>,
}

const EAST: f32 = std::f32::consts::FRAC_PI_2;

pub fn start_row(key: u64) -> i32 { 1 + (hash01(key, 1, 1) * (BLOCKS - 1) as f32) as i32 % (BLOCKS - 1) }

fn manhattan(a: V3, b: V3) -> f32 { (a.x - b.x).abs() + (a.z - b.z).abs() }

/// A goal intersection 300-600 m (Manhattan) from the west start; a route that ends in range is used when found,
/// otherwise a point straight east along the start street.
pub fn cargo(c: &City, key: u64) -> Urban {
    let row = start_row(key); let start = c.west_edge(row);
    for k in 0..40u64 {
        let segs = 8 + (hash01(key, k as i32, 41) * 5.0) as usize % 5;
        if let Some(&(i, j)) = c.route(key ^ (k << 8), (0, row), segs).last() {
            let g = c.intersection(i, j);
            if (300.0..=600.0).contains(&manhattan(start, g)) { return Urban { start, yaw: Some(EAST), points: vec![g] }; }
        }
    }
    let i = 7 + (hash01(key, 5, 42) * 5.0) as i32 % 5;            // 7..=11 blocks east: 336-528 m
    Urban { start, yaw: Some(EAST), points: vec![c.intersection(i, row)] }
}

/// Checkpoints at the turning intersections of a street route plus its end (6-10 of them); a staircase is the fallback.
pub fn race(c: &City, key: u64) -> Urban {
    let row = start_row(key); let start = c.west_edge(row);
    let turning = |route: &[(i32, i32)]| -> Vec<(i32, i32)> {
        let mut pts = Vec::new();
        let all: Vec<(i32, i32)> = std::iter::once((0, row)).chain(route.iter().copied()).collect();
        for w in 1..all.len() {
            let last = w == all.len() - 1;
            let turn = !last && (all[w].0 - all[w - 1].0, all[w].1 - all[w - 1].1) != (all[w + 1].0 - all[w].0, all[w + 1].1 - all[w].1);
            if turn || last { pts.push(all[w]); }
        }
        pts
    };
    for k in 0..40u64 {
        let segs = 10 + (hash01(key, k as i32, 43) * 7.0) as usize % 7;
        let pts = turning(&c.route(key ^ (k << 8), (0, row), segs));
        if pts.len() >= 6 { return Urban { start, yaw: Some(EAST), points: pts.into_iter().take(10).map(|(i, j)| c.intersection(i, j)).collect() }; }
    }
    // staircase: east, then alternately north/south and east (never leaves the grid)
    let up = row < GRID / 2;
    let mut cur = (0, row); let mut pts = Vec::new();
    for s in 0..9 { cur = if s % 2 == 0 { (cur.0 + 1, cur.1) } else { (cur.0, if up { cur.1 + 1 } else { cur.1 - 1 }) }; pts.push(cur); }
    Urban { start, yaw: Some(EAST), points: pts.into_iter().map(|(i, j)| c.intersection(i, j)).collect() }
}

#[cfg(test)]
#[path = "urban_tests.rs"]
mod tests;
