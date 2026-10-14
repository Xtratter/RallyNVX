# Car lights Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline, no review subagent: the user opted out of the final review). Steps use checkbox (`- [ ]`) syntax.

**Goal:** automatic headlights that light the ground, red tail/brake lights, white reverse lights with a coloured glow behind the car.

**Architecture:** `game/carlights.rs` (pure: light state and lamp positions) feeds `WorldView`; `Vehicle` remembers `braking`/`reversing`; four more `vec4` in `Uni` carry one spot cone and one small point light to `shade_with` in `raster.wgsl`; glowing cubes show the lamps.

**Tech Stack:** Rust, WGSL, Kotlin (one debug extra), `~/.cache/devtest.sh`.

**Spec:** `docs/superpowers/specs/2026-10-14-car-lights-design.md`

## Global Constraints

- Headlights on when `daylight.night > 0.25`; cone half-angle 30 deg, range 48 m, intensity 6; point light behind: radius 7 m.
- `Uni` 352 -> 416 bytes, new fields appended at the end in the same order in WGSL.
- Lamp height above ground: SUV 0.95 m, rally 0.62 m; front/rear at +-(length/2 - 0.08), x at +-0.34 of the width.
- Docs EN+RU; commit trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`; no release (ships with day-night as v2.13.0 after the user's go).

## Review Focus

- A car tilted or rolled must carry its lamps with it (rotation applied): tested with several yaws and a pitch.
- A car reversing slowly (-0.3 m/s) must not flicker the reverse lamps: threshold -0.5 m/s plus the throttle rule, tested.
- Headlights off in daylight even when braking (tail lights only glow with the headlights or braking).
- Garage car at night is not special-cased (garage is always day).

---

### Task 1: Pure car-lights logic and `Vehicle` state

**Files:**
- Create: `rust/src/game/carlights.rs`, `rust/src/game/carlights_tests.rs`
- Modify: `rust/src/game/mod.rs` (`pub mod carlights;`), `rust/src/game/vehicle.rs` (`braking`, `reversing`), `rust/src/game/vehicle_tests.rs`

**Interfaces:**
- Produces: `CarLights { head, brake, reverse }`, `Lamps { head: [V3;2], tail: [V3;2], rev: [V3;2], head_dir: V3 }`, `carlights::state(&Vehicle, night: f32) -> CarLights`, `carlights::state_with(mode: u8, &Vehicle, night) -> CarLights`, `carlights::lamps(&Vehicle) -> Lamps`, `carlights::set_debug(Option<u8>)`; `Vehicle.braking`, `Vehicle.reversing`.

- [ ] **Step 1: Failing tests.** `carlights_tests.rs`:

```rust
use super::*;
use crate::game::{input::Input, math::{v3, Q}, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};

fn cars() -> Vec<VehicleSpec> { VehicleSpec::all().into_iter().map(|(_, s)| s).collect() }
fn settled(spec: VehicleSpec) -> Vehicle {
    let t = Terrain::flat(); let mut v = Vehicle::new(spec, v3(0., 1.2, 0.), 0.0);
    for _ in 0..180 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v
}

#[test] fn lamps_sit_at_the_right_ends_symmetric_and_at_a_plausible_height() {
    for spec in cars() {
        let v = settled(spec); let l = lamps(&v); let f = v.forward(); let r = v.right();
        for p in l.head { assert!((p - v.pos).dot(f) > spec.size.z * 0.4, "head lamp not at the front"); assert!(p.y > 0.4 && p.y < 1.25, "height {}", p.y); }
        for p in l.tail.iter().chain(&l.rev) { assert!((*p - v.pos).dot(f) < -spec.size.z * 0.4, "tail lamp not at the rear"); }
        for pair in [l.head, l.tail, l.rev] { assert!(((pair[0] - v.pos).dot(r) + (pair[1] - v.pos).dot(r)).abs() < 1e-3, "lamps are symmetric"); assert!((pair[0] - pair[1]).len() > 0.3); }
        assert!(l.head_dir.dot(f) > 0.99 && l.head_dir.y < 0.0, "the beam points ahead and slightly down");
    }
}
#[test] fn lamps_follow_the_rotation_of_the_body() {
    let spec = VehicleSpec::offroader();
    let mut v = Vehicle::new(spec, v3(10., 2., -5.), 0.0);
    v.rot = Q::axis_angle(v3(0., 1., 0.), 1.2).mul(Q::axis_angle(v3(1., 0., 0.), 0.3));
    let l = lamps(&v);
    for p in l.head { assert!((p - v.pos).dot(v.forward()) > spec.size.z * 0.4); }
    assert!((l.head_dir.len() - 1.0).abs() < 1e-4);
}
#[test] fn the_headlights_follow_the_night_and_the_other_lamps_follow_the_driver() {
    let mut v = settled(VehicleSpec::offroader());
    assert_eq!(state_with(u8::MAX, &v, 0.0), CarLights { head: false, brake: false, reverse: false });
    assert!(state_with(u8::MAX, &v, 0.3).head && !state_with(u8::MAX, &v, 0.2).head);
    v.braking = true;
    assert!(state_with(u8::MAX, &v, 0.0).brake && !state_with(u8::MAX, &v, 0.0).head, "braking in daylight: tail lights only");
    v.braking = false; v.reversing = true;
    assert!(state_with(u8::MAX, &v, 0.0).reverse);
}
#[test] fn the_debug_modes_force_the_lamps() {
    let v = settled(VehicleSpec::offroader());
    assert_eq!(state_with(1, &v, 0.0), CarLights { head: true, brake: true, reverse: false });
    assert_eq!(state_with(2, &v, 0.0), CarLights { head: true, brake: false, reverse: true });
    assert_eq!(state_with(0, &v, 0.0), CarLights { head: true, brake: false, reverse: false });
}
#[test] fn the_vehicle_remembers_brake_and_reverse() {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.2, 0.), 0.0);
    v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 60.0); assert!(v.braking && !v.reversing);
    v.step(&t, &[], &Input { handbrake: true, ..Default::default() }, 1.0 / 60.0); assert!(v.braking);
    v.step(&t, &[], &Input::default(), 1.0 / 60.0); assert!(!v.braking);
    v.step(&t, &[], &Input { throttle: -1.0, ..Default::default() }, 1.0 / 60.0); assert!(v.reversing, "throttle asks for reverse");
    for _ in 0..200 { v.step(&t, &[], &Input { throttle: -1.0, ..Default::default() }, 1.0 / 60.0); }
    v.step(&t, &[], &Input::default(), 1.0 / 60.0);
    assert!(v.speed() < -0.5 && v.reversing, "rolling backwards: reverse lamps stay on, speed {}", v.speed());
    v.vel = v3(0., 0., 0.3); v.step(&t, &[], &Input::default(), 1.0 / 60.0); assert!(!v.reversing, "creeping forward is not reversing");
}
```

- [ ] **Step 2: Run, expect compile errors** (`carlights` missing): `cd rust && cargo test --lib carlights 2>&1 | tail -5`.

- [ ] **Step 3: Implement.** `mod.rs`: add `pub mod carlights;` (keep alphabetical-ish near `camera_follow`). `vehicle.rs`: add `/// Driver input of the last step, for the lights.\n    pub braking: bool, pub reversing: bool,` to `Vehicle`, `braking: false, reversing: false` in `Vehicle::new`, and at the start of `step` after `let inp = inp.sanitized();`:

```rust
        self.braking = inp.brake || inp.handbrake;
        self.reversing = inp.throttle < -0.05 || self.speed() < -0.5;
```
`carlights.rs`:

```rust
//! Car lights: which lamps are lit (the night factor and the driver's input) and where they sit on the body.
use super::math::{v3, V3};
use super::vehicle::{CarStyle, Vehicle};
use std::sync::atomic::{AtomicU8, Ordering};

/// The headlights switch on when the night factor passes this.
pub const HEAD_ON_NIGHT: f32 = 0.25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CarLights { pub head: bool, pub brake: bool, pub reverse: bool }

#[derive(Clone, Copy, Debug)]
pub struct Lamps { pub head: [V3; 2], pub tail: [V3; 2], pub rev: [V3; 2], pub head_dir: V3 }

/// Debug/test: 0 headlights only, 1 + brake, 2 + reverse; `u8::MAX` = normal behaviour. Set over JNI only.
static DEBUG: AtomicU8 = AtomicU8::new(u8::MAX);
pub fn set_debug(mode: Option<u8>) { DEBUG.store(mode.filter(|m| *m <= 2).unwrap_or(u8::MAX), Ordering::Relaxed); }

pub fn state(v: &Vehicle, night: f32) -> CarLights { state_with(DEBUG.load(Ordering::Relaxed), v, night) }

pub fn state_with(mode: u8, v: &Vehicle, night: f32) -> CarLights {
    match mode {
        0 => CarLights { head: true, brake: false, reverse: false },
        1 => CarLights { head: true, brake: true, reverse: false },
        2 => CarLights { head: true, brake: false, reverse: true },
        _ => CarLights { head: night > HEAD_ON_NIGHT, brake: v.braking, reverse: v.reversing },
    }
}

/// Lamp positions in the world: front and rear at +-(length/2 - 0.08) from the centre, x at +-0.34 of the width, at a
/// lamp height above the ground that depends on the body style.
pub fn lamps(v: &Vehicle) -> Lamps {
    let s = &v.spec;
    let half_l = s.size.z * 0.5 - 0.08;
    let x = s.size.x * 0.34;
    let h = match s.style { CarStyle::Suv => 0.95, CarStyle::Rally => 0.62 };
    let y = s.wheels[0].y - s.rest - s.radius + h;
    let at = |lx: f32, lz: f32| v.pos + v.rot.rotate(v3(lx, y, lz));
    Lamps {
        head: [at(-x, half_l), at(x, half_l)],
        tail: [at(-x, -half_l), at(x, -half_l)],
        rev: [at(-x * 0.55, -half_l), at(x * 0.55, -half_l)],
        head_dir: v.rot.rotate(v3(0.0, -0.09, 1.0).norm()),
    }
}

#[cfg(test)]
#[path = "carlights_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run to verify:** `cd rust && cargo test --lib 2>&1 | grep -E "test result|FAILED|panicked" -A4 | head`. Expected: all pass. If the height test fails for the settled car (the body sinks by the static compression, about 0.1 m) widen only if the value is still plausible (0.4..1.25).

- [ ] **Step 5: Commit** `git add -A && git commit -m "feat: car lights logic (state and lamp positions), Vehicle remembers brake and reverse"` (with the trailer).

---

### Task 2: World view, debug extra

**Files:**
- Modify: `rust/src/game/world.rs` (`WorldView.lights`, `.lamps`), `rust/src/game_jni.rs`, `app/.../Native.kt`, `app/.../GameActivity.kt`, `rust/src/game/world_tests.rs`

**Interfaces:**
- Consumes: `carlights::{state, lamps, CarLights, Lamps}`, `Daylight.night`.
- Produces: `WorldView.lights: CarLights`, `WorldView.lamps: Lamps`; `Native.gameLightsDebug(mode: Int)` (negative = off); intent extra `lights`.

- [ ] **Step 1: Failing test** in `world_tests.rs`:

```rust
#[test] fn the_view_carries_the_car_lights_which_follow_the_night() {
    let w = World::new(1);
    let v = w.view(1);
    assert!(!v.lights.head, "levels start by day");
    assert!((v.lamps.head[0] - v.lamps.head[1]).len() > 0.3);
    assert!(v.lamps.head[0].y > v.vehicle.pos.y - 1.0);
}
```
Run: expect compile error (`lights` field).

- [ ] **Step 2: Implement.** In `world.rs`: `use super::carlights::{self, CarLights, Lamps};`; add `pub lights: CarLights, pub lamps: Lamps,` to `WorldView` and in `view()` build `lights: carlights::state(&self.vehicle, self.daylight().night), lamps: carlights::lamps(&self.vehicle)` (compute `self.daylight()` once if convenient).
JNI (next to `gameTimeOfDay`): 

```rust
    /// Debug/test: force the car lights (0 headlights, 1 + brake, 2 + reverse); negative = normal.
    #[no_mangle] pub extern "system" fn Java_dev_starinin_raytrace_Native_gameLightsDebug(_e: JNIEnv, _c: JClass, mode: jint) {
        crate::game::carlights::set_debug(if mode >= 0 { Some(mode as u8) } else { None });
    }
```
`Native.kt`: `/** Debug/test: force the car lights (0 headlights, 1 + brake, 2 + reverse); negative = normal. */ @JvmStatic external fun gameLightsDebug(mode: Int)`. `GameActivity.kt` before `Native.gameStart`: `intent.getIntExtra("lights", -1).let { if (it >= 0) Native.gameLightsDebug(it) }`.

- [ ] **Step 3: Test + build:** `cd rust && cargo test --lib 2>&1 | grep "test result"`; `cd .. && ./gradlew -q testDebugUnitTest 2>&1 | grep -iv "daemon\|ready" | tail -2`. Commit.

---

### Task 3: Light on the ground and visible lamps

**Files:**
- Modify: `rust/src/raster/gpu.rs` (`Uni`, fill, glowing cubes), `rust/src/shaders/raster.wgsl` (`struct U`, `shade_with`)

**Interfaces:**
- Consumes: `wv.lights: CarLights`, `wv.lamps: Lamps`.
- Produces: `u.hl_pos` (xyz head cone origin, w on/off), `u.hl_dir` (xyz beam, w cos(outer)), `u.tl_pos` (xyz glow origin behind the car), `u.tl_col` (rgb glow colour, w radius, 0 = off).

- [ ] **Step 1: Uniforms.** `Uni`: append `hl_pos: [f32; 4], hl_dir: [f32; 4], tl_pos: [f32; 4], tl_col: [f32; 4],` after `disc`; assertion `== 416`. Fill (after `disc`):

```rust
            hl_pos: [hc.x, hc.y, hc.z, if wv.lights.head { 1.0 } else { 0.0 }],
            hl_dir: [wv.lamps.head_dir.x, wv.lamps.head_dir.y, wv.lamps.head_dir.z, 0.866],
            tl_pos: [tc.x, tc.y, tc.z, 0.],
            tl_col: tail_glow };
```
with, before the literal: `let hc = (wv.lamps.head[0] + wv.lamps.head[1]) * 0.5; let tc = (wv.lamps.tail[0] + wv.lamps.tail[1]) * 0.5 - v3(0., 0.3, 0.); let tail_glow = if wv.lights.reverse { [0.8, 0.8, 0.78, 7.0] } else if wv.lights.brake { [0.5, 0.04, 0.02, 7.0] } else if wv.lights.head { [0.12, 0.01, 0.005, 4.0] } else { [0.0; 4] };`

- [ ] **Step 2: WGSL.** `struct U`: append `hl_pos: vec4<f32>, hl_dir: vec4<f32>, tl_pos: vec4<f32>, tl_col: vec4<f32>,`. In `shade_with`, after `var c = albedo * (u.sun_col.rgb * diff + amb);` insert:

```wgsl
    if (u.hl_pos.w > 0.0) {
        let dv = wp - u.hl_pos.xyz;
        let dist = length(dv);
        let l = dv / max(dist, 0.001);
        let cone = smoothstep(u.hl_dir.w, u.hl_dir.w + 0.12, dot(l, normalize(u.hl_dir.xyz)));
        let fall = clamp(1.0 - dist / 48.0, 0.0, 1.0);
        c = c + albedo * vec3<f32>(1.0, 0.93, 0.80) * (6.0 * cone * fall * fall * max(dot(n, -l), 0.0));
    }
    if (u.tl_col.w > 0.0) {
        let dv = wp - u.tl_pos.xyz;
        let dist = length(dv);
        let att = clamp(1.0 - dist / u.tl_col.w, 0.0, 1.0);
        c = c + albedo * u.tl_col.rgb * (att * att * (0.35 + 0.65 * max(dot(n, -dv / max(dist, 0.001)), 0.0)) * 4.0);
    }
```

- [ ] **Step 3: Visible lamps** in `gpu.rs`, right after the `for p in vm::car_wheel_poses(...)` line that pushes the wheels (the car is drawn there):

```rust
        {
            let (lights, lamps, rot) = (&wv.lights, &wv.lamps, wv.vehicle.rot);
            let mut glow = |p: V3, s: V3, col: [f32; 3]| lists[0].push(inst_glow(rot, p, s, col));
            if lights.head { for p in lamps.head { glow(p, v3(0.24, 0.13, 0.08), [1.0, 0.97, 0.85]); } }
            let tail_col = if lights.brake { [1.0, 0.05, 0.03] } else if lights.head { [0.35, 0.02, 0.01] } else { [0.0; 3] };
            if tail_col[0] > 0.0 { for p in lamps.tail { glow(p, v3(0.28, 0.1, 0.06), tail_col); } }
            if lights.reverse { for p in lamps.rev { glow(p, v3(0.16, 0.1, 0.06), [1.0, 1.0, 0.95]); } }
        }
```
(Check that `lists` is declared before this point and `lists[0]` is the cube list; if `lists` is declared later, move the block after its declaration.)

- [ ] **Step 4: Device check** (phone awake; hybrid; both cars if the profile allows, the off-roader is the default):

```bash
cd ~/projects/raytrace && for m in 0 1 2; do ~/.cache/devtest.sh 2.13.0 L_$m --ei mission -1 --ef tod 0.0 --ei lights $m --el seed 7 --ei hybrid 1 2>&1 | grep -E "^e:|FATAL|panic" | tail -2; done
```
Read `L_0..L_2.png`: the cone lights the grass and trees ahead with a soft edge, tail lamps show at the rear (red, brighter with `lights=1`), reverse lamps white with `lights=2` and a white glow on the ground behind. Daytime (`--ef tod 0.5`, no `lights`) must look exactly as before (no cone, no lamps). Expected fps >= 45. If the glow cubes are hidden inside the body, move the lamps outwards by 0.05 m in `carlights::lamps` (and keep the test green).

- [ ] **Step 5: Commit** (trailer) `feat: headlight cone, tail and reverse glow, visible lamps`.

---

### Task 4: Docs and final run

**Files:** `docs/game.md`, `CHANGELOG.md`, `CHANGELOG.ru.md`

- [ ] **Step 1: Docs.** Add to `docs/game.md` (EN, next to the sound/graphics paragraph) a sentence: `At dusk and at night the car switches its headlights on by itself: a light cone lights the ground and objects ahead, tail lights glow red (brighter when braking) and white reverse lights glow when the car backs up.` and RU: `В сумерки и ночью машина сама включает фары: конус света освещает землю и предметы впереди, задние огни светятся красным (ярче при торможении), а при движении назад горят белые фонари заднего хода.` Add to both changelogs, in the 2.13.0 section, a bullet (`Added: headlights (automatic at dusk and night) that light the ground, red tail and brake lights, white reverse lights with a glow on the ground behind the car` / `Добавлено: фары (включаются сами в сумерки и ночью), освещающие землю, красные задние и стоп-сигналы, белые фонари заднего хода с отсветом на земле позади`) and change the heading to `## 2.13.0 - day and night, car lights (unreleased)` / `## 2.13.0 - день и ночь, свет машины (пока не выпущено)`.
- [ ] **Step 2: Final tests and commit:** `cd rust && cargo test --lib 2>&1 | grep "test result"; cd .. && ./gradlew -q testDebugUnitTest 2>&1 | grep -iv "daemon\|ready" | tail -2`; commit `docs: car lights`. Expected: all green; the branch `feat/day-night` holds cycles 1 and 2, ready for the release step after the user's go.
