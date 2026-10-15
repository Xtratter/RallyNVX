# Stage A2: wheel sets (tarmac, gravel, snow)

Part of `2026-10-15-peugeot-full-model-roadmap.md`. A1 (2.31.0) gave the Peugeot textured wheels, discs and calipers; the source model has three complete sets.

## Intent
The player picks a wheel set for a car in the garage: tarmac (the current one), gravel (taller tread, a different rim) or snow (studded-look winter tyres and a deeper rim). The choice shows on the car (in the garage preview and in every drive) and (in a later stage) changes the grip. Success: three clearly different sets on screen, the choice saved per car in the profile (old profiles load as tarmac).

## Design
1. **Source parts** (left-front, as in A1; the same disc and caliper for all sets): tarmac = `tyre_tarmac_lf` + `x0_wheel_fl_5radios` (now); gravel = `tyre_gravel_lf` (880 triangles, 2048 texture) + `x0_wheel_fl_gravel` (3 274, 1024); snow = `snow_tire_lf` + `rim_snow_lf` (4 010, 1024). Each set is simplified to 2 800 triangles and centred on the same wheel centre as the tarmac wheel (its radius stays 0.326 m; the physics wheel radius does not change per set).
2. **Format.** `RCA3` mesh count 2..=6, order: body, wheel (tarmac), disc, caliper, wheel (gravel), wheel (snow); files with fewer meshes fall back to the tarmac wheel for the missing sets. The atlas grows by the sets' textures (tyres at 256, rims at 384 pixels; the livery stays 2048 wide); the atlas height is whatever the packer needs (at most 3072; the test pins the memory).
3. **Garage and profile.** A ninth level per car, `wheel_set` (0 tarmac, 1 gravel, 2 snow; free, like the wing angle): `garage::NLEVELS = 9`, `default_levels` ends with 0, profile key `wheel_set` (missing = 0, above 2 clamps to 2), `api::garage_set_wheel_set`, the `garage_info` array gets the value after `wing_angle` (the Kotlin reader and the doc comment follow), JNI `garageWheelSet`. The garage screen gets a "Wheel set" row with one cycling button (no sliders).
4. **Spec and renderer.** `VehicleSpec.wheel_set: u8` (set by `garage::apply`); `car_key` includes it, so the GPU rebuilds the wheel mesh when it changes; `CarModel.wheels: [Mesh3; 3]` (`wheel` stays the tarmac one for older code: replace the field by an accessor).
5. **Grip: later.** The set will change the grip by surface (tarmac tyres best on asphalt, gravel tyres on loose surfaces, snow tyres a compromise), but that is a gameplay change for a later stage (a table of grip multipliers per set and surface in `surfaces.json`, `grip::tyre_factor`, tests that each set is best on its own surface). In A2 the set only changes the look; `VehicleSpec.wheel_set` is there so the grip stage can read it.
6. **Docs.** `docs/vehicles.md` (sets), `docs/game.md` (garage, EN + RU), `ASSET-LICENSE.md` (the gravel and snow wheels), changelog EN + RU.

## Constraints
fps unchanged (one wheel mesh drawn at a time); APK up by about 0.3 MB; atlas memory at most 3072 rows x 2048 (about 34 MB with mips); wheel radius, all physics numbers, the lamps and the livery unchanged; a profile written by this version loads in the old one only by losing the set (the old reader ignores the key).

## Tests (host)
Profile: a missing key = tarmac, clamping, save and load round trip; `garage_info` carries the set; `garage_set_wheel_set` accepts 0..=2 and refuses 3; `apply` sets `spec.wheel_set`; format: `RCA3` with 6 meshes parses, fewer meshes fall back to the tarmac wheel; the embedded asset has three wheel meshes with UVs, similar radius (0.326 +- 1 cm) and centred; the packer keeps the livery 2048 wide; the Kotlin garage maths test for the new field. Device check (after the user's yes): the three sets on screen in the garage, fps.

## Not in A2
The grip effect of the sets (next, as its own small stage), choosing the set automatically by stage or weather (with the weather work), per-set wear, snow on the ground. Cars other than the Peugeot have one generated wheel: the choice exists for them but changes nothing visible.
