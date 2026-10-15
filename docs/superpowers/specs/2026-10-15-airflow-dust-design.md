# Dust driven by the airflow (aero step 3) — design

Step 3 of the aero plan ([2026-10-15-aero-design.md](2026-10-15-aero-design.md)); needs the airflow box of step 2 ([2026-10-15-airflow-design.md](2026-10-15-airflow-design.md)). Target: **the whole game stays at 30 fps or more** on the POCO F3 (user decision).

## Goal and intent

Dust that behaves like the particles driven by a moving fluid grid in PhysX (APEX Turbulence; the NVIDIA "translating Eulerian grid" scheme): inside the box around the car the particles take the grid's air velocity (so they swirl in the wake, are pushed aside by the body and its parts, are sucked behind the car), outside it they are ballistic with inertia, wind and settling. When the car stops, the dust keeps going forward. The user wants this to look physically right and to grow with the real aerodynamics and the garage body parts.

## Approach (chosen: the particles move to the GPU)

The solver is on the GPU, the particles are on the CPU today (`game/dust.rs`); reading the grid back every frame would stall the phone's pipeline. So the particles move to the GPU too, as in PhysX:

- One storage buffer of particles (also usable as the vertex buffer of the existing dust sprite pipeline, so rendering needs no change): per particle `pos.xyz + size`, `vel.xyz + age`, `colour.rgb + alpha`, class (fine / coarse), seed. Capacity by dust level (Normal 2000, Lots 6000).
- Emission stays on the CPU (it knows the wheels, slip and surface looseness, as today): up to 16 emit requests per frame go in a small uniform; a compute pass `emit` writes them into the ring buffer at `head`.
- A compute pass `dust_step` per frame integrates every particle:
  1. air velocity: inside the airflow box (position taken into the car frame: translate by the car, rotate by its yaw) the grid velocity `v_grid` (air relative to the car) is sampled trilinearly and the world air velocity is `car_vel + R · v_grid`; towards the box edge it blends (smoothstep over 1.5 m) into the outside air, the ground wind profile of `dust::air_velocity` (wind growing with height, turbulence);
  2. Stokes relaxation as in variant B: `v += (v_air − v) · (1 − exp(−dt/τ))`, `τ` by class (`dust_physics.tau_fine / tau_coarse`), then the settling speed of the class;
  3. the car's solid cells (the grid's solid texture) push particles out: a particle inside a solid cell is moved back to its previous position and its velocity along the cell's normal is dropped;
  4. the ground: a height texture of the box area (the CPU samples the terrain there as `fluid_gpu` does); a particle that reaches it deposits (fades out), outside the texture particles only age;
  5. age, fade, and retire (alpha 0).
- The Eulerian grid does the part PhysX's PBF particles do by density constraints: incompressible flow around the body. Particle–particle interaction is not simulated.
- The old CPU particles stay as the fallback when the device cannot run the compute passes (or the airflow could not be created), and for the Fluid (experimental) level 3.
- The airflow becomes part of the game at dust levels 1-2 (not only a debug view); the slice stays a debug switch. Its cost is part of the 30 fps budget; if the game falls below 30 fps the box is reduced (`cell` in `surfaces.json`).

## Components

- `game/dust_gpu_ref.rs`: the CPU reference of one particle step (`step_particle(p, air, car, ph, dt)`), the car-frame transform and the box blend; the shader is its translation (host-tested like `airflow.rs`).
- `shaders/dust_gpu.wgsl` (`emit`, `dust_step`) and `raster/dust_gpu.rs` (buffers, pipelines, the per-frame uniform: car pose and velocity, emit list, ground plane, wind and time).
- `raster/gpu.rs`: draw from the GPU buffer when the GPU dust is on; the airflow created and updated when dust level 1-2 and the compute path is available.
- `game/dust.rs`: the emission logic factored out so both paths use it (wheel slip, loose surface, class split).

## Testing

- Reference: a particle at rest in still air stays; in a uniform air velocity it relaxes with the time constant of its class (1/e after `τ`); a fine particle settles slower than a coarse one; a particle inside the box follows the grid velocity, outside it follows the wind; the blend is continuous across the edge; a particle in a solid cell comes out of it; deposition at the ground plane; world↔car frame transform is the identity round trip.
- `shaders_validate` includes the new WGSL.
- Phone: with the Vulpes and a garage rear wing, the dust cloud behind the car is shaped by the wake (compared with the CPU variant B), the dust runs on after a hard stop; fps with 6000 particles and the airflow on stays ≥ 30; numbers go into this spec.

## Measurements (POCO F3, 15.10.2026)

Autopilot run on a gravel stage, dust level Lots, three runs each (the world seed differs per run, so the scenes differ): CPU particles 55 / 38 / 49 fps (mean 47); GPU dust with the airflow (0.3 m cells, 30 Hz) 31 / 41 / 42 fps (mean 38). The target of 30 fps is met, the cost is about 9 fps. The airflow was moved from 60 to 30 Hz for this (the first measurement at 60 Hz gave 31 fps with a scene at 47 fps for the CPU dust). The buffer holds 4096 particles; the level caps (400 / 800) are still the CPU numbers and can be raised for the GPU later within the budget.

## Scope in stages

Everything the first draft left out is added, in this order; each stage is released and measured against the 30 fps budget before the next one starts (a stage that does not fit is cut down or made optional, not dropped silently).

1. **Step 3 (the core above)** plus two cheap parts that belong to it: the **ground as a height texture** instead of the plane (the box area of the terrain heights, same idea as `fluid_gpu`'s height texture), and a **Settings switch "Airflow"** (On / Off; Off falls back to the CPU particles, for weak devices).
2. **Step 3b, dust feeds back into the air and particles push each other:** the particles are splatted into the grid as a density field every frame; where the density is high the grid gets a drag (dust drags the air along, a thick cloud rolls as one) and a pressure-like push apart that the particles feel (a cheap stand-in for PBF's density constraints, in the spirit of PIC/FLIP; no neighbour search). Cost: one splat pass and one gradient read per particle.
3. **Step 4, the airflow acts on the car:** the pressure of the grid over the voxel body's faces and the wake give the drag and the lift/downforce per axle that now come from the analytic formulas of step 1; the analytic numbers stay as the reference and the fallback. This needs a converged pressure (the Jacobi residual noted in the airflow spec), so it starts with a better solver (red-black or multigrid-lite) and a check against the analytic forces on a box.
4. **Step 5, other particle effects on the same GPU system:** rain, snow and splash (rain drops hitting the car, wet-road spray), reusing the emit/step passes with other class constants.

## Out of scope

A real PBF/SPH neighbour-search fluid (stage 2's density field covers the look at a fraction of the cost).

## Risks

- The compute path and its storage buffers as vertex buffers on the phone's driver: checked by the first task (a spike: particles move on the GPU and are drawn) before the rest is built.
- The height texture for the ground costs a texture upload per frame (the box area only); if that is slow it is updated only when the car moves a cell.
- Stage 2 and 4 depend on stage 1's solver cost; the 30 fps budget decides how much of them fits.
