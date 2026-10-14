# Stage relief Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline). Steps use checkbox syntax.

**Goal:** crests/jumps, ditches, a gravel verge and roadside rocks on the stage road.

**Architecture:** numbers in `rust/roads.json` -> `roads/cfg.rs`; crests added to the profile in `build_road`; ditch in `Terrain::height`; verge in `surface_at`; rocks in `props.rs`.

**Tech Stack:** Rust (host tests `cargo test --lib`), docs.

**Spec:** `docs/superpowers/specs/2026-10-14-stage-relief-design.md`

## Global Constraints

- Branch `feat/relief`, release v2.18.0 only after the user's go; EN+RU docs and changelog; no opus review subagent; one device check, ask the user for the phone.
- Deterministic per seed; the road network stays connected; existing 361 tests stay green; naga shader test untouched (no shader change).

## Review Focus

- Crest next to a lake: no crest on a causeway. - Crest near a sharp bend: skipped. - Tiny/huge road (short stage): no crest within 150 m of ends, no panic on `n < 3`. - Bad `roads.json` override: readable error, falls back to the embedded numbers. - Ditch on the liaison/city gate: shallow, never under a city.

### Task 1: roads.json and the config loader

**Files:** Create `rust/roads.json`, `rust/src/game/roads/cfg.rs`, `rust/src/game/roads/cfg_tests.rs`; modify `roads/mod.rs` (`pub mod cfg;`), the place where the profile dir override of `surfaces.json` is read (`Game::with_profile` in `game/app.rs`).

**Interfaces:** Produces `RoadsCfg { crest: CrestCfg, ditch_stage: DitchCfg, ditch_liaison: DitchCfg, verge: f32, rock_band: (f32, f32), rock_candidates: u32 }`, `RoadsCfg::from_json(&str) -> Result<RoadsCfg, String>`, `cfg() -> &'static RoadsCfg`, `load_override(path)`, `reset_cfg()`.

- [ ] Write `cfg_tests.rs`: embedded json parses; each numeric range rejected when invalid (crest height min>max, length<=0, ditch depth<0, verge<0) with a message naming the field; garbage json returns Err.
- [ ] Run `cargo test --lib roads::cfg` - FAIL (module missing).
- [ ] Implement by copying the `GripTable` pattern from `game/grip.rs` (embedded `include_str!`, `OnceLock`/`RwLock`, override file); values: crest height [0.6,2.0], length [30,60], spacing [150,250], series [3,5], series_gap [500,900], min_radius 120, keep_ends 150, asym 0.35, max_slope 0.10, causeway_clear 1.5; ditch stage {depth 0.55, width 1.3, offset 1.8}, liaison {0.25, 1.0, 1.8}; verge 0.8; rocks band [4,8], candidates 24.
- [ ] Wire `load_override` next to the `surfaces.json` override in `Game::with_profile` and `RAYTRACE_ROADS` env for the desktop.
- [ ] Run tests - PASS. Commit `feat: roads.json and its loader`.

### Task 2: crests in the profile

**Files:** `roads/shape.rs`, `roads/shape_tests.rs`, `roads/gen.rs`, `roads/gen_tests.rs`.

**Interfaces:** Produces `shape::add_crests(y: &mut [f32], xz: &[(f32, f32)], spacing: f32, floor: f32, seed: u64, c: &CrestCfg)`; `floor` is the water level (or `f32::MIN`). Consumes `CrestCfg` of Task 1.

- [ ] Tests (`shape_tests.rs`): on a straight flat line 3000 m long, `add_crests` gives >= 4 crests (local maxima above 0.5 m over the base), `y[0]` and `y[n-1]` unchanged, no slope added above `max_slope + 1e-3`, no crest within `keep_ends` of the ends, identical result for the same seed, different for another seed; on a line with a 40 m radius bend in the middle no crest within its window; with `floor` just under the road (road 1.0 m above the floor) no crest; `n < 3` returns unchanged.
- [ ] Run - FAIL.
- [ ] Implement: walk `s` from `keep_ends` to `len - keep_ends`; series loop picks count in [3,5] and spacing per crest from `hash01(seed ^ 0xC7E57, k, 0)`; for each candidate centre check the curve radius (use `radius()` on points +-window/2 steps) >= min_radius and `y[i] - floor >= causeway_clear`; add `h * (1 + cos(pi*u)) / 2` with `u = d / half` where `half` is the take-off half-length (`L/2*(1+asym)`) before the centre and the landing half-length (`L/2*(1-asym)`) after it; `h = min(h, max_slope * 2 * landing_half / pi)`.
- [ ] In `build_road` call `add_crests` after `shape::profile` when `kind == Stage` (needs `cfg()`); gen_tests: for seeds 1..=8 the generated stage has >= 4 crests, the grade between consecutive points (spacing apart) stays < 0.25, the network still passes the existing connectivity/fold tests.
- [ ] Run `cargo test --lib roads` - PASS. Commit `feat: crests and jumps on the stage`.

### Task 3: ditches and the gravel verge

**Files:** `game/terrain.rs`, `game/terrain_roads_tests.rs`, `game/surface.rs` (+ its tests).

**Interfaces:** Consumes `RoadsCfg::ditch_*`, `verge`; `Terrain::height` and `surface_at` keep their signatures.

- [ ] Tests (`terrain_roads_tests.rs`, synthetic net via `with_net`): height 1.8 m from the stage edge is lower than the ground there without the ditch by about 0.55 m (+-0.05); the road centre height is unchanged; 1.8 m from a liaison edge the ditch is ~0.25 m; no ditch where the road is 0.5 m above the water; `surface_at` 0.5 m beyond a stage edge is Gravel, 1.5 m Dirt, and 0.5 m beyond a liaison edge Dirt.
- [ ] Run - FAIL.
- [ ] Implement in `Terrain::height` road blend: `e = hit.dist - hit.half`; `t = ((e - offset).abs() / width)`; `ditch = depth * (1 - smoothstep(0, 1, t))`; skip when the road y is below `water + causeway_clear`; subtract after the blend. In `surface_at`: Stage edge to `verge` gives Gravel.
- [ ] Run the whole suite - PASS (tests relying on the old verge surface adjusted with a stated reason). Commit `feat: ditches and a gravel verge`.

### Task 4: roadside rocks

**Files:** `game/props.rs`, `game/props_tests.rs`.

- [ ] Test: for seed 21, count rocks whose edge distance to a stage road is in [4, 8] over the tiles around the stage: > 40; none closer than 4 m; deterministic.
- [ ] Run - FAIL. Implement: after the main loop, `rock_candidates` more candidates per tile (hash keys distinct from the existing ones), accepted when `road_edge_distance` in the band, the nearest road is a stage, and normal.y >= 0.85, not in water/city; produce `PropKind::Rock` with radius 0.5-1.4.
- [ ] Run - PASS. Commit `feat: rocks along the stage edge`.

### Task 5: physics sanity, docs, version

**Files:** `game/vehicle_tests.rs` or a new `game/jump_tests.rs`, `docs/game.md`, `docs/vehicles.md`?, `CHANGELOG(.ru).md`, README bullets, `app/build.gradle.kts` (versionCode 30, `2.18.0`), `rust/Cargo.toml`.

- [ ] Test: a synthetic road with one 2 m asymmetric crest; the Vulpes at 30 m/s: some wheel has `grounded == false` for >= 0.2 s on the way over, then all four are grounded again, speed finite and below 60 m/s, `up.y > 0.3` at the end.
- [ ] Run - adjust only the numbers in `roads.json` if the test shows a bad landing; commit.
- [ ] Docs EN+RU (Roads section: crests, ditches, verge; `roads.json` fields and override), changelog 2.18.0 EN+RU, README bullets, versions.
- [ ] Full `cargo test --lib` + Gradle unit tests pass. Commit `docs: stage relief 2.18.0`.
- [ ] Ask the user for the phone for one screenshot (crest, ditch, fps) before the release; release only after their go.
