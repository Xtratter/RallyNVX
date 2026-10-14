# Grip by surface and a loose gravel layer (v2.16.0) - design

Physics cycle after the roads. The user chose option 2 (with the loose layer). Design presented in chat and approved on 2026-10-14.

## Goal

The car feels different on asphalt, gravel, dirt, grass, sand and rock. On the gravel stage the loose top layer is swept off by the wheels into a packed line (more grip) and stays loose at the sides (less grip), as in a real rally.

## Not in this cycle

Tyre dust and tracks per surface (dust cycle), engine/transmission/suspension realism (later physics cycles), mud, weather, tyre wear, brake-fade.

## Design

- `game/grip.rs` (pure): `SurfaceGrip { mu, lateral, rolling }` multipliers per `Surface` (asphalt 1.00 / 1.0 / 1.0; gravel 0.62 loose .. 0.78 packed / 0.75 / 1.3; dirt 0.70 / 0.8 / 1.2; grass 0.60 / 0.7 / 1.5; sand 0.50 / 0.6 / 3.0; rock 0.80 / 0.9 / 1.0; shallow water 0.55 / 0.7 / 1.0), applied to the car's own `mu` (tuned for tarmac), `lateral_k` (tyre side stiffness) and `rolling`.
- Loose layer: `LooseMap` holds, for every stage road, a grid of 1 m along x 0.5 m across (+-6 m) of looseness 0..1, starting at 1. A wheel on gravel lowers the looseness of the cells under it by `0.35 * speed * dt` (so one pass at 20 m/s packs a cell by about a third); `loose_at` reads it. Gravel grip = packed + (loose - packed) * looseness. The map belongs to the `World` and resets with a new level.
- `RoadHit` gets a signed `side` (offset across the road, positive to the left) for the map lookup.
- `surface_at` treats the flat test terrain (`scale == 0`, no roads) as asphalt, so every existing physics test keeps its tarmac behaviour; a test-only `Vehicle.force_surface` lets tests put the car on any surface.
- `Vehicle::step_with(.., &mut LooseMap)` looks up the surface under every wheel contact (`surface_at`, road hit for gravel), scales `mu`, `lateral_k`, `rolling` per wheel, wears the loose map, and stores the surface (`Vehicle.surface`, for the HUD). `Vehicle::step` keeps its old signature (an empty map).
- HUD: state slot `[28]` carries the surface code; the HUD line shows its name (grass, dirt, sand, rock, gravel, asphalt, water; RU names).

## Testing

Pure: the table ordering and bounds; looseness interpolation; the map wears along a pass and not outside it. Physics (host): braking distance from 20 m/s orders asphalt < gravel < grass < sand; steady cornering lateral acceleration orders asphalt > gravel > sand; the Vulpes 0-100 km/h stays 3.5-4.4 s on asphalt and is at least 1.3 times slower on gravel; after five passes over a straight stage the line is packed (looseness < 0.5) and 1.2 m beside it still loose; a gravel pass is never faster in braking than asphalt. Device: the HUD shows the surface name; speed after 8 s of full throttle on the liaison versus the stage; a drift on gravel.

## Risks

- Off-road driving gets noticeably slipperier (grass 0.6 of the tarmac grip): the numbers are tuned by feel after the device check.
- Per-wheel `surface_at` runs 16 times per frame (4 wheels x 4 sub-steps): cheap (hash lookup + a noise evaluation), measured on the device.
