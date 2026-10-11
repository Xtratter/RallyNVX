//! Point generation for air missions (rings, sling pads, recon zones).
use super::city::City;
use super::math::{v3, V3};
use super::missions::{nearest_landing, nearest_ok};
use super::terrain::{hash01, Terrain};
use super::world::WORLD_LIMIT;

fn clampw(v: f32) -> f32 { v.clamp(-(WORLD_LIMIT - 40.0), WORLD_LIMIT - 40.0) }

/// 5-8 rings along a 500-1400 m path with a zigzag of at most 80 m; heights above ground 15..90 (drone ..120).
pub fn rings_points(t: &Terrain, key: u64, drone: bool) -> Vec<V3> {
    let len = 500.0 + 900.0 * hash01(key, 31, 1);
    let a = hash01(key, 31, 2) * std::f32::consts::TAU;
    let dir = v3(a.cos(), 0.0, a.sin()); let side = v3(-dir.z, 0.0, dir.x);
    let n = 5 + (hash01(key, 31, 3) * 4.0) as usize % 4;
    let hmax = if drone { 120.0 } else { 90.0 };
    (1..=n).map(|i| {
        let k = i as f32 / n as f32;
        let zig = if i % 2 == 0 { 1.0 } else { -1.0 } * 80.0 * hash01(key, 100 + i as i32, 4);
        let (x, z) = (clampw(dir.x * len * k + side.x * zig), clampw(dir.z * len * k + side.z * zig));
        v3(x, t.height(x, z) + 15.0 + (hmax - 15.0) * hash01(key, 200 + i as i32, 5), z)
    }).collect()
}

/// Pick-up pad A (80-200 m from the start) and drop pad B (400-1000 m from A), both on gentle ground.
pub fn sling_points(t: &Terrain, key: u64) -> Vec<V3> {
    let a1 = hash01(key, 41, 1) * std::f32::consts::TAU; let d1 = 80.0 + 120.0 * hash01(key, 41, 2);
    let pa = nearest_ok(t, a1.cos() * d1, a1.sin() * d1);
    let a2 = hash01(key, 41, 3) * std::f32::consts::TAU; let d2 = 400.0 + 600.0 * hash01(key, 41, 4);
    let pb = nearest_ok(t, clampw(pa.x + a2.cos() * d2), clampw(pa.z + a2.sin() * d2));
    vec![pa, pb]
}

/// 3-5 capture points at 20..100 m height around a loop of 600-1200 m that passes the start.
pub fn recon_points(t: &Terrain, key: u64) -> Vec<V3> {
    let len = 600.0 + 600.0 * hash01(key, 51, 1);
    let r = len / std::f32::consts::TAU;
    let a = hash01(key, 51, 2) * std::f32::consts::TAU;
    let centre = v3(a.cos(), 0.0, a.sin()) * r;
    let n = 3 + (hash01(key, 51, 3) * 3.0) as usize % 3;
    (1..=n).map(|i| {
        let th = a + std::f32::consts::PI + i as f32 * std::f32::consts::TAU / (n as f32 + 1.0);
        let (x, z) = (clampw(centre.x + r * th.cos()), clampw(centre.z + r * th.sin()));
        v3(x, t.height(x, z) + 20.0 + 80.0 * hash01(key, 300 + i as i32, 6), z)
    }).collect()
}


/// Rescue: `difficulty` survivor sites (1..=3) followed by the base pad, all helicopter landing spots at least 60 m apart.
pub fn rescue_points(t: &Terrain, key: u64, difficulty: u8) -> Vec<V3> {
    let n = difficulty.clamp(1, 3) as usize;
    let mut out: Vec<V3> = Vec::new();
    for i in 0..=n {
        let (dist, from) = match i {
            0 => (200.0 + 300.0 * hash01(key, 61, 1), v3(0.0, 0.0, 0.0)),
            k if k < n => (150.0 + 250.0 * hash01(key, 61, 10 + k as i32), out[k - 1]),
            _ => (300.0 + 400.0 * hash01(key, 61, 99), out[i - 1]),
        };
        let mut a = hash01(key, 61, 20 + i as i32) * std::f32::consts::TAU;
        let mut p = nearest_landing(t, clampw(from.x + a.cos() * dist), clampw(from.z + a.sin() * dist));
        let mut tries = 0;
        while out.iter().any(|q| (q.x - p.x).hypot(q.z - p.z) < 60.0) && tries < 8 {
            a += 0.9; tries += 1;
            p = nearest_landing(t, clampw(from.x + a.cos() * (dist + 120.0)), clampw(from.z + a.sin() * (dist + 120.0)));
        }
        out.push(p);
    }
    out
}

/// People waiting at each of `sites` rescue sites: 1..=3 each, at most 4 in total (the helicopter's capacity).
pub fn rescue_people(seed: u64, sites: usize) -> Vec<u8> {
    let mut p = vec![1u8; sites];
    for i in 0..sites {
        if p.iter().map(|c| *c as u32).sum::<u32>() < 4 && hash01(seed, 62, i as i32) < 0.5 { p[i] += 1; }
    }
    p
}

/// Lifts every ring above the tallest building it would touch (ring radius `r` plus 5 m of margin and 2 m of clearance).
pub fn clear_of_buildings(t: &Terrain, seed: u64, pts: &mut [V3], r: f32) {
    let cities: Vec<City> = t.sites().iter().map(|s| City::build(*s, seed)).collect();
    for p in pts.iter_mut() {
        for c in &cities {
            loop {
                let top = c.buildings_near(*p, r + 6.0).into_iter()
                    .filter(|b| p.x >= b.min.x - r - 5.0 && p.x <= b.max.x + r + 5.0 && p.z >= b.min.z - r - 5.0 && p.z <= b.max.z + r + 5.0 && p.y - r - 2.0 < b.max.y)
                    .map(|b| b.max.y).fold(None, |a: Option<f32>, h| Some(a.map_or(h, |x| x.max(h))));
                match top { Some(h) => p.y = h + r + 3.0, None => break }
            }
        }
    }
}

#[cfg(test)]
#[path = "airgen_tests.rs"]
mod tests;
