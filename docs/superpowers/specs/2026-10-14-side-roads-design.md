# Side roads and junctions (B1, v2.20.0 with the autopilot panel) - design

Step B1 of the roads work, approved in chat on 2026-10-14 (B2, the city super-special loop, comes later). 2.19.1 (autopilot settings on a long press) ships with it as 2.20.0.

## Goal

The special stage has junctions with side roads that branch off and end in a dead end, as on real rally stages (junctions mark a change of character), without breaking the network's connectivity or the autopilot.

## Design

- `RoadKind::Side`: gravel (same surface, loose layer, ditches and rocks as the stage; `RoadKind::is_gravel()` replaces the `== Stage` tests), width 5 m with a flare to 8 m at the junction.
- `rust/roads.json` `side`: `count` [4, 7], `spacing` [600, 1000] m along the stage, `keep_ends` 300 m, `min_radius` 80 m (the stage's curve radius around a junction), `angle` [80, 100] degrees, `length` [150, 450] m, `width` 5 m, `min_clear` 25 m (distance to the stage after the first 60 m).
- Generation (`roads/side.rs`, after the stage): junction candidates walk the stage from `keep_ends`; a candidate is skipped in a tight bend, on a causeway or on a crest; the side road leaves alternately left and right, 80-100 degrees off the stage tangent, through 3-4 waypoints with gentle turns (80-150 m legs, heading wander of up to 25 degrees) over dry, not-too-steep ground; a leg that would go into water, climb above 14 %, or come within `min_clear` of the stage (after 60 m) or of another side road is dropped, and a side road shorter than 100 m is dropped. Built with `build_road(Side)` (curve smoothing, profile, no crests); the height profile of its first points is pulled to the stage's height at the junction (a taper over 20-90 m), so the two roads meet flush.
- The autopilot is untouched: its chain is the liaison, the city and the stage. `place_on_road` keeps using `roads[1]`.
- Rendering: the same ribbon with the gravel code; the side ribbon lies 1 cm higher than the stage's so the overlapping mouth does not flicker.

## Testing

Per seed (1..=14, coarse grid): 3-7 side roads, each 100-460 m, starting within 2 m of the stage axis at the stage's height (+-0.15 m), never closer than `min_clear` to the stage after 60 m, grade <= 0.25, at most 5 % wet points, deterministic, junction spacing >= 500 m along the stage; the nearest-road query returns a side road next to it; terrain tests for the ditch and verge on a side road; props: rocks line side roads; the whole-stage autopilot run still passes.

## Risks

Generation time grows a little (side roads are short, no grid A*); the junction mouth is a simple overlap (no fillet): good enough at 4 m resolution.
