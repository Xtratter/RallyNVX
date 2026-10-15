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
  4. the ground: the terrain under the car is approximated by a plane (height at the car and two neighbours, sent by the CPU each frame); a particle that reaches it deposits (fades out), outside the plane's reach (more than 14 m from the car) particles only age;
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

## Out of scope

Feedback of the dust on the air or on the car, particle–particle collisions (PBF), rain, snow and other effects, a UI toggle for the airflow, forces from the airflow on the car (the aero forces of step 1 stay analytic).

## Risks

- The compute path and its storage buffers as vertex buffers on the phone's driver: checked by the first task (a spike: particles move on the GPU and are drawn) before the rest is built.
- The ground as a plane is crude on rough terrain; particles may float or sink by a few decimetres on slopes. Acceptable for a first version; a height texture is the next step if it shows.
