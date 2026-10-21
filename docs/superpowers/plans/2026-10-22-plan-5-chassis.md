# Chassis setup Implementation Plan (version 2.78.0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The suspension setup of the car in the car menu: **anti-roll bars** (front, rear), **camber** (front, rear) and **toe** (front, rear), with presets for tarmac, gravel and snow. They change how the car rolls, how its grip is shared between the axles and how it turns in.

**Architecture:** A pure module `game/chassis.rs` (the anti-roll bar's force, the tyre's load sensitivity, the camber grip, the toe angles) and a `Chassis` setup struct that `Vehicle` holds. In the wheel loop of `Vehicle::substep`: the bar's force joins the suspension force of the axle's two wheels; the grip `fmax` is scaled by the load sensitivity (a tyre's grip grows less than its load, so a bar that moves load to one axle costs that axle grip: that is how the bars change the balance) and by the camber factor (the static camber plus the body's roll); the toe turns each wheel's heading. Six tuning parts (13 to 18, multipliers; the menu shows degrees) and a preset button.

**Spec:** `docs/superpowers/specs/2026-10-21-driveline-design.md` (section 3.4, item 3); the ride height and the dampers are left for later (tech debt).

## Global Constraints
- Branch `feat/chassis`; release `~/bin/rt-release 2.78.0`; versionCode 110; EN + RU changelog; notes `~/projects/releases/rallynvx-2.78.0.md`.
- Never launch the game on the phone without being asked; report tests only for what they cover.
- The regression anchors (`anchors_tests.rs`) and the autopilot benches hold with the defaults (the car as built: the bars 1.0, camber and toe as in `chassis::BASE`).
- Numbers (first guesses): the bars' base stiffness front 20000, rear 12000 N m/rad (the springs' own roll stiffness is about 33000 per axle); load sensitivity `1 - 0.12 (fs / fs0 - 1)` clamped 0.6..1.15; camber grip `1 - 0.006 (camber_eff + 1)^2` clamped 0.8..1 (best at -1 degree), the effective camber = the static one (+ outward) + 0.6 x the body's roll towards the wheel's outside; toe -1..1 degrees: half on each wheel, the base front toe-out 0.0, rear toe-in 0.1; base camber front -1.5, rear -1.0; the multipliers: bar 0..2 of the base, camber `-1.5 v` / `-1.0 v` degrees, toe `(v - 1) 0.4` degrees (more than 1 is toe-out).

## Tasks
1. `chassis.rs` + tests: bar force (zero for equal compression, antisymmetric, proportional), load factor, camber grip (best near -1, symmetric, floor), toe angle per wheel.
2. Vehicle integration + tests: roll angle in a turn falls with stiffer bars; a front-heavy bar understeers, a rear-heavy one oversteers; camber has an optimum; toe-out heats the tyres on a straight; anchors hold.
3. Tuning parts 13 to 18, world wiring + tests.
4. Kotlin: part names, ranges, degree texts, a chassis preset row + tests.
5. Docs, release.
