# Differentials Implementation Plan (version 2.75.0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A real four-wheel driveline: a front, a centre and a rear differential (open, limited-slip with preload and power / coast ramps, viscous), the centre one with a torque split and an **active** lock that follows the throttle, the steering, the brake and the speed, adjustable in the car menu with Tarmac / Gravel / Snow presets.

**Architecture:** Two pure modules: `game/diff.rs` (how a torque is shared between two shafts) and `game/driveline.rs` (the chain centre -> front / rear -> four wheels: the torques of the wheels from the engine's wheel torque and the wheels' speeds, and the speed the engine sees). The world's per-substep closure from Plan 2 calls `Driveline::input_speed` and `Driveline::torques` instead of splitting the torque four ways equally. Four new tuning parts (front lock, rear lock, centre lock, centre split) reuse the existing tuning plumbing end to end.

**Tech Stack:** Rust (host tests via `~/bin/t`), Kotlin (UI, JUnit).

**Spec:** `docs/superpowers/specs/2026-10-21-driveline-design.md` (section 3.3). Needs Plan 2 (2.74.0) merged first.

## Global Constraints

- Branch `feat/differentials`; release `~/bin/rt-release 2.75.0`; versionName "2.75.0", versionCode 105; EN + RU changelog; release notes `~/projects/releases/rallynvx-2.75.0.md`.
- Never launch the game on the phone; never touch the real `profile.json`; report what the tests cover and that the feel is unseen on a device.
- 30 fps: no allocation per substep (the structs are `Copy`); the regression anchors of Plan 2 (`anchors_tests.rs`) must stay green with the default driveline (the centre split 50/50 and no speed difference means no torque is moved).
- The Peugeot's default values below are **approximations** (the sources give a mechanical front and rear LSD, an electronically controlled centre diff and a nominal 50/50 split, no numbers); say so in the docs.
- Wheel order everywhere: FL, FR, RL, RR (indices 0..3; 0 and 1 are the front axle).
- Comments in the codebase's style (short English sentences).

## Review Focus

1. Negative input torque (engine braking, the reverse gear) must split and lock with the coast ramp and keep the sum (Task 1 tests `coast_uses_the_coast_ramp_and_keeps_the_sum`, Task 2 `reverse_and_coast_keep_the_sum`).
2. A wheel in the air (its speed far above the others) or garbage speeds: the lock moves a bounded torque, nothing goes NaN (Task 1 `abuse_keeps_the_sum_and_stays_finite`).
3. The menu can send any tuning value (0, 2, NaN is dropped by `Tuning::set`): the driveline stays finite and the split stays in 30..70 % (Task 2 `extreme_tuning_stays_sane`).
4. The active centre lock must be continuous in its inputs (a flickering throttle or steering must not make the lock jump) (Task 2 `the_active_lock_is_continuous`).
5. Low-grip stability: one rear wheel on ice with an open diff must not oscillate; an LSD must pull away clearly better (Task 2 `an_lsd_pulls_away_where_an_open_diff_spins`).

---

## File Structure

- Create `rust/src/game/diff.rs`, `rust/src/game/diff_tests.rs`: one differential.
- Create `rust/src/game/driveline.rs`, `rust/src/game/driveline_tests.rs`: the chain, the Peugeot's values, the active centre lock, the tuning.
- Modify `rust/src/game/mod.rs`: `pub mod diff; pub mod driveline;`.
- Modify `rust/src/game/assists.rs` (+ `assists_tests.rs`): four tuning parts.
- Modify `rust/src/game/world.rs`: use the driveline in the substep closure.
- Modify Kotlin: `CarMath.kt` (+ `CarMathTest.kt`), `Controls.kt`, `GameActivity.kt` (value text, presets row).
- Docs: `CHANGELOG.md`, `CHANGELOG.ru.md`, `docs/game.md`, `docs/vehicles.md`, release notes, `app/build.gradle.kts`.

---

### Task 1: One differential as pure functions

**Files:**
- Create: `rust/src/game/diff.rs`, `rust/src/game/diff_tests.rs`
- Modify: `rust/src/game/mod.rs`

**Interfaces:**
- Produces:
  - `pub enum Kind { Open, Lsd, Viscous }`
  - `pub struct Diff { pub kind: Kind, pub split: f32, pub preload: f32, pub power: f32, pub coast: f32, pub visc: f32, pub cap: f32 }` (`Copy`)
  - `pub fn moved(d: &Diff, t_in: f32, dw: f32) -> f32`: the torque taken from side A and given to side B (positive when A is faster)
  - `pub fn split(d: &Diff, t_in: f32, w_a: f32, w_b: f32) -> (f32, f32)`: the torques of A and B, they add up to `t_in`
  - `pub fn input_speed(d: &Diff, w_a: f32, w_b: f32) -> f32`: `split x w_a + (1 - split) x w_b` (power-consistent: `t_in x input_speed` equals the sum of `t x w` of the sides when nothing locks)

- [ ] **Step 1: Write the failing tests**

Add `pub mod diff;` to `rust/src/game/mod.rs`. Create `rust/src/game/diff_tests.rs`:

```rust
use super::*;

fn lsd() -> Diff { Diff { kind: Kind::Lsd, split: 0.5, preload: 40.0, power: 0.35, coast: 0.2, visc: 0.0, cap: 900.0 } }
fn open() -> Diff { Diff { kind: Kind::Open, ..lsd() } }
fn visc() -> Diff { Diff { kind: Kind::Viscous, visc: 25.0, cap: 500.0, ..lsd() } }

#[test] fn an_open_diff_splits_by_its_share_whatever_the_speeds() {
    let (a, b) = split(&open(), 1000.0, 200.0, 5.0);
    assert!((a - 500.0).abs() < 1e-3 && (b - 500.0).abs() < 1e-3);
    let c = Diff { split: 0.4, ..open() };
    let (a, b) = split(&c, 1000.0, 0.0, 50.0);
    assert!((a - 400.0).abs() < 1e-3 && (b - 600.0).abs() < 1e-3);
}
#[test] fn an_lsd_moves_torque_from_the_faster_side_to_the_slower_up_to_its_lock() {
    let d = lsd();
    // a small speed difference: proportional (60 N m per rad/s)
    let (a, b) = split(&d, 1000.0, 10.5, 10.0);
    assert!((a - (500.0 - 30.0)).abs() < 1e-3 && (b - (500.0 + 30.0)).abs() < 1e-3, "{a} {b}");
    // a big one: saturates at the preload + power x torque = 40 + 350
    let (a, b) = split(&d, 1000.0, 80.0, 10.0);
    assert!((a - (500.0 - 390.0)).abs() < 1e-3 && (b - (500.0 + 390.0)).abs() < 1e-3, "{a} {b}");
    // the slower side is A: the other way round
    let (a, _) = split(&d, 1000.0, 10.0, 80.0);
    assert!(a > 500.0);
}
#[test] fn the_preload_locks_a_little_even_without_torque() {
    let (a, b) = split(&lsd(), 0.0, 20.0, 0.0);
    assert!((a + 40.0).abs() < 1e-3 && (b - 40.0).abs() < 1e-3, "{a} {b}");
}
#[test] fn coast_uses_the_coast_ramp_and_keeps_the_sum() {
    let d = lsd();
    let (pa, pb) = split(&d, 1000.0, 80.0, 10.0); let (ca, cb) = split(&d, -1000.0, 80.0, 10.0);
    assert!(((pa - 500.0).abs() - (ca + 500.0).abs()).abs() > 100.0, "the two ramps differ: power moves {} coast {}", pa - 500.0, ca + 500.0);
    assert!((ca + cb + 1000.0).abs() < 1e-3 && (pa + pb - 1000.0).abs() < 1e-3);
    assert!((ca + 500.0).abs() < 40.0 + 200.0 + 1e-3, "40 + 0.2 x 1000");
}
#[test] fn a_viscous_diff_moves_torque_in_proportion_to_the_speed_difference_up_to_its_cap() {
    let d = visc();
    let (a, _) = split(&d, 800.0, 12.0, 10.0);
    assert!((a - (400.0 - 50.0)).abs() < 1e-3, "{a}");
    let (a, b) = split(&d, 800.0, 100.0, 0.0);
    assert!((a - (400.0 - 500.0)).abs() < 1e-3 && (b - (400.0 + 500.0)).abs() < 1e-3);
}
#[test] fn the_input_speed_is_power_consistent_for_an_open_diff() {
    let d = Diff { split: 0.4, ..open() };
    let (wa, wb, t) = (30.0f32, 50.0f32, 700.0f32);
    let (ta, tb) = split(&d, t, wa, wb);
    assert!(((ta * wa + tb * wb) - t * input_speed(&d, wa, wb)).abs() < 1e-2);
}
#[test] fn abuse_keeps_the_sum_and_stays_finite() {
    let mut seed = 5u32; let mut r = || { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); (seed >> 8) as f32 / 16_777_216.0 };
    for _ in 0..20000 {
        let k = [Kind::Open, Kind::Lsd, Kind::Viscous][(r() * 3.0) as usize % 3];
        let d = Diff { kind: k, split: 0.2 + r() * 0.6, preload: r() * 200.0, power: r(), coast: r(), visc: r() * 100.0, cap: r() * 2000.0 };
        let (t, wa, wb) = ((r() - 0.5) * 8000.0, (r() - 0.5) * 2000.0, (r() - 0.5) * 2000.0);
        let (a, b) = split(&d, t, wa, wb);
        assert!(a.is_finite() && b.is_finite() && (a + b - t).abs() < 1e-2 * (1.0 + t.abs() * 1e-3), "{d:?} {t} {wa} {wb} -> {a} {b}");
        assert!((moved(&d, t, wa - wb)).abs() <= d.cap.max(d.preload + t.abs() * d.power.max(d.coast)) + 1e-3);
    }
}
```

- [ ] **Step 2: Run to see the failure**

Run: `~/bin/t diff`
Expected: compile error (`Diff`, `split`, ... missing).

- [ ] **Step 3: Implement**

Create `rust/src/game/diff.rs`:

```rust
//! A differential: how a torque is shared between two shafts (the two wheels of an axle, or the front and the rear axle) and how the speed of its input follows the two outputs.
//! An open one gives each side its share whatever the speeds; a limited-slip one (a clutch pack pressed by ramps) moves torque from the faster side to the slower with a lock that is a preload
//! plus a share of the input torque (a different share on the power and on the coast); a viscous one moves it in proportion to the speed difference.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Open, Lsd, Viscous }

/// `split` is the share of the input torque that side A gets when nothing locks (0.5 for an axle; the front share for a centre diff). `preload` (N m) locks with no load,
/// `power` and `coast` are the shares of the input torque that lock on the power and on the coast (0..1), `visc` is N m per rad/s of the viscous coupling, `cap` the most the lock can move (N m).
#[derive(Clone, Copy, Debug)]
pub struct Diff { pub kind: Kind, pub split: f32, pub preload: f32, pub power: f32, pub coast: f32, pub visc: f32, pub cap: f32 }

/// How stiffly a limited-slip lock pulls the two sides together (N m per rad/s) before it saturates: it keeps the lock from chattering at a small speed difference.
const LOCK_K: f32 = 60.0;

/// The torque taken from side A and given to side B; positive when A is the faster side (the lock slows the faster side and drives the slower).
pub fn moved(d: &Diff, t_in: f32, dw: f32) -> f32 {
    let m = match d.kind {
        Kind::Open => return 0.0,
        Kind::Lsd => {
            let mag = (d.preload + t_in.abs() * if t_in >= 0.0 { d.power } else { d.coast }).clamp(0.0, d.cap.max(0.0));
            (LOCK_K * dw).clamp(-mag, mag)
        }
        Kind::Viscous => (d.visc * dw).clamp(-d.cap.abs(), d.cap.abs()),
    };
    if m.is_finite() { m } else { 0.0 }
}

/// The torques of side A and side B; they always add up to `t_in`.
pub fn split(d: &Diff, t_in: f32, w_a: f32, w_b: f32) -> (f32, f32) {
    let m = moved(d, t_in, w_a - w_b);
    (t_in * d.split - m, t_in * (1.0 - d.split) + m)
}

/// The speed of the input shaft: the share-weighted mean of the two outputs (the power in equals the power out for an open diff).
pub fn input_speed(d: &Diff, w_a: f32, w_b: f32) -> f32 { d.split * w_a + (1.0 - d.split) * w_b }

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run to see them pass**

Run: `~/bin/t diff`
Expected: `test result: ok. 7 passed`. (In `coast_uses_the_coast_ramp...` the power move is `40 + 350 = 390` and the coast move `40 + 200 = 240`; the difference of 150 exceeds the asserted 100.)

- [ ] **Step 5: Commit**

```bash
cd ~/projects/RallyNVX && git checkout -b feat/differentials
git add rust/src/game/diff.rs rust/src/game/diff_tests.rs rust/src/game/mod.rs
git commit -m "feat: differentials as pure functions (open, limited-slip with ramps and preload, viscous)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The driveline: centre, front and rear

**Files:**
- Create: `rust/src/game/driveline.rs`, `rust/src/game/driveline_tests.rs`
- Modify: `rust/src/game/mod.rs`

**Interfaces:**
- Consumes: Task 1's `diff::{Diff, Kind, split, input_speed}`; Plan 2's `drivetrain::Drivetrain` (`begin`, `sub_coupled`, `finish`, `DriveIn`), `wheel::{step, In, INERTIA}`; `assists::Tuning` (fields added in Task 3 are used here through the `DiffTuning` struct, so this task does not depend on them).
- Produces:
  - `pub struct Ctl { pub throttle: f32, pub brake: bool, pub steer: f32, pub speed: f32 }` (throttle 0..1, steer -1..1, speed m/s, all absolute where noted)
  - `pub struct DiffTuning { pub front: f32, pub rear: f32, pub centre: f32, pub split: f32 }` (multipliers, 1 = as built; `Default` = all 1)
  - `pub struct Driveline { pub front: Diff, pub rear: Diff, pub centre: Diff, pub active: bool }`
  - `Driveline::peugeot() -> Driveline`, `Driveline::tuned(self, &DiffTuning) -> Driveline`
  - `Driveline::input_speed(&self, w: [f32; 4]) -> f32` (rad/s), `Driveline::torques(&self, t: f32, w: [f32; 4], c: &Ctl) -> [f32; 4]`
  - `pub fn active_lock(c: &Ctl) -> f32` (0..1.3)

- [ ] **Step 1: Write the failing tests**

Add `pub mod driveline;` to `mod.rs`. Create `rust/src/game/driveline_tests.rs`:

```rust
use super::*;
use crate::game::{drivetrain::{DriveIn, Drivetrain}, wheel};

const R: f32 = 0.326;
fn calm() -> Ctl { Ctl { throttle: 1.0, brake: false, steer: 0.0, speed: 10.0 } }
fn sum(a: [f32; 4]) -> f32 { a.iter().sum() }

#[test] fn the_peugeot_is_four_wheel_drive_with_a_nominal_half_and_half_split() {
    let d = Driveline::peugeot();
    assert!((d.centre.split - 0.5).abs() < 1e-6 && d.front.split == 0.5 && d.rear.split == 0.5 && d.active);
    let t = d.torques(1200.0, [30.0; 4], &calm());
    for x in t { assert!((x - 300.0).abs() < 1e-3, "{t:?}"); }
}
#[test] fn the_wheel_torques_always_add_up_to_the_input_forward_coast_and_reverse() {
    let d = Driveline::peugeot();
    for &t in &[2500.0f32, 0.0, -300.0, -1800.0] {
        for w in [[30.0, 31.0, 29.0, 30.5], [80.0, 5.0, 30.0, 30.0], [0.0, 0.0, 120.0, 0.0], [-20.0, -21.0, -19.0, -20.0]] {
            let q = d.torques(t, w, &calm());
            assert!((sum(q) - t).abs() < 1e-2, "{t} {w:?} -> {q:?}");
        }
    }
}
#[test] fn reverse_and_coast_keep_the_sum() {
    let d = Driveline::peugeot();
    let q = d.torques(-900.0, [-10.0, -12.0, -8.0, -9.0], &Ctl { throttle: 0.0, ..calm() });
    assert!((sum(q) + 900.0).abs() < 1e-2, "{q:?}");
}
#[test] fn a_spinning_rear_wheel_loses_torque_to_the_other_side_through_the_lsd() {
    let d = Driveline::peugeot();
    let even = d.torques(1600.0, [30.0; 4], &calm());
    // the rear axle turns as fast as the front one on the whole (the centre diff stays quiet), but its left wheel spins and the right one is held
    let spin = d.torques(1600.0, [30.0, 30.0, 50.0, 10.0], &calm());
    assert!(spin[2] < even[2] - 100.0 && spin[3] > even[3] + 100.0, "{even:?} -> {spin:?}");
}
#[test] fn the_centre_diff_sends_torque_to_the_slower_axle() {
    let d = Driveline::peugeot();
    let q = d.torques(1600.0, [90.0, 90.0, 30.0, 30.0], &calm());   // the front axle spins
    assert!(q[0] + q[1] < q[2] + q[3] - 100.0, "{q:?}");
}
#[test] fn the_input_speed_of_equal_wheels_is_their_speed_and_a_split_shifts_the_weight() {
    let d = Driveline::peugeot();
    assert!((d.input_speed([42.0; 4]) - 42.0).abs() < 1e-4);
    let front = Driveline { centre: Diff { split: 0.7, ..d.centre }, ..d };
    assert!(front.input_speed([60.0, 60.0, 20.0, 20.0]) > d.input_speed([60.0, 60.0, 20.0, 20.0]));
}
#[test] fn tuning_scales_the_locks_and_moves_the_split_inside_thirty_to_seventy_percent() {
    let base = Driveline::peugeot();
    let t = base.tuned(&DiffTuning { front: 0.0, rear: 2.0, centre: 1.0, split: 1.4 });
    assert!(t.front.preload == 0.0 && t.front.power == 0.0 && (t.rear.preload - 2.0 * base.rear.preload).abs() < 1e-4);
    assert!((t.centre.split - 0.7).abs() < 1e-4);
    assert!((base.tuned(&DiffTuning { split: 0.6, ..Default::default() }).centre.split - 0.3).abs() < 1e-4);
}
#[test] fn extreme_tuning_stays_sane() {
    for &(a, b, c, s) in &[(0.0, 0.0, 0.0, 0.0), (2.0, 2.0, 2.0, 2.0), (-5.0, 9.0, f32::NAN, f32::INFINITY), (1.0, 1.0, 1.0, -3.0)] {
        let d = Driveline::peugeot().tuned(&DiffTuning { front: a, rear: b, centre: c, split: s });
        assert!(d.centre.split >= 0.3 - 1e-4 && d.centre.split <= 0.7 + 1e-4, "{s} -> {}", d.centre.split);
        let q = d.torques(2000.0, [30.0, 50.0, 10.0, 70.0], &calm());
        assert!(q.iter().all(|x| x.is_finite()) && (sum(q) - 2000.0).abs() < 1e-2, "{q:?}");
    }
}
#[test] fn the_active_lock_is_continuous() {
    let mut worst = 0.0f32; let mut last = active_lock(&Ctl { throttle: 0.0, brake: false, steer: 0.0, speed: 0.0 });
    for k in 1..=2000 {
        let x = k as f32 / 2000.0;
        let l = active_lock(&Ctl { throttle: x, brake: false, steer: 1.0 - x, speed: 50.0 * x });
        worst = worst.max((l - last).abs()); last = l;
    }
    assert!(worst < 0.02, "the largest jump of the lock per step: {worst}");
}
#[test] fn the_active_lock_opens_for_turn_in_and_closes_on_the_power_and_at_speed() {
    let power = active_lock(&Ctl { throttle: 1.0, brake: false, steer: 0.0, speed: 10.0 });
    let turn_in = active_lock(&Ctl { throttle: 0.0, brake: false, steer: 0.8, speed: 10.0 });
    let braking = active_lock(&Ctl { throttle: 0.0, brake: true, steer: 0.0, speed: 10.0 });
    let fast = active_lock(&Ctl { throttle: 0.0, brake: false, steer: 0.0, speed: 45.0 });
    let slow = active_lock(&Ctl { throttle: 0.0, brake: false, steer: 0.0, speed: 5.0 });
    assert!(power > turn_in + 0.4 && power > braking + 0.3 && fast > slow + 0.1, "{power} {turn_in} {braking} {fast} {slow}");
}

/// A bench for the whole loop: the real engine and clutch (`Drivetrain`), a diff and two driven wheels on the same axle, one on grip and one on ice, pulling a car; returns the car's speed.
fn mu_split_launch(axle: Diff, secs: f32) -> (f32, f32) {
    let mut d = Drivetrain::new(); d.opts.tc = false;
    let (h, mass, grip, ice) = (1.0 / 240.0, 1230.0f32, 3600.0f32, 450.0f32);
    let (mut wl, mut wr, mut v) = (0.0f32, 0.0f32, 0.0f32);
    let (mut tyre, mut spin_var) = (0.0f32, 0.0f32);
    let mut prev = 0.0f32;
    for k in 0..(secs / h) as usize {
        if k % 4 == 0 { d.begin(4.0 * h, &DriveIn { throttle: 1.0, speed: v, wheel_r: R, ..Default::default() }); }
        let t = d.sub_coupled(h, crate::game::diff::input_speed(&axle, wl, wr), 2.0 * wheel::INERTIA, tyre);
        let (tl, tr) = crate::game::diff::split(&axle, t, wl, wr);
        let ol = wheel::step(&wheel::In { w: wl, vf: v, radius: R, fmax: grip, kp: 0.07, slide: 0.85, drive: tl, brake: 0.0, abs: false, h, inertia: wheel::INERTIA });
        let or = wheel::step(&wheel::In { w: wr, vf: v, radius: R, fmax: ice, kp: 0.07, slide: 0.85, drive: tr, brake: 0.0, abs: false, h, inertia: wheel::INERTIA });
        wl = ol.w; wr = or.w; tyre = R * (ol.fx + or.fx);
        v += (ol.fx + or.fx - 0.5 * 1.2 * 0.62 * 2.1 * v * v) / mass * h;
        if k % 4 == 3 { d.finish(); }
        if k as f32 * h > secs * 0.5 { spin_var += (wr - prev).abs(); }
        prev = wr;
    }
    (v, spin_var)
}
#[test] fn an_lsd_pulls_away_where_an_open_diff_spins() {
    let open = Diff { kind: Kind::Open, ..Driveline::peugeot().rear };
    let lsd = Driveline::peugeot().rear;
    let (v_open, _) = mu_split_launch(open, 3.0);
    let (v_lsd, wobble) = mu_split_launch(lsd, 3.0);
    assert!(v_open < 4.0, "an open diff on a split surface barely moves: {v_open} m/s");
    assert!(v_lsd > v_open * 2.0, "the LSD pulls: {v_lsd} against {v_open} m/s");
    assert!(wobble.is_finite() && wobble < 2000.0, "the ice wheel's speed does not saw: {wobble}");
}
```

- [ ] **Step 2: Run to see the failure**

Run: `~/bin/t driveline`
Expected: compile errors.

- [ ] **Step 3: Implement**

Create `rust/src/game/driveline.rs`:

```rust
//! The four-wheel driveline: the engine's torque at the wheels goes through the centre differential to the front and the rear axle, and through the axle's differential to its two wheels.
//! The engine sees the wheels through the same chain, each differential weighting its two outputs by its share, so the power in equals the power out.
use super::diff::{input_speed, split, Diff, Kind};

/// What the active centre differential reads from the driver and the car.
#[derive(Clone, Copy, Debug)]
pub struct Ctl { pub throttle: f32, pub brake: bool, pub steer: f32, pub speed: f32 }

/// The driver's tuning of the differentials as multipliers (1 = as built): the lock of each differential and the centre split (1.4 = 70 % to the front, 0.6 = 30 %).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiffTuning { pub front: f32, pub rear: f32, pub centre: f32, pub split: f32 }
impl Default for DiffTuning { fn default() -> DiffTuning { DiffTuning { front: 1.0, rear: 1.0, centre: 1.0, split: 1.0 } } }

#[derive(Clone, Copy, Debug)]
pub struct Driveline { pub front: Diff, pub rear: Diff, pub centre: Diff, pub active: bool }

/// The active centre differential's lock as a multiplier of its built value: it closes on the power and at speed (traction, stability), opens for the turn-in (the nose turns) and under the brake.
/// Every term is continuous in its input.
pub fn active_lock(c: &Ctl) -> f32 {
    let sm = |a: f32, b: f32, x: f32| { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) };
    let thr = if c.throttle.is_finite() { c.throttle.clamp(0.0, 1.0) } else { 0.0 };
    let steer = if c.steer.is_finite() { c.steer.abs().clamp(0.0, 1.0) } else { 0.0 };
    let speed = if c.speed.is_finite() { c.speed.abs() } else { 0.0 };
    let turn_in = sm(0.25, 0.7, steer) * (1.0 - sm(0.2, 0.6, thr));   // steering without the power
    let brake = if c.brake { 1.0 } else { 0.0 };
    (0.5 + 0.5 * thr - 0.35 * turn_in - 0.2 * brake + 0.2 * sm(20.0, 45.0, speed)).clamp(0.0, 1.3)
}

impl Driveline {
    /// The Peugeot 206 WRC's: a mechanical limited-slip differential on each axle and an electronically controlled centre differential, a nominal half and half split. The numbers are
    /// approximations (the sources name the layout, not the values).
    pub fn peugeot() -> Driveline {
        Driveline {
            front: Diff { kind: Kind::Lsd, split: 0.5, preload: 30.0, power: 0.25, coast: 0.15, visc: 0.0, cap: 700.0 },
            rear: Diff { kind: Kind::Lsd, split: 0.5, preload: 40.0, power: 0.35, coast: 0.20, visc: 0.0, cap: 900.0 },
            centre: Diff { kind: Kind::Lsd, split: 0.5, preload: 60.0, power: 0.40, coast: 0.25, visc: 0.0, cap: 1200.0 },
            active: true,
        }
    }

    /// The driveline with the driver's tuning applied (any value is made safe: the locks 0..2 times, the split 30 to 70 % to the front).
    pub fn tuned(self, t: &DiffTuning) -> Driveline {
        let f = |v: f32, lo: f32, hi: f32| if v.is_finite() { v.clamp(lo, hi) } else { 1.0 };
        let lock = |d: Diff, k: f32| Diff { preload: d.preload * k, power: d.power * k, coast: d.coast * k, visc: d.visc * k, cap: d.cap * k.max(0.25), ..d };
        let mut c = lock(self.centre, f(t.centre, 0.0, 2.0));
        c.split = (c.split * f(t.split, 0.6, 1.4)).clamp(0.3, 0.7);
        Driveline { front: lock(self.front, f(t.front, 0.0, 2.0)), rear: lock(self.rear, f(t.rear, 0.0, 2.0)), centre: c, active: self.active }
    }

    /// The speed the engine sees (rad/s): the centre differential's input follows the front and the rear axle, each follows its two wheels.
    pub fn input_speed(&self, w: [f32; 4]) -> f32 {
        let (wf, wr) = (input_speed(&self.front, w[0], w[1]), input_speed(&self.rear, w[2], w[3]));
        input_speed(&self.centre, wf, wr)
    }

    /// The torque of each wheel (FL, FR, RL, RR) from the torque `t` the driveline puts on the four wheels together; they add up to `t`.
    pub fn torques(&self, t: f32, w: [f32; 4], c: &Ctl) -> [f32; 4] {
        let (wf, wr) = (input_speed(&self.front, w[0], w[1]), input_speed(&self.rear, w[2], w[3]));
        let mut centre = self.centre;
        if self.active {
            let k = active_lock(c);
            centre = Diff { preload: centre.preload * k, power: centre.power * k, coast: centre.coast * k, visc: centre.visc * k, cap: centre.cap * k.max(0.25), ..centre };
        }
        let (tf, tr) = split(&centre, t, wf, wr);
        let (a, b) = split(&self.front, tf, w[0], w[1]);
        let (cc, d) = split(&self.rear, tr, w[2], w[3]);
        [a, b, cc, d]
    }
}

#[cfg(test)]
#[path = "driveline_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run to see them pass**

Run: `~/bin/t driveline`
Expected: `test result: ok. 11 passed`. If `an_lsd_pulls_away_where_an_open_diff_spins` shows `v_open` above 4 m/s, print both speeds: with the engine and a 450 N grip on the ice wheel the open diff passes the same torque to the grip wheel as the ice wheel can carry, so a car speed far above 4 m/s in 3 s means the coupling is not holding torque to what the ice wheel can take: re-check Plan 2's `sub_coupled` (the implicit term `a`), not the diff. If the LSD ratio is below 2, raise the bench to 4 s before touching the diff's numbers.

- [ ] **Step 5: Commit**

```bash
git add rust/src/game/driveline.rs rust/src/game/driveline_tests.rs rust/src/game/mod.rs
git commit -m "feat: the four-wheel driveline (front, centre, rear differentials, active centre lock, tuning)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The world uses the driveline; four new tuning parts

**Files:**
- Modify: `rust/src/game/assists.rs`, `rust/src/game/assists_tests.rs`
- Modify: `rust/src/game/world.rs`

**Interfaces:**
- Consumes: `Driveline`, `DiffTuning`, `Ctl`.
- Produces: `Tuning` fields `diff_front`, `diff_rear`, `diff_centre`, `split` (parts 7, 8, 9, 10; `TUNING_PARTS = 11`); ranges 0..2 for 7 to 9 and 0.6..1.4 for 10; `Tuning::diffs(&self) -> DiffTuning`.

- [ ] **Step 1: Write the failing tests**

In `assists_tests.rs`: replace the first line of `tuning_is_clamped_to_its_range_ignores_garbage_and_scales_the_parts`'s default check
`let mut t = Tuning::default(); assert_eq!(t, Tuning { engine: 1.0, turbo: 1.0, brakes: 1.0, grip: 1.0, suspension: 1.0, downforce: 1.0, gearing: 1.0 });`
by
`let mut t = Tuning::default(); for i in 0..TUNING_PARTS { assert_eq!(t.get(i), 1.0, "part {i} starts as built"); }`
and add the tests:
```rust
#[test] fn the_differential_parts_are_tuning_parts_with_their_own_ranges() {
    assert_eq!(TUNING_PARTS, 11);
    let mut t = Tuning::default();
    t.set(7, 9.0); assert_eq!(t.diff_front, 2.0); t.set(8, -1.0); assert_eq!(t.diff_rear, 0.0); t.set(9, 0.5); assert_eq!(t.diff_centre, 0.5);
    t.set(10, 9.0); assert_eq!(t.split, 1.4); t.set(10, 0.0); assert_eq!(t.split, 0.6); t.set(10, f32::NAN); assert_eq!(t.split, 0.6, "garbage is ignored");
    let d = t.diffs(); assert_eq!((d.front, d.rear, d.centre, d.split), (2.0, 0.0, 0.5, 0.6));
}
```
and inside `mod physics` (it has `track()`, `time_to`, `Input`, `World`):
```rust
    #[test] fn the_centre_split_sends_the_torque_where_the_driver_set_it() {
        let share = |part10: f32| { let mut w = track(); w.tuning.set(10, part10);
            for _ in 0..90 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
            let q = w.vehicle.drive_torque.unwrap(); (q[0] + q[1]) / q.iter().sum::<f32>() };
        let (even, front, rear) = (share(1.0), share(1.4), share(0.6));
        assert!((even - 0.5).abs() < 0.06 && (front - 0.7).abs() < 0.08 && (rear - 0.3).abs() < 0.08, "{even} {front} {rear}");
    }
    #[test] fn the_wheel_torques_add_up_to_the_engines_wheel_torque() {
        let mut w = track();
        for _ in 0..120 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
        let sum: f32 = w.vehicle.drive_torque.unwrap().iter().sum();
        let mean = w.drivetrain.last.wheel_torque;
        assert!((sum - mean).abs() < 0.2 * mean.abs().max(100.0), "the last substep {sum} against the frame's mean {mean}");
    }
```

- [ ] **Step 2: Run to see the failure**

Run: `~/bin/t assists`
Expected: compile errors (`diff_front`, `diffs`, 11 parts).

- [ ] **Step 3: Implement**

`assists.rs`:
1. `Tuning` gets four more fields after `gearing` (with a doc line): `pub diff_front: f32, pub diff_rear: f32, pub diff_centre: f32, pub split: f32` (the locks of the front, rear and centre differentials and the centre split as multipliers; the split 1.4 is 70 % to the front); `TUNING_PARTS` becomes 11; `Default` sets the four to 1.0.
2. Replace `range`, `set` and `get` by:
```rust
    pub fn range(part: usize) -> (f32, f32) { match part { 0 => (0.3, 2.0), 1 => (0.0, 2.0), 2 => (0.3, 2.5), 3 => (0.6, 1.5), 4 => (0.5, 1.8), 5 => (0.0, 3.0), 7..=9 => (0.0, 2.0), 10 => (0.6, 1.4), _ => (0.7, 1.3) } }
    /// Sets part `i` (0 engine, 1 turbo, 2 brakes, 3 grip, 4 suspension, 5 downforce, 6 gearing, 7 front diff, 8 rear diff, 9 centre diff, 10 centre split) clamped to its range; garbage is ignored.
    pub fn set(&mut self, i: usize, v: f32) {
        if !v.is_finite() || i >= TUNING_PARTS { return; }
        let (lo, hi) = Tuning::range(i); let v = v.clamp(lo, hi);
        match i { 0 => self.engine = v, 1 => self.turbo = v, 2 => self.brakes = v, 3 => self.grip = v, 4 => self.suspension = v, 5 => self.downforce = v, 6 => self.gearing = v, 7 => self.diff_front = v, 8 => self.diff_rear = v, 9 => self.diff_centre = v, _ => self.split = v }
    }
    pub fn get(&self, i: usize) -> f32 { match i { 0 => self.engine, 1 => self.turbo, 2 => self.brakes, 3 => self.grip, 4 => self.suspension, 5 => self.downforce, 6 => self.gearing, 7 => self.diff_front, 8 => self.diff_rear, 9 => self.diff_centre, _ => self.split } }
    /// The differentials' part of the tuning.
    pub fn diffs(&self) -> super::driveline::DiffTuning { super::driveline::DiffTuning { front: self.diff_front, rear: self.diff_rear, centre: self.diff_centre, split: self.split } }
```
   and in the struct's literal `Default` add the four. (A `Tuning { .. }` literal elsewhere? `grep -rn 'Tuning {' rust/src` must show only `assists.rs`.)
`world.rs`: in the drivetrain block from Plan 2 Task 4, before `let dtr = &mut self.drivetrain;` add
```rust
            let dl = super::driveline::Driveline::peugeot().tuned(&self.tuning.diffs());
            let ctl = super::driveline::Ctl { throttle: inp.throttle.abs(), brake: inp.brake, steer: inp.steer, speed: fwd.abs() };
```
   and replace the closure by
```rust
            Some(&mut |h, w, tyre| { let t = dtr.sub_coupled(h, dl.input_speed(w), iw, tyre); dl.torques(t, w, &ctl) })
```
   (the same `step_driven` call; only the closure changes).

- [ ] **Step 4: Run, then the whole suite and the anchors**

Run: `~/bin/t assists` -> pass. `~/bin/t` -> all pass; `~/bin/t anchors` must stay green (the default driveline moves no torque when the wheels turn alike). If `the_wheel_torques_add_up...` fails by more than the tolerance, compare `sum` with the last substep's `t` printed from the closure before loosening it.
Autopilot: `~/bin/t stage_tests -- --ignored 2>&1 | tail -15` as in Plan 2.

- [ ] **Step 5: Commit**

```bash
git add rust/src/game && git commit -m "feat: the world drives the four wheels through the driveline; four differential tuning parts

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The car menu: differential rows and presets

**Files:**
- Modify: `app/src/main/java/dev/starinin/rallynvx/CarMath.kt`, `Controls.kt`, `GameActivity.kt`
- Modify: `app/src/test/java/dev/starinin/rallynvx/CarMathTest.kt`

**Interfaces:**
- Consumes: the game's `gameTuning(part, value)` (parts 7..10 now exist).
- Produces: `CarMath.PARTS = 11`, `CarMath.valueText(i, v)`, `CarMath.PRESETS`, `CarMath.presetName(i, en)`, `CarMath.PRESET_COUNT = 3`; `Controls.diffPreset/setDiffPreset`.

- [ ] **Step 1: Write the failing tests**

Append to `CarMathTest.kt` (inside the class):
```kotlin
    @Test fun differentialPartsHaveNamesRangesAndTexts() {
        assertEquals(11, CarMath.PARTS)
        assertEquals("Front diff lock", CarMath.partName(7, true)); assertEquals("Задний дифференциал", CarMath.partName(8, false))
        assertEquals(0f, CarMath.lo(9), 0f); assertEquals(2f, CarMath.hi(9), 0f); assertEquals(0.6f, CarMath.lo(10), 1e-6f); assertEquals(1.4f, CarMath.hi(10), 1e-6f)
        assertEquals("50/50", CarMath.valueText(10, 1f)); assertEquals("70/30", CarMath.valueText(10, 1.4f)); assertEquals("30/70", CarMath.valueText(10, 0.6f))
        assertEquals("150 %", CarMath.valueText(7, 1.5f))
    }
    @Test fun presetsAreInsideTheRangesAndWrap() {
        for (p in 0 until CarMath.PRESET_COUNT) for (k in 0 until 4) { val v = CarMath.PRESETS[p][k]; assertEquals(v, CarMath.clamp(7 + k, v), 1e-6f) }
        assertEquals("Gravel", CarMath.presetName(1, true)); assertEquals("Снег", CarMath.presetName(2, false)); assertEquals("Tarmac", CarMath.presetName(99, true))
    }
```

- [ ] **Step 2: Run to see the failure**

Run: `cd ~/projects/RallyNVX && ./gradlew --no-daemon testDebugUnitTest -q 2>&1 | tail -15`
Expected: compile errors in `CarMathTest`.

- [ ] **Step 3: Implement**

`CarMath.kt`:
- `const val PARTS = 11`; the doc line mentions the differentials.
- `partName`: add before `else`: `7 -> if (en) "Front diff lock" else "Передний дифференциал"`, `8 -> if (en) "Rear diff lock" else "Задний дифференциал"`, `9 -> if (en) "Centre diff lock" else "Центральный дифференциал"`, `10 -> if (en) "Centre split (front/rear)" else "Распределение (перед/зад)"`; the `else` stays the final drive (part 6).
- `private val LO = floatArrayOf(0.3f, 0f, 0.3f, 0.6f, 0.5f, 0f, 0.7f, 0f, 0f, 0f, 0.6f)` and `HI = floatArrayOf(2f, 2f, 2.5f, 1.5f, 1.8f, 3f, 1.3f, 2f, 2f, 2f, 1.4f)`.
- Add:
```kotlin
    /** The text of a part's value: a percentage of the built value, and for the centre split (part 10, 1 = 50/50) the share to the front and to the rear. */
    fun valueText(i: Int, v: Float): String = if (i == 10) { val f = Math.round(50f * v); "$f/${100 - f}" } else percent(v)
    /** Differential presets for the surface (front lock, rear lock, centre lock, centre split): tarmac turns the car in with a looser centre and a rear bias, gravel is as built, snow locks the centre. */
    const val PRESET_COUNT = 3
    val PRESETS = arrayOf(floatArrayOf(0.8f, 1.2f, 0.7f, 0.9f), floatArrayOf(1f, 1f, 1f, 1f), floatArrayOf(0.8f, 0.8f, 1.6f, 1f))
    fun presetName(i: Int, en: Boolean): String = when (i) { 1 -> if (en) "Gravel" else "Гравий"; 2 -> if (en) "Snow" else "Снег"; else -> if (en) "Tarmac" else "Асфальт" }
```
`Controls.kt` (next to `tuning`):
```kotlin
    /** The differential preset last chosen in the car menu (0 tarmac, 1 gravel, 2 snow); only the button's label, the values are the tuning parts. */
    fun diffPreset(c: Context): Int = p(c).getInt("diff_preset", 1).coerceIn(0, CarMath.PRESET_COUNT - 1)
    fun setDiffPreset(c: Context, v: Int) = p(c).edit().putInt("diff_preset", v.coerceIn(0, CarMath.PRESET_COUNT - 1)).apply()
```
`GameActivity.kt` `showCarPanel`:
- in the tuning rows replace `fun show() { v.text = CarMath.percent(Controls.tuning(this, i)) }` by `fun show() { v.text = CarMath.valueText(i, Controls.tuning(this, i)) }`.
- the `change(dir)` step for the split should move by 5 points of share: the existing `CarMath.step` uses 0.1 steps: 0.1 of the multiplier is 5 points of share (50 x 0.1), which is fine; no change.
- add a preset row to `colB` just after its heading label (before the `for (i in 0 until CarMath.PARTS)` loop):
```kotlin
        run {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { text = t("Differentials", "Дифференциалы"); textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            fun lab() = CarMath.presetName(Controls.diffPreset(this), en)
            val b = M3Widgets.button(this, lab(), M3Widgets.ButtonKind.TONAL) {}
            b.setOnClickListener {
                val n = GearMath.next(Controls.diffPreset(this), CarMath.PRESET_COUNT); Controls.setDiffPreset(this, n)
                for (k in 0 until 4) { Controls.setTuning(this, 7 + k, CarMath.PRESETS[n][k]); if (running) Native.gameTuning(7 + k, CarMath.PRESETS[n][k]) }
                (b as? TextView)?.text = lab(); refreshers.forEach { it() }
            }
            refreshers.add { (b as? TextView)?.text = lab() }
            r.addView(b, LinearLayout.LayoutParams(px(110f), px(42f)))
            colB.addView(r, LinearLayout.LayoutParams(px(300f), -2).apply { topMargin = px(6f) })
        }
```
  (`GearMath.next` is from Plan 1; if Plan 1 is not merged, use `(Controls.diffPreset(this) + 1) % CarMath.PRESET_COUNT`.)
- the "Reset tuning" button already loops `0 until CarMath.PARTS`, so the four new parts reset with it; also add `Controls.setDiffPreset(this, 1)` there.

- [ ] **Step 4: Run the Kotlin tests**

Run: `./gradlew --no-daemon testDebugUnitTest -q 2>&1 | tail -15`
Expected: no failures.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: the car menu has the differentials (lock of each, the centre split, surface presets)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Documentation and release

**Files:**
- Modify: `CHANGELOG.md`, `CHANGELOG.ru.md`, `docs/game.md`, `docs/vehicles.md`, `app/build.gradle.kts`
- Create: `~/projects/releases/rallynvx-2.75.0.md`

- [ ] **Step 1: Changelog EN** (above 2.74.0):
```
## 2.75.0 - differentials
- Added: a real four-wheel **driveline**: the engine's torque goes through a **centre differential** (a nominal 50/50 split, an **active** lock that closes on the power and at speed and opens for the turn-in and under the brake) to the front and the rear axle, and through a **limited-slip differential** on each axle (a clutch pack with a preload and different power and coast ramps) to its two wheels. A wheel that spins loses torque to the one with grip (one rear wheel on ice with an open differential barely moves the car; with the limited-slip one it pulls away), and the engine feels the wheels through the same chain
- Added: in the car menu a **Differentials** row with the presets Tarmac / Gravel / Snow, and four parts to tune: the lock of the front, the rear and the centre differential (0 to 200 %) and the centre split (30/70 to 70/30 front/rear)
- The Peugeot's numbers (a limited-slip differential on each axle, an electronically controlled centre one, a nominal 50/50) follow the layout of the real car; the values are approximations
- Not yet: the brake bias in the menu, launch control
```
RU:
```
## 2.75.0 - дифференциалы
- Добавлено: настоящий полноприводный **привод**: момент двигателя идёт через **центральный дифференциал** (номинально 50/50, **активная** блокировка закрывается на газу и на скорости и открывается на входе в поворот и при торможении) на переднюю и заднюю ось, а через **самоблокирующийся дифференциал** на каждой оси (пакет дисков с предварительным натягом и разными рампами на газе и на сбросе) на два колеса. Буксующее колесо отдаёт момент колесу со сцеплением (одно заднее колесо на льду при открытом дифференциале почти не двигает машину, с самоблокирующимся она трогается), а двигатель чувствует колёса через ту же цепь
- Добавлено: в меню машины строка **Дифференциалы** с пресетами Асфальт / Гравий / Снег и четыре настройки: блокировка переднего, заднего и центрального дифференциала (0–200 %) и распределение между осями (от 30/70 до 70/30 перед/зад)
- Числа Peugeot (самоблокирующийся дифференциал на каждой оси, электронно управляемый центральный, номинально 50/50) следуют схеме настоящей машины; значения приближённые
- Пока нет: баланса тормозов в меню, launch control
```
- [ ] **Step 2: Docs**: `docs/game.md` gets a `**Driveline / Привод.**` paragraph (EN + RU: the chain, `diff::moved` with `LOCK_K = 60`, the lock = preload + share of the input torque by ramp, the power-consistent input speed, the active lock formula, the Peugeot's numbers marked as approximations, the tuning parts 7 to 10, the implicit clutch from 2.74.0 that makes the open-diff behaviour right); `docs/vehicles.md` gets one line about the driveline being per-car code (`Driveline::peugeot()`), not yet in `vehicles.json`.
- [ ] **Step 3: Version and notes**: `versionCode = 105`, `versionName = "2.75.0"`; `~/projects/releases/rallynvx-2.75.0.md` (EN + RU).
- [ ] **Step 4: Verify**: `~/bin/t 2>&1 | tail -4`, `~/bin/t anchors`, `./gradlew --no-daemon testDebugUnitTest -q 2>&1 | tail -5`: all green.
- [ ] **Step 5: Merge and release**:
```bash
git add -A && git commit -m "docs: 2.75.0 differentials

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
git checkout main && git merge --no-ff feat/differentials -m "Merge feat/differentials" && ~/bin/rt-release 2.75.0 2>&1 | tail -4
```
Expected: the release URL for v2.75.0. Report: host tests only; the feel of the diffs (power oversteer, the nose on turn-in, the lock on gravel) needs the user's drive; the defaults are approximations they can tune in the menu.
