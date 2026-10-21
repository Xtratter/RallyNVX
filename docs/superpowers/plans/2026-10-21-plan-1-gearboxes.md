# Gearboxes Implementation Plan (version 2.73.0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A real automatic (profiles, remembered pedal, aggressiveness, best-gear kick-down, braking down-shifts, cornering/grade holds) and a manual box that is **sequential by default** (no clutch needed, spark cut up, blip down), with the H-pattern kept as an option.

**Architecture:** The automatic's decisions move out of `Drivetrain` into a pure module `game/shiftlogic.rs` (`choose(profile, memory, situation) -> Choice`), tested without a car; `Drivetrain` owns a `Memory`, feeds the situation, and executes the choice. A `BoxType` (Sequential / HPattern) changes the manual's clutch, shift times, spark cut and blip. Settings travel Kotlin -> JNI -> `Game` -> `world.drivetrain`.

**Tech Stack:** Rust (cdylib, host tests via `~/bin/t`), Kotlin (UI, JUnit via `./gradlew --no-daemon testDebugUnitTest -q`).

**Spec:** `docs/superpowers/specs/2026-10-21-driveline-design.md` (section 3.1).

## Global Constraints

- Work on a branch `feat/gearboxes`; merge to `main` and release with `~/bin/rt-release 2.73.0` (release notes `~/projects/releases/rallynvx-2.73.0.md`, EN + RU). versionName "2.73.0", versionCode 103.
- Never launch the game on the phone; verify with host tests only and say so in the report. Never touch the real `profile.json`.
- Docs and changelog bilingual (EN in `CHANGELOG.md`, RU in `CHANGELOG.ru.md`, newest entry on top, format `## 2.73.0 - title`).
- Comments in the codebase's style: short English sentences, no task numbers.
- fps target 30: nothing in the per-step path may allocate.
- The Peugeot's default box is the **sequential** (decided by the user); the automatic's default profile is **Sport** (the 2.72.0 behaviour).
- Run the whole Rust suite (`~/bin/t`) and Kotlin suite before the release; both must be green.

## Review Focus

1. NaN/inf or negative speed/rpm fed to the shift logic: it must hold the gear, never panic or pick a gear outside 1..=6 (Task 1 test `garbage_in_holds_the_gear`).
2. Reverse and neutral: the new logic must not touch R (selector), N (manual) or the autopilot's auto-reverse (Task 2 test `reverse_and_the_selector_are_untouched`).
3. Switching the box type or profile in the middle of a shift or while stalled must not leave the drivetrain in a broken state (Task 3 test `switching_the_box_during_a_shift_is_safe`).
4. A pedal flickering around a threshold or a brake tapped on and off must not make the box hunt (Task 2 test `the_automatic_does_not_hunt_under_a_wandering_pedal`, kept; plus `brake_taps_do_not_hunt` in Task 2).
5. A standstill and creeping in 1st: no shifts, no stall in the sequential without a pedal (Task 3 test `the_sequential_idles_in_gear_without_stalling`).

---

## File Structure

- Create `rust/src/game/shiftlogic.rs` + `rust/src/game/shiftlogic_tests.rs`: `Profile`, `Table`, `Memory`, `Situation`, `Choice`, `choose`.
- Modify `rust/src/game/mod.rs`: `pub mod shiftlogic;`.
- Modify `rust/src/game/drivetrain.rs`: `DriveIn` (+`brake`, `lat_g`, `slope`, `Default`), `BoxType`, fields `mem`, `box_type`, `profile`, `auto_gear` rewrite, sequential rules.
- Modify `rust/src/game/drivetrain_tests.rs`: harness, new tests, H-pattern tests pinned to `BoxType::HPattern`.
- Modify `rust/src/game/world.rs`: pass `lat_g`, `slope`.
- Modify `rust/src/game/app.rs`, `rust/src/game_jni.rs`: `GCmd::GearCfg`, `Game::set_gear_cfg`.
- Modify Kotlin: `Native.kt`, `Controls.kt`, `GearMath.kt` (+ `GearMathTest.kt`), `GameActivity.kt` (car panel rows, send on start).
- Docs: `CHANGELOG.md`, `CHANGELOG.ru.md`, `docs/game.md`, release notes, `app/build.gradle.kts`.

---

### Task 1: The shift logic as pure functions

**Files:**
- Create: `rust/src/game/shiftlogic.rs`, `rust/src/game/shiftlogic_tests.rs`
- Modify: `rust/src/game/mod.rs`

**Interfaces:**
- Consumes: `super::drivetrain::{engine_torque, ratio, rpm_in_gear}` (all `pub`, exist).
- Produces:
  - `pub enum Profile { Comfort, Sport, Race }` (`Default` = Sport; `Profile::from_i32(i32) -> Profile`).
  - `pub struct Table { up_lo, up_hi, floor_lo, floor_hi, dwell_up, dwell_down, kick_rpm, over_rev, brake_rpm, shift_time: f32 }`, `pub fn table(Profile) -> Table`.
  - `pub struct Memory` (`Default` has `since_shift = 9.0`), `fn update(&mut self, dt, gas, brake, lat_g) -> f32` (returns the pedal rate 1/s), `fn shifted(&mut self)`, pub fields `load`, `aggr`, `since_shift`.
  - `pub struct Situation { gear: i32, rpm, gas, rate: f32, brake: bool, speed, wheel_r, gearing, lat_g, slope: f32 }`.
  - `pub enum Choice { Hold, Up(i32), Down(i32) }`, `pub fn choose(p: Profile, m: &Memory, s: &Situation) -> Choice`.

- [ ] **Step 1: Register the module and write the failing tests**

In `rust/src/game/mod.rs` add `pub mod shiftlogic;` next to `pub mod drivetrain;`. Create `rust/src/game/shiftlogic_tests.rs`:

```rust
use super::*;

const R: f32 = 0.326;
/// rpm per unit of ratio per m/s at the Peugeot's wheel radius.
fn sit(gear: i32, speed: f32, gas: f32) -> Situation {
    Situation { gear, rpm: rpm_in_gear(speed, R, gear), gas, rate: 0.0, brake: false, speed, wheel_r: R, gearing: 1.0, lat_g: 0.0, slope: 0.0 }
}
fn mem(load: f32) -> Memory { Memory { load, ..Memory::default() } }

#[test] fn a_lift_never_upshifts() {
    let s = sit(3, 25.0, 0.0);                        // 5900 rpm, the pedal up
    assert_eq!(choose(Profile::Sport, &mem(0.0), &s), Choice::Hold);
}
#[test] fn the_upshift_point_grows_with_the_remembered_pedal() {
    // gear 2 at about 5000 rpm: a light pedal shifts, a floored one holds for the power
    let v = 5000.0 / (rpm_in_gear(1.0, R, 2));
    assert_eq!(choose(Profile::Sport, &mem(0.3), &sit(2, v, 0.3)), Choice::Up(3));
    assert_eq!(choose(Profile::Sport, &mem(1.0), &sit(2, v, 1.0)), Choice::Hold);
}
#[test] fn the_profiles_differ_in_when_they_climb() {
    let v = 5000.0 / rpm_in_gear(1.0, R, 2);
    let s = sit(2, v, 0.3);
    assert_eq!(choose(Profile::Comfort, &mem(0.3), &s), Choice::Up(3));
    assert_eq!(choose(Profile::Sport, &mem(0.3), &s), Choice::Up(3));
    assert_eq!(choose(Profile::Race, &mem(0.3), &s), Choice::Hold, "race holds the gear for the power");
}
#[test] fn a_floored_pedal_in_a_high_gear_skips_to_the_gear_with_the_most_force() {
    let s = sit(6, 22.0, 1.0);                        // about 3000 rpm in sixth
    assert_eq!(choose(Profile::Sport, &mem(1.0), &s), Choice::Down(3));
}
#[test] fn a_kick_down_does_not_pick_a_gear_that_overreves() {
    let s = sit(6, 24.0, 1.0);                        // 3200 rpm in sixth: second would be 7500 rpm, over the limit, so it stops at third
    match choose(Profile::Sport, &mem(1.0), &s) { Choice::Down(g) => { assert_eq!(g, 3); assert!(rpm_in_gear(24.0, R, g) < 6600.0); }, c => panic!("{c:?}") }
}
#[test] fn braking_steps_down_to_the_band_of_the_engine_brake_and_never_up() {
    let mut s = sit(6, 23.0, 0.0); s.brake = true;    // 3100 rpm
    assert_eq!(choose(Profile::Sport, &mem(0.0), &s), Choice::Down(4));
    let mut up = sit(2, 14.0, 0.0); up.brake = true;  // high revs, braking: no up-shift
    assert_eq!(choose(Profile::Sport, &mem(0.0), &up), Choice::Hold);
}
#[test] fn cornering_holds_the_gear() {
    let v = 5000.0 / rpm_in_gear(1.0, R, 2);
    let mut s = sit(2, v, 0.3); s.lat_g = 0.5;
    assert_eq!(choose(Profile::Sport, &mem(0.3), &s), Choice::Hold);
}
#[test] fn an_uphill_grade_raises_the_floor() {
    let v = 13.55;                                    // fourth at about 2600 rpm
    let mut s = sit(4, v, 0.3);
    assert_eq!(choose(Profile::Sport, &mem(0.3), &s), Choice::Hold);
    s.slope = 0.15;
    assert_eq!(choose(Profile::Sport, &mem(0.3), &s), Choice::Down(3));
}
#[test] fn a_downhill_with_the_pedal_nearly_up_does_not_climb() {
    let v = 5000.0 / rpm_in_gear(1.0, R, 2);
    let mut s = sit(2, v, 0.2); s.slope = -0.1;
    assert_eq!(choose(Profile::Sport, &mem(0.2), &s), Choice::Hold);
}
#[test] fn the_dwell_stops_the_box_from_hunting() {
    let v = 5000.0 / rpm_in_gear(1.0, R, 2);
    let mut m = mem(0.3); m.since_shift = 0.2;
    assert_eq!(choose(Profile::Sport, &m, &sit(2, v, 0.3)), Choice::Hold);
    m.since_shift = 1.0;
    assert_eq!(choose(Profile::Sport, &m, &sit(2, v, 0.3)), Choice::Up(3));
}
#[test] fn lugging_below_the_floor_steps_down_even_inside_the_dwell() {
    let mut m = mem(0.0); m.since_shift = 0.1;
    let s = Situation { rpm: 1200.0, ..sit(3, 6.0, 0.1) };
    assert_eq!(choose(Profile::Sport, &m, &s), Choice::Down(2));
}
#[test] fn memory_remembers_a_press_at_once_and_forgets_a_lift_slowly() {
    let mut m = Memory::default();
    m.update(1.0 / 60.0, 1.0, false, 0.0);
    assert!(m.load > 0.99 && m.aggr > 0.5, "{m:?}");
    for _ in 0..60 { m.update(1.0 / 60.0, 0.0, false, 0.0); }
    assert!(m.load > 0.3 && m.load < 0.6, "after a second: {}", m.load);
    for _ in 0..60 * 30 { m.update(1.0 / 60.0, 0.0, false, 0.0); }
    assert!(m.load < 0.01 && m.aggr < 0.05, "{m:?}");
}
#[test] fn a_stab_counts_more_than_a_slow_press() {
    let mut stab = Memory::default(); let mut slow = Memory::default();
    stab.update(1.0 / 60.0, 0.6, false, 0.0); stab.update(1.0 / 60.0, 0.6, false, 0.0);
    for k in 0..60 { slow.update(1.0 / 60.0, 0.6 * k as f32 / 60.0, false, 0.0); }
    assert!(stab.aggr > slow.aggr + 0.1, "stab {} slow {}", stab.aggr, slow.aggr);
}
#[test] fn garbage_in_holds_the_gear() {
    let m = Memory::default();
    for &(rpm, speed, gas) in &[(f32::NAN, 10.0, 0.5), (3000.0, f32::INFINITY, 0.5), (-5.0, -20.0, 2.0), (1e9, 1e9, f32::NAN), (0.0, 0.0, 0.0)] {
        for g in -1..=7 {
            let s = Situation { gear: g, rpm, gas, rate: f32::NAN, brake: true, speed, wheel_r: R, gearing: 1.0, lat_g: f32::NAN, slope: f32::NAN };
            match choose(Profile::Sport, &m, &s) { Choice::Hold => {}, Choice::Up(t) | Choice::Down(t) => assert!((1..=6).contains(&t) && (t - g).abs() >= 1, "{g} -> {t} for {rpm} {speed} {gas}") }
        }
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/bin/t shiftlogic`
Expected: compile error (the module `shiftlogic` has no items).

- [ ] **Step 3: Write the implementation**

Create `rust/src/game/shiftlogic.rs`:

```rust
//! The automatic gearbox's mind: when to change gear, as pure functions of the situation and a small memory of the driver, so the rules can be tested without a car.
//! A real automatic stays in the highest gear that pulls, climbs later the harder the pedal was pressed, drops one or several gears when the pedal is floored (kick-down),
//! steps down under braking for the engine brake, holds its gear in a corner and does not hunt (a pause between two shifts).
use super::drivetrain::{engine_torque, ratio, rpm_in_gear};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Profile { Comfort, #[default] Sport, Race }

impl Profile {
    /// 0 comfort, 1 sport, 2 race; anything else is sport.
    pub fn from_i32(v: i32) -> Profile { match v { 0 => Profile::Comfort, 2 => Profile::Race, _ => Profile::Sport } }
}

/// The numbers of one profile: the up-shift revs at a light touch and at the floor, the down floor likewise, the pause (s) before an up-shift and a down-shift,
/// the revs under which a floored pedal kicks down, the most revs a shift may land on, the revs the engine brake aims at and the time a shift takes.
#[derive(Clone, Copy, Debug)]
pub struct Table { pub up_lo: f32, pub up_hi: f32, pub floor_lo: f32, pub floor_hi: f32, pub dwell_up: f32, pub dwell_down: f32, pub kick_rpm: f32, pub over_rev: f32, pub brake_rpm: f32, pub shift_time: f32 }

pub fn table(p: Profile) -> Table {
    match p {
        Profile::Comfort => Table { up_lo: 2600.0, up_hi: 6000.0, floor_lo: 1400.0, floor_hi: 2400.0, dwell_up: 1.2, dwell_down: 1.5, kick_rpm: 2800.0, over_rev: 6200.0, brake_rpm: 3200.0, shift_time: 0.30 },
        Profile::Sport => Table { up_lo: 3800.0, up_hi: 7300.0, floor_lo: 1900.0, floor_hi: 3600.0, dwell_up: 0.9, dwell_down: 1.2, kick_rpm: 3300.0, over_rev: 6600.0, brake_rpm: 4300.0, shift_time: 0.18 },
        Profile::Race => Table { up_lo: 5200.0, up_hi: 7500.0, floor_lo: 3500.0, floor_hi: 5200.0, dwell_up: 0.5, dwell_down: 0.6, kick_rpm: 4200.0, over_rev: 7000.0, brake_rpm: 5200.0, shift_time: 0.10 },
    }
}

/// What the box remembers of the driver: the pedal (a press counts at once, a lift fades over about a second), the aggressiveness (a stab, the brake, a corner raise it, it fades over about 8 s)
/// and the seconds since its last shift.
#[derive(Clone, Copy, Debug)]
pub struct Memory { pub load: f32, pub aggr: f32, prev_gas: f32, pub since_shift: f32 }
impl Default for Memory { fn default() -> Memory { Memory { load: 0.0, aggr: 0.0, prev_gas: 0.0, since_shift: 9.0 } } }

impl Memory {
    /// One step; returns the pedal's rate of change (1/s).
    pub fn update(&mut self, dt: f32, gas: f32, brake: bool, lat_g: f32) -> f32 {
        let dt = dt.clamp(1e-4, 0.1);
        let gas = if gas.is_finite() { gas.clamp(0.0, 1.0) } else { 0.0 };
        let rate = (gas - self.prev_gas) / dt;
        self.prev_gas = gas;
        self.load = if gas > self.load { gas } else { self.load + (gas - self.load) * (dt / 1.2).min(1.0) };
        let lat = if lat_g.is_finite() { (lat_g.abs() / 0.7).clamp(0.0, 1.0) * 0.7 } else { 0.0 };
        let target = (0.5 * gas + 0.5 * (rate / 4.0).clamp(0.0, 1.0)).max(if brake { 0.5 } else { 0.0 }).max(lat);
        self.aggr = if target > self.aggr { target } else { self.aggr + (target - self.aggr) * (dt / 8.0).min(1.0) };
        self.since_shift += dt;
        rate
    }
    pub fn shifted(&mut self) { self.since_shift = 0.0; }
}

/// What the box sees.
#[derive(Clone, Copy, Debug)]
pub struct Situation {
    pub gear: i32, pub rpm: f32,
    /// The pedal (0..1) and its rate (1/s).
    pub gas: f32, pub rate: f32,
    pub brake: bool,
    /// The car's speed (m/s, along the heading), the wheels' radius (m) and the final drive multiplier of the tuning.
    pub speed: f32, pub wheel_r: f32, pub gearing: f32,
    /// The lateral acceleration (g) and the grade (the sine of the pitch, positive uphill).
    pub lat_g: f32, pub slope: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice { Hold, Up(i32), Down(i32) }

/// The gear the box wants now (never a gear outside 1..=6; garbage in holds the gear).
pub fn choose(p: Profile, m: &Memory, s: &Situation) -> Choice {
    let finite = s.rpm.is_finite() && s.speed.is_finite() && s.wheel_r.is_finite() && s.gearing.is_finite() && s.gas.is_finite();
    if !finite || s.gear < 1 || s.gear > 6 || s.rpm < 0.0 || s.wheel_r <= 0.0 { return Choice::Hold; }
    let (rate, lat_g, slope) = (if s.rate.is_finite() { s.rate } else { 0.0 }, if s.lat_g.is_finite() { s.lat_g } else { 0.0 }, if s.slope.is_finite() { s.slope } else { 0.0 });
    let t = table(p);
    let at = |g: i32| rpm_in_gear(s.speed, s.wheel_r, g) * s.gearing;   // the revs the wheels would give the engine in gear g
    let eff = m.load.max(0.7 * m.aggr).clamp(0.0, 1.0);
    let floor = t.floor_lo + (t.floor_hi - t.floor_lo) * eff + (slope.max(0.0) * 3000.0).min(900.0);
    let up_at = t.up_lo + (t.up_hi - t.up_lo) * eff;
    let moving = s.speed > 4.0;
    let braking = s.brake && moving && s.gas < 0.1;
    let lifted = s.gas < 0.05 && moving;
    let g = s.gear;
    // a floored or stabbed pedal at low revs asks for the gear with the most force at the wheels (it may skip gears), without the pause of an ordinary shift
    let stab = s.gas > 0.85 || (rate > 3.0 && s.gas > 0.5);
    if stab && g > 1 && s.rpm < t.kick_rpm && m.since_shift >= 0.35 {
        let force = |h: i32| engine_torque(at(h), 1.0) * ratio(h);
        let cur = force(g);
        let best = (1..g).filter(|&h| at(h) < t.over_rev).max_by(|&a, &b| force(a).partial_cmp(&force(b)).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(b) = best { if force(b) > cur * 1.12 { return Choice::Down(b); } }
    }
    // braking at speed: down to the gear whose revs are nearest to the engine brake's band
    if braking && g > 1 && s.rpm < t.brake_rpm - 400.0 && m.since_shift >= t.dwell_down {
        let target = (1..g).filter(|&h| at(h) <= t.over_rev - 600.0).min_by(|&a, &b| (at(a) - t.brake_rpm).abs().partial_cmp(&(at(b) - t.brake_rpm).abs()).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(h) = target { return Choice::Down(h); }
    }
    // up: only with the pedal down, not braking, not cornering, not coasting downhill, and only if the next gear still has the revs to pull
    let downhill_coast = slope < -0.04 && s.gas < 0.3;
    if g < 6 && !braking && !lifted && lat_g.abs() <= 0.35 && !downhill_coast && s.rpm > up_at && at(g + 1) > floor + 500.0 && m.since_shift >= t.dwell_up { return Choice::Up(g + 1); }
    // down: under the floor (lugging ignores the pause)
    if g > 1 && s.rpm < floor && at(g - 1) < t.over_rev && (m.since_shift >= t.dwell_down || s.rpm < 1300.0) { return Choice::Down(g - 1); }
    Choice::Hold
}

#[cfg(test)]
#[path = "shiftlogic_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/bin/t shiftlogic`
Expected: `test result: ok. 14 passed`. If `the_upshift_point...` or `an_uphill_grade...` fail by a few rpm, print the values with `eprintln!` in the test, adjust the **test's speed** (not the table) so the premise holds, and keep the assertion's meaning.

- [ ] **Step 5: Commit**

```bash
cd ~/projects/RallyNVX && git checkout -b feat/gearboxes
git add rust/src/game/shiftlogic.rs rust/src/game/shiftlogic_tests.rs rust/src/game/mod.rs
git commit -m "feat: shift logic as pure functions (profiles, memory, kick-down, braking, holds)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The drivetrain uses the shift logic

**Files:**
- Modify: `rust/src/game/drivetrain.rs` (struct `DriveIn`, struct `Drivetrain`, `new`, `reset`, `auto_gear`, `step`)
- Modify: `rust/src/game/drivetrain_tests.rs` (harness + tests)
- Modify: `rust/src/game/world.rs:313`

**Interfaces:**
- Consumes: Task 1's `Profile`, `table`, `Memory`, `Situation`, `Choice`, `choose`.
- Produces: `DriveIn` gains `lat_g: f32`, `slope: f32` and derives `Default`; `Drivetrain` gains `pub profile: Profile` and replaces `load_s`/`since_shift` by `mem: Memory`; the automatic's shift time comes from `table(profile).shift_time` (the `SHIFT_AUTO` constant is removed).

- [ ] **Step 1: Write the failing tests**

Append to `rust/src/game/drivetrain_tests.rs`:

```rust

#[test] fn a_stab_in_a_high_gear_kicks_down_several_gears() {
    let mut d = Car::new(); d.v = 22.0; d.run(0.12, 10.0);
    let g = d.dt.gear; assert!(g >= 5, "cruising in {g}");
    d.run(1.0, 0.6);
    assert!(d.dt.gear <= g - 2, "kick-down skipped to {} from {g}", d.dt.gear);
}
#[test] fn race_holds_its_gears_longer_than_comfort() {
    let run = |p: Profile| { let mut c = Car::new(); c.dt.profile = p; c.run(1.0, 3.0); (c.dt.gear, c.dt.last.rpm) };
    let (comfort, race) = (run(Profile::Comfort), run(Profile::Race));
    assert!(comfort.0 >= race.0, "comfort {comfort:?} race {race:?}");
}
#[test] fn brake_taps_do_not_hunt() {
    let mut c = Car::new(); c.run(1.0, 20.0); let mut shifts = 0; let mut last = c.dt.gear;
    for k in 0..60 * 12 {
        c.brake = (k / 20) % 2 == 0;                    // 1/3 s on, 1/3 s off
        let o = c.step(if c.brake { 0.0 } else { 0.4 });
        if o.gear != last { shifts += 1; last = o.gear; }
    }
    assert!(shifts <= 8, "{shifts} shifts in 12 s of brake taps");
}
#[test] fn reverse_and_the_selector_are_untouched() {
    let mut c = Car::new(); c.dt.reverse_sel = true; c.run(0.6, 3.0);
    assert_eq!(c.dt.gear, -1);
    c.dt.reverse_sel = false; c.run(0.0, 1.0);
    assert!(c.dt.gear >= 1, "back in drive: {}", c.dt.gear);
}
```

Also change the harness `Car::step` call to the new `DriveIn` (it has `brake: self.brake`; add the two fields via `..Default::default()`): replace
`&DriveIn { throttle: thr, speed: self.v, wheel_r: R, airborne: false, brake: self.brake }` with
`&DriveIn { throttle: thr, speed: self.v, wheel_r: R, brake: self.brake, ..Default::default() }` and the abuse test's literal
`&DriveIn { throttle: r * 2.0 - 1.0, speed: (r - 0.3) * 80.0, wheel_r: R, airborne: k % 500 < 20, brake: k % 7 == 0 }` with
`&DriveIn { throttle: r * 2.0 - 1.0, speed: (r - 0.3) * 80.0, wheel_r: R, airborne: k % 500 < 20, brake: k % 7 == 0, lat_g: r - 0.5, slope: r - 0.5 }`.

- [ ] **Step 2: Run to see the failure**

Run: `~/bin/t drivetrain`
Expected: compile errors (`Profile`, `lat_g`, `slope`, `profile` do not exist).

- [ ] **Step 3: Implement**

In `drivetrain.rs`:

1. Imports: after `use std::sync::OnceLock;` add `use super::shiftlogic::{choose, table, Choice, Memory, Profile, Situation};`.
2. `DriveIn`: add `#[derive(Default)]` to the derive list (`#[derive(Clone, Copy, Debug, Default)]`) and fields at the end:
```rust
    /// The lateral acceleration (g) and the grade (the sine of the pitch, positive uphill): the automatic holds its gear in a corner and downshifts uphill.
    pub lat_g: f32, pub slope: f32,
```
3. Remove `const SHIFT_AUTO: f32 = 0.18;` (keep `SHIFT_MANUAL` for now, Task 3 replaces it).
4. In `struct Drivetrain` replace `load_s: f32, since_shift: f32,` (and its doc comment) with
```rust
    /// What the automatic remembers of the driver, and its character.
    mem: Memory, pub profile: Profile,
```
and in `new()` replace `load_s: 0.0, since_shift: 9.0,` with `mem: Memory::default(), profile: Profile::Sport,`. In `reset()` keep the profile: add `let profile = self.profile;` before `*self = Drivetrain::new();` and `self.profile = profile;` after it.
5. Replace the whole `fn auto_gear` with:
```rust
    /// The automatic's choice of the gear (the rules are in `shiftlogic`).
    fn auto_gear(&mut self, i: &DriveIn, thr: f32, rpm: f32, dt: f32) {
        // R is chosen by hand (the autopilot alone asks for the reverse with a negative throttle); a car that merely rolls back in D takes the first gear as soon as the gas is pressed
        let holding = self.gear == -1 && i.speed < -0.5 && thr < 0.05;
        if self.reverse_sel || (self.auto_reverse && ((i.throttle < -0.1 && i.speed < 1.0) || holding)) { if self.gear != -1 { self.gear = -1; } return; }
        if self.gear <= 0 { self.gear = 1; self.mem.shifted(); return; }
        let rate = self.mem.update(dt, thr, i.brake, i.lat_g);
        if self.shift > 0.0 { return; }
        let s = Situation { gear: self.gear, rpm, gas: thr, rate, brake: i.brake, speed: i.speed, wheel_r: i.wheel_r, gearing: self.opts.gearing, lat_g: i.lat_g, slope: i.slope };
        match choose(self.profile, &self.mem, &s) {
            Choice::Up(g) => self.begin_shift(g, true),
            Choice::Down(g) => self.begin_shift(g, false),
            Choice::Hold => {}
        }
    }

    fn begin_shift(&mut self, g: i32, up: bool) { self.gear = g; self.shift = table(self.profile).shift_time; self.shift_up = up; self.mem.shifted(); }
```
6. In `step`, nothing else uses `load_s`; check with `grep -n 'load_s\|SHIFT_AUTO\|since_shift' drivetrain.rs` (expect no hits).
7. `world.rs:313`: extend the `DriveIn` literal:
```rust
let out = self.drivetrain.step(dt, &DriveIn { throttle: inp.throttle, speed: fwd, wheel_r: self.vehicle.spec.radius, airborne: self.vehicle.contacts == 0, brake: inp.brake,
    lat_g: self.vehicle.omega.dot(self.vehicle.up()) * fwd / 9.81, slope: self.vehicle.forward().y });
```
8. Make `rpm_in_gear`, `ratio`, `engine_torque` reachable: they are already `pub`.

- [ ] **Step 4: Run drivetrain, then the whole suite**

Run: `~/bin/t drivetrain` -> Expected: all pass (the 2.72.0 tests included). If `the_automatic_does_not_hunt_under_a_wandering_pedal` fails on "two shifts X s apart", the cause is the dwell: the `Down` kick (0.35 s) is allowed; relax that test's assertion to `> 0.3` (it already is) and check the shift pair is a kick-down; do not lower the dwells.
Run: `~/bin/t` -> Expected: all pass. If `without_traction_control_the_launch_is_slower` or the autopilot's `stage_tests` shift, report numbers; the TC margin is already 1%.

- [ ] **Step 5: Commit**

```bash
git add rust/src/game && git commit -m "feat: the automatic decides by the shift logic (profile, memory, kick-down, braking, holds)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The sequential manual box and the H-pattern

**Files:**
- Modify: `rust/src/game/drivetrain.rs` (`BoxType`, fields, `shift`, `step`)
- Modify: `rust/src/game/drivetrain_tests.rs`

**Interfaces:**
- Consumes: Task 2's `Drivetrain`.
- Produces: `pub enum BoxType { Sequential, HPattern }` (`Default` = Sequential, `BoxType::from_i32(i32)`: 1 is HPattern, else Sequential), `Drivetrain.box_type` (pub), consts `SEQ_UP = 0.08`, `SEQ_DOWN = 0.10`, `H_SHIFT = 0.20`.

- [ ] **Step 1: Write the failing tests**

Pin the existing clutch-dependent manual tests to the H-pattern: in `drivetrain_tests.rs` add after `fn new() -> Car` a helper
```rust
    fn h_manual() -> Car { let mut c = Car::new(); c.dt.box_type = BoxType::HPattern; c.dt.set_mode(Mode::Manual); c.dt.reset(); c }
```
and in each of the tests `the_manual_has_neutral_and_the_engine_revs_free_to_the_limiter`, `the_manual_stalls_when_the_clutch_is_let_go_in_gear_at_a_standstill_and_restarts`, `a_slipping_clutch_pulls_away_in_the_manual_and_a_full_release_follows`, `manual_shifts_go_through_neutral_to_the_reverse_cut_the_spark_and_take_a_moment` (both `c` and `d`), replace `let mut c = Car::new(); c.dt.set_mode(Mode::Manual); c.dt.reset();` by `let mut c = Car::h_manual();` (and `d` likewise). The shift-time waits in `manual_shifts...` use `c.run(0.0, 0.2)`; change them to `0.25` because the H shift is now 0.20 s.

New tests:

```rust

fn seq_manual() -> Car { let mut c = Car::new(); c.dt.set_mode(Mode::Manual); c.dt.reset(); c }

#[test] fn the_sequential_launches_without_a_clutch_pedal_and_does_not_stall() {
    let mut c = seq_manual(); assert_eq!(c.dt.box_type, BoxType::Sequential);
    c.run(0.0, 1.5); c.dt.shift(1); c.run(0.0, 0.3);
    assert_eq!(c.dt.gear, 1);
    let o = c.run(1.0, 3.0);
    assert!(c.v > 10.0 && !o.stalled, "{} m/s, stalled {}", c.v, o.stalled);
}
#[test] fn the_sequential_idles_in_gear_without_stalling() {
    let mut c = seq_manual(); c.run(0.0, 1.5); c.dt.shift(1); c.run(0.0, 0.3);
    let o = c.run(0.0, 6.0);
    assert!(!o.stalled && o.rpm > 600.0, "idle in gear: {} rpm", o.rpm);
    assert!(c.v < 3.0, "creeps at most: {}", c.v);
}
#[test] fn the_sequential_upshift_cuts_the_spark_briefly_and_is_quick() {
    let mut c = seq_manual(); c.run(0.0, 1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.run(1.0, 2.0);
    c.dt.shift(1); let o = c.step(1.0);
    assert!(o.cut, "flat shift: the spark is cut");
    let mut n = 0; while c.step(1.0).cut && n < 30 { n += 1; }
    assert!(n >= 2 && n <= 8, "the cut lasts {} steps (about 0.08 s)", n);
    assert_eq!(c.dt.gear, 2);
}
#[test] fn the_sequential_downshift_blips_the_throttle() {
    let mut c = seq_manual(); c.run(0.0, 1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.run(1.0, 2.5); c.dt.shift(1); c.run(1.0, 0.3);
    c.run(0.0, 0.5);
    c.dt.shift(-1); let o = c.step(0.0);
    assert!(o.throttle >= 0.55, "the blip: {}", o.throttle);
}
#[test] fn the_h_pattern_shift_takes_twenty_hundredths_and_ignores_a_second_request() {
    let mut c = Car::h_manual(); c.dt.set_clutch(1.0);
    c.dt.shift(1); assert_eq!(c.dt.gear, 1);
    c.run(0.0, 0.1); c.dt.shift(1); assert_eq!(c.dt.gear, 1, "still shifting");
    c.run(0.0, 0.15); c.dt.shift(1); assert_eq!(c.dt.gear, 2);
}
#[test] fn the_clutch_pedal_still_overrides_the_sequential() {
    let mut c = seq_manual(); c.run(0.0, 1.0); c.dt.shift(1); c.run(0.0, 0.3);
    c.dt.set_clutch(1.0);                              // the pedal down: the clutch is open whatever the revs
    let o = c.run(1.0, 1.5);
    assert!(c.v < 0.5 && o.rpm > 5000.0, "free revs with the pedal down: {} m/s {} rpm", c.v, o.rpm);
}
#[test] fn switching_the_box_during_a_shift_is_safe() {
    let mut c = seq_manual(); c.run(0.0, 1.0); c.dt.shift(1); c.run(0.0, 0.1);
    for k in 0..600 {
        if k % 17 == 0 { c.dt.box_type = if (k / 17) % 2 == 0 { BoxType::HPattern } else { BoxType::Sequential }; c.dt.shift(if k % 3 == 0 { -1 } else { 1 }); }
        let o = c.step(if k % 5 < 3 { 1.0 } else { 0.0 });
        assert!(o.force.is_finite() && o.rpm.is_finite() && o.rpm >= 0.0 && o.rpm < 12000.0, "step {k}: {o:?}");
    }
}
```

- [ ] **Step 2: Run to see the failure**

Run: `~/bin/t drivetrain`
Expected: compile error (`BoxType`, `box_type`).

- [ ] **Step 3: Implement**

In `drivetrain.rs`:

1. Constants: replace `const SHIFT_MANUAL: f32 = 0.12;` with
```rust
/// Seconds a shift takes in the sequential box (up with the spark cut, down with a blip) and in the H-pattern box (a synchro).
const SEQ_UP: f32 = 0.08;
const SEQ_DOWN: f32 = 0.10;
const H_SHIFT: f32 = 0.20;
```
2. After `pub enum Mode`, add:
```rust
/// The manual box: the sequential of the real car (shifts without the clutch pedal: the spark is cut on the way up, the throttle blipped on the way down; the pedal is only an override)
/// or the H-pattern with a clutch that must be worked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoxType { #[default] Sequential, HPattern }
impl BoxType { pub fn from_i32(v: i32) -> BoxType { if v == 1 { BoxType::HPattern } else { BoxType::Sequential } } }
```
3. In `Drivetrain`: add field `pub box_type: BoxType,` (after `pub profile: Profile,`), in `new()` `box_type: BoxType::Sequential,`, and keep it in `reset()` like the profile (`let box_type = self.box_type;` ... `self.box_type = box_type;`).
4. `shift()`: replace the body's time assignment `self.shift = SHIFT_MANUAL;` with
```rust
        if g != self.gear { self.gear = g; self.shift = match (self.box_type, dir > 0) { (BoxType::Sequential, true) => SEQ_UP, (BoxType::Sequential, false) => SEQ_DOWN, (BoxType::HPattern, _) => H_SHIFT }; self.shift_up = dir > 0; }
```
(i.e. update the existing `if g != self.gear { ... }` line accordingly).
5. In `step`:
   - after `let auto = ...;` add `let seq = !auto && self.box_type == BoxType::Sequential;`
   - the blip line: `if auto && self.shift > 0.0 && !self.shift_up && self.opts.blip` becomes `if (auto || seq) && self.shift > 0.0 && !self.shift_up && self.opts.blip`
   - spark cut: `let cut_shift = self.opts.flat_shift && self.shift > 0.0 && self.shift_up && pedal > 0.6;` becomes `... && pedal > if seq { 0.2 } else { 0.6 };`
   - the clutch capacity: factor the automatic's curve into a local closure and add the sequential:
```rust
        let auto_cap = (10.0 + (CLUTCH_MAX - 10.0) * smooth(1300.0, 3000.0, rpm0)).max(CLUTCH_MAX * smooth(2.0, 5.0, i.speed.abs())).min(CLUTCH_MAX);
        let pedal_cap = CLUTCH_MAX * smooth(0.85, 0.30, self.clutch_pedal);
        let cap = if g == 0.0 || self.shift > 0.0 || i.airborne { 0.0 }
            else if auto { auto_cap }
            // the sequential's electronic clutch bites by itself like the automatic's; the pedal, if pressed, can only open it more
            else if seq { if self.clutch_pedal > 0.05 { pedal_cap.min(auto_cap) } else { auto_cap } }
            else { pedal_cap };
```
     (replace the existing `let cap = ...` expression and keep its comment line above, extended with "the sequential has an electronic one").
   - starter: `let free = auto || self.gear == 0 || self.clutch_pedal > 0.6;` becomes `let free = auto || seq || self.gear == 0 || self.clutch_pedal > 0.6;`
   - the rev-matching torque: `let t_s = if auto && self.shift > 0.0 ...` becomes `let t_s = if (auto || seq) && self.shift > 0.0 ...`

- [ ] **Step 4: Run drivetrain tests, then the suite**

Run: `~/bin/t drivetrain` -> Expected: pass. If `the_sequential_downshift_blips_the_throttle` fails because the throttle reading comes after the shift timer decremented, assert `o.throttle >= 0.55` on the **first** step only (as written); if the first step already shows the idle governor's value because `self.shift` was decremented before the check, move the blip check to `self.shift > 0.0 || ...` is wrong; instead read `shift` before the decrement: the order in `step` is decrement first; the blip uses `self.shift > 0.0` after it, which is true for 0.10 s - dt, so the test holds.
Run: `~/bin/t` -> Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add rust/src/game && git commit -m "feat: the manual box is sequential by default (no clutch needed, spark cut up, blip down); H-pattern kept

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Settings from the UI to the drivetrain

**Files:**
- Modify: `rust/src/game_jni.rs` (enum line 36, dispatch ~96, extern ~198)
- Modify: `rust/src/game/app.rs` (struct fields line ~75, constructor line ~83, `set_clutch` neighbourhood ~108, `frame` ~176)
- Modify: `app/src/main/java/dev/starinin/rallynvx/{Native.kt,Controls.kt,GearMath.kt,GameActivity.kt}`, `app/src/test/java/dev/starinin/rallynvx/GearMathTest.kt`

**Interfaces:**
- Consumes: `BoxType::from_i32`, `Profile::from_i32`.
- Produces: JNI `Native.gameGearCfg(box: Int, profile: Int)`; `Controls.boxType/setBoxType`, `Controls.shiftProfile/setShiftProfile`; `GearMath.BOXES = 2`, `GearMath.PROFILES = 3`, `GearMath.boxName(i, en)`, `GearMath.profileName(i, en)`, `GearMath.next(i, n)`.

- [ ] **Step 1: Write the failing Kotlin test**

Append to `GearMathTest.kt` (inside the class):
```kotlin
    @Test fun boxAndProfileNames() {
        assertEquals("Sequential", GearMath.boxName(0, true)); assertEquals("H-коробка", GearMath.boxName(1, false))
        assertEquals("Sport", GearMath.profileName(1, true)); assertEquals("Гоночный", GearMath.profileName(2, false)); assertEquals("Sport", GearMath.profileName(99, true))
    }
    @Test fun nextWraps() { assertEquals(1, GearMath.next(0, GearMath.BOXES)); assertEquals(0, GearMath.next(GearMath.BOXES - 1, GearMath.BOXES)); assertEquals(0, GearMath.next(-5, 3)) }
```

- [ ] **Step 2: Run to see it fail**

Run: `cd ~/projects/RallyNVX && ./gradlew --no-daemon testDebugUnitTest -q 2>&1 | tail -15`
Expected: compile errors in `GearMathTest` (missing members).

- [ ] **Step 3: Implement the Kotlin helpers**

`GearMath.kt`, inside the object:
```kotlin
    /** The manual box types (0 sequential, the real car's; 1 H-pattern with a clutch) and the automatic's characters (0 comfort, 1 sport, 2 race). */
    const val BOXES = 2
    const val PROFILES = 3
    fun boxName(i: Int, en: Boolean): String = if (i == 1) (if (en) "H-pattern" else "H-коробка") else (if (en) "Sequential" else "Секвентальная")
    fun profileName(i: Int, en: Boolean): String = when (i) { 0 -> if (en) "Comfort" else "Комфорт"; 2 -> if (en) "Race" else "Гоночный"; else -> if (en) "Sport" else "Спорт" }
    /** The next value of a cycle of `n`; a value out of range starts it again. */
    fun next(i: Int, n: Int): Int = if (i < 0 || i >= n - 1) 0 else i + 1
```
Check the test `next(-5,3) == 0` and `next(99...)`: ok. `Controls.kt` (next to `manual`):
```kotlin
    /** The manual box: 0 sequential (the real car's), 1 H-pattern with a clutch. */
    fun boxType(c: Context): Int = p(c).getInt("box_type", 0).coerceIn(0, GearMath.BOXES - 1)
    fun setBoxType(c: Context, v: Int) = p(c).edit().putInt("box_type", v.coerceIn(0, GearMath.BOXES - 1)).apply()
    /** The automatic's character: 0 comfort, 1 sport (the default), 2 race. */
    fun shiftProfile(c: Context): Int = p(c).getInt("shift_profile", 1).coerceIn(0, GearMath.PROFILES - 1)
    fun setShiftProfile(c: Context, v: Int) = p(c).edit().putInt("shift_profile", v.coerceIn(0, GearMath.PROFILES - 1)).apply()
```
`Native.kt` (after `gameClutch`):
```kotlin
    @JvmStatic external fun gameGearCfg(box: Int, profile: Int)   // the manual box type (0 sequential, 1 H-pattern) and the automatic's character (0 comfort, 1 sport, 2 race)
```

- [ ] **Step 4: Implement the Rust plumbing**

`game_jni.rs`: add `GearCfg(i32, i32)` to `enum GCmd` (line 36, e.g. after `Clutch(f32)`), a dispatch arm after `Ok(GCmd::Clutch(c)) => g.set_clutch(c),`:
```rust
                            Ok(GCmd::GearCfg(b, p)) => g.set_gear_cfg(b, p),
```
and the extern after `gameClutch`:
```rust
    #[no_mangle] pub extern "system" fn Java_dev_starinin_rallynvx_Native_gameGearCfg(_e: JNIEnv, _c: JClass, boxt: jint, profile: jint) { send(GCmd::GearCfg(boxt, profile)); }
```
`app.rs`: add `gear_cfg: (i32, i32),` to the struct (line 75 list, after `tuning: [f32; ...]`), `gear_cfg: (0, 1),` to the constructor (line 83 after the `tuning: [1.0; ...]`), a method next to `set_clutch`:
```rust
        pub fn set_gear_cfg(&mut self, boxt: i32, profile: i32) { self.gear_cfg = (boxt, profile); }
```
and in `frame()` right after `self.world.manual = self.manual; ...` line, inside the `else if !self.paused` block, add:
```rust
                self.world.drivetrain.box_type = crate::game::drivetrain::BoxType::from_i32(self.gear_cfg.0); self.world.drivetrain.profile = crate::game::shiftlogic::Profile::from_i32(self.gear_cfg.1);
```
Run `~/bin/t` (compiles the cdylib lib tests) and expect green.

- [ ] **Step 5: Implement the activity rows**

`GameActivity.kt`: add a private function near `gearLabel()`:
```kotlin
    private fun sendGearCfg() { if (running) Native.gameGearCfg(Controls.boxType(this), Controls.shiftProfile(this)) }
```
Send once at start: after the line `Native.gameGearMode(if (manual) 1 else 0)` (line ~459) add `Native.gameGearCfg(Controls.boxType(this), Controls.shiftProfile(this))`.
In `showCarPanel`, after the `for (bit in 0 until CarMath.AIDS) { ... }` loop add:
```kotlin
        // the gearbox: the manual's type and the automatic's character; one button each, a tap goes to the next
        fun cycleRow(title: String, value: () -> String, onTap: () -> Unit) {
            val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
            r.addView(TextView(this).apply { text = title; textSize = 14f; setTextColor(M3.TEXT) }, LinearLayout.LayoutParams(0, -2, 1f))
            val b = M3Widgets.button(this, value(), M3Widgets.ButtonKind.TONAL) {}
            b.setOnClickListener { onTap(); (b as? TextView)?.text = value() }
            refreshers.add { (b as? TextView)?.text = value() }
            r.addView(b, LinearLayout.LayoutParams(px(150f), px(42f)))
            colA.addView(r, LinearLayout.LayoutParams(px(330f), -2).apply { topMargin = px(6f) })
        }
        cycleRow(t("Manual box", "Ручная коробка"), { GearMath.boxName(Controls.boxType(this), en) }) { Controls.setBoxType(this, GearMath.next(Controls.boxType(this), GearMath.BOXES)); sendGearCfg() }
        cycleRow(t("Automatic", "Автомат"), { GearMath.profileName(Controls.shiftProfile(this), en) }) { Controls.setShiftProfile(this, GearMath.next(Controls.shiftProfile(this), GearMath.PROFILES)); sendGearCfg() }
```
Run: `./gradlew --no-daemon testDebugUnitTest -q 2>&1 | tail -15` -> Expected: no failures.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: the car menu picks the manual box type and the automatic's character

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Documentation, version, release

**Files:**
- Modify: `CHANGELOG.md`, `CHANGELOG.ru.md`, `docs/game.md`, `app/build.gradle.kts`
- Create: `~/projects/releases/rallynvx-2.73.0.md`

- [ ] **Step 1: Changelog (EN)**: insert above `## 2.72.0` in `CHANGELOG.md`:
```
## 2.73.0 - a real automatic and a sequential box
- Changed: the **automatic** is rebuilt on a separate shift logic. Three characters in the car menu: Comfort (early, smooth), Sport (the default) and Race (holds the gears near the power peak, 0.10 s shifts). It remembers the pedal and the driver's **aggressiveness** (a stab, the brake and a corner raise it, it fades in about 8 s), picks the **gear with the most force** on a kick-down (it may skip several gears), steps down under braking to the engine brake's band, holds its gear in a corner and does not climb on a lift or coasting downhill, drops a gear uphill, and keeps a pause between shifts so it does not hunt
- Changed: the **manual** box is now **sequential**, as on the real car: no clutch pedal needed (an electronic clutch bites by itself and the pedal only opens it more), an up-shift in 0.08 s with a short spark cut, a down-shift in 0.10 s with an automatic blip. The **H-pattern** box with the clutch pedal is kept (car menu: "Manual box"), with a 0.20 s synchro
- Tests: the shift rules as pure functions (hold on a lift, skip-gear kick-down, braking band, corner hold, grade, dwell, garbage input), the drivetrain with both boxes (no stall in the sequential, spark cut, blip, H-pattern timing, switching the box during a shift). Not seen on a device yet
```
RU entry in `CHANGELOG.ru.md` above `## 2.72.0` (same content, Russian):
```
## 2.73.0 - настоящий автомат и секвентальная коробка
- Изменено: **автомат** перестроен на отдельную логику переключений. В меню машины три характера: Комфорт (рано, плавно), Спорт (по умолчанию) и Гоночный (держит передачи у пика мощности, переключение 0,10 с). Он помнит педаль и **агрессивность** водителя (удар по педали, тормоз и поворот повышают её, она спадает за ~8 с), при kickdown выбирает **передачу с наибольшей силой** на колёсах (может пропустить несколько), при торможении понижает до полосы торможения двигателем, держит передачу в повороте, не повышается на сбросе и на накате под уклон, понижает в горку и выдерживает паузу между переключениями, чтобы не метаться
- Изменено: **ручная** коробка теперь **секвентальная**, как на настоящей машине: педаль сцепления не нужна (электронное сцепление захватывает само, а педаль может только раскрыть его сильнее), повышение за 0,08 с с короткой отсечкой зажигания, понижение за 0,10 с с автоматической перегазовкой. **H-коробка** с педалью сцепления сохранена (меню машины: «Ручная коробка»), синхронизатор 0,20 с
- Тесты: правила переключений как чистые функции (удержание на сбросе, kickdown с пропуском передач, полоса торможения, удержание в повороте, уклон, пауза, мусор на входе), трансмиссия с обеими коробками (нет остановки двигателя в секвентальной, отсечка, перегазовка, время H-коробки, смена коробки во время переключения). На устройстве пока не видел
```
- [ ] **Step 2: Docs**: in `docs/game.md` replace the paragraph starting `**Automatic gearbox / Автомат.**` (added in 2.72.0) with a description of `game/shiftlogic.rs` (profile tables, memory, the order of the rules: kick-down, braking, up, down; the sequential box's electronic clutch) in the same EN + RU style; keep it under 12 lines.
- [ ] **Step 3: Version and notes**: in `app/build.gradle.kts` set `versionCode = 103`, `versionName = "2.73.0"`. Write `~/projects/releases/rallynvx-2.73.0.md` (EN paragraph + RU paragraph, the same short style as `rallynvx-2.72.0.md`).
- [ ] **Step 4: Full verification**: `~/bin/t 2>&1 | tail -4` (Expected: `test result: ok.`), `./gradlew --no-daemon testDebugUnitTest -q 2>&1 | tail -5` (Expected: no failures).
- [ ] **Step 5: Merge and release**:
```bash
git add -A && git commit -m "docs: 2.73.0 gearboxes

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
git checkout main && git merge --no-ff feat/gearboxes -m "Merge feat/gearboxes" && ~/bin/rt-release 2.73.0 2>&1 | tail -4
```
Expected: the release URL `https://github.com/Xtratter/RallyNVX/releases/tag/v2.73.0`.
