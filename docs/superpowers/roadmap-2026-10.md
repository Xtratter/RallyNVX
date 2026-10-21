# Roadmap, October 2026 (car game)

Order agreed with the user on 2026-10-14. Each item is brainstormed (spec, plan) when its turn comes; v2.13.0 ships items 1 and 2 together.

1. **Day and night** - DONE on `feat/day-night` (spec/plan `2026-10-14-day-night*`).
2. **Car lights** - headlights lighting the ground, tail and brake lights, reverse lights (`2026-10-14-car-lights*`). Releases v2.13.0 with item 1.
3. **Camera control** - rotate the chase camera by touch; it returns behind the car when the car starts moving forward; when reversing the camera swings to the front of the car so the player sees what is behind it.
4. **Roads** - asphalt roads in the open terrain (join the start and the cities), with markings, higher grip; also gives the "asphalt" surface.
5. **Tyre dust** - physically plausible dust and spray from the wheels per surface (grass, sand, dirt, mud, asphalt, water): amount and colour from the surface, wheel slip, speed and sun/night light; needs a surface lookup (`game/materials.rs` weights, roads, mud near water).
6. **Realistic car physics** (tyres, suspension, transmission, drift), then flight, sling cargo and water (aircraft are removed from the game for now).

Update 2026-10-14 (late): the old delivery/race missions were removed (v2.14.0, free drive only, garage free). The rally mode (special stages with times) is a planned OPTION beside free drive; the stage runner, markers, HUD and results will be built for rally when item 4 and the roads exist.

Tech debt (user, 2026-10-14): the wind's side force on the car (a light push at speed in the open) is NOT done; the wind (direction/speed/gusts) is visible and audible only (dust, clouds, grass and tree sway, wind sound).

Tech debt (user, 2026-10-14): weather conditions (rain, fog, snow, wet surfaces) and the seasons (summer, autumn, winter, spring: foliage, ground colour, snow, grip) are NOT done; planned later.

Roads order (user, 2026-10-14): first A (stage relief: crests/jumps, ditches and banks, roadside props), then B (topology: junctions, side roads, city super-special loop), then the rally mode. Research on real rally stages first.

Tech debt (user, 2026-10-14): the autopilot's further improvement: machine vision (the car sees the road through a camera/depth view instead of reading the route) and self-training (the car learns by itself how to drive faster and steadier: reinforcement learning or numeric tuning of the style numbers over many simulated runs). Not now; the current autopilot (`game/autopilot`, styles in `rust/autopilot.json`) stays as the baseline and the reference driver for tests.

Tech debt (user, 2026-10-14): recording your own pace notes (the player records or edits the co-driver's calls for a stage); the generated notes with the system voice (R2, v2.23.0) come first.

## Tech debt after the driveline stages (2026-10-21, versions 2.72 to 2.75)

Left undone on purpose; each item is a candidate for its own small version.
- **Tyres and wheels:** trail-braking understeer is not modelled (the side force has priority while the tyre grips; a real tyre loses side grip under braking); the wheel's inertia, the slip peaks and the sliding shares are constants in `rust/src/game/wheel.rs`, to move into `surfaces.json` / `vehicles.json`; tyre temperature, pressure and wear (planned 2.77.0).
- **Driveline:** the Peugeot's differentials are code (`Driveline::peugeot()`), not in `vehicles.json`; no spool / locked differential kind (an LSD with the full ramp is nearly locked); the active centre lock is a formula, not a tunable 3D map; the numbers are approximations of the real car (the sources name the layout only); the car menu's preset label does not follow a manual change of the parts.
- **Brakes:** the front/rear bias default is a constant (`wheel::BRAKE_BIAS = 0.62`) and the same for every car (the menu setting exists since 2.76.0); launch control has one fixed launch rpm (4800) and does not adapt to the surface; no handbrake bias or brake fade.
- **Gearboxes:** the sequential's idle creep is the automatic's; the box type and the automatic's character are not shown on the driving screen; no clutch overheating; no H-pattern grinding when the clutch is not pressed.
- **Engine and sound (older):** the tuning's turbo is not linked to the audio turbo whistle; the engine torque still comes from a table, not from the audio engine model; audio stage 4 (tyres on surfaces, water, rain, trees) not done; the shimmer of the hot air over the exhaust pipe is not done; ESC is not implemented.
- **Verification:** none of the 2.72 to 2.75 feel (shifts, wheelspin, ABS, handbrake, the differentials) has been seen on a device; the second phone's loading hang and the Settings crash are waiting for its log.

