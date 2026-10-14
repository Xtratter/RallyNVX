# Day and night (v2.13.0) - design

Cycle 1 of 3 (day/night, car lights, roads). Approved in chat 2026-10-14: approach A (one light source: the sun until it sets, then the moon), night "dark but passable".

## Goal

A real day-night cycle: the sun and the moon are visible in the sky, stars appear, dusk and dawn colour the sky, the night is dark enough that headlights will matter (cycle 2) but the level stays playable by moonlight. City windows and street lamps glow at night.

## Not in this cycle

Car headlights, tail and reverse lights (cycle 2), roads (cycle 3), a clock in the HUD, weather, a setting for the cycle length.

## Time model (`game/daylight.rs`)

- `DAY_SECONDS` stays 720 s; the phase runs 0..1 with phase 0 = midnight and phase 0.5 = noon (today phase 0 is dawn and the sun never sets).
- Sun elevation `e(phase) = MID - AMP * cos(2 pi phase)` with `MID = 12 deg`, `AMP = 55 deg`: midnight -43 deg, noon +67 deg; above the horizon about 56 % of the cycle, below -12 deg (deep night) about 30 %.
- The level start phase comes from the seed but only inside the day band (sun elevation between +8 and +50 deg, rising or falling): a level never starts at night; night arrives while driving. `daylight(seed, t)` keeps its signature.
- The sun azimuth drifts as today. The moon sits opposite the sun, turned around the vertical axis by a per-level offset in [-70, +70] deg so that its phase differs between levels.

## Light model

`Daylight` gains: `light` (direction of the one light the world uses, `sun` today), `light_col`, `sun` (the sun disc direction), `moon` (direction), `night` (0 day .. 1 deep night), `sun_vis` stays implicit in `sun.y`.

- Sun light fades with elevation: full at +12 deg, zero at -4 deg (smoothstep).
- Moon light: blue-white, colour value about 0.18 of the noon sun, fades in as the moon rises above 0 and out as it sets.
- The light direction switches from sun to moon only where both intensities are near zero (between -4 and -9 deg of the sun) so shadows never jump; `light`/`light_col` are what the shaders and the hybrid shadows already read as `u.sun`/`u.sun_col`.
- Ambient follows `night`: the shader's fixed ambient floor (0.20, 0.22, 0.26) is multiplied by `mix(1, 0.22, night)` and the sky term already follows `sky_top`.
- Sky colours by sun elevation, interpolated through four key states: day (as today), golden hour/sunset (warm horizon, as today's "warm"), twilight (horizon deep orange to violet, zenith dark blue, e between 0 and -10 deg), night (zenith 0.012/0.02/0.05, horizon 0.03/0.05/0.10). Fog and the clear colour use `sky_hor`, so the distance darkens with the night.

## Sky (`shaders/raster.wgsl`, `fs_sky`)

- Sun disc: angular radius about 1.2 deg, bright core (HDR value so it bloom-looks after tonemapping), a soft limb and the existing glow; reddish at low elevation through `sun_col`; hidden below the horizon.
- Moon disc: angular radius about 1.0 deg, grey-white, soft glow, phase from the angle between the moon and the sun directions (lit side towards the sun); only drawn above the horizon.
- Stars: a hash-based point field on the direction, a few thousand visible points, per-star brightness and slow twinkle, scaled by `night` and by cloud cover; none near the horizon haze.
- Clouds darken to a blue-grey at night (their lit colour is multiplied by the light colour already; the minimum floor is lowered with `night`).

## Night details

- City windows (`emi > 2.5` branch): a hash of the window cell picks about 45 % of windows to emit a warm colour scaled by `night` (a small value even at dusk).
- Lamps (material 5, street lamps): already emissive; they are brighter relative to the dark scene automatically; raise the emissive factor with `night`.
- Water reflects the (dark) sky and the moon glint uses the light direction; no change besides the uniforms.

## Uniforms

`Uni` (320 B) gets two more `vec4`: `moon` (xyz direction, w phase offset) and `disc` (xyz sun-disc direction, w `night`). Size assertion becomes 352 B; `struct U` in `raster.wgsl` is extended in the same order. Any other shader that declares the same `U` struct (`scene.wgsl`/`trace.wgsl` belong to the old ray tracer and use their own uniforms) is not touched.

## Debug and testing

- Pure host tests in `daylight_tests.rs`: elevation range and continuity; the start phase is always in the day band for 1000 seeds; night fraction about 30 % below -12 deg; the light direction never jumps (consecutive 0.5 s samples differ by under 0.02) while the light intensity is above a small threshold; light colour continuity; all values finite and non-negative; the cycle repeats; moon is opposite the sun plus the offset.
- A debug start time: `Native.gameTimeOfDay(phase)` (JNI, `GCmd::TimeOfDay`) overrides the world clock offset; the `GameActivity` intent extra `tod` (0..1) calls it at start. Used by `devtest.sh` for screenshots at midnight (0.0), dawn (0.25), noon (0.5), dusk (0.72). Not exposed in any menu.
- Device check on the POCO F3: screenshots at the four times, a frame-rate check (stars and moon must not cost more than about 1 fps in the hybrid mode).

## Risks

- The hybrid mode's terrain ray shadows and height atlas read `u.sun`: they get the light direction, which at the sun-moon switch has near-zero intensity, so no visible jump.
- Night makes the car hard to see before the headlights exist (cycle 2): the moon light keeps terrain readable; the release for this cycle is held until cycle 2 is ready unless the user wants it earlier.
