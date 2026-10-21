# The driveline: gearboxes, wheels, differentials and the race car's systems

Status: draft for review (2026-10-21). Author's decisions made with the user are marked **[decided]**; the rest are recommendations.

## 1. What the user wants (and what we assume)

Said: make the automatic right "as it is done properly", review the manual box; simulate the differentials "and the other systems a race car has"; study how it is done and make a plan; the wheels are modelled in full **[decided]**.

Assumed: the target is the Peugeot 206 WRC ("Vulpes WRC") as a *credible* rally simulator that fits 30 fps on a POCO F3; fidelity is chosen where the player can feel it (launches, traction on gravel, corner exit, braking), not for its own sake; every stage ships as its own version with tests; nothing is seen on a device by the author, the user judges the feel.

Success: (a) the automatic never shifts "against logic" in the situations listed in 3.1; (b) the sequential box behaves like the real car's (no clutch, spark cut, blip); (c) the four wheels spin, slip and lock on their own, so the diffs, ABS and traction control act on real wheel speeds; (d) a differential change is felt (a locked centre diff pushes the nose wide, an open rear spins the inside wheel); (e) 30 fps is not lost (the whole stack costs under 0.3 ms of a frame on the phone).

## 2. Research (what real cars and good simulators do)

- Automatic shift logic: stays in the highest useful gear, **kick-down** by one or several gears with the pedal, holds gears on grades, adapts to the driver ([cdahmedeh](https://www.cdahmedeh.net/blog/2018/4/8/automatic-transmission-simulation-in-games), [Adaptive Auto Shifter](https://www.overtake.gg/downloads/adaptive-auto-shifter.85931/): pedal position and speed, load, brake, aggressiveness memory, hunting protection, over-rev protection, Comfort/Normal/Sport; [a fork's goals](https://github.com/AnnoyingTechnology/assetto-corsa-real-automatic-gearbox/blob/main/README.md): no up-shift with zero throttle or the brake on, a pause between down-shifts, several gears down under braking; BeamNG's [controller](https://documentation.beamng.com/modding/vehicle/vehicle_system/controller/main/vehiclecontroller/shiftlogic-automaticgearbox/): torque converter and lock-up).
- The real car: a 6-speed **sequential** Xtrac box and electronically controlled 4WD, about 50/50 nominal ([WRC.com](https://www.wrc.com/en/misc/peugeot-206-wrc), [rallycars.com](https://rallycars.com/cars/peugeot-206-wrc/myspecs/); the 50/50 is a single source: a base value, not a fact); the WRC generation had mechanical LSDs front and rear and an active centre diff driven by throttle, steering and speed ([Yahoo](https://autos.yahoo.com/wrc-car-center-differential-explained-134100037.html)).
- Wheel and tyre: the wheel is a rotating body, `dw/dt = (T_drive - T_brake - R Fx) / I`; the traction force follows the slip ratio `(w R - v) / |v|` with a peak near 6 % ([Car Physics for Games](https://www.asawicki.info/Mirror/Car%20Physics%20for%20Games/Car%20Physics%20for%20Games.html)).
- Limited-slip: a clutch-pack LSD adds a locking torque = preload + drive torque x ramp factor (separate for power and coast) that pulls the two wheels to their mean speed ([thread](https://www.gamedev.net/forums/topic/695333-limited-slip-differential-and-friends/), [Lenzo](https://shura.shu.ac.uk/30090/1/Lenzo-RevisitingMechanicalLimited-slip(VoR).pdf): 45 deg power and 30 deg coast ramps, 6 friction faces are a motorsport example).
- A race car's other systems ([Assetto Corsa](https://en.wikipedia.org/wiki/Assetto_Corsa), [Automobilista 2](https://automobilista2.wiki.gg/wiki/Features), [Live for Speed](https://en.wikipedia.org/wiki/Live_for_Speed)): brake bias, anti-roll bars, tyre pressure/temperature/wear, clutch and engine overheating, damage, fuel and mass, launch control, traction control, ABS.

## 3. Design

### 3.1 Stage 1 — the gearboxes (version 2.73.0; changes `game/drivetrain.rs`, no new layer)

**Automatic** (`auto_gear`, today's 2.72.0 logic is the base). Add:
1. *Profiles* Comfort / Sport / Race (a setting in the gearbox menu). Each is a table of up-shift rpm over the load, the down floor, the dwell (0.9 / 0.6 / 0.4 s), the shift time and the kick-down aggressiveness. Race = the sequential's near-power-peak shifts.
2. *Aggressiveness memory* `a` in 0..1: raised by the pedal's speed (a stab counts more than a slow press), the brake, the lateral acceleration and a high throttle, decays over ~8 s. It moves the up-shift point and the down floor, and means a car that was driven hard does not climb at the first lift.
3. *Best-gear kick-down*: with a floored or quickly-pressed pedal pick the gear that gives the largest force at the wheels (engine torque x ratio at that gear's rpm, under the over-rev limit), not just one down; it may skip gears. Pause 1.2 s between ordinary down-shifts; a kick-down ignores it.
4. *Cornering hold*: above 0.35 g lateral, no up-shift; braking before a corner steps down through the gears (already so, tuned by `a`).
5. *Grade*: `DriveIn.slope` (the sine of the pitch): uphill raises the down floor, downhill with the brake keeps the engine brake.
6. *Over-rev protection and a torque converter*: the existing 6600 rpm check for down-shifts stays; the launch clutch curve stays (it is the converter's stall), plus a creep torque with the brake released.

**Manual**: two box types, chosen in the gearbox menu; the Peugeot's default is the sequential **[recommended]**.
- *Sequential* (the real car): shift with paddles/buttons, no clutch pedal needed (an electronic clutch opens for a launch and a stop). Up-shift: the spark is cut for 60 ms, the box changes in ~80 ms, no blip; down-shift: an automatic blip, ~100 ms; shifting at the limiter is allowed. The clutch pedal, if the player uses it, still works as an override (a launch with a slipping clutch).
- *H-pattern* (today's): the clutch pedal is needed; adds a synchro time of 150 to 250 ms, a stall, no automatic blip unless the assist is on.
Tests: the 2.72.0 set plus: pedal-speed memory, multi-gear kick-down choice, cornering hold, grade, both manual box types' shift sequences and timings.

### 3.2 Stage 2 — the wheels (2.74.0; new `game/wheel.rs`, rewrites the longitudinal part of `Vehicle::step_with`) **[decided: full model]**

- Each wheel has a state `w` (rad/s) and an inertia `I_w` (wheel + tyre + disc + half-shaft share, ~1.3 kg m2 for 0.326 m at 7.5 kg wheel and tyre, tunable in `rust/wheels.json`).
- Longitudinal force from the slip ratio `k = (w R - vf) / max(|vf|, v_min)` through a curve `Fx = Fz mu g(k)` with a peak at `k_peak` (0.06 tarmac, 0.10 to 0.15 loose surfaces) and a fall to 0.8 to 0.85 of the peak when sliding (per surface in `rust/surfaces.json`). The lateral force keeps today's model; the two are combined by the friction circle that `step_with` already applies, so the cornering feel is kept and the traction is new.
- Integration: the wheel's equation is stiff at low speed (the slip stiffness is far above `I / dt`), so each substep (4 per frame, as the vehicle already has) integrates it **semi-implicitly** (one linearised implicit step, the slope of the curve kept positive). The engine and the clutch are solved **together with the wheels' inertia in the same substeps** by an implicit update of their relative speed (an explicit once-a-frame coupling would make the clutch chatter as soon as a wheel spins); at |v| < 0.5 m/s with the brake or handbrake the wheel locks (`w = 0`) as a constraint, which also keeps the parked car still on a slope (today's handbrake behaviour).
- Brake: a torque on the wheel, `brake_force x R / 4` split by the **brake bias** (front/rear; default 0.55 front, closer to equal than first thought so that the stopping distance stays within the regression anchors; the setting comes in 2.76.0), the handbrake on the rear; ABS releases a wheel's torque to hold `k` near -0.12; TC cuts the engine torque or brakes a spinning wheel to hold `k` near +0.10 (these replace today's "reduce the grip when beyond the peak" shortcut; with the aids off the wheel really locks or spins).
- `WheelInfo` gains `w`, `slip` (for the dust, the tyre noise, the squeal and the tracks, which today guess from the car's lateral speed).
- Compatibility: the legacy path (`drive_force: None`, `engine_force / 4`) stays for any car without a drivetrain.
- Tests: a free wheel under a driving torque reaches the slip peak and settles; a locked wheel slides at the reduced force; the braking distance and the 0 to 80 km/h time stay within 5 % of 2.72.0 on tarmac with the aids on (this is the regression anchor), and get worse by the measured margins with the aids off; energy is conserved through the clutch/wheel coupling; no NaN under abuse.

### 3.3 Stage 3 — the driveline and the differentials (2.75.0; new `game/driveline.rs`, `game/diff.rs`)

Chain: engine (inertia) -> clutch -> gearbox -> **centre diff** -> front and rear propshaft -> **front and rear diff** -> half-shafts -> four wheels. The engine's speed is now fed by the wheels' own speeds (today it is fed by the car's speed through `w_in`).

- *Coupling stiffness*: the clutch/driveline coupling is solved **implicitly** (the relative speed of the two sides has an exact update, `d_new = (d + h (Te/Ie + ...)) / (1 + h k (1/Ie + 1/Ii))`), so it stays stable at the wheel substep rate instead of needing a tiny step.
- *Diff types* (each a pure function `(T_in, w_a, w_b, params) -> (T_a, T_b)` so it is testable alone):
  - Open: equal torques; the speeds follow the loads.
  - Limited slip (clutch pack with ramps): locking torque `T_lock = preload + |T_in| x ramp(power or coast)`, applied `+-` so that it pulls the faster wheel back and the slower one forward, capped by the pack's capacity; parameters: preload, power ramp, coast ramp, plates.
  - Viscous: torque proportional to the speed difference.
  - Locked (spool): a stiff coupling.
  - Torsen-like (torque-bias ratio) as a later option.
- *Centre diff*: a torque split (base 50/50, adjustable 30 to 70 front/rear) with a locking coupling; the **active** mode raises or lowers the lock by the throttle, the steering angle, the speed and the brake (a 3D map in `rust/diffs.json`: corner entry unlocks for turn-in, exit locks for traction).
- *Defaults for the Peugeot*: front and rear LSD (preload 40 N m, power 45 deg-equivalent ratio 0.35, coast 0.20), centre active, base 50/50. Marked as approximations.
- *Car menu* (Kotlin): per diff a type, preload, power and coast lock; centre split and mode; presets Tarmac / Gravel / Snow.
- Tests: open diff passes equal torque and the inside wheel spins in a corner on low grip; an LSD moves torque to the slower wheel up to its capacity; a locked centre diff understeers (yaw rate smaller for the same steering) and an open one lets one axle spin; torque conservation (`T_a + T_b = T_in`); the sum of wheel torques equals the engine torque x ratio x efficiency; regression of the straight-line acceleration.

### 3.4 Stage 4 and later (each its own version; ordered by what the player feels)

1. 2.76.0 *Braking and launch*: brake bias adjustable in the car menu, front/rear disc heat per wheel (the glow exists), launch control (an rpm hold with the clutch bite and a TC target slip), the handbrake as a plain rear brake (decision 2 below).
2. 2.77.0 *Tyres*: temperature (heated by slip, cooled by speed), pressure, wear; grip over the thermal window; per-surface coefficients.
3. 2.78.0 *Chassis setup*: anti-roll bars front/rear, camber and toe, ride height, dampers bump/rebound, saved per surface preset.
4. 2.79.0 *Heat and damage*: clutch temperature (slipping, hill starts), coolant and oil temperature with a cooling flow from speed, turbo overheating, damage to body/suspension/engine from impacts (affects handling, power).
5. 2.80.0 *Fuel and mass*, *ESC* (a yaw-rate controller using the steering and the lateral acceleration, brakes single wheels; off by default for a rally sim).
Out of scope for now: a real tyre brush/Pacejka model; the hybrid systems of the Rally1 era; multiplayer.

## 4. Performance and risks

- Cost: 4 wheels x 4 substeps x ~100 flops, 3 diffs, one implicit coupling: negligible next to the renderer; measured on the host by a timing test with a budget of 0.05 ms per world step.
- Risk 1 (high): the handling changes when traction becomes slip-based. Mitigation: stage 2 is tuned against the 2.72.0 numbers (acceleration, braking, cornering benches) and ships with the aids on by default; the old model can be restored from the tag.
- Risk 2 (medium): numerical stiffness (see the semi-implicit wheel and implicit coupling); tests with random abuse, as the drivetrain already has.
- Risk 3 (medium): the autopilot and its tests were tuned on the old traction; its benches (`stage_tests`) are rerun after stage 2 and its style numbers retuned if the lap times shift by more than 5 %.
- Risk 4 (low): the second phone's loading hang is unrelated but must stay diagnosable; no changes to loading.
- Unverified by the author on a device: the *feel*; the user judges it. Every version states what the tests cover.

## 5. Decisions (user, 2026-10-21)

1. The Peugeot's default box is the **sequential**, as on the real car **[decided]**.
2. The handbrake works **as on the real car** **[decided]**: a plain rear brake that locks the rear wheels (no grip "assist"); the slide comes from the physics of the locked wheels.
3. After stage 3 the **tyres** come first (2.77.0), then the chassis setup (2.78.0) **[decided]**.
