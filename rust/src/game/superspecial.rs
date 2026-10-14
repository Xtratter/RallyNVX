//! The city super-special: a closed loop of 1.2-1.8 km through the streets of the first city with a start/finish line
//! and check lines across the street (see docs/superpowers/specs/2026-10-14-city-loop-design.md).
use super::autopilot::streets::round_path;
use super::city::{CitySite, GRID, PITCH};
use super::math::V3;
use super::roads::cfg::CityLoopCfg;
use super::terrain::hash01;

/// A line across the street: its centre, the unit direction of travel and whether it is the start/finish line.
#[derive(Clone, Copy, Debug)]
pub struct Line { pub pos: V3, pub dir: V3, pub start: bool }

#[derive(Clone, Debug)]
pub struct SuperSpecial { pub pts: Vec<V3>, pub lines: Vec<Line>, pub length: f32 }

type G = (f32, f32);

/// The loop of a city: a rectangle of `size` blocks inside the street grid with 1-2 inward detours, driven clockwise or
/// counter-clockwise; it starts in the middle of its top side.
pub fn build(site: &CitySite, seed: u64, c: &CityLoopCfg) -> SuperSpecial {
    let key = seed ^ 0xC17_100B;
    let pick = |lo: u32, hi: u32, k: i32| lo + (hash01(key, k, 1) * (hi - lo + 1) as f32) as u32;
    let (w, h) = (pick(c.size.0, c.size.1, 1).min(9) as i32, pick(c.size.0, c.size.1, 2).min(9) as i32);
    let last = GRID - 2;   // keep one street between the loop and the city's edge
    let i0 = 1 + (hash01(key, 3, 1) * ((last - w).max(0) + 1) as f32) as i32;
    let j0 = 1 + (hash01(key, 4, 1) * ((last - h).max(0) + 1) as f32) as i32;
    let (i0, j0) = (i0.min(last - w).max(1), j0.min(last - h).max(1));
    let (i1, j1) = (i0 + w, j0 + h);
    let start: G = ((i0 + i1) as f32 * 0.5, j0 as f32);
    let corners: [G; 4] = [(i1 as f32, j0 as f32), (i1 as f32, j1 as f32), (i0 as f32, j1 as f32), (i0 as f32, j0 as f32)];
    // the sides after the start side: right, bottom, left (inward normals), each may get one detour
    let mut sides: Vec<(G, G, G)> = vec![(corners[0], corners[1], (-1.0, 0.0)), (corners[1], corners[2], (0.0, -1.0)), (corners[2], corners[3], (1.0, 0.0))];
    let jogs = pick(c.jogs.0, c.jogs.1, 5).min(3) as usize;
    let mut order = [0usize, 1, 2];
    for k in (1..3).rev() { let r = (hash01(key, 6, k as i32) * (k + 1) as f32) as usize; order.swap(k, r.min(k)); }
    let mut poly: Vec<G> = vec![start, corners[0]];
    for (n, side) in sides.drain(..).enumerate() {
        let (p, q, inward) = side;
        let len = ((q.0 - p.0).abs() + (q.1 - p.1).abs()) as i32;
        let d = (((q.0 - p.0) / len as f32).round(), ((q.1 - p.1) / len as f32).round());
        if order[..jogs].contains(&n) && len >= 5 {
            let dw = 1 + (hash01(key, 7, n as i32) * 2.0) as i32;
            let a = 1 + (hash01(key, 8, n as i32) * ((len - dw - 2).max(1)) as f32) as i32;
            let a = a.min(len - dw - 1).max(1);
            let at = |t: i32, depth: f32| (p.0 + d.0 * t as f32 + inward.0 * depth, p.1 + d.1 * t as f32 + inward.1 * depth);
            poly.extend([at(a, 0.0), at(a, 1.0), at(a + dw, 1.0), at(a + dw, 0.0)]);
        }
        poly.push(q);
    }
    poly.push(start);
    // the loop is dropped on the street grid, then driven in a random direction
    if hash01(key, 9, 1) < 0.5 { poly.reverse(); }
    let world: Vec<(f32, f32)> = poly.iter().map(|g| (site.centre.x - site.half + g.0 * PITCH, site.centre.z - site.half + g.1 * PITCH)).collect();
    let pts = round_path(&world, site.centre.y);
    let mut length = 0.0f32;
    let mut s_at = vec![0.0f32; pts.len()];
    for i in 1..pts.len() { length += (pts[i] - pts[i - 1]).len(); s_at[i] = length; }
    let dir_at = |i: usize| { let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(pts.len() - 1)]); let d = b - a; let l = d.len().max(1e-3); V3 { x: d.x / l, y: 0.0, z: d.z / l } };
    let mut lines = vec![Line { pos: pts[0], dir: dir_at(0), start: true }];
    let mut s = c.check_every;
    while s < length - 80.0 {
        if let Some(i) = s_at.iter().position(|v| *v >= s) { lines.push(Line { pos: pts[i], dir: dir_at(i), start: false }); }
        s += c.check_every;
    }
    SuperSpecial { pts, lines, length }
}

#[cfg(test)]
#[path = "superspecial_tests.rs"]
mod tests;
