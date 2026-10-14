# Missions and menus (v2.1) — design

Status: draft for review. Date: 2026-10-11. Builds on `2026-10-11-game-core-design.md` (v2.0, released).

## 1. Goal

Turn free drive into a game loop: pick a mission from a list, drive it, get a result, a medal and money.
Two mission types in this stage: **cargo delivery A→B** and **checkpoint race**. Main menu, mission list,
briefing, HUD, pause, results and a saved profile.

Decisions made with the user (this stage): missions are chosen from a list of random missions (no open-world
markers, no fixed campaign); delivery difficulty = time + cargo integrity.

Assumptions (editable): one vehicle (the off-roader; garage and spending money come in v2.3); money is only
earned and shown; Material 3 Expressive UI like the rest of the app; no sound; offline.

Out of scope: garage and spending, rescue, NPC orders, air vehicles, cities, ghost replays, sound.

## 2. Architecture

New pure-CPU modules under `rust/src/game/` (no wgpu; unit-tested on the host):

- `missions.rs`: `Mission`, `MissionKind{Cargo,Race}`, `MissionGen`.
- `run.rs`: `MissionRun`, the state machine of one attempt.
- `meta.rs`: `Profile`, JSON save/load.
- `medals.rs`: `Medal{None,Bronze,Silver,Gold}`, `grade()`, `reward()` (pure functions).

`World` gets an optional `MissionRun`; the loop (`game::app::Game`) steps both; the renderer reads
`WorldView.markers` (beacon pillar, checkpoint rings) and draws them with the existing instance pipelines.
Kotlin polls a flat state array through JNI and shows menus and HUD.

### 2.1 Mission model

```
Mission { kind, seed: u64 (24-bit level seed), difficulty: 1..=3, start: V3, goal: V3,
          checkpoints: Vec<V3> (race, goal is the last), length_m: f32, time_limit: f32, reward_base: f32 }
```
- Level seed is the terrain seed; the vehicle spawns at the origin plateau as in v2.0.
- `length_m` = path length start → checkpoints → goal. `time_limit = 1.6 * length_m / 12 + 20` seconds.
- `reward_base = (100 + 0.15 * length_m) * difficulty_factor` with factors 1.0, 1.4, 1.9.

### 2.2 Generation (`MissionGen::roll(profile_seed, round) -> Vec<Mission>`)

- Always 4 missions: 2 cargo, 2 race; deterministic in `(profile_seed, round)`; "Refresh" increments `round`.
- Difficulty by slot: 1–2 stars until the profile holds 3 gold medals, then 3 stars may appear.
- Points: goal 400–1500 m from the origin (distance scales with difficulty), checkpoints 5–8 for races along
  the line start→goal with lateral zigzag ≤ 80 m. Every point must satisfy: slope normal.y ≥ 0.94 (about 20°),
  not within 8 m of a prop, at least 40 m inside `WORLD_LIMIT`. Reachability: the straight line start→point has
  no height jump above 40 m between 10 m samples; otherwise resample (max 20 tries, then pick the best
  candidate found).
- Seeds always `level_seed(...)` (24-bit).

### 2.3 Run state machine (`MissionRun`)

`MissionRun::new(&Mission)`, `update(&mut self, w: &World, input_active: bool, dt: f32) -> Option<Event>`.
States: `Ready` (timer not started) → `Running` → `Done(Passed{medal,reward}|Failed(reason))`.
- Timer starts on the first non-zero throttle; `Failed(TimeUp)` when it passes `time_limit`.
- Cargo integrity (cargo missions only), 0–100, starts 100:
  - impact: per physics step `dv = |vel_now - vel_prev|`; if `dv > 6` damage `(dv - 6) * 2.5`;
  - landing: airborne (no wheel contact) longer than 0.4 s, then `vy < -7` at touchdown: damage `(|vy| - 7) * 3`;
  - flipped: 8 per second;
  - reset button: 5 damage.
  At 0: `Failed(CargoDestroyed)`.
- Cargo: reaching the goal radius (6 m) ends the run. Race: checkpoints in order, each within 6 m; the last
  one is the finish.
- `Failed(Abandoned)` when the player leaves via the menu.
- A fixed-step determinism rule: all time comes from `dt`, never from wall clock.

### 2.4 Medals and money (`medals.rs`)

- Cargo: Gold if `t <= 0.6*limit && integrity >= 90`; Silver if `t <= 0.8*limit && integrity >= 60`; else Bronze.
- Race: Gold `t <= 0.6*limit`, Silver `t <= 0.8*limit`, else Bronze.
- Reward = `reward_base * {1.0, 1.5, 2.2}` for Bronze/Silver/Gold, rounded to whole units.
- Replaying the same mission seed: pays only the difference to the best medal earned so far for that mission
  seed (improving a medal pays the difference, equal or worse pays 50% of the base for that medal).

### 2.5 Profile (`meta.rs`)

`Profile { version: u32, money: u32, profile_seed: u64, round: u32, results: Vec<{seed, kind, medal, best_time}> }`.
- JSON in the app's internal storage (`<files>/profile.json`; desktop `~/.config/rallynvx/profile.json`).
- Atomic write (temp file + rename). A corrupt file or a newer `version` is renamed to `profile.json.bak` and a
  fresh profile is created (never silently overwritten). `profile_seed` is random on first run.

### 2.6 Rendering of markers

`WorldView.markers: Vec<Marker{kind: Beacon|Ring|Next, pos, radius}>`. Beacon = tall translucent-looking
cylinder (opaque, bright emissive-looking colour, 60 m high) at the goal; rings = cylinders scaled flat
at checkpoints, the next checkpoint highlighted. Drawn through the existing `cylinder` instance list with the
fog applied; no new pipeline.

### 2.7 JNI surface and UI

Native additions (all non-blocking, state copied from the game thread under the existing mutex):
`missionList(round) -> FloatArray` (flattened mission cards), `missionStart(index)`, `missionState() -> FloatArray`,
`missionAbort()`, `profileState() -> FloatArray`, `missionRefresh()`. Result events are carried in the state array
(`phase`, `result`, `medal`, `reward`).

Kotlin (M3 Expressive, reusing the existing widgets):
- `MenuActivity` (replaces `StartActivity` as the launcher): Missions, Free drive, Scenes, Quality, money shown.
- Mission list: 4 cards (type icon text, stars, reward, length, best medal and time), Refresh button.
- Briefing: card details, Start. The level loads (spawn at origin, beacon visible).
- HUD in `GameActivity`: timer, integrity bar (cargo only), distance and an arrow to the next target,
  checkpoint counter (race), Pause button. Pause menu: Resume, Restart, Abandon.
- Results: medal, time, integrity, reward, money total, buttons Retry / Next missions.
- All strings bilingual (`t(en, ru)`).

## 3. Testing

Host tests (no GPU), each with a failing-first run:
- medals: threshold edges for each type; reward multipliers; replay payout differences.
- cargo damage: impact threshold (5.9 vs 6.1 m/s), landing, flip rate, reset penalty; zero damage for race.
- `MissionRun`: win, time-up, cargo destroyed, abandoned, timer start on throttle, checkpoint order, no skipping.
- `MissionGen`: determinism; 4 missions with 2/2 split; for 200 profile seeds every point satisfies slope,
  prop distance and world-limit rules and the line-height rule; `time_limit` formula; difficulty gating.
- Profile: save/load roundtrip; corrupt file moved to `.bak`; newer version; atomic write leaves no temp file.
- Marker list from `MissionRun` (beacon plus rings, next highlighted).
- CI smoke: a mission started from an env var on desktop (`RALLYNVX_MISSION=0`) produces a screenshot with the
  beacon visible.

## 4. Risks

- A random goal may be unreachable by car (cliffs between): mitigated by the line-height rule, but not a full
  path check; the run can be abandoned and the list refreshed.
- Impact detection from velocity deltas can false-positive on terrain bumps at speed: threshold 6 m/s per
  step is high; tuned with a host test that drives over seeded terrain without damage on gentle slopes.
- UI size: the Kotlin screens are the largest part of this stage; kept to plain M3 widgets, no custom drawing
  except the arrow.

## 5. Documentation and release

`docs/game.md` (EN+RU) gets a Missions section; README and CHANGELOG (EN+RU) 2.1.0. Release v2.1.0 with APK,
tar.gz, AppImage via the existing workflow. Phone check only with the owner's permission.
