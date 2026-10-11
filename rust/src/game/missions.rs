//! Mission model and deterministic generation of the mission list.
use super::airgen;
use super::app::level_seed;
use super::flight::AircraftKind;
use super::math::{v3, V3};
use super::props::tile_props;
use super::terrain::{hash01, Terrain};
use super::world::{tile_of, WORLD_LIMIT};

pub const GOAL_RADIUS: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionKind { Cargo, Race, Rings, Sling, Recon }

impl MissionKind {
    pub fn is_air(self) -> bool { matches!(self, MissionKind::Rings | MissionKind::Sling | MissionKind::Recon) }
}

pub const RING_RADIUS_HELI: f32 = 8.0;
pub const RING_RADIUS_DRONE: f32 = 6.0;
pub const ZONE_RADIUS: f32 = 40.0;

#[derive(Clone, Debug)]
pub struct Mission {
    pub kind: MissionKind, pub seed: u64, pub difficulty: u8,
    pub start: V3, pub goal: V3, pub checkpoints: Vec<V3>,
    pub length_m: f32, pub time_limit: f32, pub reward_base: f32,
    /// `None` for ground missions.
    pub aircraft: Option<AircraftKind>,
}

impl Mission {
    pub fn is_air(&self) -> bool { self.kind.is_air() }
}

/// Plane normal of ring `i`: along the path from the previous point (or the start).
pub fn ring_normal(m: &Mission, i: usize) -> V3 {
    let prev = if i == 0 { m.start } else { m.checkpoints[i - 1] };
    (m.checkpoints[i] - prev).norm()
}

pub fn path_length(start: V3, pts: &[V3]) -> f32 {
    let mut prev = start; let mut sum = 0.0;
    for p in pts { sum += (*p - prev).len(); prev = *p; }
    sum
}
pub fn time_limit_for(len: f32) -> f32 { 1.6 * len / 12.0 + 20.0 }
pub fn reward_base_for(len: f32, difficulty: u8) -> f32 {
    let f = match difficulty { 1 => 1.0, 2 => 1.4, _ => 1.9 };
    (100.0 + 0.15 * len) * f
}

/// A mission point must be on gentle ground (normal.y >= 0.94), clear of props (8 m) and inside the world wall.
pub fn point_ok(t: &Terrain, p: V3) -> bool {
    if p.x.abs() > WORLD_LIMIT - 40.0 || p.z.abs() > WORLD_LIMIT - 40.0 { return false; }
    if t.in_city(p.x, p.z, 20.0) { return false; }
    if t.normal(p.x, p.z).y < 0.94 { return false; }
    let (tx, tz) = (tile_of(p.x), tile_of(p.z));
    for dz in -1..=1 { for dx in -1..=1 { for pr in tile_props(t, tx + dx, tz + dz) {
        if (pr.pos.x - p.x).hypot(pr.pos.z - p.z) < 8.0 + pr.radius { return false; }
    } } }
    true
}

/// No 10 m step of the straight line a->b rises or falls more than 8 m (rough reachability check).
pub fn line_ok(t: &Terrain, a: V3, b: V3) -> bool {
    let len = (b - a).len(); let n = (len / 10.0).ceil().max(1.0) as i32;
    let mut prev = t.height(a.x, a.z);
    for i in 1..=n {
        let k = i as f32 / n as f32;
        let h = t.height(a.x + (b.x - a.x) * k, a.z + (b.z - a.z) * k);
        if (h - prev).abs() > 8.0 { return false; }
        prev = h;
    }
    true
}

pub(super) fn at(t: &Terrain, x: f32, z: f32) -> V3 { v3(x, t.height(x, z), z) }

/// The point nearest to (x, z) that passes `point_ok`, searching rings of 8 directions up to 300 m out.
pub(super) fn nearest_ok(t: &Terrain, x: f32, z: f32) -> V3 {
    let base = at(t, x, z);
    if point_ok(t, base) { return base; }
    for ring in 1..=20 {
        let r = ring as f32 * 15.0;
        for d in 0..8 {
            let a = d as f32 * std::f32::consts::FRAC_PI_4;
            let p = at(t, x + a.cos() * r, z + a.sin() * r);
            if point_ok(t, p) { return p; }
        }
    }
    base
}

/// Tries up to 20 goal candidates; returns the first fully valid one, else the one with fewest violations.
fn pick_goal(t: &Terrain, key: u64, difficulty: u8) -> V3 {
    let (dmin, dmax) = match difficulty { 1 => (400.0, 800.0), 2 => (700.0, 1100.0), _ => (1000.0, 1500.0) };
    let origin = at(t, 0.0, 0.0);
    let mut best = (u32::MAX, origin);
    for k in 0..20i32 {
        let a = hash01(key, k, 1) * std::f32::consts::TAU;
        let d = dmin + (dmax - dmin) * hash01(key, k, 2);
        let p = at(t, a.cos() * d, a.sin() * d);
        let bad = (!point_ok(t, p)) as u32 + (!line_ok(t, origin, p)) as u32;
        if bad == 0 { return p; }
        if bad < best.0 { best = (bad, p); }
    }
    nearest_ok(t, best.1.x, best.1.z)
}

/// Race checkpoints along start->goal with a lateral zigzag of at most 80 m; each point nudged sideways until valid.
fn race_points(t: &Terrain, key: u64, goal: V3) -> Vec<V3> {
    let n = 5 + (hash01(key, 7, 7) * 4.0) as usize % 4; // 5..=8
    let dir = v3(goal.x, 0.0, goal.z).norm();
    let side = v3(-dir.z, 0.0, dir.x);
    let mut out = Vec::new();
    for i in 1..n {
        let k = i as f32 / n as f32;
        let zig = if i % 2 == 0 { 1.0 } else { -1.0 } * 80.0 * hash01(key, 100 + i as i32, 3);
        let chosen = nearest_ok(t, goal.x * k + side.x * zig, goal.z * k + side.z * zig);
        out.push(chosen);
    }
    out.push(goal);
    out
}

pub fn roll(profile_seed: u64, round: u32, gold_medals: u32) -> Vec<Mission> {
    let kinds = [MissionKind::Cargo, MissionKind::Race, MissionKind::Cargo, MissionKind::Race, MissionKind::Rings,
        if round % 2 == 0 { MissionKind::Sling } else { MissionKind::Recon }];
    kinds.iter().enumerate().map(|(slot, &kind)| {
        let key = profile_seed ^ ((round as u64) << 20) ^ ((slot as u64 + 1) * 0x9E37_79B9);
        let r = hash01(key, slot as i32, 9);
        let difficulty = if gold_medals >= 3 { 1 + (r * 3.0) as u8 % 3 } else { 1 + (r * 2.0) as u8 % 2 };
        // difficulty is part of the identity: the same slot at another difficulty is a different mission
        let mut h = key.wrapping_add((difficulty as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        h ^= h >> 33; h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD); h ^= h >> 33; h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53); h ^= h >> 33;
        let seed = level_seed(h);
        let t = Terrain::new(seed);
        let (aircraft, checkpoints) = match kind {
            MissionKind::Cargo => (None, vec![pick_goal(&t, key, difficulty)]),
            MissionKind::Race => { let g = pick_goal(&t, key, difficulty); (None, race_points(&t, key, g)) }
            MissionKind::Rings => {
                let drone = hash01(key, slot as i32, 77) < 0.5;
                (Some(if drone { AircraftKind::Drone } else { AircraftKind::Heli }), airgen::rings_points(&t, key, drone))
            }
            MissionKind::Sling => (Some(AircraftKind::Heli), airgen::sling_points(&t, key)),
            MissionKind::Recon => (Some(AircraftKind::Drone), airgen::recon_points(&t, key)),
        };
        let goal = *checkpoints.last().unwrap();
        let start = at(&t, 0.0, 0.0);
        let length_m = path_length(start, &checkpoints);
        let time_limit = match kind {
            MissionKind::Rings => 1.5 * length_m / if aircraft == Some(AircraftKind::Drone) { 15.0 } else { 18.0 } + 20.0,
            MissionKind::Sling => 1.6 * length_m / 12.0 + 35.0,
            MissionKind::Recon => 1.4 * length_m / 15.0 + 25.0,
            _ => time_limit_for(length_m),
        };
        Mission { kind, seed, difficulty, start, goal, checkpoints, length_m, time_limit, reward_base: reward_base_for(length_m, difficulty), aircraft }
    }).collect()
}

#[cfg(test)]
#[path = "missions_tests.rs"]
mod tests;
