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

## Measurements (POCO F3, 15.10.2026)

Autopilot run on a gravel stage, same scene: no airflow 57 fps; 0.35 m cells (32x16x64) 46-49 fps; 0.3 m cells (40x20x80, 64 000 cells) 47-49 fps; 0.25 m cells (48x24x96, 110 000 cells) about 31 fps. The target is **30 fps for the whole game** (user decision, 15.10.2026), so the default is 0.3 m (the 0.25 m cell is right at the limit); the airflow stays a debug view until step 3.

What the prototype showed:
- The ideal flow of a projected solver leaves no wake behind the car (D'Alembert), so a wake model (`Airflow::wake`) slows the air in the car's shadow; it is stable and depends on the voxel body.
- BFECC without a limiter blew up from a cold start (220 m/s after 3 s); the limiter, an initial fill with the inflow, a smoothed inflow (0.3 s), a tilt limit and a speed limit keep it bounded on the phone, also on a rough ride.
- Found and fixed: started from rest, the long box did not fill (the slice showed slow air everywhere, the CPU reference started at the inflow speed): a few Jacobi iterations cannot carry a change of the inflow from the front to the back of an 80-cell box. The car's frame accelerates, so the whole air gets the same shift when the inflow changes (`Airflow::move_inflow`, the `shift` uniform of the shader); the pressure solve then only handles the obstacles. After the fix the free stream is at the car's speed and the slow wake sits behind the body (43-49 fps on the phone).
- The Jacobi pressure solve does not converge in a long box (the measured divergence stays about 8 1/s whatever the iteration count, the collocated stencil is the limit); fine for a picture, to be revisited before the flow drives the dust.

## Success criteria

- Wake behind the car is visible in the slice and grows with speed.
- the whole game stays above 30 fps on the POCO F3 with the airflow on (met at 0.3 m cells, see Measurements).
- Existing tests stay green; with `airflow.debug` off nothing is allocated.

## Out of scope

Dust coupling (step 3), flow feedback on the car forces, voxels from the real car mesh, a UI toggle.
