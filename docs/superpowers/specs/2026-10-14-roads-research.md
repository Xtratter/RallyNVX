# Roads for the rally game: research and options (for brainstorming, not yet a spec)

Status 2026-10-14: the user asked for roads in the open terrain and a rally-sim direction, and asked for the options after a study of real rally stages. The user has not chosen yet. Next step: they pick option 1, 2 or 3; then write the spec (`2026-10-xx-roads-design.md`), the plan, and execute inline (no review subagent).

## What real rally stages look like (sources: wrc.com stage guides, Wikipedia "Special stage (rallying)", dirtfish.com, racetrackdesigner.com)

- Point to point (A to B), 10-30 km in WRC (super specials about 5 km), average speed 100-120 km/h on fast gravel, organisers add chicanes above about 130 km/h.
- Mostly one surface per stage (gravel or tarmac, sometimes snow); mixed surfaces mainly in super specials.
- Width changes inside a stage: wide and fast at the start, then narrower and technical, sometimes a very narrow rutted forest road (my estimate: 5-8 m, narrow parts about 4 m; the sources give no numbers).
- Rhythm: fast crested run, technical hairpin section, flowing section; one type of corner is boring. Junctions mark a change of character. Ditches on both sides, trees behind.
- Crests and "yumps" (Finland): blind crests taken flat, 30-40 m jumps; pace notes warn about them.
- Loose gravel: the first car sweeps the loose top layer, later cars get a cleaner racing line.
- Stages are linked by liaison roads at legal speed, loops of 2-3 stages from a service park.

## Options

1. One stage road per level: 6-10 km (the map is about 8 x 8 km), one surface (gravel by default), width 5-9 m, sections (wide fast, hairpins, flowing, narrow forest with ditches and crests), junction stubs. Simplest, most rally-like, one surface only.
2. A road network: tarmac main roads linking the start plateau and the cities plus gravel/dirt branches. More open world, less rally.
3. (Recommended) Stage plus liaison: a gravel special stage as in 1, a short tarmac liaison road from the start plateau to a city and a super-special loop through the city on tarmac. Gives gravel, tarmac, dirt, grass, sand for grip and dust. Rally mode later: liaison -> stage -> super special.

## Technical sketch (all options)

- Centre line: a smooth spline walked over the height field with curvature limits and a terrain cost (avoid slopes above about 12 degrees, water, cities); the road bed is carved into the height field; ditches at the sides; trees and rocks pushed back from the road.
- One function `surface_at(x, z)` returning a surface enum (grass, dirt, sand, rock, gravel, asphalt, later mud) feeds grip (physics), tyre dust, sound.
- Rendering: a draped road ribbon mesh (sharp edges, tarmac markings, a darker worn line on gravel). Painting the road only through the per-vertex terrain material weights is too soft: the terrain grid is 2 m per cell at the best LOD (`CELL = 2.0`, `TILE_CELLS = 64`).
- Gravel "loose layer" and racing line are later physics/dust features.

## Related roadmap items

See `roadmap-2026-10.md`: camera (done, v2.14.0), roads and surfaces (this), tyre dust per surface, realistic car physics, rally mode as an option next to free drive.
