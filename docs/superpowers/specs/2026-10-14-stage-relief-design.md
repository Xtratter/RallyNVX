# Stage relief (v2.18.0) - design

Roads step A, agreed with the user on 2026-10-14 ("A, then B, then the rally mode"). Design approved in chat the same day.

## Goal

The gravel special stage feels like a real rally stage: a rolling profile with crests and jumps, ditches and banks beside the road, a loose gravel verge, rocks near the edge. Everything stays deterministic per seed and the road network stays connected.

## Research summary

Finnish stages (Ouninpohja, Ruuhimaki) are wide fast gravel roads with series of crests ("yumps", some blind, the best-known jump is taken above 170 km/h), ditches on both sides and trees behind them, fine gravel on the shoulders; stages change character (fast crests, technical section, flowing section). Sources: Wikipedia "Rally Finland", Skoda Storyboard (Ouninpohja), Autosport, DirtFish, WRC stage guides. The sources say nothing on banks: the bank shape is our own estimate.

## Not in this step

Junctions, side roads, the city super-special loop (step B); marker posts and fences (need a new prop model); weather and seasons (tech debt); realistic suspension (physics cycle).

## Design

- `rust/roads.json` (embedded, overridable by a `roads.json` beside `profile.json` on Android or `RAYTRACE_ROADS` on the desktop, same pattern as `surfaces.json`): crest, ditch, verge and rock numbers. `game/roads/cfg.rs` parses and validates it (readable errors, ranges).
- Crests (`shape::add_crests`, stage roads only): after the smoothed profile is built, series of 3-5 crests are added to the height, spacing 150-250 m inside a series, 500-900 m between series. A crest is a half-cosine bump of height 0.6-2.0 m and length 30-60 m, asymmetric (the take-off side is longer than the landing side, factor 0.35), its height capped so the added slope never exceeds 0.10. Candidates are skipped where the curve radius within the crest window is below 120 m, within 150 m of the road ends, and on a causeway (the road lies less than 1.5 m above the water). The two ends keep their exact heights.
- Ditches (`Terrain::height`, in the road blend): beyond the road edge a smooth trench, stage 0.55 m deep and 1.3 m wide centred 1.8 m from the edge, liaison 0.25 m / 1.0 m; no ditch where the road lies less than 1.5 m above the water. The existing shoulder easing (8 m) gives the bank on the outside.
- Verge: `surface_at` returns gravel up to 0.8 m beyond the edge of a stage road (dirt after that up to 2 m, as before), so the verge is loose gravel for grip and dust.
- Roadside rocks (`props.rs`): extra rock candidates per tile accepted only 4-8 m from a stage road (trees and rocks already keep 4 m away), so the stage has a rocky edge.

## Testing

Pure: config bounds and bad JSON; `add_crests` keeps the ends, never exceeds the slope cap, respects spacing, curvature, ends and causeways, is deterministic. Terrain: ditch depth at the centre, none on a causeway, the road centre height unchanged. Generation: for a range of seeds the network stays connected, the stage has at least 4 crests, the grade (road plus crest) stays below 0.25, `has_fold` free. Physics: a car at 30 m/s over a 2 m asymmetric crest has its wheels leave the ground and lands with finite, bounded velocity and a sane attitude. Device (asked from the user when needed): a screenshot of a crest and a ditch, fps on the stage.

## Risks

- Crests steepen the stage; if the car flips too easily the numbers in `roads.json` are tuned by feel (height and take-off factor).
- The ditch is evaluated per height query (`road_hit` is already there); measured on the device.
