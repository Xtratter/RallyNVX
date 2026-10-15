# Airflow around the car (aero step 2) — design

Step 2 of the aero plan ([2026-10-15-aero-design.md](2026-10-15-aero-design.md)). Step 1 (forces as physics, garage parts) is released in 2.25.0; step 3 (dust driven by the flow) follows.

## Goal and intent

A **prototype** of the airflow around the car, to find out whether a flow solver can run on the POCO F3 next to the game, and how good the wake looks. It only computes and shows the flow (debug view); it changes no forces and no dust yet. The user plans real aerodynamics and body-part editing, so the grid and the car voxels must be ready to take the real parts later.

## Approach (chosen: improved Eulerian, LBM kept as a fallback)

Reuse the existing GPU solver of the fluid dust (`fluid.wgsl`: advection, divergence, Jacobi pressure, projection, solids mask) but in the **car's frame of reference**:

- A box around the car, moving and turning with it: 24 m long, 12 m wide, 6 m high (z forward), cell 0.25 m (about 96×48×24 = 110 000 cells). Config block `airflow` in `surfaces.json` (`cell`, `nx`, `ny`, `nz`, `iterations`, `substeps`, `debug`).
- Inflow: uniform `-v` (the car's velocity relative to the air, wind included) at the front face; free outflow at the back; the ground is a slip wall moving at `-v` (the road under the car); sides and top free.
- The car body is a voxel solid built from the vehicle's box and the garage parts (splitter, wing, underbody add boxes), so the wake changes with the parts.
- Better advection than the dust's semi-Lagrangian: BFECC, plus vorticity confinement so the wake keeps its swirl at 0.25 m cells.
- No wrap-around window: the box is fixed to the car, so `Window::recycle` is not needed.

Fallback if the quality is poor: a lattice Boltzmann (D3Q19) solver in the same box. The voxel body, the box and the debug view stay.

## Components

- `game/airflow.rs`: the CPU reference solver in the car frame (small grid, for tests), box config, `Body` voxelizer (`fn voxelize(spec, kit) -> Vec<bool>`).
- `shaders/airflow.wgsl` + `raster/airflow_gpu.rs`: GPU passes (solids, advect BFECC, vorticity, divergence, Jacobi, project) sharing the structure of `fluid_gpu.rs`.
- Debug view: a velocity slice through the car's middle plane drawn as colour (speed relative to the inflow), switched by `airflow.debug`; shown only in the debug builds' Settings flag, not in the normal UI.
- Timing: the GPU time of one airflow step, written to the log once a second.

## Testing

- CPU reference: after projection the max divergence is small; a cube in the flow gives a low-speed wake behind it and a speed-up at its sides; flow through an empty box stays uniform.
- Voxelizer: the body fills the expected cells; each garage part adds cells; a car with no parts matches the base box.
- `shaders_validate` includes the new WGSL.
- Phone measurement (debug extra `airflow=1`): fps with the airflow on at 0.25 m cells and at 0.35 m; the numbers go into this spec.

## Success criteria

- Wake behind the car is visible in the slice and grows with speed.
- fps stays above 50 at 0.25 m, otherwise a coarser cell is chosen and noted here.
- Existing tests stay green; with `airflow.debug` off nothing is allocated.

## Out of scope

Dust coupling (step 3), flow feedback on the car forces, voxels from the real car mesh, a UI toggle.
