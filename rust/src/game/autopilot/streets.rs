//! A way through a city along its street grid, from the gate where the liaison ends to the gate where the stage starts.
use crate::game::city::{CitySite, GRID, PITCH};
use crate::game::math::{v3, V3};

/// Radius of the corner arcs at intersections (the street is 12 m wide).
const CORNER: f32 = 8.0;
const STEP: f32 = 4.0;

/// Grid indices (column, row) of the intersection nearest to `p`, clamped to the grid.
fn index(site: &CitySite, p: V3) -> (i32, i32) {
    let f = |v: f32, c: f32| (((v - (c - site.half)) / PITCH).round() as i32).clamp(0, GRID - 1);
    (f(p.x, site.centre.x), f(p.z, site.centre.z))
}
fn at(site: &CitySite, i: i32, j: i32) -> (f32, f32) { (site.centre.x - site.half + i as f32 * PITCH, site.centre.z - site.half + j as f32 * PITCH) }

/// The street corners of the way from gate `a` to gate `b` (both on the city boundary), as grid indices.
fn corners(site: &CitySite, a: V3, b: V3) -> Vec<(i32, i32)> {
    let (ia, ib) = (index(site, a), index(site, b));
    let horizontal = |p: (i32, i32)| p.0 == 0 || p.0 == GRID - 1;   // entered through a west or east gate
    let mid = |x: i32, y: i32| ((x + y) / 2).clamp(1, GRID - 2);
    let mut c = vec![ia];
    match (horizontal(ia), horizontal(ib)) {
        (true, true) => { let m = mid(ia.0, ib.0); c.push((m, ia.1)); c.push((m, ib.1)); }
        (false, false) => { let m = mid(ia.1, ib.1); c.push((ia.0, m)); c.push((ib.0, m)); }
        (true, false) => c.push((ib.0, ia.1)),
        (false, true) => c.push((ia.0, ib.1)),
    }
    c.push(ib);
    c.dedup();
    // drop corners that lie on a straight line between their neighbours
    let mut out = vec![c[0]];
    for k in 1..c.len() - 1 {
        let (p, q, r) = (*out.last().unwrap(), c[k], c[k + 1]);
        if (q.0 - p.0) * (r.1 - q.1) != (q.1 - p.1) * (r.0 - q.0) { out.push(q); }
    }
    out.push(*c.last().unwrap());
    out
}

/// Points every ~4 m along the streets from `a` to `b` with the corners rounded by arcs of `CORNER` metres.
pub fn city_path(site: &CitySite, a: V3, b: V3) -> Vec<V3> {
    let pts: Vec<(f32, f32)> = corners(site, a, b).into_iter().map(|(i, j)| at(site, i, j)).collect();
    if pts.len() < 2 { return vec![v3(a.x, site.centre.y, a.z), v3(b.x, site.centre.y, b.z)]; }
    round_path(&pts, site.centre.y)
}

/// A polyline of street corners (x, z) as points every ~4 m at height `y`: every inner corner is replaced by an arc of
/// `CORNER` metres (the street is 12 m wide); the first and last points stay.
pub fn round_path(pts: &[(f32, f32)], y: f32) -> Vec<V3> {
    let mut poly: Vec<(f32, f32)> = vec![pts[0]];
    for k in 1..pts.len() - 1 {
        let (p, c, n) = (pts[k - 1], pts[k], pts[k + 1]);
        let (u, w) = (norm((c.0 - p.0, c.1 - p.1)), norm((n.0 - c.0, n.1 - c.1)));
        let (t0, t1) = ((c.0 - u.0 * CORNER, c.1 - u.1 * CORNER), (c.0 + w.0 * CORNER, c.1 + w.1 * CORNER));
        // a quarter circle: centre = t0 + w * CORNER; sweep from t0 to t1
        let o = (t0.0 + w.0 * CORNER, t0.1 + w.1 * CORNER);
        let (a0, a1) = ((t0.1 - o.1).atan2(t0.0 - o.0), (t1.1 - o.1).atan2(t1.0 - o.0));
        let mut d = a1 - a0;
        while d > std::f32::consts::PI { d -= 2.0 * std::f32::consts::PI; }
        while d < -std::f32::consts::PI { d += 2.0 * std::f32::consts::PI; }
        let n_arc = ((CORNER * d.abs() / STEP).ceil() as usize).max(2);
        for m in 0..=n_arc { let t = a0 + d * m as f32 / n_arc as f32; poly.push((o.0 + CORNER * t.cos(), o.1 + CORNER * t.sin())); }
    }
    poly.push(*pts.last().unwrap());
    // densify to ~4 m
    let mut out = vec![v3(poly[0].0, y, poly[0].1)];
    for w in poly.windows(2) {
        let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let n = ((dx.hypot(dz) / STEP).ceil() as usize).max(1);
        for m in 1..=n { out.push(v3(w[0].0 + dx * m as f32 / n as f32, y, w[0].1 + dz * m as f32 / n as f32)); }
    }
    out
}
fn norm(v: (f32, f32)) -> (f32, f32) { let l = v.0.hypot(v.1).max(1e-6); (v.0 / l, v.1 / l) }

#[cfg(test)]
#[path = "streets_tests.rs"]
mod tests;
