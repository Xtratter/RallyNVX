# Changelog

[Русский](CHANGELOG.ru.md) · **English**

## 2.42.0 - wipers and raindrops

- Added: the Peugeot's two windscreen wipers (the model's own arms and blades, each about its spindle): parked when it is dry, sweeping in the rain - slowly in a drizzle (2.6 s a wipe), quickly in a downpour (1.3 s)
- Added: raindrops on the glass: small lens-like drops that grow in between wipes and are cleared when the sweep starts over; more of them the harder it rains
- Changed: `rally_car.rcar` carries up to 14 meshes (the wipers and their spindles)

## 2.41.0 - physically based lighting

- Changed: all surfaces are lit by one physically based function: a GGX specular (Smith visibility, Schlick Fresnel) for the sun, the moon and the headlights, and the sky as the environment (diffuse and reflected, blurred by roughness); the car's paint, chrome, glass, rubber and the brake discs differ by roughness and metalness, and wet ground is smooth, so it mirrors the sky and the headlights; terrain, roads and foliage stay matte and cost nothing extra

## 2.40.0 - the low beam reaches the road, not the ground under the car

- Fixed: the headlights lit the ground under and just ahead of the bumper, so an airborne car still seemed to drive on the ground: the beam now fades out below its axis (between about 6 and 16 degrees down), as a real low beam does; the lit ground moves away from the car as it rises, and nothing below an airborne car is lit; the beam's axis is 1.7 degrees down (was 2.9), so the road is lit further ahead

## 2.39.0 - the beam is a light

- Changed: the headlights light only what is really in front of them: the terrain between the lamps and a point blocks the beam (a crest keeps the beam off the ground behind it; 4 samples of the height atlas along each ray), so a nose-down car no longer paints a light pool on ground it cannot see
- Added: the beam is visible in the air (a short ray march per low-resolution pixel along the view ray, in the dust pass): a faint shaft in clear air at night, a strong one in rain and fog, faint from behind and bright from in front
- Changed: cheaper night lighting (an early rejection before the beam's arctangents), the dust is drawn at one eighth of the screen (was one sixth), the glass turns solid and the interior is skipped beyond 4-8 m (was 9-16 m)

## 2.38.0 - the fluid dust is gone

- Removed: the experimental fluid dust (Settings, Dust, "Fluid (test)": a CPU fluid grid with a volume render, about 40 fps): the GPU particle dust with the airflow replaced it and looks and runs better; the Dust setting is Off / Normal / Lots; the code, the shader and the `fluid` block of `surfaces.json` are removed

## 2.37.0 - weather

- Added: weather (Settings, Weather, and the Weather button in the drive: Auto / Clear / Cloudy / Rain / Heavy rain / Fog; Auto from the level's seed): clouds thicken, the sky greys and darkens, the sun and moon discs fade, fog thickens, rain streaks fall round the camera, the ground gets wet (darker, glossy with the sky in it), the grip drops by surface (`surfaces.json` -> `wet`) and on wet ground the wheels throw almost no dust but a pale cold spray mist (also on wet asphalt) and dark mud clods off the road, as real tyres do; wet ground dries in a few minutes

## 2.36.0 - tyre sets grip

- Changed: the wheel set now matters on the road: gravel tyres grip 4-8 % better on gravel, dirt, grass and sand and 10 % worse on asphalt than the tarmac set (the baseline the cars were tuned with), snow tyres are a compromise and the worst on asphalt; the numbers are in `surfaces.json` -> `tyre_sets` (grip and side stiffness per set and surface, editable)

## 2.35.0 - the cockpit

- Added: a sixth camera view, **Cockpit** ("Салон"): the camera sits at the driver's eyes inside the Peugeot and you drive looking at the dashboard, the steering wheel and the road through the windscreen (the pinch zooms, the two-finger height tilts the gaze, as in the bonnet view); the driver is not drawn when the camera is inside, and the near plane goes down to 8 cm there
- Added: the steering wheel turns with the steering (up to about 1.2 turns lock to lock), in every view
- Changed: the model format carries up to eleven meshes (steering wheel, its pivot, the driver separately)
- Fixed: a dark triangle across the Peugeot's windscreen (a decimation artefact in the body and the cabin meshes): the body and the cabin are simplified with locked borders and a tighter error

## 2.34.0 - transparent glass and the interior

- Added: the Peugeot's windows and lamp covers are transparent glass (a blended pass with a dark tint, sky reflection and a sun glint): near the car you see through the windows into a real interior (the dashboard, seats, steering wheel, door panels and a driver from the source model, a 1024 atlas, about 25 000 triangles); from beyond 9-16 m the windows turn solid dark and the interior is not drawn; the lamp covers are clear over the reflectors
- Changed: the model format carries up to eight meshes (glass and interior added); the dust wraps the car with its windows closed

## 2.33.0 - suspension

- Added: the Peugeot's suspension (wishbones, struts and springs of all four corners, from the source model) is seen through the wheel arches and moves with the wheels: the outer ends follow the hub over bumps, jumps and in corners, the inner ends stay on the body

## 2.32.0 - wheel sets

- Added: the Vulpes has three wheel sets from the source model, tarmac, gravel and snow (different tyres and rims); a free **Wheel set** button in the garage cycles them, the preview shows the choice at once, it is saved per car in the profile (old profiles load as tarmac); for now it changes the look only, the grip effect is planned
- Changed: the model format `RCA3` carries up to six meshes and the livery atlas is 2048 x 2688
- Fixed: the garage turntable shows the car with the profile's upgrades and wheel set (it showed the base car and ignored the saved profile after a change)

## 2.31.0 - real wheels

- Changed: the Peugeot's wheels are textured (the tyre's sidewall and tread, the rim's spokes) and more detailed (about 2 500 triangles instead of 1 500); the brake disc and the caliper are separate parts seen through the spokes; the caliper no longer spins with the wheel (it follows the steering and the suspension); the mud flaps are on the car
- Added: the brake discs glow orange-red when hot (per wheel: hard braking heats them, the front ones more, they cool in a few seconds; strongest at night); the model format is now `RCA3` (up to four meshes), older files still load, and the atlas is 2048 x 2560

## 2.30.0 - the Peugeot's livery as a texture

- Changed: the livery of the Peugeot (stickers, sponsor logos, the 206 numbers) is a real texture instead of blurred vertex colours: the model keeps its UV coordinates (new `RCA2` format, the old one still loads) and the converter packs the model's textures into a 2048 x 2048 atlas (`rally_car_livery.jpg`, about 0.6 MB); the game decodes it once and the car's shader multiplies the paint by it; lamps, glass and wheels are unchanged; a missing atlas falls back to the old vertex colours
- Changed: the headlights and tail lights of the Peugeot have their real insides: the reflector bowls, lamps and plastics of every lamp unit are kept and textured, the opaque glass covers that hid them are left out, and the reflectors light up with the car's state (headlights, tail, brake, reverse)

## 2.29.1 - lamps fixes

- Fixed: the car is no longer lit by its own lamps (the beam on its bumper, the red tail light on the ground under it); the tail lights light only what lies behind the rear bumper and the beam starts a little way ahead of the lamps
- Changed: the lamp halo is no longer a floating ball: a soft aureole hugs the bumper and wing around each lit lamp, so nothing shows through the car
- Fixed: the lamps of the Peugeot light up: the dark glass covers of the headlight and tail units now show the lamp behind them when it is on (found on the mesh next to the lamp points)
- Fixed: the beam rolls and pitches with the body (it used a level sideways axis), the cut-off rises on the car's right, and the dust is lit by the beam ahead and by the red tail lights behind; the lamp aureole on the bodywork is only a thin rim round the lamp

## 2.29.0 - lamp optics

- Changed: the car's headlights are lenses: unlit they are dark glass with a rim reflection of the sky and the sun, lit they have a hot core when seen head-on, a darker rim and a stepped reflector
- Changed: a real low beam from the two headlights (wide, a sharp upper cut-off that rises on the right, falling off with distance) lights the ground, the haze and the dust, instead of one cone
- Added: a soft halo around every lit lamp (headlights, position, tail, reverse) that follows the viewing angle, also on the Peugeot
- Added: position lamps: the corner pieces of the front lamp units are found on the mesh (any new model gets them from its colours) and burn with the low beam; a mesh without a corner piece falls back to the headlight
- Added: a quick time-of-day switch in the drive (the "Time" button next to the gear): Auto (the level's own clock), Day, Evening, Night; it applies at once and freezes the sky at that phase, so the night lamps can be checked in free ride

## 2.28.1 - dust fixes

- Fixed: a dark rectangle in the dust cloud around the car (the airflow box's edge showed through the lighting); the dust no longer takes a light or shade term from the box
- Fixed: grainy haze on grass, dirt and other loose surfaces: flatter sprite noise, less brightness variation between puffs, larger fine puffs

## 2.28.0 - realistic dust, camera views, real lamps

- Changed: the dust (Settings, Aerodynamics On, levels Normal and Lots) is tens of thousands of separate soft particles on the GPU that follow the simulated air: curl-noise turbulence (divergence-free, three scales), the car's trailing vortices and vortex street, the recirculation behind the rear face, the wake that scours the road behind the car (the garage's aero parts strengthen it), plumes that rise, particles that push each other apart and drag the air (a density field in the airflow grid), self-shadowing by the sun through the dust, warm tan colour; it never passes through the body (pushed out with a margin) and starts under the wheel arches; drawn at 1/6 resolution (soft against the ground and the car) so it fits the 30 fps target (39-48 fps measured on the POCO F3); numbers in `surfaces.json` (`dust`, `dust_physics`, `dust_air`, `airflow`)
- Changed: the chase camera hardly falls behind when the car accelerates (stiff along the heading, soft sideways and up)
- Added: Settings, Motion blur (Off by default, Low, High): the camera's motion blur of the scenery (each pixel is reprojected with last frame's view-projection from the depth buffer) that gives the sense of speed; the car stays sharp (masked), it grows with speed and is off when crawling; costs about 3-7 fps
- Added: camera views: behind (default), bonnet, bumper, roof, orbit; the "View" button under the status line cycles them (long press: default distance and height); two fingers change the distance (pinch; zoom in the bonnet and bumper views) and the height (both fingers up or down; gaze tilt in those views); everything is remembered per view
- Changed: dust as in the real world (research: USDA Forest Service on vehicle dust, Goossens et al., mining CFD, EPA AP-42): the cloud stays behind the car and is mixed up only to about 1.7 car heights by the wake (it no longer shoots up), the fine fraction hardly settles and lives 14-20 s on gravel and dirt (the trail stays for many seconds, then spreads and thins), turbulence is the ambient air's with the wake's burst for the first seconds; on asphalt and water the dust is tyre smoke or spray: pale, very fine, short-lived (3-4 s), with no warm soil tint and no grit
- Changed: the dust looks like soft smoke, not grains: large puffs with a smooth falloff and a slowly wobbling radius, a second layer of soft noise for billowing structure, every puff shaded like a soft sphere (sun side bright, a soft grey underside), a 16-bit low-resolution target with dither (no banding), fewer flying specks
- Added: Settings and quick settings, Camera drag inversion (None, Horizontal, Vertical, Both)
- Added: four PhysX-like options of the GPU dust, each on its own switch (Settings and the in-drive panel): particle pressure (position based fluids: the near particles are binned into the airflow cells and pushed apart by the density constraint with the poly6 and spiky kernels and XSPH viscosity; off by default, the heaviest), the car's exact shape (a signed distance field of the body mesh, so the dust wraps the wheels, wing and mirrors), collisions with the world (trees, rocks, and sliding along slopes and walls) and a settled dust layer (it tints the ground and the car's wake lifts it again); on the POCO F3 they cost about 1-2 ms together
- Added: quick settings in the drive: the gear button next to the View button opens a panel (dust level, aerodynamics, the four options, motion blur, camera view) that applies at once
- Changed: the Peugeot's headlights, tail and reverse lamps keep the shape of the mesh and show the car's state (headlights, brake, reverse); their positions are found on the mesh by colour class, so the next car models get their lamps in place without extra data (docs/vehicles.md)
- Added: the airflow around the car on the GPU (a box that moves with the car, BFECC, wake model, the body and garage parts as voxels) with a debug slice (`airflow=1`), and `gpuprof=1` for GPU stage timings in the log
- Changed: the dust of gravel, sand, dirt and grass is as soft as the asphalt smoke (billowing puffs, coarse grit larger and softer); only gravel throws flying stones; grass raises a quarter of the dust (a turf holds the soil, per EPA/Arizona DEQ/Washington Ecology guidance); the cloud no longer darkens near the car (weaker self-shadow)

## 2.27.0 - dust in the airflow

- Added: the air around the car is simulated on the GPU (a box that moves with the car, the body and the garage parts as voxels, a wake model) and the dust follows it: the particles run on the GPU, take the air's velocity inside the box and the wind outside, are pushed aside by the body, keep their inertia and settle onto a terrain height window; the Settings switch "Aerodynamics" (On by default) turns it off, then the simple CPU particles run as before; the "Fluid (test)" dust is unchanged
- Added: the debug extra `airflow=1` draws a side slice of the airflow (see docs/game.md)

## 2.26.0 - physical dust

- Changed: dust particles obey physics: air drag with relaxation time, fine and coarse classes (fine hangs and drifts, coarse falls), a wake behind the car, turbulence, wind that grows with height, wheel throw and settling; a nicer soft sprite lit by the sun and the headlights (numbers in `surfaces.json`, `dust_physics`)
- Added: Settings, Dust: "Fluid (experimental)": the dust is a real fluid on the GPU (advection, pressure projection, the car as a solid) rendered as a volume; heavier, off by default

## 2.25.0 - aerodynamics

- Added: car aerodynamics as physics: drag and downforce grow with the square of speed, downforce presses each axle separately, ground effect depends on the real ride height (suspension compression), sideslip adds drag, side force and loses downforce; numbers in `aero.json`
- Added: body parts in the garage: front splitter, rear wing and underbody (3 levels each), the wing angle (Low / Mid / High, free) and a readout of the downforce at 150 km/h, the drag change and the estimated top speed
- Changed: the Vulpes WRC has less rolling resistance and more air drag, so that speed depends on aerodynamics (0-100 km/h unchanged); old profiles load with no parts and a mid wing angle

## 2.24.1 - icons only

- Changed: the pictograms are at the top, in the centre (in a rally the Autopilot button moves right, next to Reset)
- Changed: the co-driver's words are no longer shown while driving; only the pictograms are, and only just before the corner, crest or junction they describe (about 110 m ahead, more at speed), as in DiRT Rally 2.0; Settings, "Co-driver": Off / Icons / Icons and voice

## 2.24.0 - studio voices, signs and the icon

- Changed: corner severity is on the usual rally scale: 6 an easy bend ... 1 sharp, plus the hairpin (new calls "left six" / "right six")
- Added: pictogram pace notes (no words; they appear only just before the corner, crest or junction and go once it is passed, as in DiRT Rally 2.0): a strip under the status island with a circle and an arrow per note (colour and bend by severity, the number, a caution mark for a corner that tightens, hump, junction and finish symbols, "into" arrows and distance bubbles)

- Added: signs along the special stage that say what the co-driver says: warning triangles (curve, double curve, hairpin, bump), chevron boards, route arrows and no-entry discs at junctions, distance boards, finish boards, a stop and a start sign; Settings -> Signs on the stage
- Added: the app icon (a rally car and the name on an orange background, adaptive and legacy)

- Added: recorded co-driver voices: three free neural voices (Russian: Dmitri; English: Alba (UK), LibriTTS 1) as 42 clips each (corners 1-5 both ways, hairpins, tightens / opens / long, into, crest, jump, junctions, the finish, distances) stitched into the calls; Settings -> Co-driver voice lists them with a sample (a play button), the system synthesizer and "My voice" with a choice per language; the voices are licensed CC0 or CC BY 4.0 (credits in the README); the clips are rendered by `tools/voices/`
- Added: record the co-driver in your own voice, phrase by phrase ("Record my voice...": the microphone, silence trimmed, loudness matched); a call uses your voice when all its phrases are recorded, otherwise the system voice speaks it
- Changed: Russian calls say "дальше" for "into"

## 2.23.0 - the co-driver

- Added: pace notes in rallies, made from the shape of the stage: corners (direction, severity 1-5 or a hairpin, tightens / opens / long, "into" the next corner), crests and jumps, junctions with side roads and the finish, called 80-140 m ahead; shown as text and spoken by the system speech synthesizer (Russian or English by the device language); Settings, "Co-driver": Off / Text / Text and voice

## 2.22.0 - the Rally mode

- Added: the Rally mode (the "Rally" button on the start screen): four rallies with fixed levels, each with the special stage and the city super-special (2 laps) against the clock: a 3-2-1-GO countdown, check lines, splits, penalties (reset +10 s, leaving the road for 3 s +5 s, a skipped check line +30 s), results after every stage and personal records per stage and per rally; free drive is unchanged
- Added: start, check and finish lines on the special stage (checkered start and finish, yellow checks every kilometre)
- Debug: the intent extras `rally` N (1-based) and `rallystage` 0/1

## 2.21.0 - the city super-special

- Added: a closed super-special loop of 1.2-1.8 km through the streets of the first city (a rectangle of 5-8 blocks with detours, clockwise or counter-clockwise) with a checkered start/finish line and yellow check lines across the street every ~400 m; numbers in `rust/roads.json` (`city_loop`)
- Added: the autopilot drives the loop (2 laps, `laps` in `rust/autopilot.json`) when the car is on it and stops on the finish line
- Debug: the intent extra `road` 5 starts on the loop's start line

## 2.20.0 - side roads and junctions

- Added: 4-7 junctions along the special stage with gravel side roads (150-450 m, 5 m wide, a flared mouth flush with the stage) that end in a dead end; they have the stage's loose layer, ditches and rocky edge; numbers in `rust/roads.json` (`side`)
- Changed: the autopilot's settings (style and the own style's numbers) moved from the Settings screen into a panel that opens with a long press on the Autopilot button; changes apply at once while driving

## 2.19.0 - autopilot's own style

- Added: the autopilot's own style (Settings -> Autopilot style -> Own): cornering speed, late braking, top speed, corner cutting, slides and the margin to the road edge are set in steps; the style also joins the long-press cycle on the button

## 2.18.1 - autopilot on the whole drive

- Added: three autopilot styles (calm / normal / aggressive): Settings -> Autopilot style, or a long press on the button while driving; the aggressive style slides through bends (steering by the direction of travel, a handbrake flick into slow bends) and is about 10 % faster on the stage; the numbers are in `rust/autopilot.json` (`styles`)
- Fixed: the autopilot no longer cuts across the fields from the start: it drives the whole way (the liaison, the city streets between its end and the stage start, the stage) and stops at the finish
- Changed: the car may use its full top speed (no speed caps except the grip in bends, braking distance and crests); cross-country the cap is 20 m/s, crests 30 m/s (`rust/autopilot.json`)

## 2.18.0 - stage relief and autopilot

- Added: crests and jumps on the special stage (series of 3-5, 0.6-2 m high, a long take-off and a short landing side; none in tight bends, near the road ends or on a causeway)
- Added: ditches along both road edges (deeper on the stage), a 0.8 m loose gravel verge and rocks 4-8 m from the stage
- Added: `rust/roads.json` holds all these numbers; a `roads.json` next to the profile or `RALLYNVX_ROADS` overrides them without a rebuild
- Added: the Autopilot button (beside Reset and New level): it joins the special stage from anywhere, drives the least-curvature line at the speed the grip allows and stops at the finish; any touch of the controls hands control back; numbers in `rust/autopilot.json`

## 2.17.0 - tyre dust

- Added: tyre dust and spray that depend on the surface (beige on gravel and dirt, thick yellow on sand, almost none on grass, smoke on tarmac in a hard slide, spray on water): rolling makes a thin trail, sliding and wheelspin make thick clouds; the puffs rise, widen, drift with the wind and fade; they are dark at night and thin out near the camera
- Added: the `Dust` setting (Off / Normal / Lots: 0 / 400 / 800 particles); the dust numbers per surface, the particle caps and the wind are in `rust/surfaces.json`
- Added: one wind per level (a slowly wandering direction and speed with gusts) that carries the dust, drifts the clouds, bends the trees, bushes and grass downwind and sets the wind sound (the apparent wind: driving into the wind is noisier); its numbers are in `rust/surfaces.json`; no side force on the car yet (tech debt)
- Debug: the intent extra `steer` (held after 1.5 s, like `throttle`) for screenshots of a slide

## 2.16.0 - grip by surface

- Added: the grip, the tyres' side stiffness and the rolling resistance depend on the surface under every wheel (asphalt, gravel, dirt, grass, sand, rock, shallow water); off the tarmac the car is noticeably slipperier, sand is slow
- Added: a loose gravel layer on the special stage: the wheels sweep it into a packed, grippier line while the sides stay loose; the map resets with a new level
- Added: the HUD shows the name of the surface under the car
- Added: the grip numbers live in `rust/surfaces.json` (embedded); a `surfaces.json` next to the profile (Android) or `RALLYNVX_SURFACES` (desktop) overrides them without a rebuild

## 2.15.0 - roads

- Added: every level has two random roads: a tarmac liaison from the spawn to the first city and a gravel special stage of about 6-10 km from the city's far gate through the terrain to the open (or to the second city). They are generated from the level seed, never double back, avoid lakes, keep a minimum bend radius (13 m) and a grade limit (14 %), and are carved into the terrain (cut and fill); trees, rocks and grass keep off them; tarmac has edge lines and a dashed centre line, gravel has wheel ruts and a loose berm. The car spawns on the liaison facing along it
- Added: `surface_at` reports grass, dirt, sand, rock, gravel, asphalt or water for every point (the base for grip, dust and sound in the next cycles)
- Debug: the intent extra `road` (1 liaison, 2 stage start, 3 stage middle, 4 stage end) puts the car on a road

## 2.14.0 - free drive only, camera control

- Added: look around by dragging a finger on free space (also while driving): swing the camera round the car and raise or lower it; it returns behind the car once the car drives forward faster than about 7 km/h. While reversing the camera swings in front of the car and looks back to show what is behind, and returns when you drive forward

- Removed: the delivery and race missions (the mission list, city routes, cargo, medals, timer, target arrow, result screen) and the Missions menu. The main menu is now Free drive, Scenes, Garage, Settings. A rally mode (special stages with times) will come back as an option, free drive stays
- Changed: while nothing earns money, the garage is free (purchases show "Free" and leave the balance alone); the profile keeps the money and the garage, old files with mission results still load (the results are ignored)
- Removed: the mission sounds (checkpoint, success, failure) and the mission HUD
- Debug: the intent extras `camyaw`, `campitch` (radians) and `throttle` (held after 1.5 s) for screenshots without touching the screen

## 2.13.0 - day and night, car lights

- Added: a full day-night cycle (a day is 12 minutes): the sun and the moon travel a full circle; sunrise and sunset colour the sky, the night is dark but passable by moonlight; one light source switches from the sun to the moon with no shadow jumps; a level always starts by day
- Added: a visible sun disc with a glow, a moon with phases (its lit side faces the sun) and stars; clouds darken at night; windows of the city buildings and the street lamps glow at night
- Added: headlights (automatic at dusk and night) that light the ground and objects ahead, red tail and brake lights, white reverse lights with a glow on the ground behind the car
- Fixed: the sky was drawn mirrored vertically, so clouds and the sun glow were never where they should be
- Debug: the intent extras `tod` and `lights` (0 headlights, 1 + brake, 2 + reverse) for screenshots; `tod` (0 midnight, 0.25 dawn, 0.5 noon, 0.75 dusk) freezes the clock for screenshots

## 2.12.0 - cars only, real Vulpes WRC data

- Removed: the helicopter, the drone and every air mission (rings, sling cargo, recon, rescue), their controls, sounds, models and settings. The mission list is now three deliveries and three checkpoint races, two of them in the city. Results saved for the removed missions are ignored; money, the garage and car results stay
- Changed: the Vulpes WRC follows the data of the real rally car: 1230 kg, 300 hp class thrust tuned to about 3.9 s (checked by a unit test) from 0 to 100 km/h, top speed 52 m/s (187 km/h); see docs/vehicles.md
- Changed: `vehicles.json` has only cars now (the aircraft, flight and sling sections are gone; a custom file may still contain them, they are ignored)

## 2.11.1 - the real Vulpes WRC model

- Changed: the Vulpes WRC now uses a simplified copy of the real model (body 20 000 triangles, wheels 2 500 each, livery baked into vertex colours) instead of the generated hatchback; `tools/carconv` converts the glb and `tools/rcar_preview.py` previews it. Measured on a POCO F3: the frame rate did not change between 9 000 and 42 000 triangles for the car, so the quality stays high
- Credit: the model is by vecarz (Sketchfab), CC BY-NC-SA 4.0 (credit, non-commercial, share alike); see `rust/assets/ASSET-LICENSE.md`; the garage shows the credit

## 2.11.0 - garage

- Added: a garage in the main menu: a turntable preview, buying and selecting cars, four upgrades per car (engine, tyres, suspension, brakes, three levels each)
- Added: a second car, the Vulpes WRC (3000): a generated rally hatchback (flared arches, roof scoop, rear wing, spot lamps), light and quick; the selected car with its upgrades is used in free drive and in ground missions
- Added: `model`, `price` and `upgrade_cost` keys for cars in `vehicles.json`; the profile keeps the garage (old profiles load with only the off-roader)

## 2.10.0 - hybrid render mode

- Added: a Hybrid render mode next to Raster (Settings, default on only with High quality): soft sun shadows (PCSS), long terrain shadows by ray marching a height map, and ambient occlusion (terrain hollows, plant and rock bases, building feet)
- Added: an optional automatic fallback to Raster when the frame rate stays below 50 for four seconds (off by default; the HUD says "Raster (auto)")
- Changed: the settings screen has "Render mode" and "Auto fallback to Raster" rows

## 2.9.0 - car controls and settings

- Added: gas and brake pedals and steering for the car instead of the two sticks: two arrow buttons by default, an on-screen wheel or tilt of the phone as options; the pedals give full gas and full brake on touch by default, with an "Adjustable" force option (the lower the touch, the harder the press); the brake pedal reverses when stopped or rolling backwards
- Added: tilt calibration; sensitivity low / normal / high; pedals on the left or right
- Added: a settings screen (quality, steering, sensitivity, volume, pedal side, tilt centre, hints) from the main menu and the pause panel; the main menu quality button moved into it
- Added: a master volume setting; an analog brake (a lighter press brakes less); a one-time hint for each steering type
- Unchanged: the helicopter and the drone still use the two sticks

## 2.8.0 - sound

- Added: procedural sound without files: car engine (pitch follows speed and throttle), rolling noise on ground and in water, tyre squeal, helicopter rotor, drone buzz, wind
- Added: one-shot sounds for touchdown, crash, splash, checkpoint, boarding and mission success or failure
- Added: a sound on/off button (remembered); the sound fades out on pause and when leaving the screen

## 2.7.0 - rescue

- Added: helicopter rescue missions: fly to 1-3 survivor sites, land and wait while the people board, then land on the base pad and wait to unload; hard touchdowns hurt the passengers; reward +50 per person
- Added: orange-vested survivor figures at the pending sites, HUD with sites done, people aboard and boarding progress
- Changed: the sixth mission card rotates sling / recon / rescue with each Refresh (before: sling / recon)

## 2.6.0 - graphics II

- Added: generated trees (4 variants), bushes, rocks and grass tufts (grass: none on Low, 1500 on Medium, 3000 on High, within 40 m), swaying in the wind
- Added: a moving sun (12 minute cycle, dawn - noon - dusk, no night) with matching sky, fog and light colours; procedural clouds; sky reflections on paint, glass, chrome and windows
- Added: shallow lakes in low ground with depth colour, waves, reflections and splashes; the car drives through them with extra drag (`water_drag` in `vehicles.json`), aircraft are unaffected; props, bushes, grass, tyre tracks and mission points avoid water
- Added: cities with ten facade colours, roof cornices, street lamps and even asphalt with lane marks
- Changed: levels look different from earlier versions (props avoid lakes)

## 2.5.1 - stability

- Changed: the off-roader's centre of mass is 0.35 m lower relative to the wheels, so hard turns make it slide instead of rolling over (rollover limit 1.36 g against the tyre limit 1.1 g, before 0.9 g); the look is unchanged
- Fixed: tyre tracks no longer appear while the car only drops straight down

## 2.5.0 - graphics I

- Added: generated material textures for the terrain (grass, dirt, sand, rock) blended by height, slope and noise, with a second scale against visible tiling
- Added: detailed generated vehicle models: a lofted off-road body with glass, bumpers, lamps and spinning wheels; a helicopter with rotor and tail rotor; a drone with four propellers. Paint, glass and chrome get sun glints
- Added: tyre tracks left on the ground behind the car
- Added: a softer horizon in the sky; the body colours of each vehicle are set in the `look` section of `rust/vehicles.json` (docs/vehicles.md)

## 2.4.0 - city missions

- Added: urban missions marked "City": a cargo delivery through the streets, a street race over turning intersections, rings flown over a street between the houses, and sling cargo carried from one roof to another; they start on a street at the west edge of the city
- Changed: the sling mission now always uses roof pads; the list still has six missions; the menu array carries an urban flag

## 2.3.0 - city

- Added: every level now has one or two procedural cities (about 530 x 530 m on a flat plateau): a grid of streets, blocks, parks and buildings of 4-90 m with windows; the tall buildings stand in the centre
- Added: buildings are solid: the car cannot drive through them, an aircraft that touches a wall or underside crashes, and a helicopter or drone can land softly on a roof (so can the sling cargo)
- Added: the quality setting also sets the city draw distance (300 / 400 / 450 m)
- Changed: levels look different from earlier versions for every seed (the terrain is flattened under cities); props and mission goals stay out of cities. City missions come next

## 2.2.1 - vehicle parameters in one file

- Changed: all numbers of the car, helicopter, drone, flight rules and the sling now live in `rust/vehicles.json` (embedded in the app); no behaviour change. Desktop can try another file with `RALLYNVX_VEHICLES=path` (docs/vehicles.md)

## 2.2.0 - flight

- Added: a helicopter and a drone with arcade controls (sticks released = hover, soft landing or crash) and three air missions: ring races, cargo on a sling (helicopter) and drone recon
- Added: the mission list now has six missions; new flight HUD (altitude, speed, rope state), goal rings, pads and recon zones in the world
- Desktop: flight keys (W/S pitch, A/D roll, Q/E yaw, Up/Down lift); `RALLYNVX_MISSION=4` starts the ring mission
- Known: no cities and no rescue yet; locking the screen mid-mission restarts it

## 2.1.0 - missions

- Added: missions: a list of four random missions (two cargo deliveries, two checkpoint races) with a goal beacon and checkpoint rings, a timer, cargo integrity (hits, hard landings and rollovers damage it), an arrow and distance to the target, pause and results screens
- Added: bronze / silver / gold medals by time and cargo condition, money rewards, a saved profile (money, best medal and time per mission; a corrupt file is kept as `.bak`), Refresh for a new list
- Added: main menu (Missions / Free drive / Scenes / Quality) with the money shown; desktop: `RALLYNVX_MISSION=<0..3>` starts a mission, `M` aborts it (docs/game.md)

## 2.0.0 - off-road game

- Added: a game next to the scene viewer: "Free drive" on a random level (every level has a seed shown in the HUD; the same seed gives the same level)
- Added: seeded hilly terrain with rocks and trees, an arcade off-road car with suspension, drift (handbrake) and collisions, a chase camera, touch controls (left stick steer/throttle, right stick down brake) and keyboard on desktop
- Added: a raster renderer (render mode C) with tiled terrain LODs, fog, sky and a sun shadow map; Low / Medium / High resolution on the start screen
- Added: start screen (Free drive / Scenes / Quality); desktop: `rallynvx-desktop --game [seed]` (docs/game.md)
- Unchanged: the ray-traced Scenes mode and its scene files

## 1.8.0 - prism

- Added: the "Prism" scene: sunlight through a low slit passes a glass prism and splits into a spectrum on the floor; the prism itself shows coloured edges
- Added: glass `ior` and `dispersion`, the `prism` shape and `caustic_receivers` for scene files (docs/scenes.md); `scene` setting: 0 Classic, 1 Sun room, 2 Materials, 3 Prism, 4 = a loaded file (was 3)
- Added: photon passes (sun photons through the prism into a photon map on the floor and walls, averaged over frames) and a hero wavelength for camera rays through dispersive glass

## 1.7.0 - scenes

- Added: scenes are data now (JSON): three built in (Classic = the old scene, Sun room, Materials) and your own files from a folder; picker on the Scene tab (Android and Linux); format in docs/scenes.md
- Added: sun light (directional, soft shadows, Day / Dusk / Night follow the Sky setting), coloured glass panes that tint the light passing through them, glossy material (diffuse under a specular coat), tinted glass, rounded and spinning boxes
- Added: "Sun room" (sunlight through a ceiling slit onto coloured glass, mirror, gold, matte, glossy and clear glass objects) and "Materials" (a showcase row)
- Changed: shadows for all lights are analytic per primitive (no scene march); the shader is generated from the scene's primitive list

## 1.6.0 - helicopter camera

- Added: "Helicopter" camera mode (Scene tab, also on the desktop panel): left stick turns (x) and moves forward/back (y), right stick changes height (y) and strafes (x); the view is fixed slightly downward. Desktop: A/D turn, W/S forward, Up/Down arrows height, Left/Right arrows strafe
- Changed: the `cam_mode` setting has a third value (2)

## 1.5.0 - PlayStation 1 look

- Added: "PlayStation 1 look" switch (Quality tab, also in the desktop panel): the shorter screen side renders at 240 px and is shown as hard pixels (no filtering), colour is cut to 15 bits (5 per channel) with a 4x4 ordered dither on the source pixels, the camera position snaps to a 1/24 grid, animation advances at 12 fps, no sub-pixel jitter; it replaces the render scale setting, temporal smoothing and the denoiser still apply
- Measured on POCO F3 (Balanced, hybrid): 43-47 fps at 240x528, GPU about 20 ms
- Tried and dropped: integer (fixed-point) Menger sponge SDF, about 60% slower on Adreno (docs/bench.md)

## 1.4.0 - Linux desktop build

- Added: `rallynvx-desktop` for x86-64 Linux (winit window + egui settings panel, mouse and keyboard camera, settings saved to `~/.config/rallynvx/settings.ini`), packaged as a tar.gz, an AppImage and an Arch `PKGBUILD`
- Changed: `Gfx` can be created from any wgpu surface and draws an optional overlay after the present pass; presets and setting keys are now also in Rust
- The Android app is unchanged (still 1.3.0)

## 1.3.0 - analytic shadow rays

- Changed: shadow rays use analytic occluders with a broad-phase instead of marching the whole scene: spheres are analytic; the Menger box gets a slab test and the torus a bound-sphere test, followed by SHORT bounded marches only when the ray reaches those objects
- Changed: Balanced default bounces 4

## 1.2.0 - half-resolution global illumination

- Added: GI resolution setting (Full / Half / Quarter) on the Light tab; it applies only in hybrid mode with GI on (in path-tracing mode GI stays inline; with GI off nothing is deferred); presets: Performance has GI off (resolution irrelevant), Balanced = Half, Quality = Full
- Added: half/quarter-resolution GI passes (gi_trace, gi_temporal, gi_atrous, composite; they run only for Half/Quarter, "Full" computes GI inline in the trace pass as in 1.1) with their own temporal accumulation, a-trous denoise and bilateral upsampling
- Added: per-pass GPU timings in the HUD "Full" mode (when timestamp queries are supported)
- Changed: default GI is half resolution
- Measured: on POCO F3 at fixed 0.33x hybrid, Half is +46% fps vs Full (10.4 -> 15.2 fps), Quarter +53%; see docs/bench.md

## 1.1.0 - on-screen sticks and fly camera

- Added: two on-screen gamepad sticks (left = move / zoom, right = look / orbit)
- Added: free-fly camera mode alongside the orbit camera
- Added: four Scene tab settings - camera mode, on-screen sticks, move speed, look speed

## 1.0.0 - initial release

- SDF scene rendered with Rust + wgpu (Vulkan): hybrid and progressive path-tracing modes
- Temporal reprojection with depth / normal / id validation, neighbourhood clamp and Halton jitter
- A-trous denoiser (0-3 passes), Catmull-Rom upscale, sharpening, exposure, tonemapping, dithering
- Adaptive resolution with a target fps, optional checkerboard tracing, frame limit
- Presets (Performance, Balanced, Quality), shadows / global illumination / caustics / reflections switches, lamp colours, sky, auto-orbit
- Material 3 Expressive settings UI (android-ui-kit): five tabs, themes, haptics, HUD; English and Russian
- Safety governor: very slow frames automatically reduce render scale and samples per pixel
- Orbit, pinch zoom and double-tap reset gestures
