# Wheels Implementation Plan (version 2.74.0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every wheel spins on its own (an angular speed, an inertia), the tyre's longitudinal force follows the slip ratio, the brakes and the handbrake are torques on the wheels, and ABS / traction control act on real slip, while the car's launch, braking and cornering stay within a few percent of version 2.72.0 with the aids on.

**Architecture:** A pure module `game/wheel.rs` (slip curve + one semi-implicit step of a wheel's spin) is called by `Vehicle::substep` for each wheel instead of the old "force = drive/4, clamp to the friction circle" shortcut. The vehicle receives a drive **torque per wheel** (`drive_torque`) instead of one force; the engine and the clutch are solved **implicitly together with the wheels' inertia in the vehicle's substeps** (so a spinning wheel cannot make the clutch chatter) and the drivetrain cuts the engine on spin (`tc_slip`). Lateral force keeps its model and takes what the longitudinal force leaves of the friction circle.

**Tech Stack:** Rust (host tests via `~/bin/t`), Kotlin untouched.

**Spec:** `docs/superpowers/specs/2026-10-21-driveline-design.md` (section 3.2). Needs Plan 1 (2.73.0) merged first.

## Global Constraints

- Branch `feat/wheels`; release `~/bin/rt-release 2.74.0`; versionName "2.74.0", versionCode 104; EN + RU changelog; release notes `~/projects/releases/rallynvx-2.74.0.md`.
- Never launch the game on the phone; never touch the real `profile.json`; report what the tests cover and that the feel is unseen on a device.
- 30 fps: no allocation in the vehicle's substep; four wheels x four substeps per step.
- Regression anchors (measured on 2.72.0 with the aids on, `anchors_tests.rs`): 0 to 22 m/s in 4.45 s, 0 to 27.8 m/s in 6.50 s, a 33 m/s stop in 56.81 m, a 20 m/s stop on asphalt 19.92 m, gravel 27.06 m, sand 33.47 m, the cornering acceleration at 20 m/s asphalt 11.14, gravel 6.96, sand 5.54 (m/s2). Tolerances: launch and braking +-6 % (braking on loose surfaces +-10 %), cornering +-8 %. If Plan 1 moved the launch times, re-measure with Task 1's `print_anchors` on the merged main and update the constants in the same commit.
- The handbrake is a plain rear brake that locks the rear wheels (user decision); `handling.handbrake_grip` stays in the JSON for compatibility and has no effect from 2.74.0.
- Brake bias is a constant for now (`wheel::BRAKE_BIAS = 0.55` front, a ruling: closer to equal than the spec's 0.62 so that the stopping distance stays within the anchors; the menu setting comes in 2.76.0).
- Comments in the codebase's style (short English sentences).

## Review Focus

1. A parked car (handbrake, or the brake on a slope) must not creep, jitter or oscillate: wheels locked at a standstill are a constraint, not a spring (Task 3 test `a_locked_car_on_a_slope_stays_put_without_jitter`).
2. Standing start forward and in reverse: the slip ratio at |v| near 0 stays finite and the car pulls away smoothly (Task 3 `standing_starts_are_smooth_forward_and_reverse`).
3. A hitch (dt up to 0.05 s) or a car dropped with spinning wheels: no NaN, no explosion (Task 3 `landing_with_spinning_wheels_is_stable`, existing `big_steps_stay_finite`).
4. Very low grip (water, wet sand, ice-like): traction almost zero but stable, the engine does not run away (Task 4 `a_car_on_water_does_not_run_away`).
5. Reverse gear with wheelspin and TC: the sign conventions hold (Task 4 `reverse_traction_control_cuts_spin_too`).

---

## File Structure

- Create `rust/src/game/wheel.rs`, `rust/src/game/wheel_tests.rs`: pure wheel physics.
- Create `rust/src/game/anchors_tests.rs`: regression anchors of the handling (test-only module, registered in `game/mod.rs` under `#[cfg(test)]`).
- Modify `rust/src/game/vehicle.rs`: `Vehicle` state (`w`, `slip`, `spin_ready`, `drive_torque`), the tyre block of `substep`, `spin_free`, `WheelInfo.slip`.
- Modify `rust/src/game/drivetrain.rs`: `DriveIn.tc_slip`, `DriveOpts.tc`, `DriveOut.wheel_torque`, `begin` / `sub_coupled` / `finish`, the TC cut.
- Modify `rust/src/game/world.rs`: feed the vehicle and the drivetrain.
- Modify `rust/src/game/{vehicle_tests,drivetrain_tests,assists_tests}.rs`, docs, changelogs.

---

### Task 1: Pin the 2.72.0 handling as regression anchors

**Files:**
- Create: `rust/src/game/anchors_tests.rs`
- Modify: `rust/src/game/mod.rs`

**Interfaces:**
- Consumes: `World::test_track(VehicleSpec)`, `World::step(&Input, dt)`, `Vehicle`, `Terrain::flat()`.
- Produces: tests `anchors_launch`, `anchors_braking`, `anchors_cornering` and the ignored `print_anchors` (re-measure helper).

- [ ] **Step 1: Write the tests (they must pass on the current code)**

Add `#[cfg(test)] mod anchors_tests;` to `rust/src/game/mod.rs` (next to the other test modules). Create `rust/src/game/anchors_tests.rs`:

```rust
//! The handling of the car with the aids on, pinned so the wheel model of 2.74.0 can be checked against 2.72.0.
use crate::game::{input::Input, math::v3, surface::Surface, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}, world::World};

fn time_to(speed: f32) -> f32 {
    let mut w = World::test_track(VehicleSpec::default_car());
    for k in 0..60 * 30 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); if w.vehicle.vel.len() >= speed { return k as f32 / 60.0; } }
    99.0
}
fn stop(speed: f32) -> f32 {
    let mut w = World::test_track(VehicleSpec::default_car());
    for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); }
    let f = w.vehicle.forward(); w.vehicle.vel = f * speed;
    let p0 = w.vehicle.pos;
    for _ in 0..60 * 15 { w.step(&Input { brake: true, brake_amt: 1.0, ..Default::default() }, 1.0 / 60.0); if w.vehicle.vel.len() < 0.2 { break; } }
    (w.vehicle.pos - p0).len()
}
fn braking(surface: Surface) -> f32 {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0); v.force_surface = Some(surface);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0; let z0 = v.pos.z;
    for _ in 0..600 { v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 120.0); if v.speed() < 0.3 { break; } }
    v.pos.z - z0
}
fn cornering(surface: Surface) -> f32 {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0); v.force_surface = Some(surface);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0; let mut best = 0.0f32;
    for i in 0..240 { v.step(&t, &[], &Input { steer: 0.35, throttle: 0.3, ..Default::default() }, 1.0 / 120.0); if i > 120 { best = best.max(v.vel.len() * v.omega.y.abs()); } }
    best
}
fn near(got: f32, want: f32, tol: f32, what: &str) { assert!((got / want - 1.0).abs() <= tol, "{what}: {got} against the anchor {want} (+-{:.0} %)", tol * 100.0); }

#[test] #[ignore] fn print_anchors() {
    eprintln!("ANCHOR t22 {:.3} t27.8 {:.3} stop33 {:.2}", time_to(22.0), time_to(27.8), stop(33.0));
    for s in [Surface::Asphalt, Surface::Gravel, Surface::Sand] { eprintln!("ANCHOR {:?} brake20 {:.2} corner {:.2}", s, braking(s), cornering(s)); }
}
#[test] fn anchors_launch() {
    near(time_to(22.0), 4.45, 0.06, "0 to 22 m/s"); near(time_to(27.8), 6.50, 0.06, "0 to 27.8 m/s");
}
#[test] fn anchors_braking() {
    near(stop(33.0), 56.81, 0.06, "stop from 33 m/s");
    near(braking(Surface::Asphalt), 19.92, 0.06, "asphalt 20 m/s"); near(braking(Surface::Gravel), 27.06, 0.10, "gravel 20 m/s"); near(braking(Surface::Sand), 33.47, 0.10, "sand 20 m/s");
}
#[test] fn anchors_cornering() {
    near(cornering(Surface::Asphalt), 11.14, 0.08, "asphalt"); near(cornering(Surface::Gravel), 6.96, 0.08, "gravel"); near(cornering(Surface::Sand), 5.54, 0.08, "sand");
}
```

- [ ] **Step 2: Run them**

Run: `~/bin/t anchors`
Expected: `test result: ok. 3 passed` on the 2.72.0 numbers. If `anchors_launch` fails because Plan 1 changed the shifts, run `~/bin/t print_anchors -- --ignored --nocapture >/dev/null; grep ANCHOR $TMPDIR/t.log`, put the new launch and stop numbers into the test and into the Global Constraints list, and note it in the commit message.

- [ ] **Step 3: Commit**

```bash
cd ~/projects/RallyNVX && git checkout -b feat/wheels
git add rust/src/game/anchors_tests.rs rust/src/game/mod.rs
git commit -m "test: pin the handling (launch, braking, cornering) before the wheel model

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The wheel as a pure module

**Files:**
- Create: `rust/src/game/wheel.rs`, `rust/src/game/wheel_tests.rs`
- Modify: `rust/src/game/mod.rs` (`pub mod wheel;` and `#[cfg(test)]` is inside the file)

**Interfaces:**
- Consumes: `super::surface::Surface`.
- Produces:
  - `pub const INERTIA: f32 = 1.3`, `pub const BRAKE_BIAS: f32 = 0.55`
  - `pub fn slip_params(Surface) -> (f32, f32)` = (slip at the peak, share of the peak kept when sliding)
  - `pub fn curve(k, kp, slide) -> f32` (-1..1 of the peak) and `pub fn slope(k, kp, slide) -> f32` (positive, damped floor)
  - `pub struct In { w, vf, radius, fmax, kp, slide, drive, brake: f32, abs: bool, h, inertia: f32 }`
  - `pub struct Out { w, fx, slip, brake_used: f32 }` and `pub fn step(&In) -> Out`

- [ ] **Step 1: Write the failing tests**

Add `pub mod wheel;` to `rust/src/game/mod.rs`. Create `rust/src/game/wheel_tests.rs`:

```rust
use super::*;

const R: f32 = 0.326;
const H: f32 = 1.0 / 240.0;
fn base(w: f32, vf: f32) -> In { In { w, vf, radius: R, fmax: 3600.0, kp: 0.07, slide: 0.85, drive: 0.0, brake: 0.0, abs: false, h: H, inertia: INERTIA } }
fn run(mut i: In, secs: f32) -> Out {
    let mut o = step(&i);
    for _ in 1..(secs / H) as usize { i.w = o.w; o = step(&i); }
    o
}

#[test] fn the_curve_rises_to_the_peak_falls_to_the_slide_and_is_odd() {
    assert_eq!(curve(0.0, 0.07, 0.85), 0.0);
    assert!((curve(0.07, 0.07, 0.85) - 1.0).abs() < 1e-5, "the peak at the peak slip");
    assert!(curve(0.03, 0.07, 0.85) > 0.0 && curve(0.03, 0.07, 0.85) < 1.0);
    assert!((curve(0.9, 0.07, 0.85) - 0.85).abs() < 0.01, "sliding keeps the share");
    assert!((curve(-0.05, 0.07, 0.85) + curve(0.05, 0.07, 0.85)).abs() < 1e-6);
    assert!(slope(0.5, 0.07, 0.85) > 0.0 && slope(0.07, 0.07, 0.85) > 0.0, "the slope never vanishes (it damps the implicit step)");
}
#[test] fn a_free_wheel_keeps_rolling_with_the_ground() {
    let o = run(base(10.0 / R, 10.0), 1.0);
    assert!((o.w - 10.0 / R).abs() < 0.05 && o.fx.abs() < 20.0, "w {} fx {}", o.w, o.fx);
}
#[test] fn a_drive_torque_settles_at_a_slip_below_the_peak_where_the_force_balances_it() {
    let mut i = base(10.0 / R, 10.0); i.drive = 800.0;
    let o = run(i, 1.0);
    assert!((R * o.fx - 800.0).abs() < 15.0, "the tyre carries the torque: {} N m", R * o.fx);
    assert!(o.slip > 0.0 && o.slip < 0.07, "slip {}", o.slip);
}
#[test] fn too_much_torque_spins_the_wheel_past_the_peak() {
    let mut i = base(10.0 / R, 10.0); i.drive = 2500.0;
    let o = run(i, 0.5);
    assert!(o.slip > 0.07 * 1.5, "spinning: {}", o.slip);
    assert!(o.fx > 0.8 * 3600.0 && o.fx < 3600.0, "the force falls to the slide: {}", o.fx);
}
#[test] fn the_brake_without_abs_locks_the_wheel_and_it_slides_at_the_reduced_force() {
    let mut i = base(20.0 / R, 20.0); i.brake = 3000.0;
    let o = run(i, 0.4);
    assert!(o.w.abs() < 0.01, "locked: {}", o.w);
    assert!((o.fx + 0.85 * 3600.0).abs() < 120.0, "sliding force {}", o.fx);
    assert!(o.brake_used > 0.0 || o.w == 0.0);
}
#[test] fn abs_holds_the_slip_near_the_peak_and_the_force_near_the_maximum() {
    let mut i = base(20.0 / R, 20.0); i.brake = 3000.0; i.abs = true;
    let o = run(i, 0.5);
    assert!(o.slip < -0.04 && o.slip > -0.12, "slip {}", o.slip);
    assert!(-o.fx > 0.93 * 3600.0, "force {}", o.fx);
}
#[test] fn a_locked_wheel_at_a_standstill_stays_put_without_chatter() {
    let mut i = base(0.0, 0.0); i.brake = 500.0;
    let mut o = step(&i);
    for _ in 0..2000 { i.w = o.w; o = step(&i); assert!(o.w.abs() < 1e-4 && o.fx.abs() < 1.0, "{o:?}"); }
}
#[test] fn an_airborne_wheel_spins_up_freely_and_the_brake_stops_it() {
    let mut i = base(0.0, 0.0); i.fmax = 0.0; i.drive = 100.0;
    let o = run(i, 1.0);
    assert!((o.w - 100.0 / INERTIA).abs() < 1.0, "w {}", o.w);
    let mut b = base(o.w, 0.0); b.fmax = 0.0; b.brake = 400.0;
    assert!(run(b, 1.0).w.abs() < 1e-3);
}
#[test] fn reverse_is_the_mirror_image() {
    let mut f = base(10.0 / R, 10.0); f.drive = 800.0;
    let mut r = base(-10.0 / R, -10.0); r.drive = -800.0;
    let (a, b) = (run(f, 1.0), run(r, 1.0));
    assert!((a.w + b.w).abs() < 1e-3 && (a.fx + b.fx).abs() < 1.0 && (a.slip + b.slip).abs() < 1e-4);
}
#[test] fn abuse_stays_finite_and_bounded() {
    let mut seed = 9u32; let mut r = || { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); (seed >> 8) as f32 / 16_777_216.0 };
    for _ in 0..20000 {
        let i = In { w: (r() - 0.5) * 600.0, vf: (r() - 0.5) * 120.0, radius: 0.3, fmax: r() * 8000.0, kp: 0.05 + r() * 0.12, slide: 0.6 + r() * 0.35, drive: (r() - 0.5) * 6000.0, brake: r() * 6000.0, abs: r() > 0.5, h: 1.0 / 240.0, inertia: 1.3 };
        let o = step(&i);
        assert!(o.w.is_finite() && o.fx.is_finite() && o.slip.is_finite() && o.w.abs() < 3000.0 && o.fx.abs() <= i.fmax + 1.0, "{i:?} -> {o:?}");
    }
}
```

- [ ] **Step 2: Run to see the failure**

Run: `~/bin/t wheel`
Expected: compile error (`In`, `step`, ... do not exist).

- [ ] **Step 3: Write the implementation**

Create `rust/src/game/wheel.rs`:

```rust
//! One wheel on the ground: its spin, the slip curve of the tyre's longitudinal grip, the brake (with ABS) and the drive torque.
//! The tyre is stiff (a small slip already gives a large force), so the wheel's equation is stepped semi-implicitly: the tyre force is linearised about the current slip
//! and its slope (kept positive, so the step is always damped) joins the wheel's inertia.
use super::surface::Surface;

/// Below this ground speed (m/s) the slip ratio's denominator stays at it, so the ratio is finite at a standstill.
const V0: f32 = 1.0;
/// A wheel's inertia with its tyre, rim, disc and the half-shaft's share, kg m2.
pub const INERTIA: f32 = 1.3;
/// The share of the brake force on the front axle.
pub const BRAKE_BIAS: f32 = 0.55;

/// The slip ratio at the peak of the grip and the share of the peak that is kept when the tyre slides, per surface: tarmac peaks early and holds well, loose ground peaks later.
pub fn slip_params(s: Surface) -> (f32, f32) {
    match s {
        Surface::Asphalt => (0.07, 0.85), Surface::Gravel => (0.12, 0.88), Surface::Dirt => (0.13, 0.85), Surface::Grass => (0.14, 0.80),
        Surface::Sand => (0.16, 0.80), Surface::Rock => (0.10, 0.85), Surface::Water => (0.10, 0.60),
    }
}

/// The force in shares of the peak (odd in the slip): up to the peak a parabola (slope 2/kp at zero), then it falls to `slide`.
pub fn curve(k: f32, kp: f32, slide: f32) -> f32 {
    let a = (k.abs() / kp.max(1e-3)).min(30.0);
    let g = if a <= 1.0 { a * (2.0 - a) } else { slide + (1.0 - slide) * (-(a - 1.0) * 1.2).exp() };
    if k < 0.0 { -g } else { g }
}

/// The magnitude of the curve's slope per unit of slip, never below 0.4/kp (so the implicit step stays damped past the peak).
pub fn slope(k: f32, kp: f32, slide: f32) -> f32 {
    let a = (k.abs() / kp.max(1e-3)).min(30.0);
    let d = if a <= 1.0 { 2.0 * (1.0 - a) } else { 1.2 * (1.0 - slide) * (-(a - 1.0) * 1.2).exp() };
    d.max(0.4) / kp.max(1e-3)
}

/// One wheel's step: its spin `w` (rad/s, positive rolls forward), the ground speed of the contact along the wheel's heading `vf`, the grip it has (`fmax`, N; 0 in the air),
/// the drive torque from the driveline (positive pushes the car forward), the brake torque available (>= 0), ABS, the step `h` and the inertia.
#[derive(Clone, Copy, Debug)]
pub struct In { pub w: f32, pub vf: f32, pub radius: f32, pub fmax: f32, pub kp: f32, pub slide: f32, pub drive: f32, pub brake: f32, pub abs: bool, pub h: f32, pub inertia: f32 }

/// The new spin, the force on the car along the heading (N), the slip ratio and the brake torque that really acted.
#[derive(Clone, Copy, Debug)]
pub struct Out { pub w: f32, pub fx: f32, pub slip: f32, pub brake_used: f32 }

pub fn step(i: &In) -> Out {
    let r = i.radius.max(0.05);
    let fmax = i.fmax.max(0.0);
    let den = i.vf.abs().max(V0);
    let k0 = (i.w * r - i.vf) / den;
    let f0 = fmax * curve(k0, i.kp, i.slide);
    let stiff = r * r * fmax * slope(k0, i.kp, i.slide) / den;
    let mut w = i.w + i.h * (i.drive - r * f0) / (i.inertia + i.h * stiff);
    // the brake only ever slows the wheel towards a stop; ABS will not take the slip beyond the band that holds the peak of the grip
    let mut used = 0.0;
    if i.brake > 0.0 {
        let dw = i.h * i.brake / i.inertia;
        let band = 1.2 * i.kp * den;
        if w > 0.0 {
            let floor = if i.abs && fmax > 0.0 && i.vf > 0.0 { ((i.vf - band) / r).max(0.0).min(w) } else { 0.0 };
            let nw = (w - dw).max(floor); used = (w - nw) * i.inertia / i.h; w = nw;
        } else if w < 0.0 {
            let ceil = if i.abs && fmax > 0.0 && i.vf < 0.0 { ((i.vf + band) / r).min(0.0).max(w) } else { 0.0 };
            let nw = (w + dw).min(ceil); used = (nw - w) * i.inertia / i.h; w = nw;
        }
    }
    let k1 = (w * r - i.vf) / den;
    Out { w, fx: fmax * curve(k1, i.kp, i.slide), slip: k1, brake_used: used }
}

#[cfg(test)]
#[path = "wheel_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run to see them pass**

Run: `~/bin/t wheel`
Expected: `test result: ok. 9 passed`. If `a_drive_torque_settles...` is off by more than 15 N m, the step has not converged in 1 s: print `o` and increase the run time to 2 s before touching the model. If `abs_holds...` shows the slip below -0.12, check the `band` (1.2 kp den = 0.084): ABS holds `(vf - band) / r`, i.e. a slip of -0.084 at vf = 20 (den 20: band = 1.2 x 0.07 x 20 = 1.68 m/s, slip = -0.084): that is inside the asserted range.

- [ ] **Step 5: Commit**

```bash
git add rust/src/game/wheel.rs rust/src/game/wheel_tests.rs rust/src/game/mod.rs
git commit -m "feat: wheel physics as a pure module (slip curve, semi-implicit spin, brake with ABS)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The vehicle's wheels spin

**Files:**
- Modify: `rust/src/game/vehicle.rs` (`WheelInfo`, `Vehicle`, `new`, resets, `substep`)
- Modify: `rust/src/game/world.rs:314` (the drive feed, temporary equivalent)
- Modify: `rust/src/game/vehicle_tests.rs` (new tests; adjust only the tests that the numbers demand)

**Interfaces:**
- Consumes: Task 2's `wheel::{In, step, slip_params, INERTIA, BRAKE_BIAS}`.
- Produces: `Vehicle.w: [f32; 4]` (spin, rad/s), `Vehicle.slip: [f32; 4]` (slip ratio over the peak slip: 1 = at the peak; negative when braking), `Vehicle.drive_torque: Option<[f32; 4]>` (replaces `drive_force`), `WheelInfo.slip: f32`.

- [ ] **Step 1: Write the failing tests**

Append to `vehicle_tests.rs` (the file's helpers `flat_car(y)` and `run(&mut v, &t, input, secs)` exist):

```rust

#[test] fn a_coasting_car_keeps_its_wheels_rolling_with_the_ground() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input::default(), 1.0);
    v.vel = v.forward() * 15.0;
    run(&mut v, &t, Input::default(), 1.0);
    for k in 0..4 { assert!((v.w[k] * v.spec.radius - v.speed()).abs() < 0.8, "wheel {k}: {} m/s against the car's {}", v.w[k] * v.spec.radius, v.speed()); }
}
#[test] fn without_abs_the_brake_locks_the_wheels_and_with_it_they_keep_turning() {
    let go = |abs: bool| { let (mut v, t) = flat_car(1.1); v.abs_on = abs; run(&mut v, &t, Input::default(), 1.0); v.vel = v.forward() * 25.0; run(&mut v, &t, Input::default(), 0.3);
        run(&mut v, &t, Input { brake: true, ..Default::default() }, 0.6); (v.w, v.slip, v.speed()) };
    let (w_off, _, _) = go(false); let (w_on, slip_on, speed_on) = go(true);
    assert!(w_off.iter().all(|w| w.abs() < 1.0), "locked: {w_off:?}");
    assert!(w_on.iter().all(|w| *w > 20.0) && speed_on > 5.0, "turning: {w_on:?} at {speed_on} m/s");
    assert!(slip_on.iter().all(|s| *s < 0.0 && *s > -2.5), "ABS holds the slip near the peak: {slip_on:?}");
}
#[test] fn the_handbrake_locks_the_rear_wheels_and_the_front_ones_keep_turning() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input::default(), 1.0); v.vel = v.forward() * 12.0; run(&mut v, &t, Input::default(), 0.3);
    run(&mut v, &t, Input { handbrake: true, ..Default::default() }, 0.6);
    assert!(v.w[2].abs() < 1.0 && v.w[3].abs() < 1.0, "rear locked: {:?}", v.w);
    assert!(v.w[0] > 5.0 && v.w[1] > 5.0, "front free: {:?}", v.w);
}
#[test] fn a_locked_car_on_a_slope_stays_put_without_jitter() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { handbrake: true, brake: true, ..Default::default() }, 1.0);
    let p = v.pos; let mut max_speed = 0.0f32;
    for _ in 0..600 { v.step(&t, &[], &Input { handbrake: true, brake: true, ..Default::default() }, DT); max_speed = max_speed.max(v.vel.len()); }
    assert!((v.pos - p).len() < 0.02 && max_speed < 0.05, "moved {} m, peak speed {}", (v.pos - p).len(), max_speed);
}
#[test] fn standing_starts_are_smooth_forward_and_reverse() {
    for sign in [1.0f32, -1.0] {
        let (mut v, t) = flat_car(1.1);
        run(&mut v, &t, Input::default(), 1.0);
        let mut last = 0.0f32; let mut worst_jump = 0.0f32;
        for _ in 0..180 {
            v.drive_torque = Some([sign * 500.0; 4]);
            v.step(&t, &[], &Input { throttle: sign, ..Default::default() }, DT);
            let s = v.speed(); worst_jump = worst_jump.max((s - last).abs()); last = s;
            assert!(v.w.iter().all(|w| w.is_finite() && w.abs() < 400.0), "{:?}", v.w);
        }
        assert!(sign * v.speed() > 3.0 && worst_jump < 0.5, "sign {sign}: {} m/s, the largest jump in one step {worst_jump}", v.speed());
    }
}
#[test] fn landing_with_spinning_wheels_is_stable() {
    let (mut v, t) = flat_car(3.0);
    v.drive_torque = Some([900.0; 4]);
    for _ in 0..90 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, DT); }
    assert!(v.pos.y.is_finite() && v.vel.len() < 60.0 && v.w.iter().all(|w| w.is_finite() && w.abs() < 800.0), "{:?} {:?}", v.vel, v.w);
}
#[test] fn the_wheel_slip_shows_in_the_info_when_driving_hard_on_gravel() {
    let (mut v, t) = flat_car(1.1); v.force_surface = Some(crate::game::surface::Surface::Gravel);
    run(&mut v, &t, Input::default(), 1.0);
    v.drive_torque = Some([1400.0; 4]);
    run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 0.4);
    assert!(v.slip.iter().all(|s| *s > 1.0), "spinning on gravel: {:?}", v.slip);
    assert!(v.wheels.iter().all(|w| w.slide > 0.0), "the dust sees it: {:?}", v.wheels.iter().map(|w| w.slide).collect::<Vec<_>>());
}
```

- [ ] **Step 2: Run to see the failure**

Run: `~/bin/t vehicle`
Expected: compile errors (`v.w`, `v.slip`, `v.drive_torque`).

- [ ] **Step 3: Implement in `vehicle.rs`**

1. At the top: `use super::wheel;`.
2. `WheelInfo`: add `pub slip: f32` (last field; the slip ratio) and `slip: 0.0` in `WheelInfo::NONE`.
3. `Vehicle`: replace `pub drive_force: Option<f32>,` (and its doc) with
```rust
    /// The torque the driveline puts on each wheel (N m at the wheel, positive pushes the car forward); `None` falls back to the throttle times the spec's engine force.
    pub drive_torque: Option<[f32; 4]>,
    /// Each wheel's spin (rad/s, positive rolls forward) and its slip ratio in shares of the slip at the peak of the grip (1 = at the peak, negative while braking or reversing).
    pub w: [f32; 4], pub slip: [f32; 4],
    /// The wheels have been set rolling with the ground (a car placed with a speed starts with turning wheels), and the velocity at the end of the last step:
    /// a jump of more than 2 m/s that no force could make in one step (a car placed with a speed, a teleport) sets the wheels rolling with the ground again.
    spin_ready: bool, last_vel: V3,
```
   In `Vehicle::new` replace `drive_force: None,` with `drive_torque: None, w: [0.0; 4], slip: [0.0; 4], spin_ready: false, last_vel: V3::ZERO,`.
   At the very top of `step_with` (after `let inp = inp.sanitized();`) add `if (self.vel - self.last_vel).len() > 2.0 { self.spin_ready = false; }` and at its very end (after the "the body touches the ground" block) add `self.last_vel = self.vel;`. In the reset (the function around line 101 that does `self.vel = V3::ZERO; self.omega = V3::ZERO; ...`) append `self.w = [0.0; 4]; self.slip = [0.0; 4]; self.spin_ready = false;`.
4. Add the method (next to `collide_props`):
```rust
    /// A wheel off the ground: it only turns under its drive torque and its brake.
    fn spin_free(&mut self, w: usize, drive: f32, brake: f32, h: f32) {
        let o = wheel::step(&wheel::In { w: self.w[w], vf: 0.0, radius: self.spec.radius, fmax: 0.0, kp: 0.1, slide: 0.8, drive, brake, abs: false, h, inertia: wheel::INERTIA });
        self.w[w] = o.w; self.slip[w] = 0.0;
    }
```
5. In `substep`'s wheel loop, right after `let front = w < 2;` insert:
```rust
            // the torques at this wheel: the driveline's (with no drivetrain, the throttle's share of the engine force) and the brake's (the handbrake is a plain rear brake)
            let lim_speed = if inp.throttle >= 0.0 { s.max_speed } else { s.max_speed * s.reverse_factor };
            let taper = (1.0 - (fspeed * inp.throttle.signum()).max(0.0) / lim_speed).clamp(0.0, 1.0);
            let drive_t = match self.drive_torque { Some(t) => t[w], None => inp.throttle * s.engine_force / 4.0 * taper * s.radius };
            let kb = if inp.brake_amt > 0.0 { inp.brake_amt.clamp(0.05, 1.0) } else { 1.0 };
            let pedal = if inp.brake { kb } else { 0.0 };
            let part = if front { wheel::BRAKE_BIAS } else { 1.0 - wheel::BRAKE_BIAS };
            let mut brake_t = s.brake_force * s.radius * 0.5 * part * pedal;
            if inp.handbrake && !front { brake_t = brake_t.max(s.brake_force * s.radius * 0.5); }
```
   and change the two early exits `if up.dot(n) < 0.2 { continue; }` and `if compress <= 0.0 { continue; }` to `{ self.spin_free(w, drive_t, brake_t, h); continue; }` (keeping their conditions).
6. Replace everything from the line `let slide = ((self.vel.x.hypot(...` (the handbrake slide factor) down to, but not including, `let tire = rw * fl + fw * fx;` by:
```rust
            // a wheel placed with a speed starts turning with the ground
            if !self.spin_ready { self.w[w] = vf / s.radius; }
            let (kp, keep) = wheel::slip_params(surf);
            let fmax = mu_s * fs;
            let o = wheel::step(&wheel::In { w: self.w[w], vf, radius: s.radius, fmax, kp, slide: keep, drive: drive_t, brake: brake_t, abs: self.abs_on, h, inertia: wheel::INERTIA });
            self.w[w] = o.w; self.slip[w] = o.slip / kp;
            let mut fx = o.fx - vf * roll;
            // the friction circle, the longitudinal force first: what the tyre spends on spinning or locking, its side grip loses
            let mut fl = -vl * wheel_mass * lat_k;
            let fl_dem = fl;
            fl = fl.clamp(-(fmax * fmax - o.fx * o.fx).max(0.0).sqrt(), (fmax * fmax - o.fx * o.fx).max(0.0).sqrt());
            if o.brake_used > 0.0 {
                // the braking power heats the disc (the front brakes do more of the work)
                self.brake_heat[w] = (self.brake_heat[w] + o.brake_used / s.radius * vf.abs() * if front { 1.3 } else { 0.7 } * 4.5e-6 * h).min(1.0);
            }
            let excess = if fmax > 1e-3 { (fl_dem.hypot(o.fx) / fmax - 1.0).clamp(0.0, 1.5).max((o.slip.abs() / kp - 1.0).clamp(0.0, 1.5)) } else { 0.0 };
            self.wheels[w] = WheelInfo { grounded: true, surface: surf, contact, speed: vc.len(), lat_slip: vl.abs(), slide: excess, slip: o.slip };
            let mag = fl.hypot(fx);
            if mag > fmax && mag > 1e-6 { let k = fmax / mag; fl *= k; fx *= k; }
```
   Remove the now unused locals (`lat_mu`, `locked`, the old `lim_speed`/`taper` inside the loop) and, after the loop ends, add `self.spin_ready = true;` (before the aerodynamics block, right after the closing brace of `for w in 0..4`).
7. `world.rs:314`: replace `self.vehicle.drive_force = Some(out.force);` with
```rust
            self.vehicle.drive_torque = Some([out.force * self.vehicle.spec.radius * 0.25; 4]);   // the same force on the four wheels (the diffs come later)
```
8. `grep -rn 'drive_force' rust/src --include=*.rs | grep -v raster/` must show nothing else.

- [ ] **Step 4: Run, then adjust**

Run: `~/bin/t vehicle` then `~/bin/t`.
Expected: the new tests pass. Existing tests that may move: `handbrake_lets_the_rear_slide`, `the_vulpes_spins_its_wheels_on_gravel`, `wheel_info_reports_sliding_but_not_rolling`, `the_handbrake_holds_the_car_on_a_slope...`, `a_locked_rear_wheel_gives_no_drive...`, `brake_heat_grows...`, `braking_distance_orders_the_surfaces`, `without_abs_a_hard_stop_is_longer`, `without_traction_control_the_launch_is_slower`, and the anchors. For each failure print the numbers and decide: (a) the physics is plausibly right and the test's threshold encoded the old shortcut (e.g. the "0.62 spinning grip" factor) -> adjust the threshold to the new number with a comment, or (b) the model is wrong (a wheel jitters, a parked car creeps, the anchors drift beyond tolerance) -> fix the model: first the slip peaks in `slip_params`, then the inertia, then the bias; do not widen the anchors' tolerances. List every adjusted test in the commit message.

- [ ] **Step 5: Commit**

```bash
git add rust/src/game && git commit -m "feat: the vehicle's wheels spin (slip-based traction, torque brakes, handbrake as a rear brake)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The engine and the wheels are solved together; traction control cuts real spin

**Why a refactor:** with the wheels free to spin, an engine stepped once a frame against a frozen wheel speed is unstable (one frame of a spinning wheel moves the engine-side speed by far more than the clutch's stiffness can follow: the clutch torque would chatter between its limits). The clutch is therefore solved **implicitly together with the wheels' inertia, in the vehicle's own substeps** (240 Hz). The old frame-rate `Drivetrain::step` stays as a thin wrapper (the car's speed given), so the gearbox tests and the legacy path keep working.

**Files:**
- Modify: `rust/src/game/drivetrain.rs` (split `step` into `begin`, `engine_now`, `sub_kin`, `sub_coupled`, `finish`; `DriveIn.tc_slip`; `DriveOpts.tc`; `DriveOut.wheel_torque`; struct `Frame`)
- Modify: `rust/src/game/vehicle.rs` (`step_driven`, `tyre_sum`)
- Modify: `rust/src/game/world.rs` (the drivetrain block)
- Modify: `rust/src/game/drivetrain_tests.rs`, `rust/src/game/assists_tests.rs`

**Interfaces:**
- Consumes: Task 3's `Vehicle` (`w`, `slip`, `drive_torque`), `wheel::INERTIA`.
- Produces:
  - `DriveIn.tc_slip: f32` (the largest slip over the peak of a wheel spinning in the driving direction; 0 if none), `DriveOpts.tc: bool` (default true), `DriveOut.wheel_torque: f32` (the torque on the four wheels together, N m, signed).
  - `Drivetrain::begin(&mut self, dt: f32, i: &DriveIn)`, `Drivetrain::sub_coupled(&mut self, h: f32, w_mean: f32, i_wheels: f32, tyre: f32) -> f32` (returns the torque to put on the four wheels together, N m), `Drivetrain::finish(&mut self) -> DriveOut`, and `step(dt, &DriveIn) -> DriveOut` unchanged in meaning (it uses `i.speed` as the wheels' speed).
  - `Vehicle::step_driven(&mut self, t, props, inp, dt, loose, drive: Option<&mut dyn FnMut(f32, [f32; 4], f32) -> [f32; 4]>)`: each substep `drive(h, wheel_spins, tyre_sum)` returns the torque for each wheel; `step_with(..)` calls it with `None`. `Vehicle.tyre_sum: f32` (the torque the tyres put against the wheels' spin in the last substep, N m, summed).

- [ ] **Step 1: Write the failing tests**

In `drivetrain_tests.rs` append (the harness `Car` and `R` exist; `DriveIn` derives `Default`):

```rust

#[test] fn traction_control_cuts_the_engine_when_a_wheel_spins_and_gives_it_back() {
    let go = |tc_on: bool, slip: f32| { let mut d = Drivetrain::new(); d.opts.tc = tc_on; let mut o = d.last;
        for _ in 0..180 { o = d.step(1.0 / 60.0, &DriveIn { throttle: 1.0, speed: 8.0, wheel_r: R, tc_slip: slip, ..Default::default() }); } o.wheel_torque };
    let (calm, spin_tc, spin_free) = (go(true, 0.5), go(true, 2.5), go(false, 2.5));
    assert!(spin_tc < calm * 0.5, "cut: {spin_tc} against {calm}");
    assert!(spin_free > calm * 0.9, "TC off does not cut: {spin_free} against {calm}");
}
#[test] fn the_wheel_torque_is_the_force_times_the_radius() {
    let mut c = Car::new(); let o = c.run(1.0, 2.0);
    assert!((o.wheel_torque - o.force * R).abs() < 1.0, "{} against {}", o.wheel_torque, o.force * R);
}
#[test] fn reverse_traction_control_cuts_spin_too() {
    let run = |slip: f32| { let mut d = Drivetrain::new(); d.reverse_sel = true; let mut o = d.last;
        for _ in 0..180 { o = d.step(1.0 / 60.0, &DriveIn { throttle: -1.0, speed: -3.0, wheel_r: R, tc_slip: slip, ..Default::default() }); } o.wheel_torque };
    let (spin, calm) = (run(2.5), run(0.0));
    assert!(spin < 0.0 && calm < 0.0 && spin.abs() < calm.abs() * 0.5, "{spin} against {calm}");
}
/// Wheels as a plain inertia with a given tyre torque against them, driven by the coupled substeps for `secs`; returns (the wheel speeds' history, the torques).
fn spin_up(d: &mut Drivetrain, i_wheels: f32, tyre_of: impl Fn(f32) -> f32, secs: f32, w0: f32) -> (Vec<f32>, Vec<f32>) {
    let h = 1.0 / 240.0; let (mut w, mut ws, mut ts) = (w0, Vec::new(), Vec::new());
    for k in 0..(secs / h) as usize {
        if k % 4 == 0 { d.begin(4.0 * h, &DriveIn { throttle: 1.0, speed: w * R, wheel_r: R, ..Default::default() }); }
        let t = d.sub_coupled(h, w, i_wheels, tyre_of(w));
        w += h * (t - tyre_of(w)) / i_wheels;
        ws.push(w); ts.push(t);
        if k % 4 == 3 { d.finish(); }
    }
    (ws, ts)
}
#[test] fn a_wheel_that_spins_freely_does_not_make_the_clutch_chatter() {
    // no tyre torque at all (ice): the engine and the wheels accelerate together; the torque must be smooth, not alternate between its limits
    let mut d = Drivetrain::new();
    let (ws, ts) = spin_up(&mut d, 5.2, |_| 0.0, 3.0, 8.0);
    assert!(ws.iter().all(|w| w.is_finite()) && ws.last().unwrap() > &ws[0], "{:?}", &ws[ws.len() - 3..]);
    let jumps = ts.windows(2).skip(240).map(|p| (p[1] - p[0]).abs()).fold(0.0f32, f32::max);
    assert!(jumps < 150.0, "the largest torque jump between two substeps: {jumps} N m");
}
#[test] fn heavy_wheels_make_the_coupled_engine_agree_with_the_kinematic_one() {
    let w = 20.0 / R;                                   // 20 m/s in third: about 4700 rpm
    let mut a = Drivetrain::new(); a.gear = 3; let mut b = a.clone();
    let mut oa = a.last;
    for _ in 0..90 { oa = a.step(1.0 / 60.0, &DriveIn { throttle: 0.6, speed: 20.0, wheel_r: R, ..Default::default() }); }
    for _ in 0..90 { b.begin(1.0 / 60.0, &DriveIn { throttle: 0.6, speed: 20.0, wheel_r: R, ..Default::default() }); for _ in 0..4 { b.sub_coupled(1.0 / 240.0, w, 1e7, 0.0); } b.finish(); }
    assert!((a.rpm() / b.rpm() - 1.0).abs() < 0.03 && oa.gear == b.last.gear, "{} against {} rpm", a.rpm(), b.rpm());
}
```
and in `assists_tests.rs` `mod physics` add:
```rust
    #[test] fn traction_control_helps_on_gravel() {
        let go = |tc: bool| { let mut w = track(); w.vehicle.force_surface = Some(crate::game::surface::Surface::Gravel); w.assists.tc = tc; time_to(&mut w, 22.0) };
        let (on, off) = (go(true), go(false));
        assert!(off > on * 1.02, "gravel launch: TC on {on} s, off {off} s");
    }
    #[test] fn a_car_on_water_does_not_run_away() {
        let mut w = track(); w.vehicle.force_surface = Some(crate::game::surface::Surface::Water);
        for _ in 0..60 * 8 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
        assert!(w.vehicle.vel.len().is_finite() && w.vehicle.vel.len() < 40.0 && w.drivetrain.last.rpm < 8300.0 && w.vehicle.w.iter().all(|x| x.is_finite() && x.abs() < 600.0), "{} m/s {} rpm {:?}", w.vehicle.vel.len(), w.drivetrain.last.rpm, w.vehicle.w);
    }
    #[test] fn a_spinning_launch_does_not_chatter() {
        let mut w = track(); w.vehicle.force_surface = Some(crate::game::surface::Surface::Gravel); w.assists.tc = false;
        let mut t = Vec::new();
        for k in 0..60 * 4 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); if k > 60 { t.push(w.drivetrain.last.wheel_torque); } }
        let mean = t.iter().sum::<f32>() / t.len() as f32;
        let var = t.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / t.len() as f32;
        assert!(var.sqrt() < mean.abs() * 0.35 + 50.0, "wheel torque mean {mean}, std {} N m", var.sqrt());
    }
```

- [ ] **Step 2: Run to see the failure**

Run: `~/bin/t drivetrain`
Expected: compile errors (`tc_slip`, `tc`, `wheel_torque`, `begin`, `sub_coupled`, `finish`).

- [ ] **Step 3: Refactor `drivetrain.rs`**

1. `DriveIn`: add `pub tc_slip: f32,` (doc: the largest slip over the peak of a wheel spinning in the driving direction; the traction control cuts the engine on it). `DriveOpts`: add `pub tc: bool` and `tc: true` in `Default`. `DriveOut`: add `pub wheel_torque: f32` after `force`; in `Drivetrain::new()` the initial `last` gets `wheel_torque: 0.0`.
2. Add the frame struct above `pub struct Drivetrain` and a field `fr: Frame,` (initialised `fr: Frame::default(),` in `new()`):
```rust
/// What one frame fixes for the engine's substeps: the gear as a ratio and a direction, the clutch's capacity, the throttle the engine gets, the spark cut, the traction control's scale,
/// whether the engine is being matched to the new gear, and the sums the substeps leave for `finish`.
#[derive(Clone, Copy, Debug, Default)]
struct Frame { radius: f32, g: f32, rd: f32, cap: f32, thr: f32, cut: bool, tc_k: f32, sync: bool, n: u32, t_c: f32, t_e: f32 }
```
3. Delete the old `pub fn step` (it is rebuilt below) and add these methods to `impl Drivetrain`. `begin` is the old `step`'s code up to the `cut` line with the clutch/capacity rules **as they are after Plan 1** (`seq`, `auto_cap`, `pedal_cap`), plus the traction control and the frame:
```rust
    /// The start of a frame: the gear, the clutch, the throttle and the starter are decided here; the engine's substeps follow (`sub_kin` with the car's speed given, `sub_coupled` against the wheels), and `finish` closes the frame.
    pub fn begin(&mut self, dt: f32, i: &DriveIn) {
        let dt = dt.clamp(0.0, 0.05);
        if self.shift > 0.0 { self.shift = (self.shift - dt).max(0.0); }
        let auto = self.mode == Mode::Auto;
        let seq = !auto && self.box_type == BoxType::Sequential;
        let rpm0 = self.rpm();
        let gas = i.throttle.max(0.0);
        if auto { self.auto_gear(i, gas, rpm0, dt); }
        // the pedal the engine feels: in the reverse of the automatic the negative throttle is the gas
        let pedal = if auto { if self.gear < 0 { if self.reverse_sel { gas.max((-i.throttle).max(0.0)) } else { (-i.throttle).max(0.0) } } else { gas } } else { i.throttle.clamp(0.0, 1.0) };
        // the throttle the engine gets: the pedal, the idle governor holding the idle, and the blip of a down-shift
        let mut thr = pedal;
        if self.running {
            let err = (IDLE_RPM - rpm0) / IDLE_RPM;
            self.idle_i = (self.idle_i + err * dt * 0.6).clamp(0.0, 0.4);
            thr = thr.max((self.idle_i + 0.5 * err).clamp(0.0, 0.4));
        }
        if (auto || seq) && self.shift > 0.0 && !self.shift_up && self.opts.blip { thr = thr.max(0.55); }
        let reverse_dir = if self.gear < 0 { -1.0 } else { 1.0 };
        let g = ratio(self.gear) * self.opts.gearing;
        let cut_shift = self.opts.flat_shift && self.shift > 0.0 && self.shift_up && pedal > if seq { 0.2 } else { 0.6 };
        // the clutch's capacity (N m): the automatic's and the sequential's electronic clutch rise with the revs and close for good on the move, the H-pattern's follows the pedal; none in neutral or in a shift
        let auto_cap = (10.0 + (CLUTCH_MAX - 10.0) * smooth(1300.0, 3000.0, rpm0)).max(CLUTCH_MAX * smooth(2.0, 5.0, i.speed.abs())).min(CLUTCH_MAX);
        let pedal_cap = CLUTCH_MAX * smooth(0.85, 0.30, self.clutch_pedal);
        let cap = if g == 0.0 || self.shift > 0.0 || i.airborne { 0.0 }
            else if auto { auto_cap }
            else if seq { if self.clutch_pedal > 0.05 { pedal_cap.min(auto_cap) } else { auto_cap } }
            else { pedal_cap };
        // the starter when the engine is off: it cranks after a pause (the clutch must be open or the box in neutral in the H-pattern)
        if !self.running {
            self.stalled_for += dt;
            let free = auto || seq || self.gear == 0 || self.clutch_pedal > 0.6;
            if self.starting <= 0.0 && self.stalled_for > 0.8 && free { self.starting = 1.0; }
        }
        let opts = self.opts;
        // the limiter: fuel cut above the limit until the revs fall a little
        if !opts.limiter { self.limiter = false; } else if rpm0 > LIMITER_RPM { self.limiter = true; } else if rpm0 < LIMITER_RPM - 150.0 { self.limiter = false; }
        let cut = self.limiter || cut_shift || !self.running;
        // traction control: a wheel spinning past 1.3 times the slip of the peak cuts the engine's torque, to 15 % by 2.3 times
        let tc_k = if opts.tc && i.tc_slip > 1.3 { 1.0 - 0.85 * ((i.tc_slip - 1.3) / 1.0).clamp(0.0, 1.0) } else { 1.0 };
        let sync = (auto || seq) && self.shift > 0.0 && self.running && self.gear != 0;
        self.fr = Frame { radius: i.wheel_r.max(0.05), g, rd: reverse_dir, cap, thr, cut, tc_k, sync, n: 0, t_c: 0.0, t_e: 0.0 };
    }

    /// The engine's torque for one substep of `h` s (the boost moves on): the map at the revs and the throttle, the boost, the tuning, the starter, the traction control's cut.
    fn engine_now(&mut self, h: f32) -> f32 {
        let (f, opts) = (self.fr, self.opts);
        let rpm = self.omega * 60.0 / std::f32::consts::TAU;
        // the boost builds with a lag when the throttle is open at the revs where the turbine has the exhaust to spin it, and falls fast
        let want = if f.thr > 0.5 && rpm > 2200.0 { 1.0 } else { 0.0 };
        self.boost += (want - self.boost) * (h / if want > self.boost { 0.45 } else { 0.25 }).min(1.0);
        let mut t_e = if f.cut { engine_torque(rpm, 0.0) } else { engine_torque(rpm, f.thr) };
        if !f.cut && t_e > 0.0 { t_e *= (0.62 + 0.38 * self.boost * opts.turbo) * opts.engine; }
        else if !f.cut { t_e *= 1.0 + (opts.engine - 1.0) * 0.3; }   // the engine braking changes a little with the engine
        // no limiter: the power gives out by itself above 7500 rpm and the friction wins at about 9000
        if !opts.limiter && !f.cut && rpm > 7500.0 { t_e = t_e.min(engine_torque(7500.0, f.thr) * (1.0 - (rpm - 7500.0) / 1500.0) * opts.engine).max(-60.0); }
        if !self.running { t_e = -engine_torque(rpm.max(1000.0), 0.0).abs() * (rpm / 1000.0).clamp(0.0, 1.0); }
        if self.starting > 0.0 && rpm < 650.0 { t_e += 140.0; }   // the starter turns it until it fires (about 450 rpm)
        if t_e > 0.0 { t_e *= f.tc_k; }
        t_e
    }

    /// A shift of the automatic or the sequential meets the new gear with matching revs: the engine falls or rises to the wheels' (the clutch-to-clutch change), so the clutch closes without a jerk.
    fn sync_torque(&self, w_in: f32) -> f32 {
        if self.fr.sync { (GRIP_K * 1.5 * (w_in.max(IDLE_RPM * std::f32::consts::TAU / 60.0) - self.omega)).clamp(-220.0, 220.0) } else { 0.0 }
    }

    /// One substep with the wheels' speed given (`w_in`, engine side, rad/s): the wheels are taken as infinitely heavy.
    fn sub_kin(&mut self, h: f32, w_in: f32) {
        let f = self.fr;
        let t_e = self.engine_now(h);
        let t_c = (GRIP_K * (self.omega - w_in)).clamp(-f.cap, f.cap);
        let t_s = self.sync_torque(w_in);
        self.omega = (self.omega + h / INERTIA * (t_e - t_c + t_s)).max(0.0);
        self.fr.n += 1; self.fr.t_c += t_c; self.fr.t_e += t_e;
    }

    /// One substep against the wheels' own inertia: `w_mean` the mean spin of the wheels (rad/s), `i_wheels` their inertia together (kg m2), `tyre` the torque the tyres put against their spin (N m, summed).
    /// The clutch is solved implicitly with the wheels' inertia (the relative speed of engine and driveline has an exact update), so it stays stable however the wheels spin.
    /// Returns the torque to put on the four wheels together (N m, signed).
    pub fn sub_coupled(&mut self, h: f32, w_mean: f32, i_wheels: f32, tyre: f32) -> f32 {
        let f = self.fr;
        let gk = f.g * f.rd;                                   // the wheels' spin times this is the driveline's speed on the engine's side
        let w_in = (w_mean * gk).max(-1e4);
        let t_e = self.engine_now(h);
        let t_s = self.sync_torque(w_in);
        let t_c = if f.cap <= 0.0 || gk == 0.0 { 0.0 } else {
            let iw = i_wheels.max(0.1);
            let a = t_e / INERTIA + gk * tyre / iw;            // what accelerates the engine's lead over the driveline, apart from the clutch
            let b = 1.0 / INERTIA + ETA * gk * gk / iw;        // and how strongly the clutch torque pulls the two together
            let d = (self.omega - w_in + h * a) / (1.0 + h * GRIP_K * b);
            (GRIP_K * d).clamp(-f.cap, f.cap)
        };
        self.omega = (self.omega + h / INERTIA * (t_e - t_c + t_s)).max(0.0);
        self.fr.n += 1; self.fr.t_c += t_c; self.fr.t_e += t_e;
        t_c * gk * ETA
    }

    /// The end of a frame: the stall and the starter, and the numbers the others read.
    pub fn finish(&mut self) -> DriveOut {
        let f = self.fr;
        let n = f.n.max(1) as f32;
        let (t_c, t_e) = (f.t_c / n, f.t_e / n);
        let rpm = self.rpm();
        if self.running && rpm < 280.0 && !(self.starting > 0.0) { self.running = false; self.stalled_for = 0.0; }
        if self.starting > 0.0 && rpm > 450.0 { self.starting = 0.0; self.running = true; self.stalled_for = 0.0; self.idle_i = 0.12; }
        let wheel_torque = t_c * f.g * f.rd * ETA;
        self.last = DriveOut { force: wheel_torque / f.radius, wheel_torque, rpm, gear: self.gear, grip: (f.cap / CLUTCH_MAX).clamp(0.0, 1.0), torque: t_e, throttle: f.thr, boost_bar: self.boost * 0.55 * self.opts.turbo, cut: f.cut, stalled: !self.running };
        self.last
    }

    /// A whole frame with the car's speed given as the wheels' (the gearbox tests and any car without wheels use this).
    pub fn step(&mut self, dt: f32, i: &DriveIn) -> DriveOut {
        let dt = dt.clamp(0.0, 0.05);
        self.begin(dt, i);
        let w_in = (i.speed / i.wheel_r.max(0.05) * self.fr.g * self.fr.rd).max(-1e4);
        for _ in 0..8 { self.sub_kin(dt / 8.0, w_in); }
        self.finish()
    }
```
   `GRIP_K`, `INERTIA`, `ETA`, `CLUTCH_MAX`, `IDLE_RPM`, `LIMITER_RPM` are the file's constants; `BoxType`, `smooth`, `ratio` exist.

- [ ] **Step 4: Vehicle and world**

`vehicle.rs`: rename the body of `step_with` into `step_driven` with the extra parameter and make `step_with` a one-line wrapper:
```rust
    pub fn step_with(&mut self, t: &Terrain, props: &[Prop], inp: &Input, dt: f32, loose: &mut LooseMap) { self.step_driven(t, props, inp, dt, loose, None); }

    /// Like `step_with`, with the driveline solved in the substeps: each one calls `drive(h, the wheels' spins, the torque the tyres put against them)` and puts what it returns on the wheels.
    pub fn step_driven(&mut self, t: &Terrain, props: &[Prop], inp: &Input, dt: f32, loose: &mut LooseMap, mut drive: Option<&mut dyn FnMut(f32, [f32; 4], f32) -> [f32; 4]>) {
```
   and in its loop `for _ in 0..SUB { self.substep(t, &inp, h, loose); }` becomes
```rust
        for _ in 0..SUB {
            if let Some(d) = drive.as_mut() { self.drive_torque = Some((*d)(h, self.w, self.tyre_sum)); }
            self.substep(t, &inp, h, loose);
        }
```
   Add the field `pub tyre_sum: f32` (init `tyre_sum: 0.0`, zeroed in the reset like `w`), and in `substep` keep a local `let mut tyre_sum = 0.0;` before the wheel loop, add `tyre_sum += s.radius * o.fx;` right after `self.w[w] = o.w; self.slip[w] = o.slip / kp;`, and `self.tyre_sum = tyre_sum;` right after the loop (next to `self.spin_ready = true;`).
`world.rs`: replace the whole drivetrain block (from the comment `// the drivetrain: the engine against the clutch ...` through the line `self.vehicle.step_with(&self.terrain, &near, &inp, dt, &mut self.loose);`) by:
```rust
        // the drivetrain: the engine against the clutch and the gear, solved together with the wheels in the vehicle's substeps (the autopilot always drives the automatic)
        {
            use super::drivetrain::{DriveIn, Mode};
            let want = if self.manual && self.autopilot.is_none() && !counting { Mode::Manual } else { Mode::Auto };
            self.drivetrain.auto_reverse = self.autopilot.is_some();   // only the autopilot asks for the reverse with the sign of its throttle; the driver picks R by hand
            if self.autopilot.is_some() { self.drivetrain.reverse_sel = false; }
            if want != self.drivetrain.mode { self.drivetrain.set_mode(want); }
            let fwd = self.vehicle.vel.dot(self.vehicle.forward());
            let tc_slip = if self.drivetrain.gear < 0 { -self.vehicle.slip.iter().cloned().fold(0.0, f32::min) } else { self.vehicle.slip.iter().cloned().fold(0.0, f32::max) };
            let dtr = &mut self.drivetrain;
            dtr.begin(dt, &DriveIn { throttle: inp.throttle, speed: fwd, wheel_r: self.vehicle.spec.radius, airborne: self.vehicle.contacts == 0, brake: inp.brake,
                lat_g: self.vehicle.omega.dot(self.vehicle.up()) * fwd / 9.81, slope: self.vehicle.forward().y, tc_slip });
            // the reverse gear of the manual: the pedal asks for backwards (the lamps and the camera follow it)
            if dtr.gear < 0 && (want == Mode::Manual || dtr.reverse_sel) { inp.throttle = -inp.throttle.abs(); } else if want == Mode::Manual { inp.throttle = inp.throttle.abs(); }
            let iw = 4.0 * super::wheel::INERTIA;
            self.vehicle.step_driven(&self.terrain, &near, &inp, dt, &mut self.loose, Some(&mut |h, w, tyre| { let t = dtr.sub_coupled(h, (w[0] + w[1] + w[2] + w[3]) * 0.25, iw, tyre); [t * 0.25; 4] }));
            self.drivetrain.finish();
        }
```
   (keep the `lat_g`/`slope` fields exactly as Plan 1 left them; the closure returns the same torque for the four wheels because the differentials come in the next version). The statement that followed (`self.vehicle.step_with(...)`) is gone: the block now owns the step. The `DriveOpts` literal elsewhere in `world.rs` (`self.drivetrain.opts = DriveOpts { ... }`) gets `tc: a.tc`.

- [ ] **Step 5: Run, then the whole suite and the anchors**

Run: `~/bin/t drivetrain` -> Expected: pass (the kinematic `step` keeps every gearbox test as it was). `~/bin/t` -> investigate failures as in Task 3 Step 4. The anchors must hold; the new chatter and water tests must pass. If `a_wheel_that_spins_freely_does_not_make_the_clutch_chatter` fails, print `ts[240..260]`; a saw-tooth between the cap and 0 means the implicit step is missing a term: re-check `a` and `b` against the derivation in the doc comment (dD/dt = T_e/I_e + k·tyre/I_w - T_c (1/I_e + eta k^2/I_w)) before touching constants.
Also run the autopilot's whole-stage benches: `~/bin/t stage_tests -- --ignored 2>&1 | tail -15`. Expected: the mean speed within 5 % of 47 to 58 km/h and the car stays on the road; if not, report the numbers and retune `rust/autopilot.json` style numbers only after the wheel model is ruled out.

- [ ] **Step 6: Commit**

```bash
git add rust/src/game && git commit -m "feat: the engine and the wheels are solved together in the vehicle's substeps; traction control cuts real wheel spin

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Documentation and release

**Files:**
- Modify: `CHANGELOG.md`, `CHANGELOG.ru.md`, `docs/game.md`, `docs/vehicles.md`, `app/build.gradle.kts`
- Create: `~/projects/releases/rallynvx-2.74.0.md`

- [ ] **Step 1: Changelog EN** (above the 2.73.0 entry in `CHANGELOG.md`):
```
## 2.74.0 - the wheels turn on their own
- Changed: **every wheel is a real spinning body** (an inertia, its own speed). The tyre's traction follows the **slip ratio** (it peaks at about 7 % on tarmac and 12 to 16 % on loose ground, then falls to 80 to 88 % of the peak when it slides); a launch on gravel really spins the wheels, the engine's revs run up with them and **traction control** cuts the engine's torque when a wheel spins past the peak; a braked wheel without **ABS** locks and slides, and with ABS it is held at the slip where the grip is greatest
- Changed: the brakes are torques on the wheels (55 % on the front axle, the discs heat by the real braking power) and the **handbrake** is a plain rear brake: it locks the rear wheels and the slide comes from the locked tyres (the old "less grip with the handbrake" factor is gone; `handling.handbrake_grip` stays in the file and has no effect)
- Changed: the side grip takes what the longitudinal force leaves of the tyre's grip: a spinning or locked wheel steers and holds the side less, as on a real car. With the aids on, the launch, braking and cornering of the car stay within 6 to 10 % of the previous version (pinned by tests)
- Not yet: the differentials (all four wheels get the same torque), the brake bias in the menu
```
RU in `CHANGELOG.ru.md`:
```
## 2.74.0 - колёса вращаются сами по себе
- Изменено: **каждое колесо стало настоящим вращающимся телом** (инерция, своя скорость). Тяга шины зависит от **проскальзывания** (пик около 7 % на асфальте и 12–16 % на рыхлом грунте, дальше падает до 80–88 % пика при скольжении); старт на гравии действительно раскручивает колёса, обороты двигателя вместе с ними улетают вверх, а **противобуксовочная система** режет момент двигателя, когда колесо крутится сверх пика; заторможенное колесо без **ABS** блокируется и скользит, с ABS его держат на проскальзывании, где сцепление наибольшее
- Изменено: тормоза стали моментами на колёсах (55 % на переднюю ось, диски греются от реальной мощности торможения), а **ручник** стал обычным задним тормозом: он блокирует задние колёса, а занос получается от заблокированных шин (прежний коэффициент «меньше сцепления с ручником» убран; `handling.handbrake_grip` остался в файле, на него ничто не влияет)
- Изменено: боковое сцепление получает то, что осталось от сцепления шины после продольной силы: буксующее или заблокированное колесо хуже рулит и держит бок, как на настоящей машине. При включённых помощниках разгон, торможение и повороты остаются в пределах 6–10 % от прежней версии (закреплено тестами)
- Пока нет: дифференциалов (все четыре колеса получают одинаковый момент), баланса тормозов в меню
```
- [ ] **Step 2: Docs**: add a `**Wheels / Колёса.**` paragraph to `docs/game.md` (EN + RU; the equation `dw/dt`, the slip curve numbers per surface from `wheel::slip_params`, the semi-implicit step, the friction circle with the longitudinal force first, ABS band `1.2 kp`, TC cut thresholds 1.3 to 2.3 over the peak, the brake bias constant) and change the two `handbrake_grip` rows of `docs/vehicles.md` to say it has no effect since 2.74.0.
- [ ] **Step 3: Version and notes**: `versionCode = 104`, `versionName = "2.74.0"`; write `~/projects/releases/rallynvx-2.74.0.md` (EN + RU paragraph).
- [ ] **Step 4: Verify**: `~/bin/t 2>&1 | tail -4` and `./gradlew --no-daemon testDebugUnitTest -q 2>&1 | tail -5`: both green; `~/bin/t anchors` green.
- [ ] **Step 5: Merge and release**:
```bash
git add -A && git commit -m "docs: 2.74.0 wheels

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
git checkout main && git merge --no-ff feat/wheels -m "Merge feat/wheels" && ~/bin/rt-release 2.74.0 2>&1 | tail -4
```
Expected: the release URL for v2.74.0. In the report to the user say: tested by host tests only; the feel of the launch, braking, handbrake and gravel traction needs their check on the phone, and the anchors' tolerance (6 to 10 %) is the measure of how much the handling may have changed.
