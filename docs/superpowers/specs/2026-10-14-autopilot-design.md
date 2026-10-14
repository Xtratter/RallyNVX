# Autopilot (in v2.18.0 with the stage relief) - design

Requested by the user on 2026-10-14: a button next to Reset / New level turns on an autopilot that drives the stage "perfectly" on its own; it is also the test driver for the author. Design approved in chat the same day. Decisions: the button sits beside Reset and New level (no pause button exists); off the road the autopilot finds its own way to the stage and follows it to the finish; it ships together with the stage relief as v2.18.0.

## Goal

One tap: the car joins the special stage (from anywhere), drives it fast and cleanly to the finish and stops there. Never leaves the road, never flips, never gets stuck (it recovers). Any touch of the wheel or pedals by the player hands control back.

## Not in this cycle

Driving the liaison or the city (they are not connected to the stage: the city between them is step B), overtaking or traffic (none exists), tuning for a specific car beyond its own numbers, a racing-line editor, an autopilot in the garage.

## Design

- `game/autopilot.rs` (pure, host-tested): `Autopilot::new(&World-free inputs)`, `drive(&mut self, v: &Vehicle, t: &Terrain, dt: f32) -> AutoCmd { input: Input, reset: bool, done: bool }`. No state outside the struct; deterministic.
- `rust/autopilot.json` (embedded, override `autopilot.json` beside the profile / `RAYTRACE_AUTOPILOT`, same loader pattern as `roads.json`): `lat_factor` 0.75 (share of the surface's cornering limit used), `brake_factor` 0.7, `look_base` 6 m, `look_gain` 0.45 s, `offset_max` 0.55 (fraction of the free half width used to cut corners), `crest_speed` 24 m/s, `field_speed` 12 m/s, `stuck_time` 4 s, `finish_stop` 25 m.
- Route (`Route`): points with arc length `s`, curvature and a target speed. Stage: the road's centre line moved sideways by a smooth offset that lowers the curvature (relaxation like the road's own, limited to `offset_max * (half - 1.4 m)`). Target speed `v = min(car max, sqrt(lat_factor * mu_surface * g / |kappa|))`, with the surface's grip from `grip::grip`, a backward pass limiting it by the braking distance (`brake_factor * mu * g`) and a forward pass limiting it by the engine; the speed is also capped at `crest_speed` within 40 m before a crest top (found from the profile) so the car lands tamely. Surface grip uses the loose-gravel value (the worst case).
- Joining: if the car is within 15 m of the stage axis, it joins at the nearest point; otherwise `Autopilot` plans a field path with the roads' A* (`path::Grid`, 32 m cells, built once on activation) from the car to the nearest stage point, smoothed with `shape::smooth_curve`, driven at `field_speed`, then switches to the stage route. The stage start is only used when it is nearer than any other stage point.
- Controller: pure pursuit: the look-ahead point at `look_base + look_gain * v` ahead along the route; desired wheel angle `atan(L * 2 sin(alpha) / d)` (wheelbase from the spec), converted to the `steer` input by the car's own steering limit (`steer_max / (1 + v / steer_speed_ref)`). Throttle and brake from the speed error (P plus feed-forward of the next target speed), brake applied through `brake_amt`. No handbrake.
- Safety: stuck (speed below 1 m/s for `stuck_time` s while the target speed is above 3 m/s) or flipped -> `reset = true` (puts the car upright on the spot) and, when it is more than 6 m off the route, `reset` to the nearest route point (a new World call `reset_on_route`). Finish: within `finish_stop` m of the end it brakes to a stop and reports `done`.
- Integration: `World.autopilot: Option<Autopilot>`; in `World::step` the commanded `Input` replaces the player's (the `reset` flag of the player still works); `done` turns it off. Commands: `GCmd::Autopilot(bool)`, `Native.gameAutopilot(on)`; any player input (touch on the pedals or wheel, tilt beyond a threshold) sends off; state slot `[29]` carries the autopilot flag; the HUD line shows "autopilot / автопилот". The button (tonal, beside Reset and New level, highlighted while on) toggles it. Debug extra `autopilot 1` starts it after the level is up (for devtest). A new level or a reset by the player turns it off.

## Testing

Pure: route curvature and target speeds (a circle gives `sqrt(lat * mu * g * R)`, a straight gives the car max, the backward pass brakes before a hairpin, the offset never leaves the allowed width and never lengthens the route by more than 2 %); pursuit steering on a circle; Joining picks the nearest point. Closed loop on the host with the real `Vehicle`: a synthetic S-bend, a hairpin, a crest on a straight, a gravel stage; then the whole generated stage for seeds 1..=6 (`Terrain::with_roads`, 100+ s of simulation each, run in parallel): finishes, never farther than half width + 0.5 m from the axis, never flipped, mean speed >= 18 m/s (65 km/h), no reset triggered. From the field: the car starts off the road 100-400 m away on several seeds and reaches the stage. Device: one run of a stretch with screenshots and fps.

## Risks

- Whole-stage runs are slow on the host (about 6 km at 20 m/s is 300 s of physics per seed): the parallel closed-loop test uses 4-6 seeds and `--release`-free numbers; if too slow it is `#[ignore]`d and run by the author.
- A field path may pass close to trees (props are not in the planner): the stuck recovery handles it; if it is too poor the field speed drops.
