//! Side roads: junctions along the special stage with gravel roads that branch off and end in a dead end.
use super::cfg::SideCfg;
use super::gen::build_road_with;
use super::shape::radius;
use super::{Road, RoadKind};
use crate::game::terrain::{hash01, Terrain};

const LIMIT: f32 = 3700.0;
const MAX_GRADE: f32 = 0.14;

/// The side roads of a level: 4-7 by default, each a few hundred metres long, leaving the stage alternately left and right.
pub fn side_roads(stage: &Road, base: &Terrain, seed: u64, c: &SideCfg) -> Vec<Road> {
    let key = seed ^ 0x51DE_0AD;
    let n = stage.pts.len();
    let mut out: Vec<Road> = Vec::new();
    if n < 40 { return out; }
    let want = c.count.0 + (hash01(key, 1, 1) * (c.count.1 - c.count.0 + 1) as f32) as u32;
    let mut sign = if hash01(key, 2, 2) < 0.5 { 1.0f32 } else { -1.0 };
    let len = stage.length();
    let mut s = c.keep_ends + hash01(key, 3, 3) * (c.spacing.1 - c.spacing.0);
    let mut k = 0;
    let water = base.water_level();
    let (stage_xz, stage_y): (Vec<(f32, f32)>, Vec<f32>) = (stage.pts.iter().map(|p| (p.pos.x, p.pos.z)).collect(), stage.pts.iter().map(|p| p.pos.y).collect());
    while s < len - c.keep_ends && (out.len() as u32) < want {
        k += 1;
        let step_on = |s: f32, extra: f32| s + extra;
        let Some(i) = stage.pts.iter().position(|p| p.s >= s) else { break };
        if i < 12 || i + 12 >= n { s = step_on(s, 150.0); continue; }
        // the stage must be calm here: no tight bend, no causeway, no crest
        let r = radius(stage_xz[i - 8], stage_xz[i], stage_xz[i + 8]);
        let crest = (stage_y[i - 5] + stage_y[i + 5] - 2.0 * stage_y[i]).abs() / 400.0;
        let wet = water.map_or(false, |l| stage_y[i] - l < 3.0);
        if r < c.min_radius || crest > 0.003 || wet { s = step_on(s, 150.0); continue; }
        let (tx, tz) = (stage_xz[i + 1].0 - stage_xz[i - 1].0, stage_xz[i + 1].1 - stage_xz[i - 1].1);
        let tl = tx.hypot(tz).max(1e-3);
        let ang = sign * (c.angle.0 + (c.angle.1 - c.angle.0) * hash01(key, k, 4)).to_radians();
        let mut h = rotate((tx / tl, tz / tl), ang);
        let p0 = stage_xz[i];
        let target = c.length.0 + (c.length.1 - c.length.0) * hash01(key, k, 5);
        let mut poly = vec![p0, (p0.0 + h.0 * 30.0, p0.1 + h.1 * 30.0)];
        let mut total = 30.0f32;
        let mut leg = 0;
        while total < target && leg < 5 {
            leg += 1;
            let step = 80.0 + 70.0 * hash01(key, k * 16 + leg, 6);
            let cur = *poly.last().unwrap();
            let wander = (hash01(key, k * 16 + leg, 7) - 0.5) * 2.0 * 25f32.to_radians();
            let mut placed = false;
            for turn in [wander, -wander, 0.0] {
                let h2 = rotate(h, turn);
                let next = (cur.0 + h2.0 * step, cur.1 + h2.1 * step);
                if ok_leg(cur, next, p0, base, &stage_xz, &out, c) { poly.push(next); h = h2; total += step; placed = true; break; }
            }
            if !placed { break; }
        }
        if total >= 100.0 && poly.len() >= 3 {
            let road = build_road_with(RoadKind::Side, &poly, base, seed ^ (k as u64 * 0x9E37), Some(stage_y[i]));
            let dry = road.pts.iter().all(|p| !base.is_water(p.pos.x, p.pos.z, 0.0));
            if dry && road.length() >= 90.0 { out.push(road); sign = -sign; s = step_on(s, c.spacing.0 + (c.spacing.1 - c.spacing.0) * hash01(key, k, 8)); continue; }
        }
        s = step_on(s, 150.0);
    }
    out
}

fn rotate(v: (f32, f32), a: f32) -> (f32, f32) { let (sn, cs) = a.sin_cos(); (v.0 * cs - v.1 * sn, v.0 * sn + v.1 * cs) }

/// A leg from `a` to `b` is fine when it stays in bounds, on dry and not-too-steep ground and keeps its distance from the stage
/// (beyond 60 m of the junction) and from the earlier side roads.
fn ok_leg(a: (f32, f32), b: (f32, f32), p0: (f32, f32), base: &Terrain, stage: &[(f32, f32)], others: &[Road], c: &SideCfg) -> bool {
    if b.0.abs() > LIMIT || b.1.abs() > LIMIT { return false; }
    let dist = |p: (f32, f32), q: (f32, f32)| (p.0 - q.0).hypot(p.1 - q.1);
    for m in 0..=8 {
        let t = m as f32 / 8.0;
        let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        if base.is_water(p.0, p.1, 3.0) || base.in_city(p.0, p.1, 60.0) { return false; }
        if dist(p, p0) > 60.0 && stage.iter().step_by(2).any(|q| dist(p, *q) < c.min_clear) { return false; }
        if others.iter().any(|r| r.pts.iter().step_by(4).any(|q| dist(p, (q.pos.x, q.pos.z)) < c.min_clear)) { return false; }
    }
    let rise = (base.height(b.0, b.1) - base.height(a.0, a.1)).abs();
    rise / dist(a, b) <= MAX_GRADE
}

#[cfg(test)]
#[path = "side_tests.rs"]
mod tests;
