# Tyre dust and spray (v2.17.0) - design

Dust cycle of the rally roadmap. Design presented in chat; the user chose "your variant" (about 400 particles, with an off/normal/lots setting) on 2026-10-14.

## Goal

Wheels throw up dust that depends on the surface under them: beige clouds on gravel and dirt, dense yellow clouds on sand, almost nothing on dry grass, a little tyre smoke on tarmac under a hard slide, spray on shallow water. The cloud rises, widens, slows down, is carried by the wind and fades in 2-4 s. Its brightness follows the sun and the night.

## Not in this cycle

Dust from other vehicles (there are none), settling dust that stains the car, tracks/ruts in the road shader (a later cycle), sound of gravel, particle lighting by the headlights, soft particles (depth fade).

## Design

- **Surface data** (`rust/surfaces.json`, per surface a `dust` object): `color` (RGB), `roll` (emission while rolling, 0..1), `slide` (emission while sliding, 0..2), `size` (m), `life` (s). Top level `dust`: `cap_normal` (400), `cap_lots` (800), `wind_speed` (2.5 m/s). The existing loader parses them (same override rules, same ranges check).
- **Wheel info** (`Vehicle.wheels: [WheelInfo; 4]`, set by the last sub-step): `grounded`, `surface`, `contact` (world point), `speed` (ground speed of the contact point), `lat_slip` (|sideways speed|), `slide` (tyre saturation excess `max(0, force demand / grip limit - 1)`, clamped to 1.5).
- **Emission** (`game/dust.rs`, pure, deterministic): per wheel per step `rate = roll * 14 * min(speed / 20, 1.5)` (only above 2 m/s) `+ slide_k * (60 * slide + 25 * min(lat_slip / 6, 1.5))` particles per second, times the quality factor; fractional parts accumulate per wheel. Spawn at the contact point (+-0.2 m, 0.1 m up) with velocity = 25 % of the wheel's ground velocity + a random spread (+-0.6 m/s) + 1.2-2.2 m/s up; life = `life` x (0.75..1.25), colour = `color` x (0.9..1.1).
- **Motion**: the velocity relaxes towards the wind (`wind_at(seed, t)`: a direction from the seed that wanders slowly, speed `wind_speed` x (0.6..1.0)) with rate 1.1/s, a small buoyancy (+0.15 m/s up that fades), size grows from `size` to 3.5 x `size` as `age^0.6`, alpha = start alpha x fade-in (0.08 life) x `(1 - age/life)^1.5`; the ground clamps the height (+0.05 m). A ring buffer of `cap` particles: when full the oldest are replaced.
- **Setting**: `dust` = 0 off / 1 normal / 2 lots (default 1): cap 0 / 400 / 800 and the emission factor 0 / 1 / 1.5. Android: a row in the settings screen (`Controls.dust`), sent with `Native.gameDust(level)`; the game applies it to every new world.
- **Rendering**: one instanced pipeline of camera-facing quads (`vs_dust`/`fs_dust`) after the water pass, alpha-blended, no depth write, depth tested. Instance = position + size, colour + alpha; sorted back to front on the CPU every frame (at most 800). The fragment makes a soft round puff, multiplies the colour by `0.30 + 0.70 * sun` (the sun's colour intensity, so night is dark) and fades into the sky colour with distance like the fog.
- **Debug**: intent extra `steer` (-1..1, held after 1.5 s together with `throttle`) for screenshots of a slide.

## Testing

Pure host tests: the table (embedded values, dust parsing and errors); wheel info (a sliding wheel reports a saturated tyre, a rolling one does not); emission (standing: none; gravel at speed: some; asphalt rolling: none; asphalt hard slide: some, sand > gravel > dirt > grass); the cap and the ring; particles die after their life; determinism for a seed; wind carries them; size grows and alpha falls; NaN and extreme inputs never poison the system; level 0 emits nothing. Render: `dust_instances` sorting and counts. Device: screenshots of a gravel run and a slide at day and at night, the settings row, fps with 800 particles.

## Risks

- Overdraw: 800 large quads close to the camera can cost fill rate; the caps keep it bounded and the setting turns it off.
- The soft-edge look without depth fade: puffs cut through the ground slightly where they touch it; the 0.05 m clamp and the upward velocity hide most of it.
