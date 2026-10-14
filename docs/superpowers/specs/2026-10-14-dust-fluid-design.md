# Fluid dust (experiment, branch `exp/dust-fluid`) - design

The user asked for dust that behaves physically correctly and chose option C (a gas-dynamics grid) as an experiment on 2026-10-14. The particle dust stays as the fallback; this is a fourth dust level, "Fluid (experimental)".

## Goal

Dust and air around the car are one simulation: the wind, the body of the car as an obstacle (a real wake with vortices), the wheels throwing dust and air, settling by weight, the ground as a solid. The cloud is rendered as a volume.

## Design

- Domain: a world-anchored, wrap-around grid of 128 x 20 x 128 cells of 0.75 m (96 m wide, 15 m tall) that follows the car; when the window moves, the cells that left it are recycled (cleared to wind and zero dust). A dust trail lives ~48 m behind the car and fades out at the edge.
- Fields (3D textures): velocity (rgba16float), dust (rgba16float: rgb colour x density, a density), pressure and divergence (r32float), a 128 x 128 height map of the ground (r32float, filled on the CPU).
- A step (30 Hz, compute shaders): advect the velocity (semi-Lagrangian) with the forces added in the same pass (the car's box moves at the car's velocity, the wheel throw, drag towards the wind), divergence, 16 Jacobi iterations of the pressure (warm start), subtract the gradient, advect the dust with the velocity plus the settling speed, decay, and the wheel sources (rate by surface, rolling and sliding as in `surfaces.json`). Ground cells (below the terrain height) and the car's box are solid.
- Render: a full-screen pass after the main pass: a ray per pixel through the window, 48 steps of 1.3 m with a per-pixel jitter, clipped by the scene depth (the main pass keeps its depth), absorption by the density, colour from the dust colour, ambient from the sky and a forward-scattering term towards the sun, darker at night. Blended over the frame (premultiplied).
- Numbers (cell size, dims, iterations, settling, decay, injection, absorption) in `rust/surfaces.json` under `fluid`.
- Fallback: without the needed GPU features or when the frame time stays above a limit, the level drops to Normal (particles).
- CPU reference `game/fluid.rs` (the same algorithm on a small grid, with tests) and the parameter packing (emitters, car box, wind) are pure and host-tested; the WGSL is validated by naga; the look and speed only on the device.

## Testing

Reference solver: advection of a blob in a uniform flow moves it and conserves its mass; the projection reduces the divergence a lot; a flow past a solid box goes around it (no flow through its cells); dust settles down at the set speed and does not go through the ground; the wrap-around window recycles exactly the cells that left; emitters follow the surface table (more on sand than on grass, none off the ground). WGSL: `naga` validation of every entry point and the render shader. Device: look, fps, the window edge.

## Risks

Unknown cost on the POCO F3; shader bugs are debugged by screenshots; the stale-cell logic of the wrap-around window must be exact; the depth buffer must be sampled after the main pass.
