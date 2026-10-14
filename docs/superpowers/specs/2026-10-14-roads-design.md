# Roads (v2.15.0) - design

Cycle "roads" of the rally roadmap. The user chose option 3 of `2026-10-14-roads-research.md` ("stage plus liaison") with three conditions: everything is generated randomly from the level seed, the connectivity of the roads must never break, and everything must look natural. Design presented in chat and approved on 2026-10-14.

## Goal

Every level has a road network: a tarmac liaison from the spawn plateau to the first city, and a gravel special stage of 6-10 km that starts at another gate of that city, winds through the terrain and ends in the open (or at the second city). The car spawns on the liaison and drives along it. Roads are carved into the terrain, drawn as ribbons, trees and rocks keep away from them, and a surface function reports gravel/asphalt/etc. for later grip, dust and sound work.

## Not in this cycle (next cycles)

The super special loop through the city streets, junction stubs and side tracks, ditches, tarmac side markings beyond the centre/edge lines, crests/jumps, a gravel loose layer, grip per surface (physics cycle), dust (dust cycle), the rally mode itself.

## Success criteria (all tested)

- Random: a different network for every seed; deterministic for the same seed.
- Connected: for every tested seed the liaison starts on the spawn plateau and ends on the boundary of city 1; the stage starts at a different gate of city 1 (>= 40 m from the liaison's gate); every road is one continuous polyline with 4 m point spacing; generation never fails (a fallback ladder always yields a valid network).
- Natural/drivable: minimum curve radius >= 10 m (target 13 m), grade <= 14 % (a steeper grade only when the two ends are too far apart in height, never above 25 %), width 5-9 m (stage) / 8 m (liaison), no road point in a lake, all points inside the world (|x|, |z| <= 3800 m), stage length 5.5-11 km in at least 70 % of seeds and 3-14 km always.

## Generation (`game/roads/`)

- `path.rs`: a cost grid (16 m cells over +-3800 m, 475 x 475) built from the roadless terrain: heights, lake mask, city mask, a low-frequency random field (about 320 m wavelength) and an 8-neighbour A* search. Step cost = distance x (1 + 0.9 min((slope/0.10)^2, 40)) x (1 + 0.8 noise) + 25 x distance on water + 20 x distance inside a city (when avoiding it). Nothing but the world edge is forbidden, so a path always exists; costs steer it round lakes, along contours (switchbacks on steep ground) and in random meanders.
- `shape.rs`: Chaikin smoothing (2 rounds), resampling at 4 m, curvature relaxation until the radius is >= 13 m everywhere; the vertical profile = terrain height along the line, moving-averaged over +-40 m and clamped to the grade limit inside the envelope between the two pinned end heights (so the road meets the city plateau and the spawn plateau without a step); width profile (stage: 5-9 m by low-frequency noise, minus up to 1 m in tight bends; liaison 8 m).
- `gen.rs`: gates on the boundary of city 1 (street intersections on the square's edge; the liaison uses the side facing the spawn, the stage another side), a straight 80 m approach outside each gate, waypoints for the stage (two or three random legs of 1.5-2.5 km, or the nearest gate of city 2 as the finish in half of the levels that have a second city), up to 10 attempts to land the total length in 5.5-11 km, a straight-line fallback with the same smoothing if anything fails.
- `mod.rs`: `RoadNet { roads }` with a 32 m spatial hash; `nearest(x, z) -> Option<RoadHit { road, seg, t, dist, y, half, s, tangent }>` looks at one cell, so it is cheap enough for `Terrain::height`.

## Terrain integration

- `Terrain` gets `roads: Option<Arc<RoadNet>>` (so `Terrain` is `Clone` but no longer `Copy`). `Terrain::new(seed)` stays roadless and fast (tests, planning); `Terrain::with_roads(seed)` generates the network and is what the game uses (`World::with_roads`).
- `height()` blends towards the road profile last: inside half-width + 1 m the ground equals the road height, over the next 7 m it eases back to the natural height (cut and fill). The 100 m city rim and the lake clamp are applied before it.
- `surface_at(terrain, x, z) -> Surface { Grass, Dirt, Sand, Rock, Gravel, Asphalt, Water }`: road kind on the road, Dirt on the 2 m shoulder, city streets are Asphalt, otherwise the strongest terrain material or Water.
- Exclusion: trees, rocks, bushes keep 4 m / 3 m from the road edge; no grass tufts or tyre tracks on the road.

## Rendering

A ribbon mesh per terrain tile (a road segment belongs to the tile of its midpoint): 4 vertices per segment with the across coordinate (-1..1), distance along (m) and kind + width packed in the normal slot, lifted 0.07 m above the road height plus a depth bias, alpha-blended edge for gravel. New `vs_road`/`fs_road` in `raster.wgsl`: gravel (warm grey-brown grain, darker wheel ruts at +-0.3, lighter loose berm at the centre and at the edges, soft edge fade) and asphalt (dark grain, white edge lines at +-0.9, a dashed centre line every 9 m), lit through `lit()` so night, headlights and shadows work. The ribbon is not drawn in the shadow pass.

## Spawn and debug

`World::with_roads` puts the car at the origin facing along the liaison. Debug: intent extra `road` (1 liaison near the start, 2 stage start, 3 stage middle, 4 stage near the end) via `Native.gameOnRoad(mode)` places the car on the road facing along it, for screenshots.

## Testing

Pure host tests: grid/A* on flat and rough terrain, curve smoothing (radius, spacing, endpoints), profile (grade, pinned ends), network properties over 30 seeds at 32 m cells plus 4 seeds at the real 16 m, determinism, `Terrain` integration (height equals the profile on the road, unchanged far away, surface function, exclusion of props), `RoadNet::nearest` against brute force. Device: screenshots from the spawn, on the stage at day and at night, hybrid fps, and the level-start time of `Terrain::with_roads` (budget: under 1.5 s on the POCO F3).

## Risks

- Level start time (A* over 226k cells plus the height grid): measured; if over budget, coarsen the grid to 24 m or build the network on the game thread while the renderer starts.
- A coarse terrain LOD (4-8 m cells) cannot follow a 7 m wide road bed exactly: the ribbon is lifted 0.07 m and uses a depth bias; check on the device at the LOD boundary.
- `Terrain` losing `Copy` touches a few call sites (`place_near_water`, tests) - fixed by the compiler.
