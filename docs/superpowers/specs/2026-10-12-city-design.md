# City (v2.3) — design

Status: draft for review. Date: 2026-10-12. Builds on the game core, missions, flight and vehicle-parameters work (v2.0-v2.2.1).

## 1. Goal and decomposition

Procedural cities in the off-road world (decided with the user: a city amid the off-road terrain, one or two per
level). The air part keeps three releases, re-cut by risk: **v2.3 City** (this document: generation, terrain
plateau, rendering, collisions with buildings, roof landing, FPS measurement), **v2.4 City missions** (goals and
checkpoints in the city, an urban street race, rings between houses, roof pads for the sling, i.e. all four options
the user chose), **v2.5 Rescue**. Garage, NPC orders, render modes A/B and polish follow.

v2.3 deliverable: levels contain one or two cities the player can drive through and fly over; buildings are solid
obstacles for the car and the aircraft; an aircraft can land softly on a roof; the city renders at a playable frame
rate on a POCO F3. Missions stay as in v2.2 (their points are not yet city-aware, but they must not break).

Assumptions (editable): stylised look (simple shapes, no textures); the city is described by code from the seed and
tested on the CPU; no traffic, pedestrians, interiors or night lighting.

## 2. Architecture

New pure-CPU module `game::city` plus a `Ground` abstraction; the renderer reads the city through `WorldView`.

### 2.1 Layout (`city.rs`)

`CityLayout::for_seed(seed: u64) -> Vec<CitySite>`; `CitySite { centre: V3 (x, base_y, z), half: f32 }`.
- One city if `hash01(seed) < 0.6`, otherwise two. Centre 600-1500 m from the origin (angle from the hash), at least
  200 m inside the world wall; a second city at least 800 m from the first. The origin plateau is never inside a city.
- Size 11 x 11 blocks, grid pitch 48 m, street width 12 m, so about 528 x 528 m (`half = 264`).
- `base_y` = the terrain height of the *unmodified* heightfield at the centre.

### 2.2 City contents (`City::build(site, seed)`)

- Block 36 x 36 m. Parks: every 7th block by hash and the central block (a square). Other blocks split into 1, 2 or
  4 lots by hash with a 2 m inset from the block edge and a 2 m gap between lots.
- Building height `8 + (90 - 8) * falloff^2 * hash` with `falloff = 1 - distance_to_centre / radius`; 15 % of
  the outskirts buildings are low (4-6 m); minimum 4 m, maximum 90 m. Colour: a palette of 6 by hash.
- `Building { min: V3, max: V3, color: u8 }` (axis-aligned, base at `base_y`). Roofs are flat. A list of roof
  landing candidates (height >= 20 m, lot at least 14 x 14 m) is kept for v2.4.
- Queries through a block grid: `buildings_near(p, radius) -> Vec<&Building>`, `roof_at(x, z) -> Option<f32>`
  (height of the roof if the point is inside a footprint), `inside(x, z) -> bool`, `street_point(rng_key)`
  (a street intersection centre, for v2.4). Streets, sidewalks and park tiles are exposed as flat rectangles for
  rendering.

### 2.3 Terrain plateau

`Terrain` gets the list of city sites. Inside a city the height blends to `base_y` with a smooth 100 m rim
(`h = lerp(h_raw, base_y, k)`, `k = smoothstep` of the distance beyond the city edge). `Terrain::new(seed)` includes
the cities (so levels change for every seed; this is accepted), `Terrain::flat()` has none. `Terrain::normal` and
the tile meshes follow automatically. Props and mission points keep working (props are rejected inside cities and
their rim; `point_ok` rejects building footprints).

### 2.4 Ground and collisions

- A small trait `Ground { fn height(&self, x, z) -> f32; fn normal(&self, x, z) -> V3 }`; `Terrain` implements it for
  the plain terrain, `WorldGround { terrain, city }` for "terrain or roof": height = max(terrain, roof), normal
  (0,1,0) on roofs. `flight.rs` and `sling.rs` take `&dyn Ground`.
- **Car** (`vehicle.rs`): after the suspension step, the hull (three spheres along the body, radius 1.0) is pushed
  out of nearby buildings with the velocity component into the wall removed (restitution 0.2); wheels keep using the
  terrain (the car cannot drive on a roof).
- **Aircraft** (`flight.rs`): a building side or underside contact (aircraft sphere vs box, not from above) is a
  crash; touching a roof from above uses the same soft-landing rules as the ground (`Ground::height` includes roofs).
- **Cargo on the sling:** rests on roofs and ground (via `Ground`); a wall hit pushes it out (no damage rule change).
- The terrain heightfield is *not* modified by buildings (no cliffs for the suspension).

### 2.5 Rendering

All through the existing instance pipelines (no new pipeline):
- Buildings: cube instances with the palette colour and a facade flag (colour alpha 3). The fragment shader draws
  windows on side faces from the world position: a 3.0 x 3.5 m cell, a lighter window inset, none on roofs.
- Streets: long thin cubes (11 + 11 stripes per city, asphalt), sidewalks as thinner stripes, parks as green flat
  tiles; a few existing tree models in parks.
- Culling: only blocks within the draw radius of the camera are emitted (700 m; quality levels Low / Medium / High
  use 400 / 600 / 700 m). A typical frame emits 150-400 buildings and 30-50 stripes and tiles.
- Budget: `MAX_INST` rises from 4096 to 16384 (a 1.5 MB buffer). Fill order: markers, vehicles, city, props.
  Trees and rocks are not placed inside cities.
- Shadows: city instances enter the shadow pass only within 150 m of the player.
- The player quality setting (already stored as the resolution scale) also selects the draw radius.

### 2.6 World, JNI, UI

- `World` owns `Vec<City>` built from the layout; `World::view` exposes them; the mission generator is untouched in
  this release (its points must still pass `point_ok`, which now also rejects city footprints and rims).
- No JNI or UI change except the quality-dependent radius passed with the existing scale. The state array is
  unchanged.

## 3. Testing

Host tests, failing first: determinism by seed; one or two cities with distances, margins and no city on the origin
plateau; plateau height equals `base_y` inside and blends to the raw terrain outside the rim; blocks, lots and
buildings never overlap and never cover streets; heights within 4-90 m and decreasing on average with distance;
parks exist; `roof_at`, `inside`, `buildings_near` agree with brute force on random points; car hull collision
(wall stops the car, no tunnelling at 30 m/s, parallel sliding works); aircraft: crash into a wall and underside,
soft landing on a roof and take-off again; sling cargo rests on a roof; props and mission points avoid cities
(`roll` over 200 seeds still passes its rules); renders: pure helpers for the instance list (culling by radius,
budget never exceeded, order of the fill, facade flag set); CI smoke: a desktop run placed next to a city shows
buildings (`RAYTRACE_SEED` + autodrive towards the first city, screenshot).

## 4. Risks

- **FPS** at the city centre on an Adreno 660 (the main risk): instance count, shadow pass and the facade branch. The
  plan has an explicit measurement step on the phone (only with the owner's permission) and a fallback: shrink the
  draw radius and the shadow radius, or drop the facade pattern.
- Terrain change shifts every seed: old saved mission seeds keep their ids but get a different world; accepted.
- Collision robustness for the car (corners, stuck inside a box): hull spheres with push-out and a "stuck" guard
  that lifts the car out; covered by tests.
- Scope: city missions, rescue and traffic are out of scope.

## 5. Documentation and release

`docs/game.md` (EN+RU): a City section; README and CHANGELOG (EN+RU) 2.3.0. Release v2.3.0 with APK, tar.gz and
AppImage via the existing workflow. Phone tests only with the owner's permission.
