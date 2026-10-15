# Realistic GPU dust — design

Follows [2026-10-15-airflow-dust-design.md](2026-10-15-airflow-dust-design.md). The user's decision (15.10.2026): build the dust **as realistically as games do it, fps is decided afterwards** (the 30 fps target stays the goal to return to). Reference: the DiRT Rally 2.0 clips the user recorded (dense warm plume, dark core, rising and trailing for 10-20 m). The first particle version (small dots, sine turbulence) was judged "not dust, moves unnaturally".

## Causes found and the techniques that fix them

| What looks wrong | Technique (source) |
|---|---|
| Particles clump into streaks | The turbulence was a sum of sines (not divergence-free). Use **curl noise** in three octaves (Bridson, Hourihan, Nordenstam, SIGGRAPH 2007): the field is incompressible, so particles swirl without piling up; time-varying by moving the noise with the wind. |
| No billowing at the scale of the cloud | Multi-scale: the grid gives the wake, curl noise the roll-ups; the amplitude grows with the particle's age (the cloud expands) and in the wake. |
| Flat, evenly lit | **Self-shadowing from the density field** (Intel "shadowing dynamic volumetric media", Crysis 2 / Bitsquid particle lighting): every particle marches toward the sun through the dust density grid and is darkened by the dust above it; forward scattering against the sun; dark core. |
| Specks instead of a cloud | Large soft, low-alpha, rotating, noisy-edged particles (Niagara/DR2 sprite practice) whose overlap builds the density; soft against the terrain. |
| Wrong behaviour | Burst from the contact patch tangentially (backward at about the car's speed, up), strong drag (Stokes), then long suspension of the fine fraction and fast fall of grit; per-particle response time spread (0.6-1.4 x). |

## Scope of this pass

1. Reference + shader: curl noise (`dust_ref::curl_noise`, host-tested: divergence-free, bounded, deterministic), per-particle drag spread, age-growing turbulence, plume.
2. Self-shadowing: a sun uniform; the step pass marches 6 samples toward the sun through the dust grid and writes the light into the instance (`e.w`).
3. Particle look: size 0.35 m, low alpha, noisy soft shape, ground softness, near fade; `fs_dust_pt`.
4. Emission: stronger tangential throw on the GPU (`dust.gpu_throw`).
5. Measure fps afterwards; cost cuts (half-resolution, order-independent blending, fewer octaves) are a later pass.

## Out of scope now

Order-independent transparency / sorting, half-resolution rendering, a true volume ray march, dust on the car body.
