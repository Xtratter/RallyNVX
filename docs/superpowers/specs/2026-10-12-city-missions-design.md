# City missions (v2.4) — design

Status: draft for review. Date: 2026-10-12. Builds on the city (v2.3), flight (v2.2) and missions (v2.1) work.

## 1. Goal

Make the cities part of the missions (decided with the user): goals and checkpoints in the city, an urban street
race, rings between houses and roof pads for the sling. Urban missions start in the city (an intersection on its
west edge); ordinary missions still start on the origin plateau. Roadmap note: the next stages are v2.5 Graphics and
materials, v2.6 Rescue, then garage and NPC orders, render modes A/B and polish.

Assumptions (editable): the mission list stays at six; every level already has a city, so any slot may be urban;
medals, rewards and the profile are unchanged; no traffic or pedestrians.

## 2. Mission slots

| Slot | Mission | Setting |
|---|---|---|
| 0 | Cargo delivery | open terrain |
| 1 | Checkpoint race | open terrain |
| 2 | Cargo delivery | **city** |
| 3 | Checkpoint race | **city streets** |
| 4 | Rings (drone or helicopter) | city or open, by hash (50 %) |
| 5 | Sling (even rounds) / Recon (odd rounds) | sling on **roofs**, recon open |

Open missions are generated as before. The menu card of an urban mission is marked "City" / "Город".

## 3. Street routes (`city.rs`)

- Intersection grid: `(BLOCKS + 1)` x `(BLOCKS + 1)` points, `City::intersection(i, j) -> V3` (the point is on the
  street centre lines, at the plateau height).
- `City::route(key: u64, from: (i32, i32), segments: usize) -> Vec<(i32, i32)>`: a self-avoiding walk over the
  grid (4-neighbourhood, never revisits a point, stays inside the grid), biased to turn (probability of a turn at
  each step about 0.5); it may stop early if boxed in, but always returns at least one step for the sizes used.
- `City::west_edge(row: i32) -> V3` is the start intersection `(0, row)`; urban missions start there facing east
  (`start_yaw = pi / 2`, i.e. forward = +x).
- `City::roof_pads(&self) -> &[V3]` (the existing `roofs`): candidate pad centres on roofs >= 20 m high with lots at
  least 14 x 14 m.

## 4. Urban missions (`airgen.rs` / `missions.rs`)

All use the level's first city built with `City::build(site, seed)` (the same city the world builds).

- **Cargo (city):** start `(0, row)` with a hashed row; the goal is an intersection reached by a route of 8-12
  segments whose Manhattan distance from the start is 300-600 m; `checkpoints = [goal]`. The player may drive through
  streets and parks, buildings are obstacles. `time_limit = 1.6 * length / 8 + 20` (length = straight path length).
- **Race (city):** a route of 10-16 segments from the start; checkpoints at every turning intersection plus the end,
  6-10 in total (route lengths are chosen so this holds; longer routes are cut at 10 checkpoints). Radius 6 m (street
  12 m). `time_limit = 1.6 * length / 8 + 20` with `length` = sum of the legs.
- **Rings (city):** 5-8 rings at the midpoints of the segments of a street route, on the street centre line, height
  18-40 m above the plateau; the plane normal follows the path (`ring_normal`, unchanged). The aircraft starts 3 m
  above the start intersection facing east. Open rings keep their v2.2 rules and the "lift above buildings" fix.
  `time_limit = 1.5 * length / 10 + 20`.
- **Sling (roofs):** pad A on a roof within 300 m of the start, pad B on another roof 200-600 m from A; both are roof
  candidates (so the roof is at least 14 x 14 m and 20 m high). The cargo rests on roof A (`Sling::on_ground` at the
  roof height; `Ground` already includes roofs); delivery needs the cargo inside the 6 m circle on roof B, speed < 2,
  within 1 m of the roof height (the existing rule `cargo.y - b.y < 1.0`). Pick-up and damage rules are unchanged.
  `time_limit = 1.6 * length / 12 + 35` as before.
- **Start:** `Mission` gets `urban: bool` and `start_yaw: Option<f32>`; `World::for_mission` places the car (or the
  aircraft 3 m above) at `m.start` (for open missions `start` is the plateau point as before) and uses `start_yaw`
  when present, else faces the first checkpoint.
- Generation never panics for any `(profile_seed, round, gold)`; extreme inputs are tested.

## 5. Menu array and UI

`menu_info` grows to 10 floats per mission (the tenth is `urban` 0/1), total `4 + 6 * 10 = 64`. `MissionsActivity`
uses stride 10 and appends "City" / "Город" to the title of urban cards. Briefing texts for urban cargo and race
mention streets; the sling briefing mentions roofs.

## 6. Testing

Host tests (no GPU), failing first:
- Route: self-avoiding, inside the grid, steps of one block, deterministic by key, turns happen; `intersection`
  and `west_edge` geometry.
- Urban cargo: start on the west street, inside the city and not inside a building; goal on an intersection inside
  the city at the required Manhattan distance; one checkpoint; time formula.
- Urban race: 6-10 checkpoints, all on intersections of the route, consecutive legs are multiples of 48 m; formula.
- Urban rings: 5-8 rings, centres on a street line (offset <= 6 m from a street centre line), height 18-40 above the
  plateau, normals along the path; start above the west edge; formula.
- Roof pads: both pads exactly at a roof candidate's height, inside a building footprint of at least 14 x 14 m,
  distance A to B 200-600 m, A within 300 m of the start; formula.
- Determinism and no panics over 300 seeds and extreme `(profile_seed, round)`; ground missions unchanged (their
  existing 200-seed rule tests still pass).
- `menu_info` has 64 floats and the urban flag; kinds and aircraft codes as before.
- World: urban starts are outside buildings (the hull push-out leaves the car where it is), the car can drive from
  the start to the first checkpoint along the street with a simple host autopilot (steer towards the target) without
  being stuck; an aircraft at the urban start can reach the first ring; the sling cargo spawns on the roof and stays.
- CI smoke: desktop `RALLYNVX_MISSION=3` (the urban race) with autodrive; screenshot.

## 7. Risks

- A route through the grid can make the autopilot-less player get stuck on a building corner: the hull push-out keeps
  the car from clipping, the streets are 12 m wide, and the race radius is 6 m.
- Rings between buildings with plane-crossing detection: the normal can be diagonal at corners; covered by tests of
  the normals and a crossing test along a street.
- Roof pads: picking up from a roof edge may be fiddly; the pad is a 6 m circle on a roof of at least 14 m.
- Scope: no traffic, no rescue, no new graphics.

## 8. Documentation and release

`docs/game.md` (EN+RU) gets an urban missions note; README and CHANGELOG (EN+RU) 2.4.0. Release v2.4.0 with APK,
tar.gz and AppImage via the existing workflow. Phone tests only with the owner's permission.
