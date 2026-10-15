# PhysX-like dust options and in-drive settings — design

User request (15.10.2026): "do all 4 options, each can be switched on separately; and put the settings right into the drive so they can be changed on the go". Follows [2026-10-15-airflow-dust-design.md](2026-10-15-airflow-dust-design.md) and [2026-10-15-dust-realism-design.md](2026-10-15-dust-realism-design.md). The target stays 30 fps; every option has a switch so the cost is the player's choice.

## The four options (GPU dust, Settings and the in-drive panel)

1. **Particle pressure (PBD).** Position-based density constraints between particles (Macklin & Mueller's position based fluids, as PhysX 5 / Flex): the particles near the car (every 8th particle takes part, up to about 8000) are binned into the airflow grid's cells (8 slots per cell), every iteration computes each particle's density with the poly6 kernel, the constraint multiplier lambda = -C / (sum of squared gradients + eps), and the position correction with the spiky gradient (plus a tensile-instability term); XSPH viscosity smooths the velocities so the cloud rolls as one. 2 iterations per frame. Default: off (the heaviest).
2. **Exact car shape.** The car's mesh is voxelized once per model (0.1 m voxels, triangle samples + a flood fill for the inside + a chamfer distance transform) into a signed distance field in the body frame; the dust step pushes particles out along its gradient with a margin and removes the speed into the body, so the dust wraps the real silhouette (wheels, wing, mirrors) and not the boxes of the airflow voxels. The mesh is whatever the car's model is, so new cars work. Default: on.
3. **World collisions.** Trees (trunk capsules), rocks (spheres) and the terrain's steep slopes: the CPU builds colliders from the props near the car and bins them into a 64 x 64 window of 1 m cells (4 per cell); the step pass tests a particle against its cell's colliders (push out, slide) and, on the ground, uses the terrain normal (from the height window) to slide along slopes and walls instead of just clamping the height. Default: on.
4. **Settled dust layer.** Fine dust that dies near the ground adds to a deposit map (64 x 64 window, 1 m cells, fixed-point counters, slow decay); the terrain and road shaders tint the ground by it (a thin warm layer), and the car's wake lifts it again: a pass over the cells behind the car turns deposit into new particles in a reserved part of the ring. Default: on.

## In-drive settings

A gear button in the drive screen (under the View button) opens a compact panel over the game with cycling rows, applied at once: Dust (Off / Normal / Lots / Fluid), Aerodynamics, the four options above, Motion blur. Values persist (`Controls`); Settings has the same rows. JNI: `gameDustOpts(mask)` (bit 0 pressure, 1 shape, 2 world, 3 deposit) and the existing calls.

## Out of scope

Particle-particle collisions for all particles (only the near subset), buildings as colliders, a deposit visible outside the 64 m window.
