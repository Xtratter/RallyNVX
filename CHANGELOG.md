# Changelog

[Русский](CHANGELOG.ru.md) · **English**

## 2.4.0 - city missions

- Added: urban missions marked "City": a cargo delivery through the streets, a street race over turning intersections, rings flown over a street between the houses, and sling cargo carried from one roof to another; they start on a street at the west edge of the city
- Changed: the sling mission now always uses roof pads; the list still has six missions; the menu array carries an urban flag

## 2.3.0 - city

- Added: every level now has one or two procedural cities (about 530 x 530 m on a flat plateau): a grid of streets, blocks, parks and buildings of 4-90 m with windows; the tall buildings stand in the centre
- Added: buildings are solid: the car cannot drive through them, an aircraft that touches a wall or underside crashes, and a helicopter or drone can land softly on a roof (so can the sling cargo)
- Added: the quality setting also sets the city draw distance (300 / 400 / 450 m)
- Changed: levels look different from earlier versions for every seed (the terrain is flattened under cities); props and mission goals stay out of cities. City missions come next

## 2.2.1 - vehicle parameters in one file

- Changed: all numbers of the car, helicopter, drone, flight rules and the sling now live in `rust/vehicles.json` (embedded in the app); no behaviour change. Desktop can try another file with `RAYTRACE_VEHICLES=path` (docs/vehicles.md)

## 2.2.0 - flight

- Added: a helicopter and a drone with arcade controls (sticks released = hover, soft landing or crash) and three air missions: ring races, cargo on a sling (helicopter) and drone recon
- Added: the mission list now has six missions; new flight HUD (altitude, speed, rope state), goal rings, pads and recon zones in the world
- Desktop: flight keys (W/S pitch, A/D roll, Q/E yaw, Up/Down lift); `RAYTRACE_MISSION=4` starts the ring mission
- Known: no cities and no rescue yet; locking the screen mid-mission restarts it

## 2.1.0 - missions

- Added: missions: a list of four random missions (two cargo deliveries, two checkpoint races) with a goal beacon and checkpoint rings, a timer, cargo integrity (hits, hard landings and rollovers damage it), an arrow and distance to the target, pause and results screens
- Added: bronze / silver / gold medals by time and cargo condition, money rewards, a saved profile (money, best medal and time per mission; a corrupt file is kept as `.bak`), Refresh for a new list
- Added: main menu (Missions / Free drive / Scenes / Quality) with the money shown; desktop: `RAYTRACE_MISSION=<0..3>` starts a mission, `M` aborts it (docs/game.md)

## 2.0.0 - off-road game

- Added: a game next to the scene viewer: "Free drive" on a random level (every level has a seed shown in the HUD; the same seed gives the same level)
- Added: seeded hilly terrain with rocks and trees, an arcade off-road car with suspension, drift (handbrake) and collisions, a chase camera, touch controls (left stick steer/throttle, right stick down brake) and keyboard on desktop
- Added: a raster renderer (render mode C) with tiled terrain LODs, fog, sky and a sun shadow map; Low / Medium / High resolution on the start screen
- Added: start screen (Free drive / Scenes / Quality); desktop: `raytrace-desktop --game [seed]` (docs/game.md)
- Unchanged: the ray-traced Scenes mode and its scene files

## 1.8.0 - prism

- Added: the "Prism" scene: sunlight through a low slit passes a glass prism and splits into a spectrum on the floor; the prism itself shows coloured edges
- Added: glass `ior` and `dispersion`, the `prism` shape and `caustic_receivers` for scene files (docs/scenes.md); `scene` setting: 0 Classic, 1 Sun room, 2 Materials, 3 Prism, 4 = a loaded file (was 3)
- Added: photon passes (sun photons through the prism into a photon map on the floor and walls, averaged over frames) and a hero wavelength for camera rays through dispersive glass

## 1.7.0 - scenes

- Added: scenes are data now (JSON): three built in (Classic = the old scene, Sun room, Materials) and your own files from a folder; picker on the Scene tab (Android and Linux); format in docs/scenes.md
- Added: sun light (directional, soft shadows, Day / Dusk / Night follow the Sky setting), coloured glass panes that tint the light passing through them, glossy material (diffuse under a specular coat), tinted glass, rounded and spinning boxes
- Added: "Sun room" (sunlight through a ceiling slit onto coloured glass, mirror, gold, matte, glossy and clear glass objects) and "Materials" (a showcase row)
- Changed: shadows for all lights are analytic per primitive (no scene march); the shader is generated from the scene's primitive list

## 1.6.0 - helicopter camera

- Added: "Helicopter" camera mode (Scene tab, also on the desktop panel): left stick turns (x) and moves forward/back (y), right stick changes height (y) and strafes (x); the view is fixed slightly downward. Desktop: A/D turn, W/S forward, Up/Down arrows height, Left/Right arrows strafe
- Changed: the `cam_mode` setting has a third value (2)

## 1.5.0 - PlayStation 1 look

- Added: "PlayStation 1 look" switch (Quality tab, also in the desktop panel): the shorter screen side renders at 240 px and is shown as hard pixels (no filtering), colour is cut to 15 bits (5 per channel) with a 4x4 ordered dither on the source pixels, the camera position snaps to a 1/24 grid, animation advances at 12 fps, no sub-pixel jitter; it replaces the render scale setting, temporal smoothing and the denoiser still apply
- Measured on POCO F3 (Balanced, hybrid): 43-47 fps at 240x528, GPU about 20 ms
- Tried and dropped: integer (fixed-point) Menger sponge SDF, about 60% slower on Adreno (docs/bench.md)

## 1.4.0 - Linux desktop build

- Added: `raytrace-desktop` for x86-64 Linux (winit window + egui settings panel, mouse and keyboard camera, settings saved to `~/.config/raytrace/settings.ini`), packaged as a tar.gz, an AppImage and an Arch `PKGBUILD`
- Changed: `Gfx` can be created from any wgpu surface and draws an optional overlay after the present pass; presets and setting keys are now also in Rust
- The Android app is unchanged (still 1.3.0)

## 1.3.0 - analytic shadow rays

- Changed: shadow rays use analytic occluders with a broad-phase instead of marching the whole scene: spheres are analytic; the Menger box gets a slab test and the torus a bound-sphere test, followed by SHORT bounded marches only when the ray reaches those objects
- Changed: Balanced default bounces 4

## 1.2.0 - half-resolution global illumination

- Added: GI resolution setting (Full / Half / Quarter) on the Light tab; it applies only in hybrid mode with GI on (in path-tracing mode GI stays inline; with GI off nothing is deferred); presets: Performance has GI off (resolution irrelevant), Balanced = Half, Quality = Full
- Added: half/quarter-resolution GI passes (gi_trace, gi_temporal, gi_atrous, composite; they run only for Half/Quarter, "Full" computes GI inline in the trace pass as in 1.1) with their own temporal accumulation, a-trous denoise and bilateral upsampling
- Added: per-pass GPU timings in the HUD "Full" mode (when timestamp queries are supported)
- Changed: default GI is half resolution
- Measured: on POCO F3 at fixed 0.33x hybrid, Half is +46% fps vs Full (10.4 -> 15.2 fps), Quarter +53%; see docs/bench.md

## 1.1.0 - on-screen sticks and fly camera

- Added: two on-screen gamepad sticks (left = move / zoom, right = look / orbit)
- Added: free-fly camera mode alongside the orbit camera
- Added: four Scene tab settings - camera mode, on-screen sticks, move speed, look speed

## 1.0.0 - initial release

- SDF scene rendered with Rust + wgpu (Vulkan): hybrid and progressive path-tracing modes
- Temporal reprojection with depth / normal / id validation, neighbourhood clamp and Halton jitter
- A-trous denoiser (0-3 passes), Catmull-Rom upscale, sharpening, exposure, tonemapping, dithering
- Adaptive resolution with a target fps, optional checkerboard tracing, frame limit
- Presets (Performance, Balanced, Quality), shadows / global illumination / caustics / reflections switches, lamp colours, sky, auto-orbit
- Material 3 Expressive settings UI (android-ui-kit): five tabs, themes, haptics, HUD; English and Russian
- Safety governor: very slow frames automatically reduce render scale and samples per pixel
- Orbit, pinch zoom and double-tap reset gestures
