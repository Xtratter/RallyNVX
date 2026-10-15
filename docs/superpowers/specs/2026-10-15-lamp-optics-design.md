# Lamp optics: lens, beam, glow, position lights

Status: design approved in chat 2026-10-15. Turn signals and hazard lights are tech debt (not in this stage).

## Intent
Cars (the Peugeot now, later models) get believable lamps at night: lamps that look like lenses with reflectors, a real low beam, a soft glow around lit lamps, and position lights (front white, rear red). Everything is found on the car's mesh by colour class (`rcar::lamp_points`), so a new model needs no hand-entered coordinates. Success: at night a lamp reads as a lens with a bright core, the beam has a cut-off and falls off with distance, lit lamps have a halo that follows the viewing angle, and position lights burn with the low beam.

## Today
- Lens: material 7 (`MAT_CARLAMP`), a flat colour times a state factor (`raster.wgsl`, `m == 7`).
- Beam: one cone from the midpoint of the headlights (`hl_pos`, `hl_dir`, `smoothstep` on the cone angle), used for the ground, the sky haze and the dust.
- Rally car: no glow (`own_lamps` skips it); indicators (orange) exist in the mesh but are unused; no position lights.

## Parts
1. **Lens** (shader, material 7). Off: dark glossy glass (sky reflection, thin rim). On: a hot core that is brighter when the view runs along the lamp's axis, a Fresnel-dark rim, a stepped reflector pattern from the vertex's local position. Same rules for tail, reverse and position lamps (colour class decides the colour).
2. **Beam.** Two beams from the real left/right headlight positions, low beam only: wide sideways, a sharp upper cut-off that rises on the right (right-hand traffic), intensity falling as 1/d^2 with a floor. The same function lights the ground, the haze and the dust, replacing the single cone. No high beam.
3. **Glow.** A soft billboard around each lit lamp (headlight, position, brake, reverse): size and brightness grow when looking along the lamp's axis, fade from the side and from behind; also on the rally car.
4. **Classes.** `lamp_points` returns more classes: head (main unit), position (small white front / red rear unit beside the main one, lit with the low beam), tail, reverse. The outermost cluster of enough vertices on the right-hand side wins, the left is its mirror; a mesh lacking a class falls back to the nearest existing one (no panic). Indicators stay found but unused.

## Constraints
- Same 30 fps target; the beam function is evaluated per pixel only where the old cone was.
- Boxy generated cars keep their glowing boxes and the old cone (no regression).
- Docs: `docs/vehicles.md` (classes by colour) and the changelog, EN + RU.

## Tests (host)
Class search on the embedded asset (positions of head/position/tail/reverse); fallbacks for a mesh without a class; beam: intensity at the axis, outside the cut-off, falls with distance, cut-off higher on the right; glow factor by view angle; WGSL validated (`shaders_validate`). Device check only after the user's yes.

## Tech debt (explicit)
Turn signals and hazard lights (blinking, buttons), high beam, light turning with the steering.
