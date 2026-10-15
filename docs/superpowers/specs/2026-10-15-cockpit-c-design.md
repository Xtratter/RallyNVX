# Stage C: the cockpit view and the steering wheel

Part of `2026-10-15-peugeot-full-model-roadmap.md`. Approved by delegation ("do what you think is best"). After B (2.34.0, glass and interior).

## Intent
You can look at the interior through the windows; now you can sit in it. A sixth camera view, **Cockpit**, puts the camera at the driver's eyes; the steering wheel turns with the steering input; the driver is not drawn when the camera is inside (it would block the view). Success: in the cockpit view the dashboard, the windscreen with its pillars and the steering wheel are seen, the wheel follows the car's steering smoothly, nothing clips at the near plane, fps stays near 40.

## Design
1. **View.** `CamView::Cockpit = 5` (`VIEWS = 6`; the Kotlin `CamMath.VIEWS = 6`, names "Cockpit" / "Салон"); a rigid view like the bonnet: the camera rides on the car at `cameras.cockpit` of `vehicles.json` (metres: `[x left, y above the road, z ahead of the centre]`; the Vulpes: the driver's eyes), the gaze follows the car's heading with the bonnet's damping, the pinch zooms (the field of view) and the two-finger height tilts the gaze, as in the bonnet view. The near plane is 0.08 m when the camera is within 2.5 m of the car and 0.3 m otherwise (the cockpit parts are closer than 0.3 m).
2. **Meshes.** `RCA3` grows to eleven meshes: ..., interior (7, the cabin without the steering wheel and the driver), **steering wheel** (8, in the body frame), **steering pivot** (9: a two-vertex mesh, vertex 0 `pos` = the wheel's centre, `nrm` = the axis pointing at the driver; computed by `tools/carconv` as the rim's centroid and the smallest principal axis of its vertices), **driver** (10). Fewer meshes still parse.
3. **Steering wheel.** `CarModel.steering: Option<Steering { mesh, pivot, axis }>`; the mesh is stored shifted by `-pivot`; its instance pose is the body pose, then a rotation about the axis by `steering_wheel_angle(steer)` = `-steer * 5.5` clamped to +-3.8 rad (about 1.2 turns lock to lock at the Peugeot's 0.5 rad of steer), the translation `pivot`. Drawn with the interior (opaque, within 16 m).
4. **Driver.** A separate slot, drawn with the interior unless the camera is within 1.2 m of the car's centre (`draw_driver(dist)`), so the cockpit view looks through where the head is.
5. **Docs.** `docs/vehicles.md` (eleven meshes, the pivot mesh, `cameras.cockpit`), `docs/game.md` (the views, EN + RU), `ASSET-LICENSE.md` (unchanged parts), changelog EN + RU.

## Constraints
fps within a few frames (the steering wheel is 6 200 source triangles, simplified to 2 500; the driver 3 000 as in B); the generated cars have no cockpit meshes: their cockpit view falls back to the bonnet's spot (`cameras.cockpit` missing = the bonnet); lamp, livery, suspension, glass tests stay green; the other views are unchanged (the table in `CamMath`).

## Tests (host)
`CamView` has 6 views and `from_index` clamps; the cockpit view is rigid; the camera sits at `cameras.cockpit` of a car (rotated with the car), falls back to the hood without it, and the near plane rule `near_for(dist)`; `steering_wheel_angle` is odd, bounded and 0 at 0; the pivot mesh parses; `draw_driver`; the asset has a steering wheel with its pivot near the front-left of the cabin and the axis a unit vector pointing backwards and up; the converter's principal-axis function on a synthetic ring (a circle of points in a tilted plane gives the plane's normal); Kotlin `CamMath` (6 views, names, rigid). Device check (after the user's yes): the cockpit view, the wheel turning on a curve, fps.

## Not in C
The driver's hands on the wheel, pedals, the gear display and the RPM LEDs, wipers (with the weather work), mirrors showing the back, a head bob.
