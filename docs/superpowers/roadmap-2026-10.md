# Roadmap, October 2026 (car game)

Order agreed with the user on 2026-10-14. Each item is brainstormed (spec, plan) when its turn comes; v2.13.0 ships items 1 and 2 together.

1. **Day and night** - DONE on `feat/day-night` (spec/plan `2026-10-14-day-night*`).
2. **Car lights** - headlights lighting the ground, tail and brake lights, reverse lights (`2026-10-14-car-lights*`). Releases v2.13.0 with item 1.
3. **Camera control** - rotate the chase camera by touch; it returns behind the car when the car starts moving forward; when reversing the camera swings to the front of the car so the player sees what is behind it.
4. **Roads** - asphalt roads in the open terrain (join the start and the cities), with markings, higher grip; also gives the "asphalt" surface.
5. **Tyre dust** - physically plausible dust and spray from the wheels per surface (grass, sand, dirt, mud, asphalt, water): amount and colour from the surface, wheel slip, speed and sun/night light; needs a surface lookup (`game/materials.rs` weights, roads, mud near water).
6. **Realistic car physics** (tyres, suspension, transmission, drift), then flight, sling cargo and water (aircraft are removed from the game for now).
