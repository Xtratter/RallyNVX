# Stage A3: suspension that follows the wheels

Part of `2026-10-15-peugeot-full-model-roadmap.md`. Approved by delegation ("do what you think is best") after A1 (2.31.0) and A2 (2.32.0).

## Intent
Through the wheel arches and under the car the Peugeot is hollow: no wishbones, struts or springs. The source model has them (`x0_wishbone_*`, `x0_suspension*_*`, about 700 triangles a corner, one 1024 texture). Success: the suspension is there and moves with the wheel's travel (jumps, bumps, cornering roll): the outer ends follow the hub, the inner ends stay on the body.

## Design
1. **Parts in the body mesh.** `tools/carconv` adds the suspension parts of all four corners to the body mesh as class 14 (`MAT_CARSUSP`) with their texture in the atlas (the `SUSP` image, a 256 tile). For every vertex: `col.r` = weight 0..1 (1 at the hub, 0 at the far end of its part: `1 - distance to the hub / the part's largest distance`), `col.g` = the corner (0 front-left (+x), 1 front-right, 2 rear-left, 3 rear-right, in the mesh's frame: +x is the left side), `col.b` = 0. No new slot, no extra draw call.
2. **Moving it.** `vs_model` displaces a class-14 vertex up by `u.susp[corner] * weight` in the body frame; `Uni.susp: vec4` holds the four hub offsets in metres: the compression of the wheel minus the static compression (the body mesh sits at the static one), clamped to +-0.25 m, mapped to the mesh corner by the sign of the wheel's x and z (`vehicle_models::susp_offsets`). The fragment shader uses the texture only (class 14 ignores the vertex colour).
3. **Cost.** About 2 800 triangles more in the body mesh, no extra pass; the dust's SDF includes them (inside the shell, harmless).
4. **Docs.** `docs/vehicles.md` (class 14, the weights), `ASSET-LICENSE.md` (suspension parts), changelog EN + RU.

## Constraints
fps unchanged within a frame or two; lamp points, lamp interiors, livery, wheel sets and bounds tests stay green (the body bounds may not grow: the parts are inside the shell); the generated cars untouched.

## Tests (host)
`susp_offsets`: zero at rest, positive when a wheel compresses more, the right corner for each wheel (fl/fr/bl/br by sign), clamped, finite; the embedded asset has class-14 vertices with weights in 0..255, corners 0..3 present, the textured share; the converter: weights are 1 at the hub and 0 at the far end of a part (unit test of the weight function); the WGSL validates. Device check (after the user's yes): the arches show the suspension, it moves over a bump, fps.

## Not in A3
Steering of the tie rods, anti-roll bars, springs that compress in length, exhaust movement.
