# Stage B: transparent glass and the interior

Part of `2026-10-15-peugeot-full-model-roadmap.md`. Approved by delegation ("do what you think is best"). After A (2.31.0-2.33.0).

## Intent
The Peugeot's windows and lamp covers are opaque dark glass and the cabin is empty. The source model has real glass (alpha-blended) and a full interior (about 74 000 triangles: dashboard, seats, steering wheel, door panels, cabin shell, a driver of 14 000 triangles). Success: from near the car the windows are transparent and you see the dashboard, seats, steering wheel and the driver; the lamp covers are clear over the reflectors; from afar the windows look dark and solid as now; the shape and the physics do not change; fps stays near 40.

## Design
1. **Meshes in `RCA3`.** Up to eight meshes: body, wheel, disc, caliper, wheel gravel, wheel snow, **glass** (index 6), **interior** (index 7). The body no longer contains glass. Glass = the windows and the windscreen (`x0_window_*`) with vertex colour red 0, and the clear lamp covers (`Light_front_*_glass`, the front turn covers, `Ligth_rear_glass`, `Light_bar_glass`) with red 255 ("clear"). Interior = the cabin parts (`int_*`, `Int_doors_*`) and the driver (helmet, suit, gloves, face), decimated to about 16 000 triangles in total, class 15 (`MAT_CARINT`), UV into its **own** 1024 x 1024 atlas `rally_car_interior.jpg` (the interior textures are many and dark; a separate small atlas keeps the livery atlas as it is, +4 MB with mips).
2. **Glass pass.** A new alpha-blended pipeline (`pl_glass`: depth test, no depth write, no culling) draws the glass mesh after the opaque car and before the dust passes. `fs_glass`: a dark blue-grey tint, the sky reflected by Fresnel, a sun glint; alpha `0.2 + 0.5 * fresnel` near the car, rising to 0.95 beyond 9..16 m (a smoothstep on the camera distance) so a far car has solid dark windows like today and nothing is seen through the cabin from afar. Lamp covers (red 255) get a quarter of that alpha. Glass does not cast shadows and is not in the depth buffer (the dust composite draws over it).
3. **Interior draw.** One more model slot (`SLOT_INTERIOR`), opaque, with the body's pose, drawn only when the camera is within 16 m of the car (beyond that the glass is opaque and hides it). Material 15 samples the interior atlas (bindings 10/11), is lit like plastic but darkened by a cabin factor (0.55: no sun shadow inside, the roof shades it) and keeps the lamp state out of it. The steering wheel is part of the interior for now (it turns in stage C).
4. **Dust SDF.** The body's signed distance field (dust wraps the car) is built from the body mesh plus the glass mesh, otherwise the window holes would let the dust into the cabin.
5. **Docs.** `docs/vehicles.md` (the eight meshes, the interior atlas, class 15), `ASSET-LICENSE.md` (interior, driver and glass parts; the interior atlas), changelog EN + RU.

## Constraints
fps within a few frames of now (39-55 on the POCO F3; the interior is skipped beyond 16 m); APK +about 1.5 MB; atlas memory +4 MB with mips; lamp points, lamp interiors, livery, wheel sets, suspension and bounds tests stay green; the generated cars (offroader, the hatchback fallback) keep their opaque glass in the body mesh (class 1 still works in `fs_model`).

## Tests (host)
Format: `RCA3` with 8 meshes parses, 7 and fewer still do (no glass or interior: `None`), 9 is an error; the embedded asset has a glass mesh (red 0 and red 255 vertices, none with class != 1), an interior of 12 000 to 18 000 triangles with UVs, the body without class-1 vertices; the SDF input includes the glass (a unit test on the vertex list builder); alpha function `glass_alpha(dist, fresnel, clear)` (in Rust, mirrored in WGSL) rises with distance and stays in 0..1, clear covers are fainter; the interior is skipped beyond 16 m (a pure function `draw_interior(dist)`); WGSL validates. Device check (after the user's yes): near a windscreen the dashboard, seats and the driver are seen, far away the windows are dark, lamps still look right, fps.

## Not in B
The steering wheel turning, the driver's hands, wipers, the cockpit camera (stage C); interior lighting at night (dash lights); mirrors' reflection; rain on the glass.
