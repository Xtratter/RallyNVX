# Flight (v2.2) — design

Status: draft for review. Date: 2026-10-11. Builds on the game core (v2.0) and missions (v2.1) specs.

## 1. Goal and decomposition

The air part of the game is split in three releases (decided with the user): **v2.2 Flight** (this document:
helicopter and drone over plain terrain, three air mission types), **v2.3 Cities** (procedural cities, city
renderer with LOD, FPS measurement), **v2.4 Rescue** (evacuation missions). The old plan's garage, NPC orders,
render modes A/B and polish move to v2.5, v2.6, v2.7.

v2.2 deliverable: from the mission list the player can fly a **ring race** (drone or helicopter), a **sling
cargo delivery** (helicopter only) and a **drone recon**, with arcade controls, a chase camera, HUD, medals and
money, on the existing terrain. No cities, no rescue, no audio, no fuel or battery.

Assumptions (editable): arcade controls with auto-hover (sticks released = hold position and altitude); the
mission list grows from 4 to 6 missions; rewards use the same medals and profile as ground missions.

## 2. Architecture

New pure-CPU modules in `rust/src/game/` (host-tested, no wgpu):

- `flight.rs`: `AircraftKind{Heli,Drone}`, `AircraftSpec`, `Aircraft`, `FlightInput`.
- `sling.rs`: `Sling` (rope with a cargo point mass).
- `airrun.rs`: `AirRun`, the state machine of an air mission (rings, sling, recon).
- extensions of `missions.rs` (new kinds and generation), `meta.rs` (unchanged format; the kind id grows),
  `world.rs` (aircraft, air run, markers), `camera_follow.rs` (generalised follow target), `app.rs`
  (`fill_state`, input), `raster/gpu.rs` (aircraft models, ring and rope instances), `game_jni.rs`, Kotlin.

### 2.1 Aircraft (`flight.rs`)

State: position, velocity, yaw, pitch, roll, yaw rate. Input `FlightInput{ yaw, lift, pitch, roll }`, each -1..1
(left stick x = yaw, left stick y = lift, right stick y = pitch (forward positive), right stick x = roll
(right positive)); non-finite values become 0 (like `Input::sanitized`).

- Tilt follows its target with a first-order lag of 0.25 s: `target = input * max_tilt`
  (heli 25 deg, drone 40 deg).
- Horizontal acceleration `g * tan(tilt)` in the heading frame plus linear drag (speed caps: heli 35 m/s,
  drone 28 m/s).
- Vertical speed follows `lift * vmax` (heli 6, drone 10 m/s) with an acceleration limit (heli 4, drone 12
  m/s^2); with lift = 0 the aircraft holds its altitude.
- Yaw rate follows `yaw * rate_max` (heli 1.6, drone 3.0 rad/s) with lag.
- Ground contact (height above terrain < 0.3 m): soft if vertical speed <= 3 m/s and tilt <= 20 deg (rests,
  horizontal speed damped); otherwise `Crashed`. Tilt above 35 deg at contact also crashes. Props (trees,
  rocks) crash the aircraft on contact (same sphere/cylinder test as the car).
- Ceiling 400 m (vertical speed forced to <= 0 above it); at the world wall (`WORLD_LIMIT - 20`) the
  horizontal velocity pointing outwards is removed.
- `AircraftSpec` carries all numbers; `Aircraft::new(kind, pos, yaw)`; `step(&mut self, t, props, input, dt)`
  with fixed dt = 1/60.

### 2.2 Sling (`sling.rs`)

Heli only. Cargo = point mass (mass 1) hanging from the aircraft on a rope of rest length 8 m, modelled as a
one-sided spring-damper (taut only beyond 8 m, k = 60, c = 6) with gravity and drag; ground contact stops the
cargo (impact speed reported). `Sling::new(attach)`, `step(&mut self, attach, attach_vel, t, dt) -> Option<f32>`
(returns the ground impact speed in the step, if any), `pos()`, `rope_taut()`, `relative_speed(vel)`.

### 2.3 Air missions (`airrun.rs`, `missions.rs`)

New `MissionKind` values: `Rings`, `Sling`, `Recon`. A mission carries `aircraft: Option<AircraftKind>`
(`None` for ground missions), plus for air missions `points` with altitudes (heights above ground).

**Common.** The clock starts at the first non-zero stick input. Start: a plateau at (0,0), 3 m above ground,
facing the first target. Failure reasons: `TimeUp`, `Crashed`, `CargoDestroyed`, `Abandoned`. Medals and
payout use `medals.rs` unchanged (gold <= 60 % of the limit, silver <= 80 %, cargo conditions as on the ground).

**Rings.** 5-8 vertical rings along a 500-1400 m path (zigzag <= 80 m), ring height above ground 15-90 m (drone
up to 120). Radius: heli 8 m, drone 6 m. A ring counts when the aircraft path segment of the step crosses the
ring plane in the right direction inside the radius; rings are taken in order. `time_limit = 1.5 * length /
v_ref + 20` with v_ref heli 18, drone 15 m/s.

**Sling (heli).** Pick-up pad A and drop pad B (400-1000 m apart, both on gentle ground, props >= 8 m away,
drop circle radius 6 m). Pick-up: the aircraft hovers lower than 4 m above the cargo for 1.5 s (cargo then
attached). Delivery: the cargo is inside the circle with speed < 2 m/s. Integrity 100: ground impact at
`v > 6` costs `(v - 6) * 2.5`; swinging (cargo speed relative to the aircraft above 12 m/s) costs 4 per second;
0 fails. Medals as for ground cargo.

**Recon (drone).** 3-5 points at 20-100 m height around a 600-1200 m loop, capture sphere 40 m, 2 s of
dwell inside, any order. `time_limit = 1.4 * route_length / 15 + 25`.

`reward_base = (100 + 0.15 * length) * {1.0, 1.4, 1.9}` as before; replay payout as before. Difficulty is part
of the seed as in v2.1.

**Generation.** `roll` returns 6 missions: Cargo, Race, Cargo, Race (unchanged), slot 4 `Rings` (aircraft
drone or heli by hash), slot 5 `Sling` on even rounds and `Recon` on odd rounds. Ground points keep the
terrain rules; air points need only the world-wall margin and a height above ground >= 15 m (landing pads of
the sling mission use the ground-point rules). Deterministic in `(profile_seed, round, gold)`.

### 2.4 World, camera, rendering

- `World` gets `aircraft: Option<Aircraft>`, `sling: Option<Sling>`, `air: Option<AirRun>`; in air missions the
  car is not stepped and not drawn. `World::for_mission` builds the right setup from the mission.
- `FollowCam::update` takes a follow target `(pos, heading, speed)` so it serves the car and the aircraft; for
  aircraft the camera sits 9 m behind and 3 m above (drone 6 m / 2 m), never below the terrain + 0.9 m.
- `WorldView` gets `aircraft`, `sling_pos`, and the existing `markers` (new marker kinds: `Ring{normal}`,
  `Pad`, `Zone`); the raster renderer draws the helicopter and the drone from primitives (body, tail boom,
  spinning rotor disc, skids / four arms with four rotors), the rope as a thin cylinder and the cargo as a
  cube, rings as 16 small cubes on a circle, pads as flat cylinders, recon zones as translucent-looking
  (glowing) spheres. No new pipeline.
- Rotor discs spin from `World.time`; no physics tie.

### 2.5 Input, JNI, UI

- `Native.gameFlight(yaw, lift, pitch, roll)` next to `gameInput`; in air missions `GameActivity` maps the
  left stick to (yaw, lift) and the right stick to (pitch, roll); Handbrake/Brake buttons are hidden.
- The state array (`fill_state`) gets aircraft fields: altitude above ground, speed, cargo integrity, rope
  state, ring/point counters, mission kind codes for the new kinds, failure code 4 = crashed.
- HUD in air: altitude, speed, timer, arrow + distance to the target, integrity bar for the sling,
  ring counter / captured points counter. Menu cards show kind (Rings / Sling / Recon) and the aircraft.

## 3. Testing

Host tests (no GPU), failing first:
- Aircraft: hover with neutral sticks; tilt follows the target and accelerates within caps; vertical speed
  and ceiling; yaw rate; soft landing vs crash thresholds; prop collision crash; world-wall behaviour; no
  NaN for 0.1 s steps; sanitising of non-finite input.
- Sling: hangs at rope length under a hovering heli, follows motion, swings and damps, ground impact speed
  reported, rope taut only beyond rest length.
- Air runs: rings order and plane crossing (wrong direction, outside radius, skipped ring), pick-up
  hover timing, delivery speed limit, swing damage, recon dwell and any-order capture, crash and time-up
  failures, clock start on first input, nothing changes after the run is done.
- Generation: 6 missions, kinds and aircraft rules, determinism, point rules over 200 seeds, `roll` never
  panics for extreme arguments, the sling/recon alternation by round parity.
- State array fields; profile keys for the new kinds (round trip, old profiles with kind ids 0/1 still load).
- CI smoke: desktop `RAYTRACE_MISSION=<air slot>` with an autopilot (flies towards the target) takes a
  screenshot showing the aircraft and a ring or pad.

## 4. Risks

- Feel of the arcade flight model cannot be judged on the host; the numbers are a starting point and need a
  short phone test (ask the owner first). Tuning constants live in `AircraftSpec`.
- Sling stability at 60 Hz: sub-stepping (4 substeps) and a damped one-sided spring; covered by tests.
- Touch controls: two sticks for four axes is fine, but fine hovering needs sensitivity tuning (stick shaping
  already exists in `StickMath`).
- Scope: cities, rescue, battery and gyro control are explicitly out of scope.

## 5. Documentation and release

`docs/game.md` (EN+RU): Flight section (controls, mission types, rules); README and CHANGELOG (EN+RU) 2.2.0.
Release v2.2.0 with APK, tar.gz, AppImage via the existing workflow.
