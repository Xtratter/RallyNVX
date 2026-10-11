# City missions (v2.4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship v2.4: urban variants of the missions (cargo delivery in the city, an urban street race, rings between houses, sling cargo between roofs) that start in the city, with generation, world setup, menu labels and tests.

**Architecture:** `City` gets an intersection grid, a self-avoiding street `route` and accessors; a new pure module `game::urban` turns a city into mission points (start, yaw, checkpoints); `missions::roll` uses it for the urban slots; `World::for_mission` starts the player at `Mission.start` with an optional `start_yaw`; the menu array carries an `urban` flag.

**Tech Stack:** Rust 2021 (host-tested), Kotlin (menu stride and label), GitHub Actions (urban smoke).

**Spec:** `docs/superpowers/specs/2026-10-12-city-missions-design.md`

## Global Constraints

- No new Cargo dependencies; gameplay modules never import wgpu; all new behaviour is deterministic in `(profile_seed, round, gold)` and tested on the host.
- Urban slots: 2 = cargo (city), 3 = race (city streets), 4 = rings (city when `hash01(key, slot, 78) < 0.5`, else open as in v2.2), 5 = sling on roofs (even rounds, always urban) or recon (odd rounds, open). Slots 0 and 1 are the open missions of v2.1 and stay as they are.
- Grid: `(BLOCKS + 1) x (BLOCKS + 1)` = 12 x 12 intersections, pitch 48 m; streets 12 m; the first route step is always east (+i), so an urban start faces east (`start_yaw = pi / 2`, forward = +x).
- Urban cargo goal: Manhattan distance 300-600 m from the start; urban race: 6-10 checkpoints on route turns plus the end, legs along streets (multiples of 48 m); urban rings: 5-8 at segment midpoints on the street centre line, 18-40 m above the plateau; sling pads on roofs (>= 20 m high, lot >= 14 x 14 m), A within 300 m of the start, B 200-600 m from A.
- Time limits: urban cargo/race `1.6 * len / 8 + 20`; urban rings `1.5 * len / 10 + 20`; sling `1.6 * len / 12 + 35` (unchanged); open missions unchanged.
- The menu array is `4 + 6 * 10 = 64` floats (tenth float per mission = `urban` 0/1); Kotlin stride 10.
- Ground and open missions keep their v2.3 tests (200-seed rule tests) except where the spec says urban ones are excluded.
- UI strings bilingual via `t(en, ru)`; docs EN+RU. Commit messages end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Rulings vs the spec (decided while planning): (a) sling is always urban now (open sling pads are gone; `airgen::sling_points` stays unused for slot 5 but is kept for fallbacks); (b) urban race checkpoints are the turning intersections of the route (the first leg is straight east), cut to 10, with a deterministic staircase fallback; (c) rings in the city use `start_yaw: None` so the aircraft faces the first ring; (d) urban missions use the city built with `City::build(sites[0], seed)`, the same one the world builds.

## Review Focus

- Urban starts for extreme seeds and every start row: on a street, outside buildings, car not pushed by the hull on spawn (Tasks 2, 4).
- Race: checkpoint count 6-10 always, each leg straight along a street, route never boxed in below the needed length (fallback staircase) (Task 2).
- Roof pads: always found (fallbacks), on roofs of at least 14 x 14 m, distances within the bands or the documented fallback (Task 2).
- Rings between houses: crossing detection with diagonal normals at street corners, an aircraft flying along a street really passes the first ring (Task 4).
- Menu array length/stride agreement Rust vs Kotlin (Tasks 3, 5); old profiles still load; open missions unchanged over 200 seeds (Task 3).
- The sling cargo on a roof stays put and can be delivered to roof B (Task 4).

## File Structure

Create: `rust/src/game/urban.rs` (+ `urban_tests.rs`).
Modify: `rust/src/game/{mod,city,missions,api,world}.rs` and their test files (`city_tests.rs`, `missions_tests.rs`, `airgen_tests.rs`, `api_tests.rs`, `world_tests.rs`, and every test file that builds a `Mission` literal: `run_tests.rs`, `airrun_tests.rs`, `meta_tests.rs`, `app_state_tests.rs`, `world_tests.rs`), `app/.../MissionsActivity.kt`, `.github/workflows/build.yml`, `docs/game.md`, README(.ru).md, CHANGELOG(.ru).md, `rust/Cargo.toml`, `app/build.gradle.kts`.
Branch `feat/city-missions`. Tests live in sibling `*_tests.rs` files wired with `#[path]`.

---

### Task 1: Street routes and intersections

**Files:** Modify `rust/src/game/city.rs`, `city_tests.rs`.

**Interfaces — Consumes:** `hash01`, `BLOCKS`, `PITCH`, `HALF`, `CitySite`. **Produces:** `City::intersection(&self, i: i32, j: i32) -> V3` (point on the street centre lines at the plateau height, `i` along +x, `j` along +z, `0..=BLOCKS`); `City::west_edge(&self, row: i32) -> V3` (= `intersection(0, row)`); `City::roof_pads(&self) -> &[V3]` (= `roofs`); `City::route(&self, key: u64, from: (i32, i32), segments: usize) -> Vec<(i32, i32)>` (the points after `from`; first step always east; self-avoiding; 4-neighbourhood; inside the grid; turn probability about 0.5 per step; may stop early when boxed in); `pub const GRID: i32 = BLOCKS + 1;`.

- [ ] **Step 1: Write failing tests** (append to `city_tests.rs`; stub the four methods with `unimplemented!()` first):
```rust
#[test] fn intersections_sit_on_the_street_centre_lines_at_the_plateau_height() {
    let c = City::build(site(), 4);
    let p = c.intersection(3, 7);
    assert_eq!((p.x, p.z, p.y), (1000.0 - HALF + 3.0 * PITCH, -700.0 - HALF + 7.0 * PITCH, 20.0));
    assert_eq!(c.west_edge(5), c.intersection(0, 5));
    assert!(c.inside(c.west_edge(5).x, c.west_edge(5).z));
    for (i, j) in [(0, 0), (GRID - 1, GRID - 1), (4, 9)] { let q = c.intersection(i, j); assert!(c.roof_at(q.x, q.z).is_none(), "intersection ({i},{j}) must be free of buildings"); }
    assert_eq!(c.roof_pads().len(), c.roofs.len());
}
#[test] fn routes_are_self_avoiding_street_walks_that_start_east() {
    let c = City::build(site(), 4);
    let mut turned = 0; let mut total_len = 0;
    for key in 0..200u64 {
        let from = (0, 1 + (key % 10) as i32);
        let r = c.route(key, from, 12);
        assert_eq!(r, c.route(key, from, 12), "deterministic");
        assert!(r.len() <= 12 && !r.is_empty());
        assert_eq!(r[0], (from.0 + 1, from.1), "the first step is east");
        let mut seen = vec![from];
        for w in std::iter::once(from).chain(r.iter().copied()).collect::<Vec<_>>().windows(2) {
            assert_eq!((w[1].0 - w[0].0).abs() + (w[1].1 - w[0].1).abs(), 1, "one block per step");
            assert!(w[1].0 >= 0 && w[1].0 < GRID && w[1].1 >= 0 && w[1].1 < GRID, "inside the grid");
            assert!(!seen.contains(&w[1]), "revisited {:?}", w[1]);
            seen.push(w[1]);
        }
        total_len += r.len();
        let dirs: Vec<(i32, i32)> = std::iter::once(from).chain(r.iter().copied()).collect::<Vec<_>>().windows(2).map(|w| (w[1].0 - w[0].0, w[1].1 - w[0].1)).collect();
        if dirs.windows(2).any(|d| d[0] != d[1]) { turned += 1; }
    }
    assert!(turned > 150, "routes must turn: {turned}/200");
    assert!(total_len > 200 * 9, "routes are rarely boxed in early: {total_len}");
}
```
- [ ] **Step 2: Run to verify failure** — `cd rust && cargo test --lib game::city` → FAIL (not implemented).
- [ ] **Step 3: Implement** in `city.rs`:
```rust
pub const GRID: i32 = BLOCKS + 1;

impl City {
    pub fn intersection(&self, i: i32, j: i32) -> V3 {
        v3(self.site.centre.x - self.site.half + i as f32 * PITCH, self.site.centre.y, self.site.centre.z - self.site.half + j as f32 * PITCH)
    }
    pub fn west_edge(&self, row: i32) -> V3 { self.intersection(0, row) }
    pub fn roof_pads(&self) -> &[V3] { &self.roofs }

    /// A self-avoiding walk over the intersection grid (the points after `from`); the first step is east.
    pub fn route(&self, key: u64, from: (i32, i32), segments: usize) -> Vec<(i32, i32)> {
        let mut out: Vec<(i32, i32)> = Vec::new();
        let mut cur = from; let mut dir = (1, 0);
        let free = |p: (i32, i32), seen: &[(i32, i32)]| p.0 >= 0 && p.0 < GRID && p.1 >= 0 && p.1 < GRID && p != from && !seen.contains(&p);
        for step in 0..segments {
            let next = if step == 0 { (cur.0 + 1, cur.1) } else {
                let turn = hash01(key, step as i32, 31) < 0.5;
                let left = (-dir.1, dir.0); let right = (dir.1, -dir.0);
                let flip = hash01(key, step as i32, 32) < 0.5;
                let order: [(i32, i32); 4] = if turn {
                    if flip { [left, right, dir, (-dir.0, -dir.1)] } else { [right, left, dir, (-dir.0, -dir.1)] }
                } else { [dir, if flip { left } else { right }, if flip { right } else { left }, (-dir.0, -dir.1)] };
                match order.iter().map(|d| (cur.0 + d.0, cur.1 + d.1)).find(|p| free(*p, &out)) { Some(p) => p, None => break }
            };
            if !free(next, &out) { break; }
            dir = (next.0 - cur.0, next.1 - cur.1); cur = next; out.push(next);
        }
        out
    }
}
```
  (Delete the `unimplemented!()` stubs; the stubs are only for the red run.)
- [ ] **Step 4: Run** — `cargo test --lib game::city` → PASS.
- [ ] **Step 5: Commit** — `git add rust/src && git commit -m "feat(game): intersection grid and street routes for the city"` (+ trailer).

---

### Task 2: Urban mission points (`urban.rs`)

**Files:** Create `rust/src/game/urban.rs`, `urban_tests.rs`; modify `game/mod.rs`.

**Interfaces — Consumes:** `City`, `GRID`, `PITCH`, `BLOCKS`, `hash01`, `V3`. **Produces:** `#[derive(Clone, Debug)] pub struct Urban { pub start: V3, pub yaw: Option<f32>, pub points: Vec<V3> }`; `pub fn cargo(c: &City, key: u64) -> Urban` (yaw `Some(FRAC_PI_2)`, one goal point); `pub fn race(c: &City, key: u64) -> Urban` (yaw `Some(FRAC_PI_2)`, 6-10 points); `pub fn rings(c: &City, key: u64) -> Urban` (yaw `None`, 5-8 points); `pub fn sling(c: &City, key: u64) -> Urban` (yaw `None`, two roof pads); `pub fn start_row(key: u64) -> i32` (1..=10).

- [ ] **Step 1: Write failing tests** — `urban.rs` with the type and `unimplemented!()` functions; `urban_tests.rs`:
```rust
use super::*;
use crate::game::{city::{City, CitySite, BLOCKS, HALF, PITCH}, math::v3, terrain::Terrain};
fn city(seed: u64) -> City { let t = Terrain::new(seed); City::build(t.sites()[0], seed) }
fn on_street(c: &City, p: V3) -> bool {
    let (u, w) = ((p.x - (c.site.centre.x - HALF)) / PITCH, (p.z - (c.site.centre.z - HALF)) / PITCH);
    let off = |v: f32| (v - v.round()).abs() * PITCH;
    c.inside(p.x, p.z) && (off(u) <= 6.0 || off(w) <= 6.0)
}
fn manhattan(a: V3, b: V3) -> f32 { (a.x - b.x).abs() + (a.z - b.z).abs() }

#[test] fn start_rows_are_inside_the_grid_and_the_start_is_a_free_west_intersection() {
    for key in 0..200u64 {
        let row = start_row(key); assert!((1..=10).contains(&row));
        let c = city(key % 7 + 1);
        for u in [cargo(&c, key), race(&c, key), rings(&c, key), sling(&c, key)] {
            assert_eq!(u.start, c.west_edge(row), "urban starts are the west edge intersection of the row");
            assert!(c.roof_at(u.start.x, u.start.z).is_none());
        }
    }
}
#[test] fn urban_cargo_goal_is_a_far_intersection_in_range() {
    for key in 0..300u64 {
        let c = city(key % 11 + 1); let u = cargo(&c, key);
        assert_eq!(u.points.len(), 1); assert_eq!(u.yaw, Some(std::f32::consts::FRAC_PI_2));
        let g = u.points[0];
        let d = manhattan(u.start, g);
        assert!(d >= 300.0 - 1e-3 && d <= 600.0 + 1e-3, "Manhattan {d}, key {key}");
        let (a, b) = ((g.x - (c.site.centre.x - HALF)) / PITCH, (g.z - (c.site.centre.z - HALF)) / PITCH);
        assert!((a - a.round()).abs() < 1e-3 && (b - b.round()).abs() < 1e-3, "an intersection");
        assert!(c.roof_at(g.x, g.z).is_none());
    }
}
#[test] fn urban_race_checkpoints_follow_straight_street_legs() {
    for key in 0..300u64 {
        let c = city(key % 11 + 1); let u = race(&c, key);
        assert!((6..=10).contains(&u.points.len()), "{} points, key {key}", u.points.len());
        let mut prev = u.start;
        for p in &u.points {
            let (dx, dz) = ((p.x - prev.x).abs(), (p.z - prev.z).abs());
            assert!((dx < 1e-3) != (dz < 1e-3), "each leg runs along one street: {prev:?} -> {p:?}");
            let len = dx.max(dz); assert!((len / PITCH - (len / PITCH).round()).abs() < 1e-3 && len >= PITCH - 1e-3, "leg length {len}");
            assert!(on_street(&c, *p) && c.roof_at(p.x, p.z).is_none());
            prev = *p;
        }
        assert!(u.points[0].x > u.start.x && (u.points[0].z - u.start.z).abs() < 1e-3, "the first leg runs east");
    }
}
#[test] fn urban_rings_hang_over_the_street_between_the_houses() {
    for key in 0..300u64 {
        let c = city(key % 11 + 1); let u = rings(&c, key);
        assert!((5..=8).contains(&u.points.len()), "{} rings, key {key}", u.points.len());
        assert_eq!(u.yaw, None);
        for p in &u.points {
            assert!(on_street(&c, *p), "ring centre over a street line: {p:?}");
            let h = p.y - c.site.centre.y; assert!(h >= 18.0 - 1e-3 && h <= 40.0 + 1e-3, "height {h}");
        }
        let first = u.points[0];
        assert!(first.x > u.start.x && (first.z - u.start.z).abs() < 1e-3, "ring 0 is the midpoint of the first (east) segment: {first:?}");
        assert!((first.x - (u.start.x + PITCH / 2.0)).abs() < 1e-3);
    }
}
#[test] fn sling_pads_are_on_roofs_within_the_distance_bands() {
    let mut exact = 0;
    for key in 0..300u64 {
        let c = city(key % 11 + 1); let u = sling(&c, key);
        assert_eq!(u.points.len(), 2); assert_eq!(u.yaw, None);
        for p in &u.points {
            let b = c.buildings.iter().find(|b| p.x >= b.min.x && p.x <= b.max.x && p.z >= b.min.z && p.z <= b.max.z).expect("a pad must be on a building");
            assert!((p.y - b.max.y).abs() < 1e-3 && b.max.y - c.site.centre.y >= 20.0 - 1e-3 && b.size().x >= 14.0 - 1e-3 && b.size().z >= 14.0 - 1e-3);
        }
        assert_ne!(u.points[0], u.points[1]);
        let (da, ab) = (((u.points[0].x - u.start.x).hypot(u.points[0].z - u.start.z)), ((u.points[1].x - u.points[0].x).hypot(u.points[1].z - u.points[0].z)));
        if da <= 300.0 && ab >= 200.0 && ab <= 600.0 { exact += 1; }
    }
    assert!(exact > 270, "the distance bands should hold almost always: {exact}/300");
}
#[test] fn generators_are_deterministic_and_never_panic_for_extreme_keys() {
    let c = city(7);
    for key in [0u64, 1, u64::MAX, 0xFF_FFFF] {
        assert_eq!(cargo(&c, key).points, cargo(&c, key).points);
        assert_eq!(race(&c, key).points, race(&c, key).points);
        let _ = (rings(&c, key), sling(&c, key));
    }
    let tiny = City::from_buildings(CitySite { centre: v3(0., 0., 0.), half: HALF }, vec![]);
    let _ = (cargo(&tiny, 1), race(&tiny, 1), rings(&tiny, 1));      // an empty city must not panic (no roofs: sling falls back)
    let s = sling(&tiny, 1); assert_eq!(s.points.len(), 2);
    let _ = BLOCKS;
}
```
- [ ] **Step 2: Run to verify failure** — `cargo test --lib game::urban` → FAIL.
- [ ] **Step 3: Implement** (`urban.rs`):
```rust
//! Mission points inside a city: starts, goals, race checkpoints, rings and roof pads.
use super::city::{City, BLOCKS, GRID, PITCH};
use super::math::{v3, V3};
use super::terrain::hash01;

#[derive(Clone, Debug)]
pub struct Urban { pub start: V3, pub yaw: Option<f32>, pub points: Vec<V3> }

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
        if pts.len() >= 6 { let p: Vec<V3> = pts.into_iter().take(10).map(|(i, j)| c.intersection(i, j)).collect(); return Urban { start, yaw: Some(EAST), points: p }; }
    }
    // staircase: east, then alternately north/south and east (never leaves the grid)
    let up = row < GRID / 2;
    let mut cur = (0, row); let mut pts = Vec::new();
    for s in 0..9 { cur = if s % 2 == 0 { (cur.0 + 1, cur.1) } else { (cur.0, if up { cur.1 + 1 } else { cur.1 - 1 }) }; pts.push(cur); }
    Urban { start, yaw: Some(EAST), points: pts.into_iter().map(|(i, j)| c.intersection(i, j)).collect() }
}

/// 5-8 rings at the midpoints of the street segments of a route, 18-40 m above the plateau.
pub fn rings(c: &City, key: u64) -> Urban {
    let row = start_row(key); let start = c.west_edge(row);
    let want = 5 + (hash01(key, 7, 44) * 4.0) as usize % 4;
    let mut route = Vec::new();
    for k in 0..40u64 { let r = c.route(key ^ (k << 8), (0, row), 12); if r.len() >= want { route = r; break; } }
    if route.len() < want { route = (1..=want as i32).map(|i| (i.min(GRID - 1), row)).collect(); }
    let mut prev = (0, row);
    let points = route.iter().take(want).enumerate().map(|(n, &p)| {
        let a = c.intersection(prev.0, prev.1); let b = c.intersection(p.0, p.1); prev = p;
        v3((a.x + b.x) / 2.0, c.site.centre.y + 18.0 + 22.0 * hash01(key, n as i32, 45), (a.z + b.z) / 2.0)
    }).collect();
    Urban { start, yaw: None, points }
}

/// Pad A on a roof within 300 m of the start, pad B on another roof 200-600 m from A (nearest fallbacks).
pub fn sling(c: &City, key: u64) -> Urban {
    let row = start_row(key); let start = c.west_edge(row);
    let roofs = c.roof_pads();
    let dist = |a: V3, b: V3| (a.x - b.x).hypot(a.z - b.z);
    let pick = |cands: Vec<V3>, k: i32| -> Option<V3> { if cands.is_empty() { None } else { Some(cands[(hash01(key, k, 46) * cands.len() as f32) as usize % cands.len()]) } };
    let a = pick(roofs.iter().copied().filter(|r| dist(*r, start) <= 300.0).collect(), 1)
        .or_else(|| roofs.iter().copied().min_by(|x, y| dist(*x, start).partial_cmp(&dist(*y, start)).unwrap()))
        .unwrap_or(v3(start.x + 150.0, start.y + 30.0, start.z));
    let b = pick(roofs.iter().copied().filter(|r| (200.0..=600.0).contains(&dist(*r, a))).collect(), 2)
        .or_else(|| roofs.iter().copied().filter(|r| dist(*r, a) > 1.0).max_by(|x, y| dist(*x, a).partial_cmp(&dist(*y, a)).unwrap()))
        .unwrap_or(v3(a.x + 300.0, a.y, a.z));
    Urban { start, yaw: None, points: vec![a, b] }
}

#[cfg(test)]
#[path = "urban_tests.rs"]
mod tests;
```
  Add `pub mod urban;` to `game/mod.rs`. (Remove the unused `PITCH` import warning with `#[allow(unused_imports)]` only if the compiler complains.) For the `tiny` empty-city test the sling fallback returns synthetic pads; that is the only place a pad can be off a roof and is never used by `roll` (a real city always has roofs; assert that in Task 3).
- [ ] **Step 4: Run** — `cargo test --lib game::urban` → 6 PASS; tune the generation constants (not the bands) if a distance band test fails.
- [ ] **Step 5: Commit** — `git commit -m "feat(game): urban mission points: cargo goals, street races, rings and roof pads"`.

---

### Task 3: Urban slots in the mission list and the menu array

**Files:** Modify `rust/src/game/missions.rs`, `api.rs`, and the tests: `missions_tests.rs`, `airgen_tests.rs`, `api_tests.rs`, plus every `Mission { ... }` literal in tests (`run_tests.rs`, `airrun_tests.rs`, `meta_tests.rs`, `app_state_tests.rs`, `world_tests.rs`).

**Interfaces — Consumes:** `urban::{cargo, race, rings, sling, Urban}`, `City::build`. **Produces:** `Mission.urban: bool`, `Mission.start_yaw: Option<f32>`; `roll` builds urban missions for the slots in the Global Constraints; `menu_info`: 10 floats per mission (the tenth is `urban`), 64 total.

- [ ] **Step 1: Update and add tests.**
  - Literals: `sed -i 's/aircraft: None }/aircraft: None, urban: false, start_yaw: None }/; s/aircraft: Some(ac) }/aircraft: Some(ac), urban: false, start_yaw: None }/' rust/src/game/{run,airrun,meta,app_state,world}_tests.rs`; check with `grep -rn "Mission {" rust/src/game/*_tests.rs` that every literal compiles later.
  - `missions_tests.rs`: `generated_points_obey_the_rules_over_many_seeds` iterates `.filter(|m| !m.is_air() && !m.urban)`; `ground_mission_points_stay_out_of_cities` iterates `!m.is_air() && !m.urban`; add:
```rust
#[test] fn the_list_has_the_urban_slots_and_open_missions_are_unchanged() {
    for ps in 0..100u64 { let l = roll(ps * 7919, (ps % 4) as u32, 0);
        assert_eq!(l.iter().map(|m| m.urban).collect::<Vec<_>>()[..4], [false, false, true, true], "slots 0-3");
        assert!(l[5].urban == (l[5].kind == MissionKind::Sling), "sling is urban, recon is open");
        for m in l.iter().filter(|m| m.urban) {
            let t = Terrain::new(m.seed); let c = crate::game::city::City::build(t.sites()[0], m.seed);
            assert!(c.inside(m.start.x, m.start.z) && c.roof_at(m.start.x, m.start.z).is_none(), "urban start in the city, seed {ps}");
            assert!(m.start_yaw.is_some() == (matches!(m.kind, MissionKind::Cargo | MissionKind::Race)));
        }
        for m in l.iter().filter(|m| !m.urban) { assert!(m.start_yaw.is_none() && m.start.x == 0.0 && m.start.z == 0.0); }
    }
    let rings_urban = (0..200u64).filter(|ps| roll(ps * 31, 0, 0)[4].urban).count();
    assert!(rings_urban > 60 && rings_urban < 140, "about half of the rings are urban: {rings_urban}");
}
#[test] fn urban_time_limits_follow_the_formulas() {
    for ps in 0..100u64 { for m in roll(ps * 7919, (ps % 2) as u32, 0).into_iter().filter(|m| m.urban) {
        let want = match m.kind {
            MissionKind::Cargo | MissionKind::Race => 1.6 * m.length_m / 8.0 + 20.0,
            MissionKind::Rings => 1.5 * m.length_m / 10.0 + 20.0,
            _ => 1.6 * m.length_m / 12.0 + 35.0,
        };
        assert!((m.time_limit - want).abs() < 1e-2, "{:?} {} vs {}", m.kind, m.time_limit, want);
        assert!((m.length_m - path_length(m.start, &m.checkpoints)).abs() < 1e-2);
    } }
}
#[test] fn urban_missions_never_panic_for_extreme_arguments() {
    for (ps, r, g) in [(0u64, 0u32, 0u32), (u64::MAX, u32::MAX, u32::MAX), (1, 1, 99)] { assert_eq!(roll(ps, r, g).len(), 6); }
}
```
  `airgen_tests.rs`: in `air_missions_carry_the_right_aircraft_and_valid_points` the Sling arm now expects the pad `p.y` at a building top (`m.urban`), so skip the gentle-ground check for urban sling and for urban rings (`if m.urban { continue }` after the aircraft/limit assertions that still apply); `rings_never_intersect_buildings` filters `!m.urban`; `the_flattest...` unchanged. `api_tests.rs`: length 64; kinds at `a[4], a[14], a[24], a[34], a[44], a[54]` = `0,1,0,1,2,3`; urban flags at `a[13], a[23], a[33], a[43], a[63]`: `a[13]==0`, `a[23]==0`... (slot 0: index 4+9=13 → 0; slot 1: 23 → 0; slot 2: 33 → 1; slot 3: 43 → 1; slot 5: 63 → 1).
- [ ] **Step 2: Run to verify failure** — `cargo test --lib` → compile errors (`urban`, `start_yaw` missing).
- [ ] **Step 3: Implement.** `missions.rs`: add `pub urban: bool, pub start_yaw: Option<f32>` to `Mission`; in `roll`, build the urban data first:
```rust
let city = || super::city::City::build(t.sites()[0], seed);          // every level has at least one city
let (urban_data, urban) = match (slot, kind) {
    (2, MissionKind::Cargo) => (Some(urban::cargo(&city(), key)), true),
    (3, MissionKind::Race) => (Some(urban::race(&city(), key)), true),
    (4, MissionKind::Rings) if hash01(key, slot as i32, 78) < 0.5 => (Some(urban::rings(&city(), key)), true),
    (5, MissionKind::Sling) => (Some(urban::sling(&city(), key)), true),
    _ => (None, false),
};
```
  then use `urban_data` when present: `checkpoints = u.points`, `start = u.start`, `start_yaw = u.yaw`; the aircraft for urban rings is chosen as before (`drone` by `hash01(key, slot, 77)`), for urban sling `Heli`; `time_limit`: urban `Cargo | Race` → `1.6 * len / 8 + 20`, urban `Rings` → `1.5 * len / 10 + 20`, urban `Sling` → `1.6 * len / 12 + 35`; open missions as before (including `clear_of_buildings` for open rings); the open `Sling` branch (`airgen::sling_points`) becomes unreachable and is removed from the `match` except for the open fallback (keep `sling_points` for tests). `Mission { …, urban, start_yaw }` with `start` from `urban_data` or `at(&t, 0.0, 0.0)`; `use super::urban;`. `api.rs`: append `if m.urban { 1.0 } else { 0.0 }` after the aircraft code and update the doc comment (10 floats per mission).
- [ ] **Step 4: Run** — `cargo test --lib` → all PASS (200-seed open-mission rules unchanged; if a ground open mission test fails, the urban filter is wrong, not the rule).
- [ ] **Step 5: Commit** — `git commit -m "feat(game): urban slots in the mission list and the urban flag in the menu array"`.

---

### Task 4: Urban starts in the world and playability checks

**Files:** Modify `rust/src/game/world.rs`, `world_tests.rs`.

**Interfaces — Consumes:** `Mission.start`, `Mission.start_yaw`, `City::resolve_hull`, `run::bearing`, `ring_normal`. **Produces:** `World::for_mission` places the car (at `terrain.height + 1.1`) or the aircraft (3 m above the ground) at `m.start`'s x/z, with yaw `m.start_yaw` or towards the first checkpoint.

- [ ] **Step 1: Write failing tests** (append to `world_tests.rs`):
```rust
use crate::game::{missions::roll, run::{bearing, Outcome}};
fn urban_missions(kind: MissionKind, n: u64) -> Vec<Mission> {
    (0..n).flat_map(|ps| roll(ps * 7919, 0, 0)).filter(|m| m.urban && m.kind == kind).collect()
}
fn hull_free(w: &World) -> bool {
    let v = &w.vehicle;
    [-1.4f32, 0.0, 1.4].iter().all(|z| { let c = v.pos + v.rot.rotate(v3(0.0, 0.5, *z)); w.cities.iter().all(|ct| ct.push_out(c, 1.0).is_none()) })
}
#[test] fn urban_cars_start_on_the_street_outside_buildings_facing_east() {
    for m in urban_missions(MissionKind::Cargo, 40).into_iter().chain(urban_missions(MissionKind::Race, 40)) {
        let w = World::for_mission(m.clone());
        assert!((w.vehicle.pos.x - m.start.x).abs() < 1e-3 && (w.vehicle.pos.z - m.start.z).abs() < 1e-3);
        assert!(w.vehicle.forward().x > 0.99, "east: {:?}", w.vehicle.forward());
        assert!(hull_free(&w), "the hull overlaps a building at the start");
    }
}
#[test] fn a_simple_autopilot_reaches_the_first_urban_race_checkpoint_along_the_street() {
    let mut reached = 0; let races = urban_missions(MissionKind::Race, 60);
    assert!(races.len() >= 30);
    for m in races {
        let mut w = World::for_mission(m.clone());
        let target = m.checkpoints[0];
        for _ in 0..(40.0 / (1.0 / 60.0)) as usize {
            let f = w.vehicle.forward();
            let steer = (bearing(f.x, f.z, w.vehicle.pos, target) * 2.0).clamp(-1.0, 1.0);
            w.step(&Input { throttle: 0.5, steer, ..Default::default() }, 1.0 / 60.0);
            if (w.vehicle.pos - target).len() <= 6.0 { reached += 1; break; }
        }
    }
    assert!(reached >= 54, "reached the first checkpoint in {reached}/60 races");
}
#[test] fn urban_aircraft_start_above_the_street_and_reach_the_first_ring() {
    use crate::game::flight::FlightInput;
    let rings: Vec<Mission> = (0..80u64).flat_map(|ps| roll(ps * 31, 0, 0)).filter(|m| m.urban && m.kind == MissionKind::Rings).collect();
    assert!(rings.len() >= 20);
    let mut passed = 0;
    for m in &rings {
        let mut w = World::for_mission(m.clone());
        assert!((w.aircraft.as_ref().unwrap().pos.y - (w.terrain.height(m.start.x, m.start.z) + 3.0)).abs() < 1e-3);
        let target = m.checkpoints[0];
        for _ in 0..(40.0 / (1.0 / 60.0)) as usize {
            let a = *w.aircraft.as_ref().unwrap();
            let f = a.forward();
            let fi = FlightInput { yaw: (bearing(f.x, f.z, a.pos, target) * 2.0).clamp(-1.0, 1.0), pitch: 0.35, lift: ((target.y - a.pos.y) * 0.15).clamp(-1.0, 1.0), roll: 0.0 };
            w.step_with(&Input::default(), &fi, 1.0 / 60.0);
            if w.air.as_ref().unwrap().next_index() >= 1 { passed += 1; break; }
            if w.aircraft.as_ref().unwrap().status == crate::game::flight::Status::Crashed { break; }
        }
    }
    assert!(passed * 10 >= rings.len() * 8, "first ring passed in {passed}/{}", rings.len());
}
#[test] fn the_sling_cargo_spawns_on_the_roof_pad_and_stays() {
    let slings: Vec<Mission> = (0..40u64).flat_map(|ps| roll(ps * 7919, 0, 0)).filter(|m| m.kind == MissionKind::Sling).collect();
    for m in slings {
        let mut w = World::for_mission(m.clone());
        for _ in 0..180 { w.step_with(&Input::default(), &crate::game::flight::FlightInput::default(), 1.0 / 60.0); }
        let s = w.sling.as_ref().unwrap();
        assert!((s.pos.y - (m.checkpoints[0].y + 0.3)).abs() < 0.2 && s.vel.len() < 0.2 && !s.attached, "cargo drifted: {:?} vs pad {:?}", s.pos, m.checkpoints[0]);
    }
}
#[test] fn open_missions_still_start_on_the_origin_plateau() {
    let m = roll(5, 0, 0).into_iter().find(|m| !m.urban && !m.is_air()).unwrap();
    let w = World::for_mission(m);
    assert!(w.vehicle.pos.x.abs() < 1e-3 && w.vehicle.pos.z.abs() < 1e-3);
    let _ = Outcome::Failed(crate::game::run::Failure::TimeUp);
}
```
- [ ] **Step 2: Run to verify failure** — `cargo test --lib` → the urban start tests FAIL (the world still starts at the origin).
- [ ] **Step 3: Implement** in `World::for_mission`: air branch `let p0 = v3(m.start.x, w.terrain.height(m.start.x, m.start.z) + 3.0, m.start.z);` and `yaw = m.start_yaw.unwrap_or(<towards the first checkpoint as before>)`; ground branch: `let pos = v3(m.start.x, w.terrain.height(m.start.x, m.start.z) + 1.1, m.start.z); let yaw = m.start_yaw.unwrap_or(<towards the first checkpoint>); w.vehicle = Vehicle::new(VehicleSpec::offroader(), pos, yaw); w.cam = FollowCam::new(&w.vehicle);` (also when `dx.hypot(dz) <= 1.0`, keep yaw 0). Open missions have `start = (0, y, 0)` so their behaviour is unchanged.
- [ ] **Step 4: Run** — `cargo test --lib` → all PASS. If the autopilot test fails because the car drives into the curb or a building at the first turn, check that the first leg really is straight east and shorter than the start street; fix generation, not the test bands (the test asks for 90 %).
- [ ] **Step 5: Commit** — `git commit -m "feat(game): urban missions start in the city"`.

---

### Task 5: Menu, CI smoke, docs and version 2.4.0

**Files:** Modify `app/.../MissionsActivity.kt`, `.github/workflows/build.yml`, `docs/game.md`, README(.ru).md, CHANGELOG(.ru).md, `rust/Cargo.toml` (`2.4.0`), `app/build.gradle.kts` (`versionCode = 14`, `versionName = "2.4.0"`), `Native.kt` doc comment.

- [ ] **Step 1: Kotlin** — `MissionsActivity`: stride 10 (`val o = 4 + i * 10`); `val urban = info[o + 9] > 0.5f`; the title becomes `kindName + acName + (if (urban) " · " + t("City", "Город") else "") + "   " + "★".repeat(diff)`; briefing texts: urban cargo `t("Drive through the streets to the goal; buildings are solid.", "Проедь по улицам до цели; здания твёрдые.")`, urban race `t("Race through the street checkpoints in order.", "Проедь по порядку чекпойнты на улицах.")`, urban rings `t("Fly through the rings over the street, between the houses.", "Пролети кольца над улицей между домами.")`, sling (always roofs) `t("Hook the cargo on one roof and set it down on the green pad of another roof; swinging and bumps damage it.", "Зацепи груз на одной крыше и опусти на зелёную площадку другой крыши; раскачивание и удары его портят.")`. Update the `missionsInfo` doc in `Native.kt` (10 floats, tenth = urban, total 64).
- [ ] **Step 2: CI** — add to the game smoke step a run `RAYTRACE_SEED=7 RAYTRACE_MISSION=3 RAYTRACE_PROFILE=$PWD/profile-smoke.json RAYTRACE_AUTODRIVE=1 ... --game` (sleep 22, `screenshot-urban.png`) with the same greps as the other smoke runs.
- [ ] **Step 3: Docs and version** — `docs/game.md` (EN then RU): in the Missions section a paragraph on urban missions (which slots, that they start in the city, roofs for the sling, cars and aircraft cannot pass through buildings); README lines; CHANGELOG `## 2.4.0 - city missions` (EN/RU); version bumps as above.
- [ ] **Step 4: Verify** — `cd rust && cargo test --lib` (all PASS); `bash tools/build-native.sh && ./gradlew -x buildNative assembleDebug` (BUILD SUCCESSFUL); push the branch and dispatch the CI workflow; look at `screenshot-urban.png` (the car on a street between buildings); report.
- [ ] **Step 5: Commit** — `git commit -m "feat: urban missions in the menu, docs and version 2.4.0"`.
- [ ] **Step 6: Hand off** — do not merge, tag or touch the phone. Ask the owner (one message): permission for a short on-phone check of an urban race, an urban rings flight and a roof sling (the feel of driving and flying between buildings), and permission to merge and release v2.4.0; act only after both answers.
