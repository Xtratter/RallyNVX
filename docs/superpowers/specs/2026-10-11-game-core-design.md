# Game core (v2.0) — design

Status: draft for review. Date: 2026-10-11.

## 1. Product vision (whole game, for context)

RallyNVX becomes an off-road driving game: random levels from a seed, cars on
rough terrain, helicopters and drones over cities, missions, garage and
progression. Offline, RU+EN, Android first, desktop kept. Near-photorealistic
look, delivered through three selectable render modes:

- **A. Hybrid** (default): raster geometry + ray-traced soft sun shadows,
  reflections, AO.
- **C. Raster only**: fastest, for weak phones.
- **B. Pure ray tracing** ("ultra"): the existing SDF engine, mainly desktop.

Decisions made with the user: arcade driving with suspension; vehicles =
off-roader, buggy/pickup/truck, helicopter, quadcopter; flying is separate
mission types (cars do not fly); missions = cargo A→B, timed checkpoint race,
rescue/evacuation, cargo orders from NPC characters; procedural audio; money
and upgrades; no accounts.

### Stages (each gets its own spec, plan, release)

1. **v2.0 Core** (this document): seeded terrain, off-roader, follow camera,
   touch controls, render mode C, free-drive.
2. v2.1 Missions and menus (cargo, race, HUD, rewards).
3. v2.2 Air and cities (helicopter, drone, cities, rescue).
4. v2.3 Garage and NPC orders (other vehicles, upgrades, saves).
5. v2.4 Render modes A and B.
6. v2.5 Polish (audio, weather, time of day, particles, icon, docs).

Order is deliberate: the game is playable from stage 1; the riskiest part for
FPS (A and B) is built on a working game where it can be measured honestly.

## 2. Scope of v2.0

**Goal.** Choose "Game" on the start screen, press "Free drive": a hilly
terrain is generated from a seed, the player drives an off-roader with
suspension, a camera follows, left stick = throttle and steering, buttons =
brake, handbrake, reset. Playable at 30+ fps on POCO F3 (Adreno 660).

**In scope:** terrain, props (rocks, trees), vehicle physics, follow camera,
input, raster renderer (mode C), Kotlin game screen, desktop game mode,
tests, CI smoke frame, docs, release.

**Out of scope:** missions, menus beyond the start screen and pause, garage,
air vehicles, cities, audio, render modes A and B, saves.

Existing viewer ("Scenes") is untouched and stays as a separate mode (and the
future backend B).

## 3. Architecture

New code in `rust/src/game/` (pure CPU, no wgpu) and `rust/src/raster/`
(wgpu). `Gfx` device/surface creation is shared with the existing code; the
raster pipeline is separate from the trace pipelines.

### 3.1 `game::terrain`
- `Terrain::new(seed: u64)`; `height(x, z) -> f32`, `normal(x, z) -> Vec3`.
- Height = sum of octaves of seeded gradient noise (hash based, no deps), plus
  low-frequency "biome" amplitude so some areas are flat, some rugged.
  Deterministic across platforms (integer hash, f32 math only).
- Tiles of 64x64 vertices, 2 m spacing at LOD0; LOD1/LOD2 double the spacing.
  `tile_mesh(tx, tz, lod) -> (vertices, indices)`. Border vertices of
  neighbouring LODs are skirted down to hide cracks.
- Streaming: tiles within the draw radius of the camera are kept; others are
  dropped. Mesh generation is cheap enough to run on the render thread, one
  tile per frame budget.

### 3.2 `game::props`
- Deterministic scatter (rocks, trees) per tile from `(seed, tx, tz)`.
  Rejects steep slopes. Rocks are collidable (sphere approximation); trees are
  trunk cylinders.

### 3.3 `game::vehicle`
- Fixed step 1/60 s. State: position, orientation (quaternion), linear and
  angular velocity, steering angle, per-wheel compression.
- 4 suspension rays: spring + damper along the ground normal at the wheel
  contact; lateral and longitudinal tyre forces with a friction-circle clamp;
  motor torque, brake, handbrake lowers rear grip (drift). Anti-roll is a
  tuning term. Parameters live in a `VehicleSpec` struct (the off-roader is
  one value; later vehicles are more values).
- Collisions with props: sphere/cylinder push-out and velocity reflection.
- "Flipped" state when roll > 100 degrees and slow; reset button respawns the
  vehicle upright at the current position.

### 3.4 `game::camera_follow`
- Third-person: target point above the vehicle, camera behind along the smoothed
  heading, critically damped follow, raised if the terrain is above the
  camera line.

### 3.5 `game::input`
- `Input { throttle: -1..1, steer: -1..1, brake: bool, handbrake: bool,
  reset: bool }`, filled from the stick and buttons (Android) or keyboard/gamepad
  (desktop).

### 3.6 `game::world`
- Owns `Terrain`, props, vehicle, camera, time of day. `step(input, dt)` and
  `view() -> WorldView`.

### 3.7 `WorldView` (interface to all render backends)
`WorldView { terrain, props, vehicle: &VehicleState, camera, sun, time_of_day }`.
Renderers read only this. Gameplay never touches wgpu.

### 3.8 `raster` (mode C)
- Pipelines: terrain (per-tile vertex/index buffers), instanced props, vehicle
  (box/cylinder primitives forming a body, cabin, 4 wheels), sky + fog
  full-screen pass, one-cascade sun shadow map (2048, fitted to the camera).
- Lighting: Lambert + shadow map + height/slope-based ambient term, distance
  fog, simple sky gradient with sun disc. Palette per biome.
- Resolution scaling reuses the `adaptive.rs` idea (target frame time).

### 3.9 Platform glue
- JNI: `Native.gameStart/gameInput/gameSeed`, new `Cmd` variants; the game
  screen is a separate Activity state in Kotlin reusing `StickView`.
- Desktop: `--game [seed]` flag; WASD/arrows + space.

## 4. Testing

CPU, deterministic, no GPU:
- Terrain: same seed → same heights; different seeds differ; continuity across
  tile borders; height range within bounds; mesh indices valid.
- Props: reproducible; none on slopes above limit.
- Vehicle: rests on flat ground without sinking or jitter; throttles up to the
  expected top speed band; brakes to a stop; steering turns the heading; lands
  from a drop without exploding; flips detected on a 60° slope; reset restores
  upright pose.
- Camera: never below the terrain.
- Shaders: parse and validate with naga (`validate_for_tests`).
- CI: lavapipe smoke frame of the game (screenshot artifact), cargo tests.

## 5. Risks

- **FPS on Adreno 660** at ~400 m draw distance. Mitigation: LOD tiles, shadow
  map fit, resolution scaling, measurement before release. Phone tests only
  with the owner's permission.
- Tyre model stability at 60 Hz: mitigated by friction-circle clamp and
  tests; tuning is iterative.
- Scope creep into stages 2+: explicitly out of scope above.

## 6. Documentation and release

README (EN+RU) and CHANGELOG (EN+RU) get a Game section; `docs/game.md` (EN+RU)
describes controls and seeds. Release v2.0.0 with APK, tar.gz, AppImage via
the existing workflow.
