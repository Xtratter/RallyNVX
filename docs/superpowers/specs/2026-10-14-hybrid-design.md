# Render mode A, hybrid (v2.10) — design

Status: approved in chat (2026-10-14); the automatic fallback is an option (default off). Builds on the raster renderer (mode C). Render mode B stays out of the plan.

## 1. Goal
A second render mode, **Hybrid**, selectable in Settings next to **Raster**: soft sun shadows (PCSS), long terrain shadows by ray marching a height map, and ambient occlusion (SSAO). All in shaders (no hardware ray tracing on the target phone). Target: at least 60 fps on High on the POCO F3.

## 2. Layers
1. **PCSS** replaces the 3x3 filter in `shadow_at` when the mode is Hybrid: a blocker search (12 taps of the raw shadow depth), a penumbra width `distance_m * 0.028` (clamped 0.04..1.2 m, shadow map texel 140/2048 m) and a 16-tap hardware-compared PCF with that radius. Raster keeps the old filter.
2. **Terrain ray shadows**: a height atlas (512x512 R16Float, 8x8 tiles of 64x64 texels, 2 m per texel, ring-addressed by `tile mod 8`, Repeat sampler) filled from the tile meshes the renderer already builds (no extra terrain evaluations; LOD 1/2 meshes are upsampled bilinearly, evicted tiles are cleared to -10000). The shader marches 20 geometric steps from the fragment towards the sun, soft term `clamp((ray_h - h) / (0.06 t + 0.5))`, stops above 150 m; tiles outside +-3 of the camera tile count as empty. The result multiplies with the shadow-map term in `shade()` (terrain, props, buildings, car).
3. **SSAO**: Hybrid renders the scene into an offscreen colour texture with a sampled depth texture; a half-resolution pass reconstructs world positions and normals from depth and takes 12 hemisphere samples (radius 1.5 m, range check) into an R8 target; a composite pass upsamples it (4 taps, depth-aware weights) and multiplies the colour (sky untouched, strength 0.85) while writing to the swapchain.

## 3. Control and safety
- Settings: "Render: Raster / Hybrid" (default: Hybrid on High, Raster on Low and Medium until the user picks one), "Auto fallback: Off / On" (default Off). With it on, 8 consecutive 0.5 s fps samples below 50 in Hybrid switch to Raster until the game restarts and the HUD says so (`FallbackMonitor`, pure).
- JNI `gameRenderMode(0|1)` and `gameAutoFallback(bool)`; state slots `[26]` active mode (0 raster, 1 hybrid), `[27]` fell back (0/1).
- Switching modes at run time allocates or frees the offscreen targets; resize recreates them. A device without the needed formats stays in Raster.

## 4. Testing
Host tests: f32 to f16 conversion, `heights_from_mesh` (lod 0 equals the terrain heights at the vertices, lod 1/2 upsampling, the ring slot of negative tiles), `HeightAtlas` bookkeeping (put, evict missing, no stale slot, clear on a new level), a CPU reference of the atlas lookup agreeing with `Terrain::height` within 0.3 m on lod 0 tiles, `FallbackMonitor` thresholds and reset, the default mode by quality, shader validation by naga (existing test). Device: screenshots of both modes (forest, lake, city, a low sun), fps comparison at High in the city, by the lake and in the forest; the look is judged by the user. One final review on the most capable model.

## 5. Out of scope
Ray-traced reflections, shadows beyond the loaded tiles, denoising beyond the depth-aware upsample, mode B.
