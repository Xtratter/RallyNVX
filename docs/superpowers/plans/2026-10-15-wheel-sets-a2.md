# Wheel sets A2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Three wheel sets (tarmac, gravel, snow) on the Peugeot, chosen per car in the garage and saved in the profile; the look only (the grip effect is a later stage).

**Architecture:** A ninth garage level `wheel_set` (like the wing angle) flows profile -> `garage::apply` -> `VehicleSpec.wheel_set` -> `car_key` -> the GPU picks one of three wheel meshes from the `RCA3` file (meshes 0..=5: body, wheel tarmac, disc, caliper, wheel gravel, wheel snow); the Kotlin garage gets one cycling button.

**Tech Stack:** Rust (`~/bin/t [filter]`; converter: `cd tools/carconv && RUSTFLAGS="-C link-arg=-lc++" cargo build --release`, `cargo test`), Kotlin (Gradle unit tests), WGSL untouched. No device use without the user's yes (hard rule).

**Spec:** `docs/superpowers/specs/2026-10-15-wheel-sets-a2-design.md`

## Global Constraints
- The set changes nothing in physics (grip is a later stage): `VehicleSpec.wheel_set` is only read by the renderer.
- Old profiles (8 levels, no `wheel_set` key) load as tarmac; `wheel_set` above 2 clamps to 2; an out-of-range write is refused.
- `garage_info` layout: per car `[owned, price, cost, engine, tyres, suspension, brakes, splitter, wing, underbody, wing_angle, wheel_set, downforce_kg, drag_pct, top_kmh]` (15 fields; the Kotlin `CAR_FIELDS` follows).
- RCA3 with 2..=6 meshes parses; missing sets fall back to the tarmac wheel; wheel radius 0.326 m for every set; the livery stays 2048 wide in the atlas; atlas height at most 3072.
- Docs and changelog EN + RU; commit trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

## Review Focus
- A profile written before this change loads and the player keeps the car, the money and the upgrades.
- The off-roader (generated wheel): all three sets draw the same wheel, no panic.
- Switching the set in the garage updates the preview at once (the GPU rebuilds the wheel when `car_key` changes).
- An RCA3 file with only 2-4 meshes: every set shows the tarmac wheel.
- The disc and caliper stay the same for all sets and fit the other rims (the gravel and snow rims are the same size as the tarmac rim).

---

### Task 1: The `wheel_set` level through the profile, the API and the garage screen

**Files:** Modify `rust/src/game/garage.rs`, `rust/src/game/vehicle.rs` (`VehicleSpec.wheel_set`), `rust/src/game/params.rs`, `rust/src/game/meta.rs`, `rust/src/game/api.rs`, `rust/src/game_jni.rs`, `app/src/main/java/dev/starinin/rallynvx/Native.kt`, `GarageMath.kt`, `GarageActivity.kt`; tests in `garage_tests.rs`, `meta_tests.rs`, `api_tests.rs`, `app/src/test/java/dev/starinin/rallynvx/GarageMathTest.kt`.

**Interfaces:** Produces `garage::NLEVELS = 9`, `garage::MAX_WHEEL_SET = 2`, `default_levels() = [0,0,0,0,0,0,0,1,0]`, `VehicleSpec.wheel_set: u8`, `Profile::set_wheel_set(id, set) -> bool`, `api::garage_set_wheel_set(path, index, set) -> bool`, JNI `Native.garageWheelSet(path, index, set): Boolean`, `GarageMath.Car.wheelSet`.

- [ ] **Step 1: failing tests.** Rust: in `garage_tests.rs` `default_levels()[8] == 0` and `apply(&spec, [..., 2]).wheel_set == 2` and a level of 9 clamps to 2; in `meta_tests.rs` a profile JSON without `wheel_set` loads with set 0, one with `"wheel_set": 7` loads as 2, and `set_wheel_set(id, 3)` returns false, `set_wheel_set(id, 1)` true with a save/load round trip keeping 1; in `api_tests.rs` `garage_set_wheel_set(&path, 0, 1)` is true, `garage_info` has 15 fields per car with `wheel_set` at offset 11 after the 3 header floats, `garage_set_wheel_set(&path, 0, 5)` is false. Kotlin `GarageMathTest.kt`: `parse` of a hand-made array with 15 fields per car gives `wheelSet` from index 11 and clamps to 0..2; a short array returns null.
- [ ] **Step 2:** `~/bin/t garage`, `~/bin/t meta`, `~/bin/t api` and `./gradlew --no-daemon testDebugUnitTest -q` -> FAIL (compile errors first; fix the test literals of the old 8-element arrays by appending `0`).
- [ ] **Step 3: implement.** `garage.rs`: `NLEVELS = 9`, `MAX_WHEEL_SET: u8 = 2`, `default_levels` ends `..., 1, 0]`, `apply` adds `s.wheel_set = levels[8].min(MAX_WHEEL_SET);` and keeps the rest. `vehicle.rs` `VehicleSpec` gets `pub wheel_set: u8` (document: 0 tarmac, 1 gravel, 2 snow; visual only for now); `params.rs` constructor `wheel_set: 0`. `meta.rs`: save `"wheel_set": l[8]`; load `let set = l.get("wheel_set").and_then(Value::as_u64).map_or(0, |a| a.min(MAX_WHEEL_SET as u64) as u8);` and the levels array gets `set` as the ninth element (`default_levels()[8]` is 0); `set_wheel_set` (same shape as `set_wing_angle`, index 8, refuses `> MAX_WHEEL_SET`). `api.rs`: `garage_info` pushes `l[8] as f32` after `l[7]`, update the doc comment; `garage_set_wheel_set`. `game_jni.rs`: `Java_dev_starinin_rallynvx_Native_garageWheelSet` (copy `garageWingAngle`). Kotlin: `Native.kt` `garageWheelSet`; `GarageMath`: `CAR_FIELDS = 15`, `Car` gets `wheelSet: Int`, `parse` reads `a[o + 11]` for it and shifts the three aero floats to `o + 12..14`; add `wheelSetLabel(set, en)`: Tarmac/Gravel/Snow and Асфальт/Гравий/Снег. `GarageActivity.refresh()`: after the wing-angle row add the same kind of row: label "Wheel set" / "Комплект колёс", button text `GarageMath.wheelSetLabel(...)`, on click `if (car.owned) { Native.garageWheelSet(profile, shown, (car.wheelSet + 1) % 3); if (running) Native.gameGarageCar(shown); refresh() }` (the second call re-reads the car's spec so the preview shows the new wheels), alpha 0.5 when not owned.
- [ ] **Step 4:** all four test commands -> PASS; `~/bin/t` (whole suite) -> PASS.
- [ ] **Step 5: commit** `feat: the wheel set level (profile, API, garage button)`

### Task 2: Three wheel meshes in the model and the renderer

**Files:** Modify `rust/src/raster/rcar.rs` (`Parts`, `parse_all`), `rust/src/raster/vehicle_models.rs` (`CarModel`), `rust/src/raster/rally_model.rs`, `rust/src/raster/gpu.rs` (`car_key`, `ensure_car_model`); tests in `rcar.rs`, `rally_model_tests.rs`.

**Interfaces:** Produces `Parts { .., alt_wheels: [Option<Mesh3>; 2] }` (gravel, snow), `CarModel { .., alt_wheels: [Option<Mesh3>; 2] }` and `CarModel::wheel_for(&self, set: u8) -> &Mesh3` (0 tarmac, 1 gravel, 2 snow; a missing set returns the tarmac wheel); `car_key` becomes `[u32; 13]` with the set last.

- [ ] **Step 1: failing tests.** `rcar.rs`: extend the `rca3(n)` helper test: `parse_all(&rca3(6))` has `alt_wheels[0]` and `[1]` (their first vertex y 4.0 and 5.0), `rca3(5)` has only gravel, `rca3(4)` neither, `rca3(7)` and `rca3(1)` are errors. `rally_model_tests.rs`: `wheel_for(0/1/2)` are three different meshes for the vulpes once the asset has them (Task 3; until then `wheel_for(1)` equals `wheel_for(0)`: write the test for the final state and mark it `#[ignore]` until Task 3 lands, then un-ignore it there); `wheel_for(9)` is the tarmac wheel (no panic).
- [ ] **Step 2:** `~/bin/t rca3` -> FAIL.
- [ ] **Step 3: implement.** `parse_all`: `RCA3` count `2..=6`; `Parts.alt_wheels = [it.nth(4-4?...)]` — simply keep the iterator order: after body, wheel, disc, caliper take `gravel = it.next()`, `snow = it.next()`. `CarModel::new` sets `alt_wheels: [None, None]`; `wheel_for(set)` returns `self.alt_wheels.get(set as usize - 1).and_then(Option::as_ref).unwrap_or(&self.wheel)` for set 1 and 2, `&self.wheel` for 0 and anything else. `build_rally`: scale the alt wheels like the wheel (`wheel_width/0.226, k, k`). `gpu.rs`: `car_key` takes `s.wheel_set as u32` as a 13th element (update its type, the field and the initialiser); `ensure_car_model`: `self.models[1] = model_buf(&self.device, car.wheel_for(spec.wheel_set));`.
- [ ] **Step 4:** `~/bin/t` -> PASS. **Step 5: commit** `feat: the car model carries the gravel and snow wheels, the GPU picks the set`

### Task 3: Converter: the gravel and snow wheels; regenerate the asset

**Files:** Modify `tools/carconv/src/main.rs`; `rust/assets/rally_car.rcar`, `rust/assets/rally_car_livery.jpg`; tests in `rust/src/raster/rcar.rs`, `rust/src/raster/rally_model_tests.rs`; `rust/src/raster/livery.rs` (atlas size test).

- [ ] **Step 1: converter.** The wheel centre `wc` stays that of the A1 tarmac group (tyre + `x0_wheel_fl_5radios` + disc + caliper); add two groups `["tyre_gravel_lf", "x0_wheel_fl_gravel"]` and `["snow_tire_lf", "rim_snow_lf"]`, collected with `atlas = true`, shifted by the same `wc`, simplified to the wheel budget (`wheel_t`, default 2 800: the run uses `13500 2800`). Their textures join the atlas as wheel textures (tiles at most 256 for tyres, 384 for rims: extend the virtual-size rule: a wheel texture of 2048 -> 256, of 1024 -> 384). `ATLAS_H` becomes 3072 if the packer needs it (the livery stays 2048 wide: the packer test pins it). Write `RCA3` with count 6 in the order body, wheel tarmac, disc, caliper, wheel gravel, wheel snow.
- [ ] **Step 2: run** (unzip the source zip into an empty dir under `$TMPDIR`): `carconv .../source/*.glb $TMPDIR/new4.rcar 13500 2800 $TMPDIR/livery4.jpg`; expect the six meshes, the livery at 2048 wide, the atlas size line; copy both files into `rust/assets`; resize the JPEG to 1000 px and look at it (the new tiles are there).
- [ ] **Step 3: tests.** `rcar.rs`: `the_embedded_peugeot_has_three_wheel_sets`: `parse_all(PEUGEOT)` has both `alt_wheels`, each with more than 2 000 triangles, more than 70 % of their vertices with a UV, bounds radius `0.326 +- 0.01` and centre `|y|,|z| < 0.02`; the three wheels differ (the vertex counts are not all equal or the first positions differ). `rally_model_tests.rs`: un-ignore the `wheel_for` test from Task 2. `livery.rs`: the decoded atlas size test follows the new height. `carconv`: `cargo test` (the packer test with the extra tiles keeps the livery >= 1900 wide).
- [ ] **Step 4:** `~/bin/t` (all), `./gradlew --no-daemon testDebugUnitTest assembleRelease -q` -> PASS. **Step 5: commit** `feat: the gravel and snow wheel sets from the source model (RCA3 with six meshes)`

### Task 4: Docs and notes

**Files:** `docs/vehicles.md` (EN + RU), `docs/game.md` (the garage: the wheel set row, EN + RU), `rust/assets/ASSET-LICENSE.md`, `CHANGELOG.md`, `CHANGELOG.ru.md` (`2.32.0`), `~/projects/releases/rallynvx-2.32.0.md`.

- [ ] **Step 1:** write them: the three sets and what they look like, the garage button (free, per car, saved in the profile, old profiles load as tarmac), the `RCA3` six-mesh layout, the licence line (gravel and snow wheels), and a note that the grip effect of the sets is planned. Changelog EN + RU, notes EN + RU.
- [ ] **Step 2:** `~/bin/t` and the Gradle build; commit `docs: wheel sets A2`. Then ask the user for the phone (the garage: the three sets on the turntable, the drive with the gravel set, fps) and for the go before a release.
