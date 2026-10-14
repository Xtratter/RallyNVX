# Surface Grip Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline, no review subagent). Code blocks starting with `// FILE: <path>` are complete new files.

**Goal:** per-surface grip, lateral stiffness and rolling resistance, and a loose gravel layer on the stage.

**Architecture:** `game/grip.rs` (table + `LooseMap`), `RoadHit.side`, `surface_at` flat-test rule, `Vehicle::step_with` with per-wheel surface lookup, `World.loose`, state slot 28 + HUD.

**Tech Stack:** Rust, Kotlin (HUD text), `~/.cache/devtest.sh`.

**Spec:** `docs/superpowers/specs/2026-10-14-surface-grip-design.md`

## Global Constraints

- Multipliers (mu / lateral / rolling): asphalt 1.00/1.0/1.0; gravel 0.62..0.78 / 0.75 / 1.3; dirt 0.70/0.8/1.2; grass 0.60/0.7/1.5; sand 0.50/0.6/3.0; rock 0.80/0.9/1.0; water 0.55/0.7/1.0.
- Loose map: cell 1 m x 0.5 m, +-6 m across, wear `0.35 * speed * dt`, initial 1.0.
- `Vehicle::step` keeps its signature; the flat test terrain (scale 0, no roads) is asphalt.
- Docs EN+RU; trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`; no release without the user's go (2.16.0).

## Review Focus

- A wheel off the end of the map or on a non-stage road must read looseness 1.0 / no wear (no panic on indices).
- `NaN`/huge speeds must not poison the map (wear clamps to [0, 1]).
- A car on the liaison (asphalt) behaves as before (multipliers 1.0): the Vulpes 0-100 test on the flat test terrain stays 3.5-4.4 s.

---

### Task 1: Table, loose map, `RoadHit.side`, flat-test asphalt

**Files:**
- Create: `rust/src/game/grip.rs`, `rust/src/game/grip_tests.rs`
- Modify: `rust/src/game/mod.rs` (`pub mod grip;`), `rust/src/game/roads/mod.rs` (`RoadHit.side`), `rust/src/game/terrain.rs` (`is_flat_test`), `rust/src/game/surface.rs`

**Interfaces:**
- Produces: `SurfaceGrip`, `grip(Surface, loose) -> SurfaceGrip`, `LooseMap::{empty, new(&RoadNet), loose_at(road, s, side), wear(road, s, side, speed, dt)}`, `ground_at(&Terrain, &LooseMap, x, z) -> (Surface, f32 /*looseness*/, Option<(usize, f32, f32)> /*road, s, side on a gravel road*/)`, `Terrain::is_flat_test()`, `RoadHit.side`.

- [ ] **Step 1: Failing tests** `grip_tests.rs`:

```rust
// FILE: rust/src/game/grip_tests.rs
use super::*;
use crate::game::{math::v3, roads::{Road, RoadKind, RoadNet, RoadPt}, surface::Surface, terrain::Terrain};

fn stage(len: f32) -> RoadNet {
    let n = (len / 4.0) as usize;
    RoadNet::new(vec![Road { kind: RoadKind::Stage, pts: (0..=n).map(|i| RoadPt { pos: v3(0.0, 0.0, i as f32 * 4.0), width: 8.0, s: i as f32 * 4.0 }).collect() }])
}

#[test] fn the_table_orders_the_surfaces_and_stays_in_bounds() {
    let g = |s| grip(s, 1.0).mu;
    assert!(g(Surface::Asphalt) > g(Surface::Rock) && g(Surface::Rock) > g(Surface::Dirt) && g(Surface::Dirt) > g(Surface::Grass) && g(Surface::Grass) > g(Surface::Sand));
    for s in [Surface::Grass, Surface::Dirt, Surface::Sand, Surface::Rock, Surface::Gravel, Surface::Asphalt, Surface::Water] {
        for l in [0.0, 0.5, 1.0, 2.0, -1.0, f32::NAN] { let k = grip(s, l); assert!((0.3..=1.0).contains(&k.mu) && (0.3..=1.0).contains(&k.lateral) && (0.9..=3.5).contains(&k.rolling), "{s:?} {l}"); }
    }
    assert!(grip(Surface::Gravel, 0.0).mu > grip(Surface::Gravel, 1.0).mu, "packed gravel grips better than loose");
    assert!((grip(Surface::Gravel, 0.0).mu - 0.78).abs() < 1e-5 && (grip(Surface::Gravel, 1.0).mu - 0.62).abs() < 1e-5);
}
#[test] fn the_map_starts_loose_and_wear_packs_the_line_not_the_sides() {
    let net = stage(400.0);
    let mut m = LooseMap::new(&net);
    assert_eq!(m.loose_at(0, 100.0, 0.0), 1.0);
    for _ in 0..60 { m.wear(0, 100.0, 0.0, 20.0, 1.0 / 240.0); }   // a quarter of a second at 20 m/s on one spot
    assert!(m.loose_at(0, 100.0, 0.0) < 0.5, "{}", m.loose_at(0, 100.0, 0.0));
    assert_eq!(m.loose_at(0, 100.0, 1.4), 1.0, "1.4 m to the side is untouched");
    assert_eq!(m.loose_at(0, 140.0, 0.0), 1.0, "40 m along is untouched");
    for _ in 0..2000 { m.wear(0, 100.0, 0.0, 30.0, 1.0 / 60.0); }
    assert!(m.loose_at(0, 100.0, 0.0) >= 0.0);
}
#[test] fn the_map_survives_odd_queries() {
    let net = stage(100.0);
    let mut m = LooseMap::new(&net);
    for (s, side) in [(-5.0, 0.0), (1e9, 0.0), (50.0, 99.0), (50.0, -99.0), (f32::NAN, 0.0), (50.0, f32::NAN)] {
        assert_eq!(m.loose_at(0, s, side), 1.0, "({s}, {side})");
        m.wear(0, s, side, 20.0, 0.01);
    }
    m.wear(0, 50.0, 0.0, f32::NAN, 0.01); m.wear(0, 50.0, 0.0, f32::INFINITY, 0.01);
    assert!((0.0..=1.0).contains(&m.loose_at(0, 50.0, 0.0)));
    assert_eq!(m.loose_at(7, 50.0, 0.0), 1.0, "an unknown road");
    assert_eq!(LooseMap::empty().loose_at(0, 1.0, 0.0), 1.0);
}
#[test] fn a_liaison_road_has_no_loose_layer() {
    let net = RoadNet::new(vec![Road { kind: RoadKind::Liaison, pts: (0..=25).map(|i| RoadPt { pos: v3(0.0, 0.0, i as f32 * 4.0), width: 8.0, s: i as f32 * 4.0 }).collect() }]);
    let mut m = LooseMap::new(&net);
    m.wear(0, 50.0, 0.0, 20.0, 1.0);
    assert_eq!(m.loose_at(0, 50.0, 0.0), 1.0);
}
#[test] fn ground_at_reports_surface_looseness_and_the_gravel_road_cell() {
    let t = Terrain::flat().with_net(stage(400.0));
    let mut m = LooseMap::new(t.roads().unwrap());
    let (s, loose, road) = ground_at(&t, &m, 1.0, 100.0);
    assert_eq!((s, loose), (Surface::Gravel, 1.0));
    let (ri, along, side) = road.expect("on the gravel stage");
    assert!(ri == 0 && (along - 100.0).abs() < 1.0 && (side - (-1.0)).abs() < 1e-3 || (side - 1.0).abs() < 1e-3, "side {side}");
    m.wear(0, 100.0, side, 20.0, 0.5);
    assert!(ground_at(&t, &m, 1.0, 100.0).1 < 1.0);
    let (s2, l2, r2) = ground_at(&t, &m, 300.0, 100.0);
    assert!(s2 == Surface::Asphalt && l2 == 1.0 && r2.is_none(), "the flat test terrain far from the road is neutral asphalt");
}
```

- [ ] **Step 2: Run, expect compile errors.**
- [ ] **Step 3: Implement.** `roads/mod.rs`: add `pub side: f32` to `RoadHit` and compute in `hit()`:

```rust
        let (tx, tz) = { let l = l2.sqrt().max(1e-6); (dx / l, dz / l) };
        let side = (x - px) * -tz + (z - pz) * tx;   // positive to the left of the travel direction
```
(add `side` to the struct literal). `terrain.rs`: `pub fn is_flat_test(&self) -> bool { self.scale == 0.0 && self.roads.is_none() }`. `surface.rs` `surface_at`: before the `terrain_weights` line add `if t.is_flat_test() { return Surface::Asphalt; }` (after the water check). `grip.rs`:

```rust
// FILE: rust/src/game/grip.rs
//! How the ground grips: multipliers per surface and the loose gravel layer of the special stage.
use super::roads::{RoadKind, RoadNet};
use super::surface::{surface_at, Surface};
use super::terrain::Terrain;

/// Multipliers on the car's own `mu`, `lateral_k` and `rolling` resistance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceGrip { pub mu: f32, pub lateral: f32, pub rolling: f32 }

pub const GRAVEL_PACKED: f32 = 0.78;
pub const GRAVEL_LOOSE: f32 = 0.62;

/// The grip of a surface; `loose` (0 packed .. 1 loose, ignored off gravel) interpolates the gravel grip.
pub fn grip(s: Surface, loose: f32) -> SurfaceGrip {
    let g = |mu, lateral, rolling| SurfaceGrip { mu, lateral, rolling };
    match s {
        Surface::Asphalt => g(1.0, 1.0, 1.0),
        Surface::Gravel => { let l = if loose.is_finite() { loose.clamp(0.0, 1.0) } else { 1.0 }; g(GRAVEL_PACKED + (GRAVEL_LOOSE - GRAVEL_PACKED) * l, 0.75, 1.3) }
        Surface::Dirt => g(0.70, 0.8, 1.2),
        Surface::Grass => g(0.60, 0.7, 1.5),
        Surface::Sand => g(0.50, 0.6, 3.0),
        Surface::Rock => g(0.80, 0.9, 1.0),
        Surface::Water => g(0.55, 0.7, 1.0),
    }
}

pub const CELL_S: f32 = 1.0;
pub const CELL_X: f32 = 0.5;
pub const HALF_W: f32 = 6.0;
const NX: usize = (2.0 * HALF_W / CELL_X) as usize;

/// Looseness (1 = untouched loose gravel .. 0 = packed) of the cells along every stage road.
#[derive(Clone, Debug, Default)]
pub struct LooseMap { roads: Vec<Vec<f32>> }

impl LooseMap {
    pub fn empty() -> LooseMap { LooseMap { roads: Vec::new() } }
    pub fn new(net: &RoadNet) -> LooseMap {
        LooseMap { roads: net.roads.iter().map(|r| if r.kind == RoadKind::Stage { vec![1.0; ((r.length() / CELL_S) as usize + 2) * NX] } else { Vec::new() }).collect() }
    }
    fn index(&self, road: usize, s: f32, side: f32) -> Option<usize> {
        let cells = self.roads.get(road)?;
        if cells.is_empty() || !s.is_finite() || !side.is_finite() || s < 0.0 || side.abs() >= HALF_W { return None; }
        let (i, j) = ((s / CELL_S) as usize, ((side + HALF_W) / CELL_X) as usize);
        let k = i * NX + j.min(NX - 1);
        if k < cells.len() { Some(k) } else { None }
    }
    pub fn loose_at(&self, road: usize, s: f32, side: f32) -> f32 { self.index(road, s, side).map_or(1.0, |k| self.roads[road][k]) }
    /// A wheel at (s, side) moving at `speed` m/s for `dt` s packs the cells under it (about 0.35 per metre travelled).
    pub fn wear(&mut self, road: usize, s: f32, side: f32, speed: f32, dt: f32) {
        if !speed.is_finite() || !dt.is_finite() { return; }
        let amount = (0.35 * speed.abs() * dt).clamp(0.0, 1.0);
        for off in [-0.25f32, 0.25] {   // the tyre's width: two half-metre cells
            if let Some(k) = self.index(road, s, side + off) { let c = &mut self.roads[road][k]; *c = (*c - amount * 0.5).clamp(0.0, 1.0); }
        }
    }
}

/// The surface at a point, its looseness and, on a gravel road, the (road, s, side) cell: what a wheel contact needs.
pub fn ground_at(t: &Terrain, map: &LooseMap, x: f32, z: f32) -> (Surface, f32, Option<(usize, f32, f32)>) {
    let surface = surface_at(t, x, z);
    if surface == Surface::Gravel {
        if let Some(h) = t.road_hit(x, z) { return (surface, map.loose_at(h.road, h.s, h.side), Some((h.road, h.s, h.side))); }
    }
    (surface, 1.0, None)
}

#[cfg(test)]
#[path = "grip_tests.rs"]
mod tests;
```
`game/mod.rs`: `pub mod grip;`.

- [ ] **Step 4: Run:** `cd rust && cargo test --lib grip 2>&1 | grep -E "^error|test result|FAILED|panicked" -A6`. Expected: pass (fix the side-sign assertion if the sign is the other way: the test accepts either).
- [ ] **Step 5: Commit** `feat: surface grip table, loose gravel map, road hit side`.

---

### Task 2: Vehicle uses the surface

**Files:** `rust/src/game/vehicle.rs`, `rust/src/game/vehicle_tests.rs` (append), `rust/src/game/world.rs`

**Interfaces:**
- Consumes: `grip::{grip, ground_at, LooseMap}`.
- Produces: `Vehicle::step_with(&mut self, t, props, inp, dt, loose: &mut LooseMap)`, `Vehicle.surface: Surface`, `Vehicle.force_surface: Option<Surface>`, `World.loose: LooseMap`.

- [ ] **Step 1: Failing tests** appended to `vehicle_tests.rs`:

```rust
use crate::game::{grip::LooseMap, surface::Surface};

fn braking_distance(surface: Surface, spec: VehicleSpec) -> f32 {
    let t = Terrain::flat();
    let mut v = Vehicle::new(spec, v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(surface);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0;
    let z0 = v.pos.z;
    for _ in 0..600 { v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 120.0); if v.speed() < 0.3 { break; } }
    v.pos.z - z0
}
fn cornering_accel(surface: Surface, spec: VehicleSpec) -> f32 {
    let t = Terrain::flat();
    let mut v = Vehicle::new(spec, v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(surface);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0;
    let mut best = 0.0f32;
    for i in 0..240 {
        v.step(&t, &[], &Input { steer: 0.35, throttle: 0.3, ..Default::default() }, 1.0 / 120.0);
        if i > 120 { best = best.max(v.vel.len() * v.omega.y.abs()); }
    }
    best
}

#[test] fn braking_distance_orders_the_surfaces() {
    for spec in [VehicleSpec::offroader(), VehicleSpec::by_id("vulpes").unwrap()] {
        let d = |s| braking_distance(s, spec);
        let (a, g, gr, sa) = (d(Surface::Asphalt), d(Surface::Gravel), d(Surface::Grass), d(Surface::Sand));
        assert!(a < g && g < gr && gr < sa, "asphalt {a:.1} gravel {g:.1} grass {gr:.1} sand {sa:.1}");
        assert!(sa > a * 1.6 && a > 10.0, "{a} {sa}");
    }
}
#[test] fn cornering_grip_orders_the_surfaces() {
    let spec = VehicleSpec::by_id("vulpes").unwrap();
    let (a, g, s) = (cornering_accel(Surface::Asphalt, spec), cornering_accel(Surface::Gravel, spec), cornering_accel(Surface::Sand, spec));
    assert!(a > g * 1.15 && g > s * 1.1, "asphalt {a:.1} gravel {g:.1} sand {s:.1}");
}
#[test] fn the_vulpes_spins_its_wheels_on_gravel() {
    let spec = VehicleSpec::by_id("vulpes").unwrap();
    let time_to = |surface: Surface| {
        let t = Terrain::flat();
        let mut v = Vehicle::new(spec, v3(0., 1.0, 0.), 0.0);
        v.force_surface = Some(surface);
        for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
        let go = Input { throttle: 1.0, ..Default::default() };
        let mut secs = 0.0;
        while v.speed() < 100.0 / 3.6 && secs < 15.0 { v.step(&t, &[], &go, 1.0 / 120.0); secs += 1.0 / 120.0; }
        secs
    };
    let (a, g) = (time_to(Surface::Asphalt), time_to(Surface::Gravel));
    assert!(a > 3.4 && a < 4.5, "asphalt {a}");
    assert!(g > a * 1.3, "gravel {g} vs asphalt {a}");
}
#[test] fn the_car_reports_its_surface_and_the_road_wears_the_loose_layer() {
    use crate::game::roads::{Road, RoadKind, RoadNet, RoadPt};
    let net = RoadNet::new(vec![Road { kind: RoadKind::Stage, pts: (0..=250).map(|i| RoadPt { pos: v3(0.0, 0.0, i as f32 * 4.0), width: 8.0, s: i as f32 * 4.0 }).collect() }]);
    let t = Terrain::flat().with_net(net);
    let mut map = LooseMap::new(t.roads().unwrap());
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0.0, 1.0, 10.0), 0.0);
    for _ in 0..120 { v.step_with(&t, &[], &Input::default(), 1.0 / 60.0, &mut map); }
    assert_eq!(v.surface, Surface::Gravel);
    v.vel = v.forward() * 15.0;
    for _ in 0..600 { v.step_with(&t, &[], &Input { throttle: 0.4, ..Default::default() }, 1.0 / 120.0, &mut map); }
    let mid = v.pos.z - 20.0;
    let on_line = (0..8).map(|i| map.loose_at(0, mid - i as f32, -v.spec.wheels[0].x)).fold(1.0f32, f32::min);
    assert!(on_line < 0.6, "the wheel line is packed: {on_line}");
    assert_eq!(map.loose_at(0, mid, 3.5), 1.0, "3.5 m to the side is untouched");
}
```
(The first test uses `Terrain::flat()`; `force_surface` overrides per-wheel surfaces.)
- [ ] **Step 2: Run, expect compile errors.**
- [ ] **Step 3: Implement** in `vehicle.rs`: add `use super::grip::{self, LooseMap}; use super::surface::Surface;`; fields in `Vehicle`: `pub surface: Surface, pub force_surface: Option<Surface>,` (init `Surface::Asphalt`, `None` in `new`); rename the old `step` body to `step_with(&mut self, t, props, inp, dt, loose: &mut LooseMap)` and add

```rust
    pub fn step(&mut self, t: &Terrain, props: &[Prop], inp: &Input, dt: f32) { let mut none = LooseMap::empty(); self.step_with(t, props, inp, dt, &mut none); }
```
Inside the wheel loop, after `let vc = ...; let base = ...; let fw = ...; let rw = ...; let (vl, vf) = ...;` compute:

```rust
            let contact = self.pos + r_c;
            let (surf, looseness, cell) = match self.force_surface { Some(f) => (f, 1.0, None), None => grip::ground_at(t, loose, contact.x, contact.z) };
            let sg = grip::grip(surf, looseness);
            if w == 0 || self.contacts == 1 { self.surface = surf; }
            if let Some((ri, s_along, side)) = cell { loose.wear(ri, s_along, side, vf.abs().max(vl.abs()), h); }
            let (mu_s, lat_k, roll) = (s.mu * sg.mu, s.lateral_k * sg.lateral, s.rolling * sg.rolling);
```
(`h` is the substep dt used in the existing code; read the loop to use the actual variable names `h`/`w`.) Replace the uses: `lat_mu = if !front && inp.handbrake { mu_s * s.handbrake_grip } else { mu_s }`; `let mut fl = -vl * wheel_mass * lat_k;`; `- vf * roll`. (`self.surface` records the first wheel in contact; fine for the HUD.)

`world.rs`: field `pub loose: LooseMap` (`use super::grip::LooseMap`), set in `from_terrain` (`loose: terrain.roads().map_or_else(LooseMap::empty, LooseMap::new)` before `terrain` is moved) and in `World::step` call `self.vehicle.step_with(&self.terrain, &near, &inp, dt, &mut self.loose);` instead of `self.vehicle.step(...)`.
- [ ] **Step 4: Run all:** `cd rust && cargo test --lib 2>&1 | grep -E "^error|test result|FAILED|panicked" -A8 | head -40`. Expected: green. If an existing test fails because the car now sits on a different surface (real terrain: grass), check whether it truly depends on grip; adjust the test's tolerance only when the behaviour change is the intended one, and note it in the commit message. If the new tests' bounds fail, tune the table numbers in `grip.rs` (not the test directions).
- [ ] **Step 5: Commit** `feat: the car's tyres follow the surface and wear the loose gravel`.

---

### Task 3: HUD, docs, device check

**Files:** `rust/src/game/app.rs` (slot 28), `rust/src/game/app_state_tests.rs`, `app/.../GameActivity.kt`, `docs/game.md`, `CHANGELOGs`, version bump.

- [ ] **Step 1: State slot.** In `fill_state` add `out[28] = surface_code(w.vehicle.surface) as f32;` with

```rust
fn surface_code(s: crate::game::surface::Surface) -> u8 { use crate::game::surface::Surface::*; match s { Grass => 0, Dirt => 1, Sand => 2, Rock => 3, Gravel => 4, Asphalt => 5, Water => 6 } }
```
update the doc comment of `fill_state` (`[28]` surface under the car: 0 grass .. 6 water) and the unused-slots assertion in `app_state_tests.rs` (`s[4..16]`, `s[17..25]` stay zero; slots 26..=28 are used) plus a test: a world on a gravel road reports 4: `World::with_roads(4, ..)` placed with `place_on_road(2)`, stepped 120 times, `s[28] == 4.0`; on the liaison (`place_on_road(1)`) `== 5.0`.
- [ ] **Step 2: Kotlin HUD.** In `GameActivity.kt` `tick`: after the fps/seed part append `+ " · " + arrayOf(t("grass","трава"), t("dirt","грунт"), t("sand","песок"), t("rock","скала"), t("gravel","гравий"), t("asphalt","асфальт"), t("water","вода")).getOrElse(s[28].toInt()) { "" }`.
- [ ] **Step 3: Docs and version.** `docs/game.md` EN/RU: add a paragraph to the Roads section: grip per surface table (short), loose layer, the HUD shows the surface; `docs/vehicles.md`: a sentence that `mu`, `lateral_stiffness` and `rolling_resistance` are the tarmac values and are scaled by the surface. CHANGELOGs `## 2.16.0 - grip by surface`. Bump `versionCode 28`, `versionName "2.16.0"`, `rust/Cargo.toml`.
- [ ] **Step 4: Tests and device:** `cargo test --lib`, `./gradlew -q testDebugUnitTest`; on the phone: `--ei road 1 --ef throttle 1.0` and `--ei road 2 --ef throttle 1.0` screenshots after the same delay (HUD speed and surface name); `--ei road 2 --ef throttle 1.0 --ef camyaw 0.0` etc. Compare speeds (gravel slower after 8 s), read the surface name on the HUD in each shot. Tune `grip.rs` numbers if the liaison or stage feels off.
- [ ] **Step 5: Commit** `feat: surface name in the HUD, docs for grip (2.16.0)`; do not merge or release without the user's go.
