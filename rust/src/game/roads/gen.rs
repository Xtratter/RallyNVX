//! Builds the road network of a level from its seed: a tarmac liaison from the spawn to a gate of city 1 and a gravel
//! stage from another gate of city 1 through the terrain to a finish in the open (or a gate of city 2).
use super::{path::Grid, shape, Road, RoadKind, RoadNet, RoadPt};
use crate::game::city::{CitySite, PITCH};
use crate::game::math::v3;
use crate::game::terrain::{hash01, Terrain};

#[derive(Clone, Copy, Debug)]
pub struct GenCfg { pub cell: f32, pub half: f32 }
impl GenCfg {
    pub fn game() -> GenCfg { GenCfg { cell: 16.0, half: 3800.0 } }
    pub fn coarse() -> GenCfg { GenCfg { cell: 32.0, half: 3800.0 } }
}

pub fn generate(seed: u64, base: &Terrain) -> RoadNet { generate_with(seed, base, GenCfg::game()) }

/// Where a road meets the square of a city: the point on the edge and the outward unit direction.
#[derive(Clone, Copy, Debug)]
struct Gate { pos: (f32, f32), out: (f32, f32) }
impl Gate { fn outside(&self, d: f32) -> (f32, f32) { (self.pos.0 + self.out.0 * d, self.pos.1 + self.out.1 * d) } }

/// The gate on side `side` (0 west, 1 east, 2 north, 3 south) at street index `k` (3..=8).
fn gate(site: &CitySite, side: usize, k: i32) -> Gate {
    let (cx, cz, h) = (site.centre.x, site.centre.z, site.half);
    let along = -h + k as f32 * PITCH;
    match side {
        0 => Gate { pos: (cx - h, cz + along), out: (-1.0, 0.0) },
        1 => Gate { pos: (cx + h, cz + along), out: (1.0, 0.0) },
        2 => Gate { pos: (cx + along, cz - h), out: (0.0, -1.0) },
        _ => Gate { pos: (cx + along, cz + h), out: (0.0, 1.0) },
    }
}
fn side_towards(site: &CitySite, p: (f32, f32)) -> usize {
    let mids = [(site.centre.x - site.half, site.centre.z), (site.centre.x + site.half, site.centre.z), (site.centre.x, site.centre.z - site.half), (site.centre.x, site.centre.z + site.half)];
    (0..4).min_by(|&a, &b| {
        let (da, db) = ((mids[a].0 - p.0).hypot(mids[a].1 - p.1), (mids[b].0 - p.0).hypot(mids[b].1 - p.1));
        da.total_cmp(&db)
    }).unwrap()
}
fn opposite(side: usize) -> usize { side ^ 1 }

/// A random point `rmin`..`rmax` from `from`, ahead of `heading` (at most about 100 degrees off it), inside the world, dry and
/// outside the cities; falls back to the last clamped try if 24 tries fail.
fn pick_point(seed: u64, key: u64, from: (f32, f32), heading: (f32, f32), goal: Option<(f32, f32)>, avoid: &[(f32, f32)], rmin: f32, rmax: f32, base: &Terrain, half: f32) -> (f32, f32) {
    let lim = half - 500.0;
    let mut best: Option<(f32, (f32, f32))> = None;   // the most forward candidate if none is fully valid
    for k in 0..64i32 {
        let a = hash01(seed ^ key, k, 1) * std::f32::consts::TAU;
        let r = rmin + (rmax - rmin) * hash01(seed ^ key, k, 2);
        let p = (from.0 + a.cos() * r, from.1 + a.sin() * r);
        if p.0.abs() > lim || p.1.abs() > lim { continue; }
        let (dx, dz) = (p.0 - from.0, p.1 - from.1);
        let fwd = (dx * heading.0 + dz * heading.1) / dx.hypot(dz).max(1.0);
        if best.map_or(true, |b| fwd > b.0) { best = Some((fwd, p)); }
        if fwd < 0.25 { continue; }   // a turn of at most about 75 degrees per leg: the road never doubles back
        if avoid.iter().any(|a| (a.0 - p.0).hypot(a.1 - p.1) < 700.0) { continue; }   // and keeps away from where it has been
        if let Some(g) = goal {
            // and the turn at the new point towards the goal (the second city) must not reverse either
            let (ex, ez) = (g.0 - p.0, g.1 - p.1);
            if (dx * ex + dz * ez) / (dx.hypot(dz).max(1.0) * ex.hypot(ez).max(1.0)) < -0.15 { continue; }
        }
        if base.is_water(p.0, p.1, 1.0) || base.in_city(p.0, p.1, 160.0) { continue; }
        return p;
    }
    best.map(|b| b.1).unwrap_or(((from.0 + heading.0 * rmin).clamp(-lim, lim), (from.1 + heading.1 * rmin).clamp(-lim, lim)))
}

/// `Grid::find`, then a check against the real terrain: ponds the grid missed are marked and the path is searched again.
fn route(grid: &mut Grid, base: &Terrain, from: (f32, f32), to: (f32, f32), avoid_city: bool) -> Vec<(f32, f32)> {
    let mut path = grid.find(from, to, avoid_city);
    for _ in 0..4 {
        let wet: Vec<(f32, f32)> = shape::resample_uniform(&path, 8.0).into_iter().filter(|p| base.is_water(p.0, p.1, 2.0)).collect();
        if wet.is_empty() { break; }
        for p in wet { grid.mark_wet(p.0, p.1, 40.0); }
        path = grid.find(from, to, avoid_city);
    }
    path
}

fn join(legs: Vec<Vec<(f32, f32)>>) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = Vec::new();
    for leg in legs { for p in leg { if out.last().map_or(true, |q| (q.0 - p.0).hypot(q.1 - p.1) > 0.5) { out.push(p); } } }
    out
}

/// Removes spurs: near-duplicate points and tips where the polyline turns back (more than about 120 degrees), repeatedly,
/// so an out-and-back spur of any length collapses. The end points always stay.
fn despike(mut p: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    for _ in 0..200 {
        let mut changed = false;
        let mut out: Vec<(f32, f32)> = Vec::with_capacity(p.len());
        for (i, q) in p.iter().enumerate() {
            let last = i + 1 == p.len();
            if let Some(b) = out.last() {
                if (b.0 - q.0).hypot(b.1 - q.1) < 1.0 && !last { changed = true; continue; }   // a duplicate
            }
            if out.len() >= 2 {
                let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
                let (u, v) = ((b.0 - a.0, b.1 - a.1), (q.0 - b.0, q.1 - b.1));
                let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
                if lu > 1e-3 && lv > 1e-3 && (u.0 * v.0 + u.1 * v.1) / (lu * lv) < -0.5 { out.pop(); changed = true; }   // a tip
            }
            out.push(*q);
        }
        p = out;
        if !changed { break; }
    }
    p
}

/// True when the route comes back to a spot it passed more than 150 m earlier (a retrace or a crossing); the arms of a
/// real hairpin are at least a grid cell apart, so they do not count.
fn has_fold(p: &[(f32, f32)]) -> bool { fold_at(p).is_some() }
fn fold_at(p: &[(f32, f32)]) -> Option<(usize, usize, (f32, f32))> {
    let q = shape::resample_uniform(p, 16.0);
    let gap = 10usize;   // 160 m of road
    for i in 0..q.len() { for j in i + gap..q.len() { if (q[i].0 - q[j].0).hypot(q[i].1 - q[j].1) < 12.0 { return Some((i, j, q[i])); } } }
    None
}

/// The road's points that lie in a lake.
fn wet_points(road: &Road, base: &Terrain) -> Vec<(f32, f32)> {
    road.pts.iter().filter(|p| base.is_water(p.pos.x, p.pos.z, 0.0)).map(|p| (p.pos.x, p.pos.z)).collect()
}

fn build_road(kind: RoadKind, poly: &[(f32, f32)], base: &Terrain, seed: u64) -> Road {
    let xz = shape::smooth_curve(&despike(poly.to_vec()));
    // a road never lies under a lake: where it must cross water it runs on a causeway 0.4 m above the surface
    let floor = base.water_level().map_or(f32::MIN, |l| l + 0.4);
    let raw: Vec<f32> = xz.iter().map(|p| base.height(p.0, p.1).max(floor)).collect();
    let spacing = shape::length(&xz) / (xz.len().max(2) - 1) as f32;
    let mut y = shape::profile(&raw, spacing);
    if kind == RoadKind::Stage { shape::add_crests(&mut y, &xz, spacing, base.water_level().unwrap_or(f32::MIN), seed, &super::cfg::cfg().crest); }
    let w = shape::widths(kind, &xz, spacing, seed);
    let mut pts = Vec::with_capacity(xz.len());
    let mut s = 0.0;
    for i in 0..xz.len() {
        if i > 0 { s += ((xz[i].0 - xz[i - 1].0).hypot(xz[i].1 - xz[i - 1].1)).max(0.0); }
        pts.push(RoadPt { pos: v3(xz[i].0, y[i], xz[i].1), width: w[i], s });
    }
    Road { kind, pts }
}

pub fn generate_with(seed: u64, base: &Terrain, cfg: GenCfg) -> RoadNet {
    let sites = base.sites();
    let Some(city1) = sites.first().copied() else { return RoadNet::new(Vec::new()) };
    let t0 = std::time::Instant::now();
    let mut grid = Grid::build(base, cfg.cell, cfg.half);
    log::info!("road grid built in {} ms", t0.elapsed().as_millis());
    let key = seed ^ 0x40AD5;

    // liaison: spawn -> the side of city 1 that faces the spawn
    let side_a = side_towards(&city1, (0.0, 0.0));
    let gate_a = gate(&city1, side_a, 3 + (hash01(key, 1, 1) * 6.0) as i32 % 6);
    let wp_a = gate_a.outside(80.0);
    let mut liaison = None;
    for round in 0..5 {
        // the road starts 40 m behind the spawn (straight back along its first step), so the car does not sit at its very end
        let first = route(&mut grid, base, (0.0, 0.0), wp_a, true);
        let (dx, dz) = (first[1].0 - first[0].0, first[1].1 - first[0].1);
        let dl = dx.hypot(dz).max(1.0);
        let poly = join(vec![vec![(-dx / dl * 40.0, -dz / dl * 40.0)], first, vec![wp_a, gate_a.pos]]);
        let road = build_road(RoadKind::Liaison, &poly, base, seed);
        let wet = wet_points(&road, base);
        if wet.is_empty() || round == 4 { liaison = Some(road); break; }
        for p in wet { grid.mark_wet(p.0, p.1, 45.0); }   // smoothing cut a corner of a pond: mark it and route again
    }

    // stage: another gate of city 1 (the opposite side) -> random legs -> a finish in the open or at city 2
    let side_b = opposite(side_a);
    let gate_b = gate(&city1, side_b, 3 + (hash01(key, 2, 2) * 6.0) as i32 % 6);
    let wp_b = gate_b.outside(80.0);
    let finish_city = sites.get(1).copied().filter(|_| hash01(key, 3, 3) < 0.5);
    let target = 6000.0 + 3000.0 * hash01(key, 4, 4);
    let mut stage = None;
    let mut tried = 0u32;
    for round in 0..3 {
    let mut best: Option<(f32, Vec<(f32, f32)>)> = None;
    for attempt in 0..24u64 {
        let legs = 2 + (hash01(key, attempt as i32, 7) * 2.0) as usize;   // 2..3 random legs before the finish
        let mut cur = wp_b;
        let mut heading = gate_b.out;
        let mut visited: Vec<(f32, f32)> = vec![gate_b.pos];
        let mut parts: Vec<Vec<(f32, f32)>> = vec![vec![gate_b.pos, wp_b]];
        tried += 1;
        for l in 0..legs {
            let next = pick_point(seed, 0x5100 + attempt * 16 + l as u64, cur, heading, finish_city.map(|c| (c.centre.x, c.centre.z)), &visited, 800.0, 1600.0, base, cfg.half);
            visited.push(next);
            parts.push(route(&mut grid, base, cur, next, true));
            let (dx, dz) = (next.0 - cur.0, next.1 - cur.1);
            let l2 = dx.hypot(dz).max(1.0);
            heading = (dx / l2, dz / l2);
            cur = next;
        }
        match finish_city {
            Some(c2) => {
                // arrive from outside the city on the side that faces the last point, through an approach point 0.4-1.1 km out
                let g2 = gate(&c2, side_towards(&c2, cur), 3 + (hash01(key, 5, 5) * 6.0) as i32 % 6);
                let w2 = g2.outside(80.0);
                let ang = (hash01(key, attempt as i32, 9) - 0.5) * 1.2;
                let (sn, cs) = ang.sin_cos();
                let dir = (g2.out.0 * cs - g2.out.1 * sn, g2.out.0 * sn + g2.out.1 * cs);
                let d = (cur.0 - w2.0).hypot(cur.1 - w2.1);
                let far = (400.0 + 700.0 * hash01(key, attempt as i32, 10)).min(0.7 * d);   // between the last point and the gate, never beyond the last point
                let lim = cfg.half - 150.0;
                if d > 700.0 {
                    let pp = ((w2.0 + dir.0 * far).clamp(-lim, lim), (w2.1 + dir.1 * far).clamp(-lim, lim));
                    parts.push(route(&mut grid, base, cur, pp, true)); parts.push(route(&mut grid, base, pp, w2, true));
                } else {
                    parts.push(route(&mut grid, base, cur, w2, true));
                }
                parts.push(vec![w2, g2.pos]);
            }
            None => { let f = pick_point(seed, 0x5900 + attempt, cur, heading, None, &visited, 700.0, 1300.0, base, cfg.half); parts.push(route(&mut grid, base, cur, f, true)); }
        }
        let poly = join(parts);
        let len = shape::length(&poly) * 1.06;
        let score = (len - target).abs();
        let ok = (5500.0..=11_000.0).contains(&len);
        let folded = has_fold(&poly);
        let score = score + if folded { 100_000.0 } else { 0.0 };   // a folded route only wins if every attempt folds
        if best.as_ref().map_or(true, |b| score < b.0) { best = Some((score, poly)); }
        if ok && !folded { break; }
    }
    let stage_poly = best.map(|b| b.1).unwrap_or_else(|| vec![gate_b.pos, wp_b, (wp_b.0 + gate_b.out.0 * 1500.0, wp_b.1 + gate_b.out.1 * 1500.0)]);
    let road = build_road(RoadKind::Stage, &stage_poly, base, seed);
    let wet = wet_points(&road, base);
    if wet.is_empty() || round == 2 { stage = Some(road); break; }
    for p in wet { grid.mark_wet(p.0, p.1, 45.0); }
    }

    log::info!("roads: {} stage attempts, {} ms in total", tried, t0.elapsed().as_millis());
    RoadNet::new(vec![liaison.expect("the liaison loop always sets it"), stage.expect("the stage loop always sets it")])
}

#[cfg(test)]
#[path = "gen_tests.rs"]
mod tests;
