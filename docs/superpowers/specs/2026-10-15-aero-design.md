# Aerodynamics and body parts (step 1 of the aerodynamics/dust plan) - design

The user wants physically correct dust and, later, real aerodynamics with body parts changed in the garage. The research (2026-10-15) gave a hybrid plan: (1) an aero model as car physics, (2) a wind-tunnel-like airflow around the car, (3) the dust field driven by both. This is step 1.

## Goal

The car gets real aerodynamic forces and the garage gets body parts (front splitter, rear wing with an angle setting, underbody with a diffuser) that change drag and downforce, so a part makes the car faster or better stuck to the road.

## Design

- `game/aero.rs` (pure): air density 1.2; dynamic pressure `q = 0.5 rho v^2`; forces are `q x area`. Drag along the flow from `CdA` = the car's base (from its `aero_drag`, `CdA = 2 aero / rho`) plus the parts. Downforce on the front and rear axle (`ClA`): the stock car's (`vehicles.json` -> `aero`: `cl_front`, `cl_rear`, `ride_ref`) plus the parts. Ground effect: each part has a ground sensitivity; the multiplier grows as the floor gets lower than the reference clearance up to a peak (+35 % at 65 % of it), then the flow stalls and the downforce falls; above the reference it decays. The clearance of each axle comes from the real suspension (the reference minus the compression above its static value), so squat and dive change the balance. Yaw: with the sideslip angle `beta` the drag grows by `2.2 sin^2 beta`, the downforce falls by `1.6 sin^2 beta` (not below 0) and a side force `q * 1.1 * sin beta |cos beta|` acts 0.9 m behind the centre of mass (it turns the nose back into the flow).
- `Vehicle::substep` replaces the old drag line: drag at the centre, downforce pushing the body down at the two axles (so the springs load the tyres and the grip grows), the side force at the rear point. A car without parts and with no stock downforce drives exactly as before (the drag is the same number).
- Parts (`rust/aero.json`): splitter, wing, underbody, levels 0-3, `ClA` and `CdA` per level, the wing angle (low / mid / high: downforce x0.7 / 1 / 1.25, drag x0.55 / 1 / 1.7), the ground sensitivities (splitter 1, underbody 1, wing 0). Prices as the other upgrades (1x, 2x, 4x the car's `upgrade_cost`); the wing angle is free.
- Garage data: the levels array of a car grows from 4 to 8 (`Upgrade` gets `Splitter`, `Wing`, `Underbody`; the 8th is the wing angle, default 1); old profiles load with the new ones at 0 / 1. `garage_info` gives per car 14 fields (the 7 old ones, then splitter, wing, underbody, wing angle, the downforce at 150 km/h in kg, the drag change in % against the stock car, the estimated top speed in km/h). The profile file and its tests stay compatible.
- Kotlin: the garage panel shows the three new parts like the others, a wing angle button and the three numbers;.. not needed.
- Autopilot: unchanged in this step (its cornering limit ignores downforce).

## Testing

Pure: q grows with the square of the speed; the downforce grows with the level of each part and splits front/rear as designed; the ground effect curve has its peak and stall; yaw lowers the downforce and raises the drag; the base car's drag equals the old `aero v^2`. Physics (host): a car at 40 m/s in a long corner holds more lateral acceleration with the wing than without; the top speed with the highest wing at the high angle is lower than with none; at low speed the difference is small; the estimated top speed is near the simulated one; downforce loads the suspension (the compression grows with speed). Garage: levels clamp, prices, an old 4-level profile loads, the new fields round-trip. Device: a garage screenshot and a run.

## Not in this step

The air flow around the car and the dust (steps 2 and 3), slipstream behind other cars, wind in the drag, the autopilot's use of downforce, an editor of the ride height.
