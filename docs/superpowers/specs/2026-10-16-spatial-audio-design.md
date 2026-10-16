# Spatial audio: physically based sound propagation (design)

Goal: every sound source of the game is heard the way it would be in the world: from its place, after its travel time, dulled by distance and air, blocked or bent by the terrain, the car and the buildings, reflected by the ground and the surroundings, ringing in the environment. The engine itself later comes from a port of the core ideas of [engine-sim](https://github.com/ange-yaghi/engine-sim) (MIT).

Approved by the user in chat (2026-10-16): blocks A (propagation) first, then B (engine model).

## Principles
- Pure geometry on the game thread (rays against the terrain, props, buildings and the car), small parameter packets to the audio thread, a cheap per-sample renderer on the audio thread. No allocations in the audio callback.
- Everything is testable on the host: paths, delays, Doppler, occlusion are checked by numbers.
- The listener is the camera; sources are real points of the car (exhaust, engine bay, four tyres, body) and of the world.

## Stages (each its own release)
1. **A1 Stereo and paths.** Stereo output; per-source emitter: fractional delay line (travel time, Doppler for free), 1/r law, air absorption, interaural time and level differences (head shadow).
2. **A2 Occlusion and the ground.** Rays from the listener to each source: terrain, props, buildings, the car's own body (the cabin muffles the outside); diffraction over the obstacle by Maekawa's Fresnel-number formula; the ground reflection as an image source (comb filter, by surface).
3. **A3 The environment.** Rays from the listener estimate the mean free path, the escape share and the absorption: the reverb time (Eyring), its level and damping (forest, open field, city street, tunnel); early reflections from the ray hits.
4. **A4 World sources.** Surface-specific tyre sounds, water, rain on the car and the ground, trees, one-shots at their real positions.
5. **B Engine.** Port of the gas-volume model, the combustion chamber and the exhaust pulse chain of engine-sim to Rust; the pulses feed the exhaust and intake sources.

## Budget
Rays: a few hundred segment queries per update (terrain by marching the height function, props by cell lists, buildings by boxes). Audio thread: about ten emitters, a few filters each.
