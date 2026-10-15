# Running gear A1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Textured wheels, separate brake discs (glowing when hot) and non-spinning calipers on the Peugeot, plus mud flaps.

**Architecture:** `RCA3` carries up to four meshes (body, wheel, disc, caliper) with UVs into the one livery atlas; `CarModel` gets optional disc and caliper meshes drawn in two new model slots with the wheel's pose (the caliper without spin); a per-wheel `brake_heat` in `Vehicle` rides in the disc instance's tint alpha and drives an emission in `fs_model`.

**Tech Stack:** Rust (`~/bin/t [filter]`; the converter: `cd tools/carconv && RUSTFLAGS="-C link-arg=-lc++" cargo build --release`, tests with `cargo test`), WGSL (`shaders_validate`), Gradle. No device use without the user's yes (hard rule).

**Spec:** `docs/superpowers/specs/2026-10-15-running-gear-a1-design.md` (ruling: the rim style stays `x0_wheel_fl_5radios`, the style the game shows now, not STYLE1).

## Global Constraints
- 30 fps target (39-45 measured now); the wheel meshes total under 3 500 triangles each wheel; APK +0.5 MB at most.
- Older `RCAR` / `RCA2` files still parse (disc and caliper `None`); the generated hatchback and the offroader keep working (`CarModel { disc: None, caliper: None }`).
- Wheel radius 0.326 m, physics and collision unchanged; lamps, lamp interiors and the livery unchanged (their tests stay green); the livery stays at least 1900 px wide in the atlas.
- Vertex alpha classes: wheel parts keep their classes (tyre = rubber 2, rim = chrome 4, caliper = plastic 3); the disc gets the new class `13` (`MAT_CARDISC`).
- Source parts (left-front, centred on the wheel centre, mirrored by the game's 180 degree flip): `tyre_tarmac_lf`, `x0_wheel_fl_5radios`, `x0_disc_fl`, `x0_hub_caliper_fl`; mud flaps `x0_mud_flap_{fl,fr,bl,br}` join the body mesh.
- Docs and changelog EN + RU; commit trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

## Review Focus
- A model with no disc or caliper (old file, generated cars): nothing drawn for them, no panic, no slot confusion.
- The right-hand wheels (the 180 degree flip): the caliper and the disc are on the right wheel, not floating.
- Heat never leaves 0..1 (long braking, no braking, standing still, reverse).
- The disc glow must not make the lit body or the other car materials glow (only class 13).
- The wheel atlas tiles do not squeeze the livery (width stays >= 1900).

---

### Task 1: `RCA3` and the optional disc and caliper meshes

**Files:** Modify `rust/src/raster/rcar.rs` (`parse_all`, `Parts`), `rust/src/raster/vehicle_models.rs` (`CarModel`, `MAT_CARDISC`, `car_caliper_poses`), `rust/src/raster/rally_model.rs` (`build_rally`, constructors), `rust/src/raster/gpu.rs` (slots, models, draw).

**Interfaces:** Produces `pub struct Parts { pub body: Mesh3, pub wheel: Mesh3, pub disc: Option<Mesh3>, pub caliper: Option<Mesh3> }`, `rcar::parse_all(&[u8]) -> Result<Parts, String>` (`parse` stays and returns `(body, wheel)`); `CarModel { body, wheel, disc: Option<Mesh3>, caliper: Option<Mesh3> }` with `CarModel::new(body, wheel)`; `pub const MAT_CARDISC: u8 = 13`; `vehicle_models::car_caliper_poses(v) -> [(Q, V3); 4]` (the wheel poses with no spin); GPU slots `SLOT_DISC`, `SLOT_CALIPER` (= `SLOT_SIGN + 17`, `+ 18`, `MODEL_SLOTS = SLOT_SIGN + 19`).

- [ ] **Step 1: failing tests** (append to `rcar.rs` tests; build RCA3 bytes with a helper like `rca2`)
```rust
    fn rca3(meshes: usize) -> Vec<u8> {
        let mut d = b"RCA3".to_vec();
        d.extend_from_slice(&(meshes as u32).to_le_bytes());
        for m in 0..meshes {
            d.extend_from_slice(&3u32.to_le_bytes()); d.extend_from_slice(&3u32.to_le_bytes());
            for k in 0..3 { for f in [k as f32, m as f32, 0.0, 0.0, 1.0, 0.0] { d.extend_from_slice(&f.to_le_bytes()); } d.extend_from_slice(&[1, 2, 3, 0]); d.extend_from_slice(&[7, 0, 9, 0]); }
            for i in 0..3u32 { d.extend_from_slice(&i.to_le_bytes()); }
        }
        d
    }
    #[test] fn rca3_carries_up_to_four_meshes_and_older_files_have_no_disc() {
        let p = parse_all(&rca3(4)).unwrap();
        assert!(p.disc.is_some() && p.caliper.is_some());
        assert_eq!(p.body.verts[0].pos[1], 0.0); assert_eq!(p.wheel.verts[0].pos[1], 1.0); assert_eq!(p.disc.unwrap().verts[0].pos[1], 2.0); assert_eq!(p.caliper.unwrap().verts[0].uv, [7, 9]);
        let p3 = parse_all(&rca3(3)).unwrap(); assert!(p3.disc.is_some() && p3.caliper.is_none());
        let old = parse_all(PEUGEOT_V1).unwrap(); assert!(old.disc.is_none() && old.caliper.is_none());
        assert!(parse_all(&rca3(2)).is_ok() && parse_all(&rca3(1)).is_err() && parse_all(&rca3(5)).is_err(), "2 to 4 meshes");
        let t = rca3(4); assert!(parse_all(&t[..t.len() - 9]).is_err(), "truncated");
    }
```
  and in `vehicle_models_tests.rs`:
```rust
#[test] fn the_caliper_does_not_spin_but_follows_steer_and_suspension() {
    use crate::game::{math::v3, vehicle::{Vehicle, VehicleSpec}};
    let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0.0, 1.0, 0.0), 0.0); v.steer = 0.3; v.comp = [0.05, 0.0, 0.1, 0.0];
    let (wheel, cal) = (car_wheel_poses(&v, 1.2), car_caliper_poses(&v));
    for w in 0..4 { assert!((wheel[w].1 - cal[w].1).len() < 1e-5, "same hub position"); }
    let spun = car_wheel_poses(&v, 0.0);
    for w in 0..4 { assert!((cal[w].0.rotate(v3(0., 1., 0.)) - spun[w].0.rotate(v3(0., 1., 0.))).len() < 1e-5, "the caliper's orientation is the wheel's with no spin"); }
    assert!((wheel[0].0.rotate(v3(0., 1., 0.)) - cal[0].0.rotate(v3(0., 1., 0.))).len() > 0.1, "the wheel itself has turned");
}
```
- [ ] **Step 2:** `~/bin/t rca3` / `~/bin/t caliper` → FAIL (missing items).
- [ ] **Step 3: implement.** `rcar.rs`: `pub struct Parts`; `parse_all`: magic `RCAR` (28) / `RCA2` (32) read two meshes; `RCA3` (32): `u32 count` (2..=4, else error) then that many meshes via `mesh(d, &mut o, 32)`; map to `Parts` (index 2 = disc, 3 = caliper); `parse` = `parse_all(d).map(|p| (p.body, p.wheel))`. `vehicle_models.rs`: `pub struct CarModel { pub body: Mesh3, pub wheel: Mesh3, pub disc: Option<Mesh3>, pub caliper: Option<Mesh3> }` + `impl CarModel { pub fn new(body: Mesh3, wheel: Mesh3) -> Self { Self { body, wheel, disc: None, caliper: None } } }`; replace the 4 `CarModel { body, wheel }` literals with `CarModel::new(..)`; `MAT_CARDISC`; `car_caliper_poses(v) = car_wheel_poses(v, 0.0)`. `rally_model.rs::build_rally`: use `parse_all`; scale `disc` and `caliper` like the wheel (`v.pos[0] * wheel_width/0.226, y*k, z*k`) and set them in the `CarModel`. `gpu.rs`: add the two slots and `MODEL_SLOTS`; in the `models` creation (the place that asserts `models.len() == MODEL_SLOTS`) add two placeholder `model_buf(&device, &Mesh3::default())`-style empties (copy how the sign slots are built; an empty mesh must draw nothing: skip a slot whose `n == 0`); in `ensure_car_model` fill `models[SLOT_DISC]`/`[SLOT_CALIPER]` from `car.disc`/`car.caliper` or empty; in the frame, after the wheel instances push `ml[SLOT_DISC]` (wheel pose, tint alpha from Task 3) and `ml[SLOT_CALIPER]` (`car_caliper_poses`) only if the model has them (store `car_has_disc`/`car_has_caliper` booleans next to `car_key`).
- [ ] **Step 4:** `~/bin/t` (all) and `~/bin/t shaders_validate` → PASS.
- [ ] **Step 5: commit** `feat: RCA3 and optional disc and caliper meshes (slots, poses)`

### Task 2: Brake heat in the vehicle

**Files:** Modify `rust/src/game/vehicle.rs` (field `brake_heat: [f32; 4]`, update in `step`), `rust/src/game/vehicle_tests.rs` (or the existing vehicle tests file).

**Interfaces:** Produces `Vehicle.brake_heat: [f32; 4]` (0..1, purely visual).

- [ ] **Step 1: failing tests**
```rust
#[test] fn brake_heat_grows_when_braking_hard_cools_down_and_stays_in_range() {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.2, 0.), 0.0);
    for _ in 0..180 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    assert!(v.brake_heat.iter().all(|h| *h == 0.0), "cold at rest");
    for _ in 0..600 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    let fast = v.vel.len(); assert!(fast > 10.0, "{fast}");
    for _ in 0..240 { v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 60.0); }
    let hot = v.brake_heat; assert!(hot.iter().all(|h| (0.0..=1.0).contains(h)) && hot[0] > 0.05, "{hot:?}");
    assert!(hot[0] >= hot[2], "the front brakes work harder");
    for _ in 0..1200 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    assert!(v.brake_heat.iter().all(|h| *h < hot[0] * 0.2 + 1e-3), "{:?} cooled from {:?}", v.brake_heat, hot);
}
```
- [ ] **Step 2:** `~/bin/t brake_heat` → FAIL.
- [ ] **Step 3: implement.** Add `pub brake_heat: [f32; 4]` to `Vehicle` and `[0.0; 4]` in `Vehicle::new`. In the wheel loop of `step`, where `k` (the brake factor) is known (`vehicle.rs` around the `brake_force` line): `let power = if k > 0.0 { s.brake_force / 4.0 * k * vf.abs() } else { 0.0 };` then `let share = if front { 1.3 } else { 0.7 };` and `self.brake_heat[w] = (self.brake_heat[w] + power * share * 1.5e-6 * dt).min(1.0);` and the cooling once per step per wheel: `self.brake_heat[w] *= (-dt * (1.0 + self.vel.len() / 15.0) / 6.0).exp();` (the loop variable names follow the file; `front` is the file's own flag, `vf` the wheel's forward speed). If the loop variables differ, keep the formulas and adapt the names.
- [ ] **Step 4:** `~/bin/t vehicle` and `~/bin/t` → PASS. If the heat asserts fail by magnitude, tune the gain `1.5e-6` (not the test thresholds) until a 4 s hard stop from about 20 m/s reaches 0.3-0.8 on the front wheels.
- [ ] **Step 5: commit** `feat: per-wheel brake heat (visual)`

### Task 3: The disc material and the heat glow

**Files:** Modify `rust/src/raster/gpu.rs` (disc instance tint), `rust/src/shaders/raster.wgsl` (`MOut.heat`, `vs_model`, `fs_model`).

**Interfaces:** Consumes `Vehicle.brake_heat`, `MAT_CARDISC = 13`. The disc instance's `tint.a = 0.1 + 0.4 * heat` (always below 0.75, so it stays "the player's car"); the body and wheel keep `0.5`. WGSL: `MOut.heat = clamp((tint.a - 0.1) / 0.4, 0.0, 1.0)`.

- [ ] **Step 1:** In `gpu.rs` when pushing the disc instances: `tint: [1.0, 1.0, 1.0, 0.1 + 0.4 * veh.brake_heat[w].clamp(0.0, 1.0)]` (index `w` is the wheel's index in `car_wheel_poses` order).
- [ ] **Step 2: shader.** `MOut` gets `@location(7) heat: f32`; `vs_model` sets `o.heat = clamp((tint.a - 0.1) / 0.4, 0.0, 1.0);`. In `fs_model`, before the plastic fallback:
```wgsl
    if (m == 13) {   // a brake disc: textured metal that glows orange-red when hot (stronger in the dark)
        let hot = i.heat * i.heat;
        let glow = vec3<f32>(1.0, 0.32, 0.08) * hot * (1.2 + 3.0 * u.disc.w);
        return vec4<f32>(shade(albedo * occl, i.wp, n, 0.9, 60.0) + finish(glow), 1.0);
    }
```
- [ ] **Step 3:** `~/bin/t shaders_validate`, `~/bin/t` → PASS.
- [ ] **Step 4: commit** `feat: the brake disc glows when hot`

### Task 4: Converter: wheel parts, mud flaps, one atlas, RCA3; regenerate the asset

**Files:** Modify `tools/carconv/src/main.rs`; Modify `rust/assets/rally_car.rcar`, `rust/assets/rally_car_livery.jpg`; tests in `rust/src/raster/rcar.rs` and `rust/src/raster/rally_model_tests.rs`.

- [ ] **Step 1: converter changes.** (a) `keep_body`: let the mud flaps in: before the `bad` check add `if n.starts_with("x0_mud_flap_") { return true; }`. (b) Class `13`: in `class_of` add `if m == "discs" { return (13, false); }`. (c) Collect the wheel parts with `atlas = true` as three groups: tyre+rim (`tyre_tarmac_lf`, `x0_wheel_fl_5radios`), disc (`x0_disc_fl`), caliper (`x0_hub_caliper_fl`); the wheel centre `c` is still the centre of the union of all four parts, subtracted from every vertex of the three meshes. (d) Atlas: the used set is the union of the textures of the body and of the three wheel groups; give `atlas::pack` virtual sizes: the livery keeps its size, every other image larger than 512 is passed as its size clamped to 512 on the long side (aspect kept), smaller ones unchanged; after the pack, remap the UVs of all four meshes with the same rectangle table (the existing remap loop, applied to each mesh's vertices) and resize each source texture to its rectangle. (e) `simplify` budgets: body 13 500 (as now), wheel (tyre + rim) 2 500, disc 220 (no simplification), caliper 500. (f) Write `RCA3`: magic, `u32 4`, then the four meshes (`write`). Update the usage line unchanged (same arguments).
- [ ] **Step 2: run it** (empty dir under `$TMPDIR`, unzip the source zip there; run the converter from another directory): `carconv .../source/*.glb $TMPDIR/new3.rcar 13500 2500 $TMPDIR/livery3.jpg`; expect `atlas:` lines with the livery at 2048 x 1366 and the wheel textures as tiles in the free band, the bounds unchanged (`x -0.954..0.954 y 0.077..1.501 z -1.870..2.088`), and four meshes written. Copy both files into `rust/assets`, look at the JPEG (resize to 1000 px) and check that the tyre and rim tiles are there.
- [ ] **Step 3: tests** (`rcar.rs`): `the_embedded_peugeot_has_four_meshes_with_textured_wheel_parts`: `parse_all(PEUGEOT)` gives `disc` and `caliper`; the wheel mesh has more than 2 000 triangles and more than 70 % of its vertices have a UV; the disc's vertices have class 13; the caliper has a UV; the wheel mesh is centred (`|centre| < 0.05` of its bounds) and its radius is within 1 cm of 0.326. `rally_model_tests.rs`: `the_rally_model_has_a_disc_and_a_caliper` (`build_rally(&vulpes)` has both, scaled with the spec radius: compare the disc's bounds with `radius`). Run `~/bin/t` (all): the lamp, livery and bounds tests must stay green; if a lamp point moves by more than its tolerance stop and investigate.
- [ ] **Step 4:** `cd tools/carconv && cargo test` → PASS (the atlas tests pass with the virtual sizes: add a test `wheel_tiles_do_not_squeeze_the_livery` packing `(0, 6144, 4096), (55, 512, 512), (47, 512, 512), (57, 256, 256), (41, 64, 64), (2, 256, 256)` into 2048 and asserting the livery tile `w >= 1900`).
- [ ] **Step 5: gradle build** `./gradlew --no-daemon testDebugUnitTest assembleRelease -q` → PASS. **Step 6: commit** `feat: textured wheel, disc and caliper meshes and mud flaps from the source model (RCA3)`

### Task 5: Docs and notes

**Files:** `docs/vehicles.md` (EN + RU: `RCA3`, the parts, the atlas), `rust/assets/ASSET-LICENSE.md` (wheel, disc, caliper, mud-flap parts), `CHANGELOG.md`, `CHANGELOG.ru.md` (`2.31.0`), `~/projects/releases/rallynvx-2.31.0.md`.

- [ ] **Step 1:** write the docs: the Peugeot's wheels are textured (tyre sidewall, rim), the brake discs are separate and glow after hard braking at night, the calipers do not spin, mud flaps; the format `RCA3` (up to four meshes) in `docs/vehicles.md`; the licence paragraph; changelog EN + RU; notes EN + RU.
- [ ] **Step 2:** `~/bin/t` and the Gradle build; commit `docs: running gear A1`. Then ask the user for the phone (daylight close shots of a wheel, a hard stop at night for the glow, fps) and for the go before a release.
