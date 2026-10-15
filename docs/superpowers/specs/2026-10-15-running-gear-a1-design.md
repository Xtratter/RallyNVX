# Stage A1: textured wheels, brake discs and calipers

Part of `2026-10-15-peugeot-full-model-roadmap.md`. A2 (wheel sets) and A3 (suspension) are separate specs and releases.

## Intent
The four wheels are one low-detail mesh (tyre, rim, disc and caliper merged, 1 500 triangles, vertex colours): blurry rim, no tyre lettering, and the caliper spins with the wheel. Success: wheels as sharp as the livery (the rim's spokes, the tyre's sidewall and tread), a disc that is seen through the spokes and glows when hot at night after hard braking, a caliper that stays put when the wheel spins (it only follows the steering and the suspension), mud flaps behind the wheels.

## Design
1. **Source parts** (left-front, mirrored to the right as now): tyre `tyre_tarmac_lf` (885 triangles, 2048 texture), rim `x0_wheel_fl_STYLE1` (the style the Peugeot shows now; 2 314 triangles, 1024 texture), disc `x0_disc_fl` (220, 256), caliper `x0_hub_caliper_fl` (838, 64-pixel AO), mud flaps `x0_mud_flap_*` (60 each, merged into the body mesh). Hubs and the suspension stay for A3.
2. **Three wheel models, one atlas.** `tools/carconv` writes `RCA3`: after the body, wheel (tyre + rim; about 2 500 triangles), disc (about 220) and caliper (about 500) meshes, all UV-mapped into the same 2048 atlas as the livery (the packer takes the wheel textures too; the livery scale only drops if the free band is too small, the tests pin it at 2048 wide). `RCAR` / `RCA2` files still load (disc and caliper absent: the wheel as before).
3. **Game.** `CarModel { body, wheel, disc: Option<Mesh3>, caliper: Option<Mesh3> }`; two new model slots; poses: wheel and disc get steer, suspension travel and spin, the caliper steer and suspension only. Drawn only for the rally style.
4. **Brake heat.** `Vehicle.brake_heat: [f32; 4]` (0..1 per wheel): it grows with the braking power (`brake_force x k x speed`, front wheels more), cools with the wheel's speed and time (time constant about 6 s), is purely visual (no physics feedback). The disc instance's tint carries the heat; in the shader, material "disc" adds an orange-red emission by heat, stronger at night. A test pins growth under hard braking, cooling, and that the values stay in 0..1.
5. **Docs and credits.** `docs/vehicles.md` (RCA3, the parts), `ASSET-LICENSE.md` (the wheel, disc, caliper and mud-flap parts are part of the derivative), changelog EN + RU.

## Constraints
fps stays near 39-45 on the POCO F3 (4 wheels x 3 000 triangles is under 12 000 extra); APK grows by under 0.5 MB; lamps, lamp interiors and the livery unchanged; the generated hatchback fallback untouched; collision, physics and wheel radius unchanged (0.326 m).

## Tests (host)
`RCA3` round trip, older formats still parse, the packer fits the wheel textures and keeps the livery at 2048 wide, the embedded asset has the three wheel meshes with UVs, the caliper mesh has no spin in the pose function (pose test), brake-heat growth, cooling and bounds, WGSL validates. Device check (after the user's yes): wheels sharp, disc glow after braking at night, fps.

## Not in A1
Wheel sets (A2), suspension arms (A3), the spare wheel (needs the interior view), the exhaust (already in the body).
