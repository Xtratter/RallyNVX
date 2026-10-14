# Car lights (v2.13.0, with day and night) - design

Cycle 2 of the night work. Chat design approved by the user on 2026-10-14 ("да, давай").

## Goal

At dusk and at night the car switches its headlights on by itself: a cone of light lights the ground, trees and buildings ahead. Tail lights glow red, brighter when braking; white reverse lights glow when the car reverses, and a faint coloured light falls on the ground behind it.

## Not in this cycle

A headlight toggle button or setting (automatic only), light shadows, headlights of other vehicles, a dashboard.

## Design

- **State** (`game/carlights.rs`, pure): `CarLights { head: bool, brake: bool, reverse: bool }` from the vehicle and the daylight `night` factor. `head` is on when `night > 0.25`; `brake` when the brake or the handbrake is pressed; `reverse` when the car moves backwards faster than 0.5 m/s or the throttle asks for reverse. `Vehicle` stores `braking`/`reversing` (set in `step` from the input).
- **Lamp positions** (`lamps(&Vehicle) -> Lamps { head: [V3; 2], tail: [V3; 2], rev: [V3; 2], head_dir: V3 }`): from `spec.size` (front/rear at +-(length/2 - 0.08) from the centre, x at +-0.34 of the width) and a lamp height above the ground by style (SUV 0.95 m, rally 0.62 m), rotated and moved with the body. `head_dir` is the forward vector pitched down 3 degrees.
- **Light** in `raster.wgsl` `shade_with` (all lit surfaces: terrain, models, instanced meshes, city): a single spot cone from the middle of the two headlamps, half-angle 22 degrees with a soft edge (smoothstep over 0.06 of cosine), range 55 m with a linear falloff, warm white, intensity about 9 times the albedo, `n.l` shading with a floor of 0.25 so the beam reaches far, no shadows. A second term, a small point light behind the car (radius 7 m), red when braking (0.5), white when reversing (0.8).
- **Visible lamps**: glowing small cubes (the existing glow instance kind) at the lamp positions: head lamps white when `head`, tail lamps dim red when `head`, bright red when braking, reverse lamps white when reversing.
- **Uniform**: four more `vec4` appended to `Uni` (`hl_pos` xyz + intensity, `hl_dir` xyz + cos(outer), `tl_pos` xyz + unused, `tl_col` rgb + radius): 352 -> 416 bytes.
- **Debug**: intent extra `lights` (0 off, 1 force brake, 2 force reverse; forces the headlights on) through `Native.gameLightsDebug(mode)` for screenshots.

## Testing

Pure host tests: lamp positions (front lamps ahead of the centre, tail behind, symmetric across the centre line, plausible height above the ground, for both cars and several yaws); light state (day off, dusk on, brake, reverse by speed, reverse by throttle); `Vehicle::step` sets `braking`/`reversing`. Device: night screenshots with the headlights, braking and reversing; fps check (the cone adds a few ALU per fragment).
