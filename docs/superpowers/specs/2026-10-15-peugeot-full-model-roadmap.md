# Using the whole Peugeot 206 WRC model: roadmap

Status: roadmap approved in chat 2026-10-15 ("use everything"). Each stage is its own spec -> plan -> release.
Source: `/sdcard/claude/peugeot-206-wrc-wwwvecarzcom.zip` (CC BY-NC-SA 4.0; 653 nodes, 243 meshes, 57 materials, 60 images, about 185 000 triangles, no animations or skeleton: every part is its own node, so code moves them).

## What is used now (2.30.0)
Body shell (13 500 of 48 000 triangles, livery atlas), lamp units with their reflector interiors, one wheel (tarmac, vertex colours) for all four wheels.

## Stages
- **A. Running gear** (first). A1: the wheel as a real textured model (rim and tyre atlas: sidewall lettering, tread, rim style), brake disc and caliper on each wheel (hot discs glow at night after braking), mud flaps, exhaust, spare wheel visible through the hatch. A2: the three wheel sets (tarmac, gravel, snow: rims and tyres) chosen in the garage, and by surface/weather later. A3: suspension arms and dampers that follow the wheel's travel (a two-bone link between the body mount and the wheel hub).
- **B. Glass and interior.** An alpha-blended pass for the car's glass (windows, lamp covers) so the interior shows through; the interior meshes (dashboard, seats, door panels, roll structure, gear stick, pedals, handbrake), the driver and the co-driver (helmets, suits, gloves). LOD: the interior is decimated (about 15 000 triangles) and drawn only when the camera is near or looks into the car; its own 2048 atlas.
- **C. Cockpit and moving parts.** A cockpit camera view; the steering wheel turns with the input, the driver's hands follow, the gear display and the RPM LED strip work, the pedals move, the wipers (with the weather tech debt), the co-driver's pace-notes sheet.
- **D. Details.** The roof light bar (garage part, night), the aerial, normal maps (body, tyres, rims, lamps), a higher-detail body by distance (LOD).
- **E. Housekeeping every stage.** `ASSET-LICENSE.md` and the credits (the share of the derivative grows), the fps budget on the POCO F3 (target 30, now 39-45), APK size (now 5.9 MB), docs EN + RU, `tools/carconv` flags.

## Order and why
A first: visible on every frame, no renderer change, the converter path (atlas, UV, nodes) is proven. B needs a render pass and is the biggest visual jump. C builds on B. D is polish.

## Risks
Triangle budget (the interior alone is 75 000 triangles): LOD and decimation per stage with a measured fps; atlas memory (each atlas 2048 x 2048 is about 22 MB with mips): share atlases or lower resolution for the interior; glass blending order with the dust pass; the file format grows (`RCA2` carries one mesh pair now: wheel sets and parts need a multi-mesh container, `RCA3`).
