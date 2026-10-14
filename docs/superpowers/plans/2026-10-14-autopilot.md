# Autopilot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline).

**Goal:** an autopilot button that joins the stage and drives it to the finish; the test driver for the author.

**Architecture:** pure `game/autopilot.rs` (route, speed profile, controller) driven from `World::step`; JNI command and a Kotlin button.

**Tech Stack:** Rust (host tests `cargo test --lib`), Kotlin.

**Spec:** `docs/superpowers/specs/2026-10-14-autopilot-design.md`

## Global Constraints

- Branch `feat/relief` (release v2.18.0 together with the relief, after the user's go); EN+RU docs; no review subagent; the phone is used only with the user's OK (they said "check it" for the relief; ask again for this).
- Numbers in `rust/autopilot.json`; deterministic; existing 374 tests stay green; every Kotlin string in EN+RU.

## Review Focus

- Start off the road in a lake region: planner must not drive into water. - Start flipped. - Stage with a crest right at the end. - Player touches the wheel while on: autopilot off at once. - New level while on: off, no stale route. - Bad `autopilot.json`: readable error, embedded numbers.

### Task 1: autopilot.json and loader
Files: `rust/autopilot.json`, `game/autopilot/cfg.rs` (+tests), wire the override next to `roads.json` in `Game::with_profile` and `desktop.rs`. Pattern: `roads/cfg.rs`. Produces `AutoCfg` and `cfg()`.
- [ ] tests (embedded parses; each range rejected with the field name; garbage JSON) - FAIL - implement - PASS - commit.

### Task 2: the route and the speed profile
Files: `game/autopilot/route.rs` (+tests). Produces `Route { pts: Vec<(V3 pos, f32 s)>, kappa, speed }`, `Route::from_road(road, cfg, mu_fn, car) `, `nearest(x, z) -> (index, dist)`, `point_at(s)`.
- [ ] tests: circle -> `sqrt(lat*mu*g*R)`; straight -> car max; backward pass brakes before a hairpin (speed at 60 m before <= the hairpin speed + sqrt(2*a*60)); offset stays inside `offset_max*(half-1.4)`; offset lowers max curvature and lengthens the route < 2 %; crest cap in the 40 m before a crest top.
- [ ] implement; commit.

### Task 3: the controller and recovery
Files: `game/autopilot/mod.rs` (+ `controller_tests.rs`). Produces `Autopilot::{on_stage(net, vehicle, ..), drive}`, `AutoCmd`.
- [ ] closed-loop tests with the real `Vehicle` on synthetic roads built with `Terrain::flat().with_net(..)`: an S-bend, a 40 m radius hairpin, a crest on a straight (lands, continues), a stuck car (pinned by `vel = 0` and a wall of props? use `force` of speed zero for 5 s) gets `reset`; a flipped car gets `reset`; at the end the car stops within `finish_stop + 10` m and `done`.
- [ ] implement; tune only `autopilot.json`; commit.

### Task 4: joining from the field
Files: `game/autopilot/join.rs` (+tests). `plan_field_path(terrain, from) -> Option<Vec<(f32, f32)>>` with `path::Grid` (32 m cells) to the nearest stage point.
- [ ] tests: for seeds 1..=4 start 100-400 m from the stage (not in water): the path ends on the stage, never crosses water (`is_water`), is shorter than 2.2 x the straight distance; a car within 15 m of the stage joins directly.
- [ ] implement; commit.

### Task 5: integration
Files: `world.rs`, `app.rs`, `game_jni.rs`, `Native.kt`, `GameActivity.kt`, `DriveControls.kt` (touch hands control back), HUD.
- [ ] `World.autopilot`, `World::set_autopilot(bool)`, `step` uses it; state slot `[29]`; `GCmd::Autopilot`; Kotlin button (tonal, highlighted when on) + HUD word; intent extra `autopilot`; a new level / reset / player input turns it off. Test: a world with autopilot on drives the first 600 m of a generated stage for seed 21 without leaving the road (host).
- [ ] whole-stage closed-loop test for seeds 1..=6 (parallel; `#[ignore]` if it needs > 90 s, run it by hand once and note the result in the ledger).
- [ ] commit.

### Task 6: docs, device check
- [ ] docs/game.md (Autopilot section EN+RU), CHANGELOG 2.18.0 EN+RU entry, README bullet, `docs/autopilot.md`? (numbers table in game.md is enough).
- [ ] ask the user for the phone: one run with screenshots, fps; tune numbers; then the release question for v2.18.0.
