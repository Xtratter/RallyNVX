# Dust driven by the airflow (stage 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** dust particles simulated on the GPU, driven by the airflow grid around the car, with a height-texture ground and a Settings switch; the old CPU particles stay as the fallback.

**Architecture:** `game/dust_ref.rs` is the host-tested reference of one particle step; `shaders/dust_gpu.wgsl` (`emit`, `step`) mirrors it; `raster/dust_gpu.rs` owns the buffers (state ring buffer, render buffer in the `DustInst` layout used as the vertex buffer of the existing dust pipeline), is fed emit requests by `game/dust.rs` (GPU mode: no CPU motion) and reads the airflow's velocity and solid textures. `Controls` has an "Aerodynamics" preference.

**Tech Stack:** Rust, wgpu compute (storage buffers + 3D textures), WGSL (naga `shaders_validate`), Kotlin Settings.

**Spec:** `docs/superpowers/specs/2026-10-15-airflow-dust-design.md` (stage 1 of "Scope in stages")

## Global Constraints

- The whole game stays at 30 fps or more on the POCO F3 (measured with the HUD fps, same autopilot scene as `docs/superpowers/specs/2026-10-15-airflow-design.md`).
- Particle capacity 1024 (the renderer's `MAX_INST`) first; raise only if the fps allows.
- Dust levels 1-2 use the GPU dust when the switch is on and the device can run it; level 3 (Fluid) and level 0 (Off) are unchanged.
- Numbers live in `surfaces.json` (`dust_physics`, `airflow`); bilingual docs; no sliders in the UI (the switch is a cycling button).

## Review Focus

- Compute not available (storage-format check fails): the CPU particles must run exactly as before.
- Car at rest / just placed (zero speed, inflow 0): no NaN, particles still settle.
- A particle exactly on the box edge: the air velocity is continuous (blend), no jump.
- Particle inside a solid voxel (car teleported onto dust): comes out, never stuck or NaN.
- Dust level switched while particles are alive: buffers cleared, no stale sprites.

---

### Task 1: Reference step and GPU spike (particles move on the GPU and are drawn)

**Files:** Create `rust/src/game/dust_ref.rs`, `rust/src/game/dust_ref_tests.rs`, `rust/src/shaders/dust_gpu.wgsl`, `rust/src/raster/dust_gpu.rs`; modify `rust/src/game/mod.rs`, `rust/src/raster/mod.rs`, `rust/src/shaders.rs` (+ validate), `rust/src/game/dust.rs` (GPU mode + pending emits), `rust/src/game/world.rs` (view fields), `rust/src/raster/gpu.rs`, `rust/src/game_jni.rs` + `Native.kt` + `GameActivity.kt` (debug extra `dustgpu`).

**Interfaces:**
- Produces: `dust_ref::GP { pos: V3, vel: V3, age: f32, life: f32, size0: f32, color: [f32;3], coarse: bool, seed: f32 }`; `dust_ref::step(p: &mut GP, air: V3, ground_y: f32, ph: &Phys, dt: f32) -> bool` (false = retire); `dust_ref::render(p: &GP) -> Particle` (same look as `Dust::particles`); `Dust::set_gpu(bool)`, `Dust::take_emits() -> Vec<GP>`.

- [ ] **Step 1: failing tests** in `dust_ref_tests.rs`: still air + no settle class keeps a particle at rest except settling; uniform air velocity relaxes with the class's tau (1/e after tau); fine settles slower than coarse; coarse particle that reaches the ground retires, fine one slides (vel.y = 0, horizontal speed shrinks); age past life retires; `render` alpha fades in over the first 8 % and out to 0 at the end; NaN air velocity does not poison the particle (treated as zero air).
- [ ] **Step 2:** `~/bin/t dust_ref` → FAIL. **Step 3:** implement `step` as the loop body of `Dust::update` (relaxation `k = 1 - exp(-dt/tau)`, target `air - (0, settle, 0)`, ground at `ground_y + 0.04`) and `render` as `Dust::particles`' body; make `Dust::update`/`particles` call them (no behaviour change; `dust_tests` stay green). **Step 4:** tests pass. Commit.
- [ ] **Step 5: GPU spike.** `dust_gpu.wgsl`: `struct S { a: vec4<f32> /*pos, size0*/, b: vec4<f32> /*vel, age*/, c: vec4<f32> /*colour, life*/, d: vec4<f32> /*coarse, seed, alive, 0*/ }`, `emit` (each invocation `i < n_emit` writes `emits[i]` into `state[(head + i) % cap]`), `step` (one invocation per particle: the reference step with air = wind only for now, ground plane `ground_y` from the uniform; writes the `DustInst` of that particle into `render[i]`: `p = (pos, size)`, `c = (colour * tone, alpha)`, `e = (seed, age/life, coarse, 0)`, alpha 0 and size 0 when dead). `shaders_validate` covers it. `dust_gpu.rs`: `DustGpu::new(adapter, dev) -> Option<_>`, `update(dev, queue, enc, frame: &DustFrame)`, `render_buffer() -> &wgpu::Buffer` (usage STORAGE | VERTEX), `count() -> u32`. `gpu.rs`: when `wv.dust_gpu` is Some use `render_buffer()` and `0..cap` instances for `pl_dust` instead of `dust_buf`.
- [ ] **Step 6:** `cargo build --lib`, `~/bin/t` green; debug extra `dustgpu=1` sets a runtime flag (`dust::set_gpu_debug`) that makes `World::view` build the GPU input. Device check: gravel scene, `--ei dustgpu 1`, screenshot the dust (it must look like variant B), fps. If the driver rejects the storage-buffer-as-vertex-buffer usage, fall back to a copy pass (`copy_buffer_to_buffer` into a vertex buffer) and note it. Commit.

### Task 2: Airflow coupling

**Files:** Modify `dust_ref.rs`(+tests), `dust_gpu.wgsl`, `dust_gpu.rs`, `airflow_gpu.rs` (expose `velocity_view()`, `solid_view()`, `box_info() -> BoxInfo { n, cell }`), `gpu.rs`.

**Interfaces:**
- Produces: `dust_ref::to_car_frame(p: V3, car_pos: V3, yaw: f32) -> V3`, `dust_ref::from_car_frame_dir(d: V3, yaw: f32) -> V3`, `dust_ref::box_weight(c: [f32;3] /*car frame, metres, origin at the box's x centre, y 0 road, z 0 back*/, size: [f32;3]) -> f32` (1 inside, smoothstep to 0 over `EDGE = 1.5` m at the faces), `dust_ref::air_in_box(p_world, car_pos, car_vel, yaw, grid_v: V3, outside: V3, size) -> V3` = `lerp(outside, car_vel + from_car_frame_dir(grid_v), box_weight)`.

- [ ] **Step 1: failing tests:** transform round trip is the identity (several yaws); `box_weight` is 1 at the centre, 0 outside, continuous (|w(x+ε)−w(x)| small across the edge), symmetric; `air_in_box` equals `outside` far away and `car_vel + R·grid_v` at the centre; with a grid velocity equal to the inflow (still air) the world air velocity is the wind (`car_vel + R·(−(car_vel − wind))` → wind, test with a yawed car).
- [ ] **Step 2-3:** implement; add `grid_sample` and `solid_at` to `dust_gpu.wgsl` (the airflow's velocity texture `vel` of the current ping-pong index and its `solid` texture bound as 3D textures; trilinear via `textureLoad` like `airflow.wgsl::sample`); `step` uses the blended air velocity; a particle in a solid cell is moved back to its previous position (`a.xyz − vel·dt`) and `vel` loses its component towards that cell. Bind groups are rebuilt each frame (the ping-pong index changes).
- [ ] **Step 4:** device check with the Vulpes + a high wing (`profile_file` test profile): dust cloud behind the car shaped by the wake; dust runs on after a hard stop (use `--ef throttle 1` then a reset to speed 0 is not possible, so compare the stopping autopilot at a corner); fps. Commit.

### Task 3: Ground height texture

**Files:** Modify `dust_gpu.rs`, `dust_gpu.wgsl`, `dust_ref.rs` (`ground_at` helper test), `gpu.rs`.

- [ ] **Step 1: failing test:** `dust_ref::height_cell(p: V3, origin: [i32;2], cell: f32, n: [usize;2]) -> Option<usize>` maps a world position to the texel (None outside), round trip with the texel centre.
- [ ] **Step 2-3:** the CPU samples `terrain.height` on a `64 x 64` window of 1 m cells centred on the car (recomputed only when the car crosses a cell), uploads as an `R32Float` 2D texture; `step` reads it (outside the window: the car's y as the ground, so particles only age). Remove the plane uniform. **Step 4:** device check on a slope; commit.

### Task 4: Settings switch and fallback

**Files:** Modify `app/src/main/java/dev/starinin/rallynvx/Controls.kt` (`aero(c)`, `setAero`), the Settings activity (a cycling button "Aerodynamics: On / Off", EN+RU strings), `rust/src/game_jni.rs` + `Native.kt` (`gameAero(on)`), `rust/src/game/dust.rs`/`world.rs` (GPU dust only when the flag is on, level 1-2, and the renderer reported it can), `rust/src/raster/gpu.rs` (`can_gpu_dust() -> bool` through a shared atomic read by `World::view`).

- [ ] **Step 1:** Kotlin unit test (`ControlsTest` style) for the default (On) and persistence if the existing tests cover prefs; Rust test that `World::view` produces no GPU dust input when the flag is off or level is 3 or 0 (`dust_tests`).
- [ ] **Step 2-3:** implement; the airflow is created whenever the GPU dust is on (no longer only for debug). **Step 4:** device check: switch Off → CPU particles as in 2.26.0; On → GPU. Commit.

### Task 5: Measure, tune, docs

- [ ] **Step 1:** autopilot scene, three runs: airflow+GPU dust On at levels 1 and 2, Off; HUD fps into the spec's Measurements; if below 30 fps: lower `airflow.cell` or the particle cap and measure again.
- [ ] **Step 2:** docs (`docs/game.md` EN+RU: the Aerodynamics switch, how the dust follows the flow; README bullet), `CHANGELOG(.ru).md` entry `2.27.0` replaces the debug-only entry, release notes `releases/rallynvx-2.27.0.md`, version already 2.27.0.
- [ ] **Step 3:** `~/bin/t`, Gradle unit tests, `assembleRelease`; commit; ask the user before releasing.
