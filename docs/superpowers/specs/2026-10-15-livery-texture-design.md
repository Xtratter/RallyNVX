# Livery texture for the rally car

Status: approach A approved in chat 2026-10-15 (a real texture instead of baked vertex colours).

## Intent
The Peugeot's livery (stickers, sponsor logos, patterns) looks blurred because `tools/carconv` samples the source textures per vertex of a decimated mesh (12 000 triangles) with a 5x5 box average. The source (`/sdcard/claude/peugeot-206-wrc-wwwvecarzcom.zip`, CC BY-NC-SA 4.0, body textures up to 6144x4096) has sharp livery. Success: the livery is as sharp on screen as the source allows at a 2048-pixel atlas, the shape and physics do not change, nothing else slows down, and the next car models use the same path.

## Design
1. **Asset format `RCA2`.** Vertices gain a UV (`u16x2`, unorm; the sentinel 0xFFFF,0xFFFF = "no texture, use the vertex colour"). The vertex struct grows from 28 to 32 bytes (`Vert { pos, nrm, col, uv }`). `RCAR` (v1) files still parse (all UV = sentinel). Vertex colours stay in the file as the fallback and as the tint of untextured parts.
2. **Livery atlas.** `carconv` packs the base-colour textures used by the body parts (paint, plastic, chrome; not glass and lamps) into one 2048x2048 atlas, each scaled to its own rectangle (the big paint textures get most of the area); every vertex's UV is remapped into its rectangle (inset by one texel against bleeding; the mesh is simplified with the UV as a weighted attribute so seams survive). The atlas is saved as `rally_car_livery.jpg` (quality 90, about 1-2 MB) next to `rally_car.rcar` in `rust/assets`.
3. **Loading.** The game decodes the JPEG once with the pure-Rust `zune-jpeg` (about 0.1 s), builds the mip chain on the CPU (box filter), uploads `Rgba8UnormSrgb` (about 22 MB with mips) and binds it at binding 8 of the main bind group with a trilinear, anisotropic (x4) sampler. If the file is missing or broken, the car keeps its vertex colours (logged, not fatal).
4. **Shader.** `vs_model` passes the UV (`@location(8)`); `fs_model` multiplies the albedo by the atlas sample where the UV is not the sentinel (paint, plastic, chrome, rubber), vertex colour otherwise. Lighting, lamps and covers are unchanged.
5. **Docs.** `docs/vehicles.md` (how to convert a new model: the converter flags, the atlas), ASSET-LICENSE (the texture derivative is under the same CC BY-NC-SA 4.0 terms), CHANGELOG EN + RU.

## Constraints
30 fps target (one extra texture fetch in the car's fragments only); APK grows by about 2 MB; the other models (trees, props) keep working unchanged (sentinel UV); the generated hatchback fallback is unchanged; no change to the physics or the model's geometry (bounds, lamp points and covers stay the same: tests pin them).

## Tests (host)
Parsing of `RCA2` (UV round trip, the sentinel, a v1 file still parses, truncated/garbage files are errors); atlas packing (rectangles inside the atlas, no overlap, UVs inside their rectangle); the embedded asset's body has UVs on the paint vertices; the JPEG decodes to 2048x2048 and the mip chain has 12 levels; WGSL validates; existing lamp and bounds tests stay green. Device check (after the user's yes): the livery sharpness against the old build, fps.

## Risks
UV seams after decimation (mitigated by the attribute weights; the converter prints the error); the source textures are 6144x4096 and not square (tiles keep the aspect); some parts share a texture with repeating UVs (clamp to the rectangle, check by eye).

## Not now
Normal and roughness maps, per-car repaint in the garage, decals on the body.
