# Lamp optics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lens-like lamps, a real two-lamp low beam with a cut-off, a view-dependent glow and position lights for every car, found on the mesh.

**Architecture:** Host-testable maths lives in `game/carlights.rs` (beam gain, glow factor, lamp positions) and `raster/rcar.rs` (lamp classes found by colour); WGSL mirrors the beam function in one place (`headbeam`) used by the ground, the dust and the sky haze; the lens look is the `m == 7/8` branch of `fs_model`; glow reuses the existing `glow` instances in `gpu.rs`.

**Tech Stack:** Rust (`~/bin/t [filter]` = `cargo test --lib`), WGSL (naga test `shaders_validate`), Gradle only for the final build. No device use without the user's yes (hard rule).

**Spec:** `docs/superpowers/specs/2026-10-15-lamp-optics-design.md`

## Global Constraints
- 30 fps target; the beam is evaluated only where the old cone was (same 4 sites).
- Boxy generated cars (`CarStyle::Suv`) keep their glowing boxes and the old behaviour (they get the new beam too, it only needs `hl_pos`, `hl_dir`, `lamps.w`).
- Turn signals, hazard, high beam, steering-following light: NOT in this stage.
- Docs and changelog bilingual EN + RU; commit trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Measured on the embedded asset (right side, mesh frame): main headlight cluster ~[0.53,0.62,1.72] (40 verts), outer corner piece ~[0.68,0.68,1.61] (13 verts, cream), centre cream sliver x=0 (11 verts), tail red [0.61,0.75,-1.65], reverse white [0.58,0.82,-1.65], orange indicators front [0.40,0.58,1.85] / rear [0.68,0.83,-1.57].

## Review Focus
- A mesh without a corner piece (no position class): position lamps fall back to the head point, no panic.
- Glow seen from behind / from the side of a front lamp: zero or faint, never a bright dot through the car.
- A car rotated or pitched: beam and glow follow the body, not world axes.
- Lamps off: beam gain exactly 0, glow 0.
- Zero-length vectors in the beam (point at the lamp itself): no NaN.

---

### Task 1: Lamp classes on the mesh

**Files:** Modify `rust/src/raster/rcar.rs` (`LampPoints`, `lamp_points`, tests); `rust/src/game/carlights.rs` (`RALLY_FALLBACK`).

**Interfaces:**
- Produces: `LampPoints { head, tail, rev: [f32;3], pos: [f32;3] }` (pos = the corner piece of the front unit, equals `head` when the mesh has none); `pub fn is_position_vertex(body: &Mesh3, v: &Vert) -> bool` is NOT needed: Task 3 uses `position_mask`.
- Produces: `pub fn position_vertices(body: &Mesh3) -> Vec<usize>`: indices of the cream front vertices of the corner piece, both sides.

- [ ] **Step 1: failing test** (append to `rcar.rs` tests)
```rust
    #[test] fn the_front_unit_splits_into_main_lamp_and_corner_position_piece() {
        let (b, _) = parse(PEUGEOT).unwrap();
        let l = lamp_points(&b).expect("lamps");
        assert!((l.head[0] - 0.53).abs() < 0.05 && (l.head[2] - 1.72).abs() < 0.05, "main {:?}", l.head);
        assert!(l.pos[0] > l.head[0] + 0.08 && l.pos[2] < l.head[2], "the corner piece is outboard and behind the lens: {:?}", l.pos);
        let idx = position_vertices(&b);
        assert!(idx.len() >= 12 && idx.iter().all(|&i| b.verts[i].pos[2] > 1.0), "{}", idx.len());
        assert!(idx.iter().any(|&i| b.verts[i].pos[0] < 0.0) && idx.iter().any(|&i| b.verts[i].pos[0] > 0.0), "both sides");
    }
    #[test] fn a_mesh_without_a_corner_piece_uses_the_head_point_for_the_position_lamp() {
        let mut m = Mesh3::default();
        let mut vert = |x: f32, z: f32, c: [u8; 4]| m.verts.push(Vert { pos: [x, 0.6, z], nrm: [0.0, 1.0, 0.0], col: c });
        for i in 0..8 { vert(0.5 + 0.005 * i as f32, 1.7, [255, 242, 204, 7]); }        // front unit
        for i in 0..8 { vert(0.6 + 0.005 * i as f32, -1.6, [242, 13, 8, 7]); }           // tail
        for i in 0..8 { vert(0.5 + 0.005 * i as f32, -1.65, [255, 255, 255, 7]); }       // reverse
        let l = lamp_points(&m).expect("lamps");
        assert_eq!(l.pos, l.head);
        assert!(position_vertices(&m).is_empty());
    }
```
- [ ] **Step 2:** `~/bin/t the_front_unit` / `a_mesh_without` → FAIL (no `pos`, no `position_vertices`).
- [ ] **Step 3: implement.** In `rcar.rs` replace `LampPoints` and `lamp_points`:
```rust
pub struct LampPoints { pub head: [f32; 3], pub tail: [f32; 3], pub rev: [f32; 3], pub pos: [f32; 3] }

/// The cream front vertices (right side) split in two: the outboard corner piece (within 6 cm of the outermost x, at least 6 vertices) and the main lens.
fn front_split(body: &Mesh3) -> (Vec<usize>, Vec<usize>) {
    let cream = |v: &Vert| (v.col[3] == 5 || v.col[3] == 7) && v.col[0] > 200 && v.col[1] > 200 && v.col[2] > 140 && v.pos[2] > 0.0;
    let right: Vec<usize> = (0..body.verts.len()).filter(|&i| cream(&body.verts[i]) && body.verts[i].pos[0] > 0.3).collect();
    // the centre sliver (x near 0) is not a lamp unit; the outermost cluster defines the corner
    let Some(max_x) = right.iter().map(|&i| body.verts[i].pos[0]).max_by(f32::total_cmp) else { return (vec![], vec![]) };
    let (corner, main): (Vec<usize>, Vec<usize>) = right.into_iter().partition(|&i| body.verts[i].pos[0] > max_x - 0.06);
    if corner.len() >= 6 && main.len() >= 6 { (main, corner) } else { (main.into_iter().chain(corner).collect(), vec![]) }
}

/// Indices of the vertices of the front corner (position-light) pieces, both sides: Task 3 gives them their own lamp material.
pub fn position_vertices(body: &Mesh3) -> Vec<usize> {
    let (_, corner) = front_split(body);
    if corner.is_empty() { return vec![]; }
    let lo = corner.iter().map(|&i| body.verts[i].pos[0]).fold(f32::MAX, f32::min) - 1e-4;
    let cream = |v: &Vert| (v.col[3] == 5 || v.col[3] == 7) && v.col[0] > 200 && v.col[1] > 200 && v.col[2] > 140 && v.pos[2] > 0.0;
    (0..body.verts.len()).filter(|&i| { let v = &body.verts[i]; cream(v) && v.pos[0].abs() >= lo && v.pos[0].abs() > 0.3 }).collect()
}

pub fn lamp_points(body: &Mesh3) -> Option<LampPoints> {
    let (main, corner) = front_split(body);
    let mut tail = Vec::new(); let mut rev = Vec::new();
    for v in &body.verts {
        if v.col[3] != 5 && v.col[3] != 7 { continue; }
        if v.pos[0] <= 0.0 { continue; }   // the right-hand side
        let (r, g, b) = (v.col[0], v.col[1], v.col[2]);
        if r > 200 && g < 70 && b < 70 { if v.pos[2] < 0.0 { tail.push(v.pos); } }
        else if r > 200 && g > 200 && b > 140 && v.pos[2] < 0.0 { rev.push(v.pos); }
    }
    // the outermost cluster of at least a few vertices: the main unit, not a centre brake light or a fog lamp sliver
    let pick = |pts: &[[f32; 3]]| -> Option<[f32; 3]> {
        let mut clusters: Vec<Vec<[f32; 3]>> = Vec::new();
        for p in pts {
            match clusters.iter_mut().find(|c| c.iter().any(|q| (0..3).all(|a| (p[a] - q[a]).abs() < 0.12))) { Some(c) => c.push(*p), None => clusters.push(vec![*p]) }
        }
        clusters.into_iter().filter(|c| c.len() >= 6).map(|c| { let n = c.len() as f32; [0, 1, 2].map(|a| c.iter().map(|p| p[a]).sum::<f32>() / n) })
            .max_by(|a, b| a[0].total_cmp(&b[0]))
    };
    let centre = |idx: &[usize]| -> Option<[f32; 3]> { if idx.is_empty() { None } else { let n = idx.len() as f32; Some([0, 1, 2].map(|a| idx.iter().map(|&i| body.verts[i].pos[a]).sum::<f32>() / n)) } };
    let head = centre(&main)?;
    Some(LampPoints { head, tail: pick(&tail)?, rev: pick(&rev)?, pos: centre(&corner).unwrap_or(head) })
}
```
Also update `the_lamp_points_are_found_on_the_asset_and_the_main_units_win` expected head to `[0.53, 0.62, 1.72]` (±0.05) and `RALLY_FALLBACK` in `carlights.rs` to `{ head: [0.53, 0.62, 1.72], pos: [0.68, 0.68, 1.61], tail: [0.611, 0.747, -1.654], rev: [0.584, 0.818, -1.651] }`. If the measured head differs from 0.53 by more than 0.05, set the test and fallback to the printed value (print with `{:?}` in the failing assert).
- [ ] **Step 4:** `~/bin/t rcar` → PASS (all rcar tests).
- [ ] **Step 5: commit** `feat: lamp classes on the mesh: main lens and corner position piece`

### Task 2: Beam, glow and position lamps (host maths)

**Files:** Modify `rust/src/game/carlights.rs`; test in `carlights_tests.rs`.

**Interfaces:**
- Consumes: `LampPoints.pos` (Task 1).
- Produces: `Lamps` gains `pub pos: [V3; 2]` (front position lamps); `pub fn beam_gain(lamp: V3, fwd: V3, right: V3, p: V3) -> f32` (one lamp, 0..1); `pub fn beam(l: &Lamps, right: V3, p: V3) -> f32` (mean of both lamps); `pub fn glow_factor(axis: V3, to_cam: V3) -> f32` (0..1).

- [ ] **Step 1: failing tests** (append to `carlights_tests.rs`)
```rust
fn flat_lamp() -> (V3, V3, V3) { (v3(0.6, 0.7, 0.0), v3(0.0, -0.05, 1.0).norm(), v3(1.0, 0.0, 0.0)) }

#[test] fn the_beam_is_strongest_on_the_axis_and_falls_with_distance() {
    let (l, f, r) = flat_lamp();
    let near = beam_gain(l, f, r, l + f * 8.0 + v3(0.0, -0.4, 0.0));
    let far = beam_gain(l, f, r, l + f * 60.0 + v3(0.0, -3.0, 0.0));
    assert!(near > 0.5 && far < near * 0.5 && far > 0.0, "{near} {far}");
}
#[test] fn the_beam_has_a_sharp_upper_cut_off_that_is_higher_on_the_right() {
    let (l, f, r) = flat_lamp();
    let below = beam_gain(l, f, r, l + f * 20.0 + v3(0.0, -0.5, 0.0));
    let above = beam_gain(l, f, r, l + f * 20.0 + v3(0.0, 3.0, 0.0));
    assert!(below > 0.3 && above < 0.02, "{below} {above}");
    let cut = |x: f32| beam_gain(l, f, r, l + f * 20.0 + r * x + v3(0.0, 0.55, 0.0));   // 0.55 m above the axis point at 20 m: between the left and right cut-offs
    assert!(cut(5.0) > cut(-5.0) * 2.0, "right {} left {}", cut(5.0), cut(-5.0));
}
#[test] fn the_beam_ignores_what_is_behind_or_far_to_the_side_and_never_gives_nan() {
    let (l, f, r) = flat_lamp();
    assert_eq!(beam_gain(l, f, r, l - f * 5.0), 0.0);
    assert!(beam_gain(l, f, r, l + r * 20.0 + f * 2.0) < 0.01);
    for p in [l, l + f * 1e-6] { assert!(beam_gain(l, f, r, p).is_finite()); }
}
#[test] fn the_beam_follows_a_turned_and_pitched_body() {
    let (l, f, r) = flat_lamp();
    let q = Q::axis_angle(v3(0., 1., 0.), 1.1).mul(Q::axis_angle(v3(1., 0., 0.), 0.2));
    let (l2, f2, r2) = (q.rotate(l), q.rotate(f), q.rotate(r));
    let a = beam_gain(l, f, r, l + f * 12.0 + v3(0.0, -0.4, 0.0));
    let b = beam_gain(l2, f2, r2, l2 + q.rotate(f * 12.0 + v3(0.0, -0.4, 0.0)));
    assert!((a - b).abs() < 1e-4, "{a} {b}");
}
#[test] fn the_glow_needs_a_view_along_the_lamp() {
    let axis = v3(0.0, 0.0, 1.0);
    assert!(glow_factor(axis, v3(0.0, 0.0, 1.0)) > 0.95);
    assert!(glow_factor(axis, v3(1.0, 0.0, 0.2).norm()) < 0.25, "from the side");
    assert_eq!(glow_factor(axis, v3(0.0, 0.0, -1.0)), 0.0, "from behind the lamp");
}
#[test] fn rally_cars_have_position_lamps_beside_the_headlights_at_the_front() {
    let v = settled(VehicleSpec::rally());
    let l = lamps(&v);
    for p in l.pos { assert!((p - v.pos).dot(v.forward()) > v.spec.size.z * 0.35); }
    assert!(((l.pos[0] - v.pos).dot(v.right()) + (l.pos[1] - v.pos).dot(v.right())).abs() < 1e-3);
}
```
(Imports at the top of the tests file already include `Q`, `v3`; add `V3` to the `math` import.)
- [ ] **Step 2:** `~/bin/t carlights` → FAIL (functions missing). If `VehicleSpec::rally()` has another name, use `VehicleSpec::all()` and pick the one whose `style == CarStyle::Rally`.
- [ ] **Step 3: implement** in `carlights.rs`:
```rust
/// One headlight's low beam at point `p` (0..1): `fwd` the beam axis (slightly down), `right` across it. Wide sideways (about 35 degrees each way), a sharp upper
/// cut-off at the axis that rises towards the right (right-hand traffic: it lights the verge), a brighter middle, 1/d^2 fall-off with a far limit. Mirrored in `raster.wgsl::headbeam`.
pub fn beam_gain(lamp: V3, fwd: V3, right: V3, p: V3) -> f32 {
    let d = p - lamp;
    let dist = d.len();
    if !(dist > 0.05) { return 0.0; }
    let l = d * (1.0 / dist);
    let (z, x) = (l.dot(fwd), l.dot(right));
    if z <= 0.05 { return 0.0; }
    let up = right.cross(fwd);
    let y = l.dot(up);
    let (h, v) = ((x / z).atan(), (y / z).atan());
    let cut = 0.07 * smooth(0.0, 0.25, h);
    let gv = 1.0 - smooth(cut, cut + 0.02, v);
    let gh = 1.0 - smooth(0.35, 0.65, h.abs());
    let hot = 0.6 + 0.4 * (-(h / 0.2) * (h / 0.2)).exp();
    let att = 1.0 / (1.0 + (dist / 18.0) * (dist / 18.0));
    gv * gh * hot * att * (1.0 - dist / 90.0).clamp(0.0, 1.0)
}
fn smooth(a: f32, b: f32, x: f32) -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

/// The beam of both lamps at `p`, the mean of the two.
pub fn beam(l: &Lamps, right: V3, p: V3) -> f32 { 0.5 * (beam_gain(l.head[0], l.head_dir, right, p) + beam_gain(l.head[1], l.head_dir, right, p)) }

/// How much of a lamp's glow shows from the camera: full along the lamp's axis, faint from the side, nothing from behind.
pub fn glow_factor(axis: V3, to_cam: V3) -> f32 { let c = axis.dot(to_cam); if c <= 0.0 { 0.0 } else { c.powf(2.5) } }
```
Add `pos: [V3; 2]` to `Lamps`; in `lamps()` (boxy cars) set `pos: head`; in `rally_lamps` set `pos: [at(p.pos, -1.0), at(p.pos, 1.0)]`. Check the vector API names (`len`, `dot`, `cross`, `Mul<f32>`); adapt to `math.rs`.
- [ ] **Step 4:** `~/bin/t carlights` → PASS. Tune constants only if a test boundary fails by construction (not the test).
- [ ] **Step 5: commit** `feat: low beam model, glow factor and position lamps on the car`

### Task 3: Lens look and position-lamp material

**Files:** Modify `rust/src/raster/vehicle_models.rs` (`MAT_CARPOS = 8`), `rust/src/raster/rally_model.rs` (`build_rally`), `rust/src/shaders/raster.wgsl` (`fs_model`), tests in `rally_model.rs`.

**Interfaces:** Consumes `rcar::position_vertices`. Produces material id 8 = a front position lamp (lit with the headlights, dim white, no beam).

- [ ] **Step 1: failing test** (rally_model.rs tests)
```rust
    #[test] fn the_corner_pieces_of_the_front_units_are_position_lamps() {
        let m = build_rally(&crate::game::vehicle::VehicleSpec::rally());
        let n = m.body.verts.iter().filter(|v| v.col[3] == MAT_CARPOS).count();
        assert!(n >= 12, "{n}");
        assert!(m.body.verts.iter().filter(|v| v.col[3] == MAT_CARPOS).all(|v| v.pos[2] > 0.5), "only at the front");
        assert!(m.body.verts.iter().any(|v| v.col[3] == MAT_CARLAMP), "the main lenses stay lamps");
    }
```
- [ ] **Step 2:** `~/bin/t corner_pieces` → FAIL (`MAT_CARPOS` undefined).
- [ ] **Step 3: implement.** `pub const MAT_CARPOS: u8 = 8;` in `vehicle_models.rs`. In `build_rally`, after the MAT_CARLAMP remap and before scaling: `for i in super::rcar::position_vertices(&body) { body.verts[i].col[3] = super::vehicle_models::MAT_CARPOS; }`. In `raster.wgsl` `fs_model` replace the `m == 7` block by the lens block and add `m == 8`:
```wgsl
    if (m == 7 || m == 8) {   // the car's own lamps: the shape is the mesh's, the state comes from the car; the class from the colour (8: a front position lamp)
        let c = i.col;
        let v = normalize(u.cam.xyz - i.wp);
        let facing = clamp(abs(dot(n, v)), 0.0, 1.0);
        let fr = 1.0 - facing;                                     // 0 looking straight into the lens, 1 at the rim
        var k = 0.4;
        var on = 0.0;
        if (m == 8) { on = select(0.0, 1.0, u.lamps.x > 0.5); k = 1.6 + 1.2 * u.disc.w; }                                  // position lamp: with the low beam
        else if (c.r > 0.8 && c.g < 0.3) { on = max(select(0.0, 0.3, u.lamps.x > 0.5), select(0.0, 1.0, u.lamps.y > 0.5)); k = select(0.0, 3.2, u.lamps.y > 0.5) + select(0.0, 1.0, u.lamps.x > 0.5); }   // tail unit: brake bright, headlights dim red
        else if (c.r > 0.8 && c.g < 0.75 && c.b < 0.4) { k = 0.5; }                                                         // indicator: not lit yet
        else if (c.b > 0.93) { on = select(0.0, 1.0, u.lamps.z > 0.5); k = 3.0; }                                          // reverse lamp
        else { on = select(0.0, 1.0, u.lamps.x > 0.5); k = 2.8 + 2.0 * u.disc.w; }                                          // headlight unit
        // a lit lens: a hot core looking straight into it, a darker rim, a stepped reflector (rings in the angle to the view)
        let rings = 0.86 + 0.14 * step(0.5, fract(fr * 6.0));
        let core = mix(0.45, 1.0, pow(facing, 3.0)) * rings;
        let lit = c * k * core * on;
        // an unlit lens: dark glass with a rim reflection of the sky and the sun
        let sunr = pow(max(dot(reflect(-v, n), normalize(u.sun.xyz)), 0.0), 40.0);
        let dark = c * 0.10 * (1.0 - on) + (u.sky_hor.rgb * 0.45 * fr * fr * fr + vec3<f32>(sunr) * 0.6) * (1.0 - on);
        return vec4<f32>(finish(lit + dark), 1.0);
    }
```
  Indicators and unknown classes keep a dim body `c * 0.10`, which is the dark glass; acceptable for this stage.
- [ ] **Step 4:** `~/bin/t corner_pieces`, `~/bin/t shaders_validate`, `~/bin/t rally_model` → PASS.
- [ ] **Step 5: commit** `feat: lens look for the car's lamps, position-lamp material`

### Task 4: One beam function for ground, dust and haze

**Files:** Modify `rust/src/shaders/raster.wgsl` (4 sites + function), `rust/src/raster/gpu.rs` (`lamps.w`).

**Interfaces:** Consumes `u.hl_pos` (midpoint, `.w` on flag), `u.hl_dir.xyz` (axis), `u.lamps.w` = half the lamp separation (metres). Produces WGSL `fn headbeam(wp: vec3<f32>) -> f32` (same maths as `carlights::beam`).

- [ ] **Step 1:** In `gpu.rs` set `lamps: [.., .., .., (wv.lamps.head[0] - wv.lamps.head[1]).len() * 0.5]`. 
- [ ] **Step 2:** Add to `raster.wgsl` before `shade_with`:
```wgsl
fn beam_one(lamp: vec3<f32>, fwd: vec3<f32>, right: vec3<f32>, p: vec3<f32>) -> f32 {
    let d = p - lamp;
    let dist = length(d);
    if (dist < 0.05) { return 0.0; }
    let l = d / dist;
    let z = dot(l, fwd);
    if (z <= 0.05) { return 0.0; }
    let up = cross(right, fwd);
    let h = atan(dot(l, right) / z);
    let v = atan(dot(l, up) / z);
    let cut = 0.07 * smoothstep(0.0, 0.25, h);
    let gv = 1.0 - smoothstep(cut, cut + 0.02, v);
    let gh = 1.0 - smoothstep(0.35, 0.65, abs(h));
    let hot = 0.6 + 0.4 * exp(-(h / 0.2) * (h / 0.2));
    let att = 1.0 / (1.0 + (dist / 18.0) * (dist / 18.0));
    return gv * gh * hot * att * clamp(1.0 - dist / 90.0, 0.0, 1.0);
}
// the low beam of both headlights at a world point (0..1); the lamps sit half `lamps.w` either side of `hl_pos`
fn headbeam(wp: vec3<f32>) -> f32 {
    if (u.hl_pos.w <= 0.0) { return 0.0; }
    let f = normalize(u.hl_dir.xyz);
    let r = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), f));
    let s = r * u.lamps.w;
    return 0.5 * (beam_one(u.hl_pos.xyz - s, f, r, wp) + beam_one(u.hl_pos.xyz + s, f, r, wp));
}
```
  (`cross(up, f)` is the car's right when `f` points ahead: check the sign against `carlights::beam_gain`'s `right` = `v.right()`; if the cut-off rises on the wrong side in the host test mirror, swap the order.)
- [ ] **Step 3:** Replace the cone code at the four sites. `shade_with` (and the identical block near line 573 of the sky/dust haze):
  `c = c + albedo * vec3<f32>(1.0, 0.93, 0.80) * (11.0 * headbeam(wp) * (0.25 + 0.75 * max(dot(n, -l), 0.0)));` where `l` is `normalize(wp - u.hl_pos.xyz)`; in `dust_pt`: `col = col + i.c.rgb * vec3<f32>(1.0, 0.93, 0.80) * (2.4 * headbeam(wp));`; in the haze block likewise `(2.4 * headbeam(wp))`. Remove the now unused `hl_dir.w` cone constant use (keep the uniform field).
- [ ] **Step 4:** `~/bin/t shaders_validate` → PASS; `~/bin/t` (all) → PASS.
- [ ] **Step 5: commit** `feat: two-lamp low beam with a cut-off lights the ground, haze and dust`

### Task 5: Glow around lit lamps (also the rally car)

**Files:** Modify `rust/src/raster/gpu.rs` (the `own_lamps` block).

**Interfaces:** Consumes `carlights::glow_factor`, `Lamps.pos/head/tail/rev`.

- [ ] **Step 1: failing test** — extract the decision into a host function in `carlights.rs` so it is testable: `pub fn glows(l: &CarLights, lamps: &Lamps, cam: V3, body_fwd: V3) -> Vec<(V3, [f32; 3], f32)>` returning (position, colour, strength) for every lit lamp (headlight, position, tail/brake, reverse) with strength from `glow_factor` (front lamps' axis = `body_fwd`, rear lamps' axis = `-body_fwd`).
```rust
#[test] fn lit_lamps_glow_only_towards_the_viewer() {
    let v = settled(VehicleSpec::rally()); let lm = lamps(&v);
    let on = CarLights { head: true, brake: true, reverse: false };
    let front_cam = v.pos + v.forward() * 20.0 + v3(0.0, 1.0, 0.0);
    let g = glows(&on, &lm, front_cam, v.forward());
    let front: Vec<_> = g.iter().filter(|(p, _, _)| (*p - v.pos).dot(v.forward()) > 0.0).collect();
    let rear: Vec<_> = g.iter().filter(|(p, _, _)| (*p - v.pos).dot(v.forward()) < 0.0).collect();
    assert!(front.iter().all(|g| g.2 > 0.5) && !front.is_empty());
    assert!(rear.iter().all(|g| g.2 == 0.0), "tail glow not seen from the front");
    assert!(glows(&CarLights::default(), &lm, front_cam, v.forward()).iter().all(|g| g.2 == 0.0), "lamps off: no glow");
}
```
- [ ] **Step 2:** `~/bin/t lit_lamps` → FAIL. **Step 3:** implement `glows`: head+pos lamps (cream `[1.0,0.95,0.8]`, pos dimmer ×0.4) when `head`; tail red `[1.0,0.05,0.03]` ×(brake ? 1 : 0.35 when head); rev white when `reverse`; strength = `glow_factor(axis, (cam - p).norm())`. In `gpu.rs` replace the `own_lamps` skip: for the rally car push, for each `(p, col, s)` with `s > 0.02`, `glow(p, v3(0.16, 0.16, 0.16) * (0.6 + 1.8 * s), col scaled by s)` (a screen-facing box is fine at this size; the boxy cars keep their old three loops, the new path serves the rally car).
- [ ] **Step 4:** `~/bin/t carlights` and `~/bin/t` (all) → PASS. **Step 5: commit** `feat: view-dependent glow around lit lamps`

### Task 6: Docs, notes, full run

**Files:** `docs/vehicles.md`, `docs/game.md` (EN + RU lamp paragraphs), `CHANGELOG.md`, `CHANGELOG.ru.md` (a `2.29.0` entry), `~/projects/releases/rallynvx-2.29.0.md`.

- [ ] **Step 1:** Update `docs/vehicles.md` (lamp classes: main lens, corner position piece = outboard cream vertices of the front unit within 6 cm of the outermost x, material 8; no corner piece = position lamp at the head point), `docs/game.md` (beam, glow), changelog EN + RU, release notes EN + RU (bump `versionName` only at release time).
- [ ] **Step 2:** `~/bin/t` → all pass; `cd ~/projects/RallyNVX && ./gradlew --no-daemon testDebugUnitTest assembleRelease -q` builds (benign "daemon terminated" → retry once).
- [ ] **Step 3: commit** `docs: lamp optics`. Then ask the user for the phone (night check: `tod 0.95 lights 0`, front/rear view) before any device run, and for the go before a release.
