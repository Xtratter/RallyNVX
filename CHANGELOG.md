# Changelog

[Русский](CHANGELOG.ru.md) · **English**

## 2.71.0 - the exhaust gas itself
- Added: the tail pipe breathes out the exhaust gas all the time, not only when it burns. The amount, the temperature and the unburnt fuel in it come from the engine model (the moles that leave the collector for the open air in every frame): a thin wisp at idle, a thick plume at full throttle. The gas is invisible while it is hot and dry, as in life; the water in it (about one part in eight) condenses into a **white vapour** where it meets cold or humid air, so the plume shows on a cold morning, at night, in the rain and in the fog, and hardly at all on a dry hot noon (the air's temperature and humidity are estimated from the sun's height, the clouds, the rain and the fog). A rich exhaust (the anti-lag's unburnt fuel) makes **dark soot** in any air
- Each puff leaves at the speed the mass flow gives over the pipe's area (a few m/s at idle, up to 55 at full throttle) and the car's own speed, is slowed by the air (the wind carries it and a moving car leaves it behind), rises while it is hot, spreads as it takes in air, thins with the square of its growth and evaporates in about two seconds; it is drawn as the soft noisy-edged puffs of the dust, lit by the sun, the sky and the lamps, in front of the fire
- Not yet: the shimmer of the hot air over the pipe (the refraction)
## 2.70.0 - a clean driving screen, pedals as pedals, the reverse only by hand
- Changed: the settings no longer hang all over the driving screen. The view, the quick settings, the time, the weather, the sound, the gearbox, the car menu and "New level" are in one **menu** (the button with three bars, top left under the status line); the screen keeps the status line, the autopilot, the reset, the cluster, the steering buttons and the pedals
- Changed: the pedals are drawn as pedals (a metal plate and a ribbed rubber pad that sinks as it is pressed and takes the accent colour from the bottom up; the gas is the tall organ pedal with a hinge) and stand against the edge of the screen: the gas outermost, the brake beside it, the clutch (the manual) next, the handbrake above the brake, the D / R selector where the clutch is (the automatic has none)
- Changed: the instrument cluster (the tachometer, the boost, the gear) is above the steering buttons, which are a little further apart
- Changed: the automatic never picks the reverse by itself: not from the brake pedal at a standstill (the brake pedal only brakes now, and holds a car that stands) and not from a negative throttle; the reverse is chosen by hand (R in the automatic, the gear in the manual); only the autopilot reverses as before
## 2.69.0 - the car menu (aids and tuning), an instrument cluster, and a start that cannot hang on the sound

- Added: the **Car** button (top left, right of D / R) opens a menu. On the left the driver aids, each on or off: anti-lag, ABS, traction control, the downshift blip, the flat shift (spark cut), the rev limiter and the blow-off valve. On the right the power of every part in steps of 10 %: engine power (30-200 %), turbo boost (0-200 %), brakes (30-250 %), tyre grip (60-150 %), suspension (50-180 %), downforce (0-300 %) and the final drive (70-130 %). It is kept, "Reset tuning" puts everything back. No sliders: a minus, the value and a plus
- Added: ABS and traction control that mean something. Without them a wheel braked past the peak of its grip locks and slides on 78 % of it, a wheel driven past the peak spins on 62 % of it, in every direction, so the stops get longer, the cars loses the steering in a hard stop and a launch loses time. Without the limiter the engine goes on until its own power gives out (about 9000 rpm); without the flat shift or the blip the shifts are plain; without the blow-off valve the pressure goes back through the compressor and the turbo flutters
- Added: the instrument cluster at the bottom: a tachometer (0-8000 rpm, the needle, the red zone from 7000), a boost gauge on an inner arc (0-0.6 bar, with the number), the gear in the middle with the mode
- Fixed: a start that could hang for ever on a slow phone. The audio thread held the synth's lock while it rendered a block and the game thread waited for the same lock every frame, so a phone that could barely keep up starved the game. The game now only leaves its message in a lock that is held for a moment; and when a block takes over half of its own length the engine is simulated with one step per sample instead of two
- Added: for a start that does not work: after 25 s on the loading screen, or when the game thread dies, a "Copy the log" button (the device, the version, the error, the tail of the app's log) to paste where it can be read; a crash of the last run is shown on the next start in the main menu with a "Copy" button
## 2.68.0 - the exhaust fire as a dense cloud that lights its surroundings

- Changed: the fire is no longer a spray of particles but a thick cloud, as dense as dust: up to 90 overlapping puffs (0.10 to 0.40 m wide, a body that is dense to its edge), kept at the tail pipe (they leave at 6 to 30 m/s and the air stops them within a metre or two), burning for about a third of a second and glowing stronger (the radiated power is 30 % up)
- Added: the fire lights everything round it: a point light at the glow-weighted centre of the cloud, in its mean black-body colour, falling with the square of the distance, on the ground, the car and the dust, with its highlight on glossy surfaces; a bigger burn lights more, a pop less

## 2.67.1 - a reverse selector for the automatic

- Added: a "D / R" button in the automatic (right of the gearbox button; hidden in the manual): in R the gas pedal drives backwards, so the reverse no longer needs a stop and a negative throttle first
- Fixed: a car that rolls backwards in D waited for a full stop before the gas worked. Now the gas takes the first gear at once (the clutch pulls the car forward and slows the roll back); the reverse by the negative throttle at a standstill and the autopilot work as before

## 2.67.0 - real fire at the tail pipe, a smooth engine

- Changed: the yellow cone is gone, the exhaust now throws real fire. The fuel that burns in the engine model's exhaust (its amount in moles, counted on the audio thread) comes out of the tail pipe as parcels of burning gas. Each one has its own temperature (about 2000 K), speed (25 to 70 m/s from the pipe's overpressure, more for a bigger burn, on top of the car's own speed), size (the volume of the hot gas, `n R T / p`: a pop of 0.002 mol is a few centimetres, a spark cut's 0.02 mol a fireball of ten) and age. They slow down by drag (a moving car leaves the fire behind in the still air), rise as hot gas does, spread as the jet takes in air and cool in the air's mixing; the colour is that of a black body at the parcel's temperature (white-yellow, orange, dull red, then out) and the glow goes with the fourth power of the temperature and thins as the parcel spreads. Drawn as additive, ragged, flickering sprites with a hotter, whiter heart; the fire lasts a quarter of a second
- Fixed: the engine revved in steps. The game sends the revs 60 times a second and each was a step for the sound engine; two smoothers of about 12 ms turn the stairs into a ramp

## 2.66.0 - the test track

- Added: "Test track" in the main menu: a bare, flat, endless sheet of tarmac (no trees, rocks, grass, water or roads) with white lines every 50 m (and bold ones every 250 m) to see the speed against, the car standing on it from the start. For trying the sound, the drivetrain, the flames and the handling. The ground is real tarmac for the grip, the dust and the tyre sound, and gets the wet-road look in the rain; "New level" leaves the track

## 2.65.1 - the game starts again

- Fixed: 2.65.0 hung on the loading screen at "Compiling shaders 64 %". The flame added a model slot, and the check that every slot has its mesh failed at start: the game thread died and nothing drew, while the loading screen waited for ever. The flame's mesh is in the list now
- Added: if the game thread dies, the loading screen says "Startup failed: the game stopped (see the log)" instead of waiting

## 2.65.0 - flames from the exhaust, by the physics

- Added: the tail pipe spits fire exactly when the engine model burns fuel in the exhaust: the anti-lag's pops on a lift at high revs (fuel and air burning in the hot exhaust, in pulses), the flame of a hard up-shift's spark cut (the unburnt fuel the engine pumps out lights up), nothing on a plain lift or at full throttle. The flame is two emissive cones (a yellow core in an orange coat) at the real tip of the exhaust pipe found in the model (x 0.46, 0.25 m high, 1.85 m behind the centre); its length follows the burn rate and flickers, it rises at once and falls in about 70 ms with the sound of the pop
- Added: the engine's tone on and off the throttle: a lift takes weight and brightness off the exhaust (the engine drags instead of pulling) and a third off the block's rattle; a flame in the pipe brings the sound back. The intake is silent off the throttle by itself (the air no longer flows)

## 2.64.0 - a real transmission: the engine, the clutch, the gears and a manual

- Added: the car is now driven through a real drivetrain. The engine has its own speed and inertia; its torque comes from a map measured from the sound's engine model (a turbocharged two-litre: 200 N m at 1000 rpm, 325 N m at 5000, engine braking with the throttle shut) with a turbo lag and an idle governor; a clutch carries the torque to the wheels through the gear (six gears and a reverse, ratios 2.92 to 0.92, final drive 5.0) and the force at the ground is what the clutch carries. The old constant engine force is gone: the pull now depends on the gear and the revs, falls at a shift and the engine brakes in the gears. The rev limiter is at 7800 rpm
- Added: the automatic: it picks the gear from the revs and the pedal, the clutch bites with the revs (a gentle creep at idle, a slipping launch at full throttle), a shift takes 0.22 s with the clutch open, a hard up-shift cuts the spark, a down-shift blips the throttle; the reverse comes from the negative throttle at a standstill. The autopilot always drives the automatic
- Added: the manual (МКПП): a button "Gearbox: auto / manual" (top left, under the view button); in the manual a clutch pedal appears left of the brake and two shift buttons (up and down) above it. The sequence is R, N, 1 to 6; a shift takes 0.12 s and a hard up-shift cuts the spark (a flat shift). The clutch bites on the pedal's travel (its pedal follows the finger's place on the pad); let it go in gear at a standstill and the engine stalls (the sound stops, the gear indicator blinks "stalled") and starts again by itself after a moment once the clutch is pressed or the box is in neutral; in neutral the engine revs free to the limiter. The choice is kept
- Added: the gear indicator at the bottom: the gear (R, N, 1-6), the mode and a bar of the revs with a red zone
- Changed: the sound follows the game's engine: the revs, the throttle and the spark cut come from the drivetrain
- Not yet: the exhaust flames (next) and the engine's tone on and off the throttle

## 2.63.1 - a natural blow-off

- Fixed: the blow-off valve's hiss was a short click-like burst. The charge volume before the throttle is now 6 L (piping and the intercooler, it was 2.5) and the valve smaller, so the air leaves over about half a second; the valve opens in proportion to how far the charge is over the atmosphere, so the flow tapers off smoothly instead of stopping short. The sound is a jet of broadband turbulence: it starts sharply, is bright while the pressure behind it is high, darkens and falls away as the charge empties, and flickers a little like a real jet
- Added: flutter. When the charge is above what the compressor delivers, the air runs back through it (surge) and chops the whistle and the air into pulses of about 14 Hz

## 2.63.0 - turbo, blow-off, anti-lag and the gearbox's voice (engine model, stage B3)

- Added: the turbo. A shaft with its own inertia is spun by the exhaust flow (the turbine's power from the flow and the pressure drop) and loaded by the compressor, whose outlet pressure follows its speed; its air fills a volume before the throttle; the wastegate holds the boost at about 0.55 bar. So the boost builds with a lag (about 0.75 s to full), the torque rises to about 330 N m at 4000 rpm, and the exhaust gets louder and hotter under boost
- Added: the sounds of the turbo: a whistle at the compressor's blade-pass pitch that rises with the shaft speed and a whoosh of the air, both from the engine bay
- Added: the blow-off valve: lifting off a boosted engine opens it (the plenum's vacuum) and the charge rushes out with a hiss; the boost falls
- Added: anti-lag: with the pedal up above 3000 rpm the spark is very late (and skipped now and then), the mixture rich and a little air comes past the throttle, so the fuel burns in the exhaust: bangs at the tail pipe, a hot exhaust (about 1300 K) and a turbo that keeps spinning (the speed on a lift is about 1.5 times that without it). Unburnt fuel and air in the hot exhaust burn by themselves, which is what makes every pop
- Added: a flat shift and a blip: a hard up-shift cuts the spark for a moment (the fuel goes unburnt into the exhaust and pops); a down-shift blips the throttle to match the revs
- Added: the gearbox's whine, a tone at 21 times the revs that grows with the torque
- Changed: the clutch grips harder (it holds the engine's torque with less slip), so the revs follow the wheels more closely
- The engine's cost grew to about 0.15 s of one core per second of sound at 7600 rpm

## 2.62.2 - less hiss while driving

- Fixed: while driving, something hissed like wind. Measured at 20-35 m/s, the rolling noise of the four tyres (a low-passed noise) was about as loud as the exhaust, and the intake's rush was an airy noise on top of the engine. The tyres' rolling noise is now 0.45 of what it was and the intake's rush half of what it was; the engine, the wind and the squeal are unchanged

## 2.62.1 - the engine is heard again, no more hiss

- Fixed: in 2.62.0 the engine drowned in a continuous hiss. The measurement of the model's sources showed two faults: the steady pressure of the pipes (the intake's depression on a closed throttle, the exhaust's back pressure) was passed on as a huge offset that saturated the output, and the intake's rush noise was driven by that pressure instead of by the air flowing through the throttle (at idle it was fifteen times louder than the exhaust). The steady part is now filtered out of the pipes, the rush follows the air flow, and a partial leveller keeps the idle audible and the roar from deafening (the loudness changes by the square root of the engine's real change). At idle the exhaust is now the main sound, the intake and the block are a tenth of it

## 2.62.0 - a simulated engine (engine model, block B)

- Added: the engine's sound now comes from a simulated engine instead of a stack of harmonics. A two-litre four (85 x 88 mm, 8.5:1, firing order 1-3-4-2) runs on the audio thread at the level of the gas: every cylinder is a volume that follows the crank, the valves open by crank angle and pass the charge and the burnt gas through orifices (isentropic, subsonic and choked), the fuel burns from the spark by a Wiebe curve and the pressure pushes the crank. The pressure at the exhaust collector goes through an exhaust pipe (a waveguide that rings at its quarter-wave resonances) and out of the tail pipe; the intake plenum goes through the intake pipe and the throttle's rush; the combustion shakes the block. It follows engine-sim (MIT) in its ideas, with our own code
- Added: it starts with the starter and idles at about 950 rpm by itself; the throttle, the clutch and the revs of the wheels drive it; at 7800 rpm the fuel is cut (the limiter); a closed throttle brakes the car's engine; the torque and power are those of a two-litre
- Added: an automatic gearbox for the sound: six gears and a reverse, it shifts up at the top of the revs (earlier with a light foot), down when the revs fall, every shift opens the clutch for 0.22 s (the engine falls and catches), the clutch slips when pulling away, and the engine runs free when the wheels are in the air
- Changed: the harmonic-stack engine of the earlier versions is gone. The cars' own forces are unchanged: the engine model makes the sound only for now

## 2.61.0 - the sound of the place (spatial audio, stage 3)

- Added: the environment reverb from rays. 64 rays are cast round the listener (a quarter of them renewed every frame, up to 150 m): they meet the ground, rocks, the crowns of the trees, houses; their mean free path, the share that escapes into the sky and what the things they hit absorb give the tail: a street between houses rings long and loud, an open field barely at all, a forest is short and dark (the crowns take the highs). A feedback delay network of eight lines plays it, its decay time follows the rays (Eyring's formula) and moves smoothly when you drive from the field into the town
- Added: echoes off the surroundings: a wall, a cliff or a house on the right, left, ahead or behind comes back after the round trip to it (distance x 2 / 343 m/s), panned to its side and as strong as the share of the rays that met it
- Changed: in the cockpit the tail is the small upholstered room of the cabin (short and dark), not the outside

## 2.60.0 - what stands in the way of a sound (spatial audio, stage 2)

- Added: the sound is blocked and bent: a ridge, a rock, a house or the car itself between a source and your ears takes the sound's highs first (a screen's diffraction, Maekawa's rule per band, with the longer path's delay); the crown of a tree dulls it a little; a wall muffles it
- Added: in the cockpit view the outside is heard through the body: the engine bay, the exhaust and the tyres are muffled (the exhaust the most), the wind too; the view from outside has no such muffling
- Added: the ground's echo: every source also reaches you after bouncing off the ground (an image source under it), by the surface: asphalt, rock and water return nearly all, gravel and dirt less, grass and sand dull it. The direct sound and the echo interfere, which is the hollow, comb-filtered sound of an engine heard low over a road

## 2.59.0 - stereo sound from real places (spatial audio, stage 1)

- Added: the sound is stereo and every source is heard from its real place: the exhaust at the tail, the engine bay at the nose, each of the four tyres where it touches the ground, the landings, crashes and splashes at the car. The listener is the camera (so in the cockpit you hear from the driver's seat, with the free camera from wherever you fly)
- Added: the sound takes time to arrive (distance / 343 m/s): a passing car bends its pitch (the Doppler effect comes out of the delay itself), the 1/r law makes it quieter with distance, the air dulls the highs, and the head gives the interaural time and level differences (the ear turned away hears it later and duller)
- Next stages: occlusion and diffraction by rays, the ground reflection, the reverb of the environment, then the engine model (see `docs/superpowers/specs/2026-10-16-spatial-audio-design.md`)

## 2.58.0 - PBR cabin surfaces

- Added: the cabin has its own surfaces by part name (new classes 22-24): soft fabric (the seats and the dash alcantara, fully matte, roughness 0.90-1.0), carbon and kevlar under a clear coat (0.22-0.40) with a glossy highlight, bare metal (the handbrake, the gear stick, the pedals; metallic 0.85, 0.30-0.55), and the hard plastic as before but with the texture's brightness varying its roughness (0.50-0.75)

## 2.57.0 - the Peugeot is the only car

- Changed: the second car (the generated off-roader) is cut from the game: the garage and the lists show only the Peugeot, it is free and selected on a new profile, and the game starts with it. A saved profile that had the off-roader selected switches to the Peugeot. The off-roader stays in `vehicles.json` only as the engine's stand-in (tests, the world before a profile picks a car); the buying logic stays for the cars to come

## 2.56.0 - PBR ground: roughness and rain puddles

- Added: the ground has its own roughness (grass, dirt, sand, rock, snow, asphalt, the painted lines) instead of one value for everything
- Added: wet ground. In the rain the ground darkens (the porous dirt and sand more), gets a thin film that shows the sky and the sun as a sheen, and the flat hollows fill with mirror-like puddles that the raindrops ring; the water collects first in the ruts of the track and the wheel tracks of the asphalt. It dries together with the surface wetness after the rain

## 2.55.1 - a double tap resets the camera

- Added: a double tap on free space (not on a control) puts the camera back to its default: the current view's default distance and height, the orbit turn dropped, and the Free camera returns behind the car looking at it. A drag or a long press is not a tap

## 2.55.0 - PBR surfaces per material

- Added: roughness and metal per material. The source model has no metallic-roughness maps, so `tools/carconv` tells carbon, painted alloy (the rims) and bare metal (screws, exhaust) from plastic and chrome by material name (new classes 19-21), and every class has its own roughness range in which the texture's brightness picks the spot (darker areas are dirtier and rougher). The rims are now painted alloy under a clear coat instead of bare chrome, the carbon is clear-coated, rubber and plastic are rougher than the paint, chrome is a polished mirror

## 2.54.2 - the wipers reach the pillar and change speed smoothly

- Fixed: the blades swung only about 57 degrees and stopped in the middle of the glass; they now sweep as far as the glass allows (the driver's blade reaches the pillar, over 70 degrees). Because the glass is curved, each arm's axis is tilted a little so the blade tip follows the glass instead of sinking into it
- Fixed: when the rain changed the wiper speed (or its intensity crossed a threshold) the blades jumped to another angle, and the start and the end of the work looked cut off; the speed change now keeps the blade where it is

## 2.54.1 - the controls work again

- Fixed: since 2.53.0 the controls did not react: the loading screen faded out but stayed on top, invisible, and ate the touches (its fade was restarted every 100 ms and never finished); it now fades once and is removed

## 2.54.0 - a pipeline cache: a fast start

- Added: the compiled GPU pipelines are kept in the app's cache folder (`pipelines.bin`, the Vulkan pipeline cache); the next start skips the shader compilation: on the test phone the start to the first frame went from 5.6 s to 1.1 s. A stale file (a driver or game update) is replaced by a fresh cache on its own

## 2.53.0 - a loading screen

- Added: a loading screen with a progress bar and the current step (graphics, textures, shaders, world, car) instead of a long black screen; it fades out when the first frame is drawn. Most of the wait (about 4 s of 5.6 s on the test phone) is the shader compilation of the car and the glass

## 2.52.1 - the cowl panel under the wipers

- Fixed: the black cowl panel between the bonnet and the windscreen (under the wipers) was missing, so the engine bay showed through; `tools/carconv` filtered it out by the word "Wipers" in its name (`x0_bonnet_Wipers_water`), it is part of the body again

## 2.52.0 - free camera and a handbrake that stays on

- Added: the **Free** camera view: one finger turns the gaze, a pinch flies forward and back along it, two fingers up or down raise and lower the camera; it starts where the camera was and stays put when the car drives off
- Changed: the handbrake button latches: a tap sets the handbrake, the next tap releases it (a dot on the button shows it is on; Reset, New level, the autopilot and leaving the game release it)
- Changed: the handbrake locks the rear wheels for good: they do not turn (also on screen), they have no drive, and a parked car is held on a slope by the full static grip of the locked tyres (a car sliding keeps the reduced grip of a slide)
- Changed: the brake lamps no longer come on for the handbrake, only for the brake pedal

## 2.51.2 - the wipers stay on the glass

- Fixed: a wiper's blade tip lifted off the windscreen and stuck out above the A-pillar when swung (the model's rig axes are tilted up to 18 degrees off the glass); the wipers now sweep about the glass's own normal at their spindles, and the sweep is limited to the angle at which the blades stay on the glass (1 rad at most)

## 2.51.1 - the wipers keep the model's own look

- Fixed: the moving wipers (the model's arm and blade) had been recoloured flat black; they now keep the model's own material and texture (its interior texture, lit by the sun like the paint)

## 2.51.0 - the model's own wipers, physically based raindrops

- Fixed: the wipers were the blade strips only, swinging about a point on the cowl while the arms stayed put; the moving wiper is now the model's arm and blade together, turning about the arm's own spindle, each on the axis of the model's own rig, so the pair sweeps in parallel over the glass
- Changed: the wipers work as car wipers do: a drizzle gets an intermittent wipe (1.6 s every 5 s), a steady rain the slow speed, a downpour the fast one; the blade swings as sin squared (slow at the turns) and parks smoothly
- Changed: raindrops on the glass are physical: each cell of the glass is hit by the rain at its own moment, so the drops appear gradually after the rain starts and grow by coalescing; after the rain they shrink by diffusion-limited evaporation (`r^2` falls linearly with time, faster in dry, moving or sunny air; the game runs it 60 times faster than life); small drops die first
- Added: running drops: a drop that grows past the size at which the pull of the air and of gravity beats the pinning of the water on the glass (Furmidge's criterion) runs; on the windscreen at speed the air pushes drops up the glass and across (they start moving from about 65 km/h), at rest the big ones run down; the runners sweep up the water in their path and grow, and leave the glass or are swept off by the wipers
- Changed: the drops are drawn as domes of water that mirror the sky and flash the sun, at their real size (sub-pixel drops fade by their area); the wipers clear them where the blade passes and the water returns from that moment

## 2.50.2 - the wipers sweep the right way

- Fixed: the wipers swung down under the bonnet instead of up the windscreen (the axis pointed the wrong way along the glass's normal); they now lift up the glass
- Fixed: after a wipe the drops vanished on every window at once; now only the cells a blade has passed over are cleared, and everywhere else (the side windows, the corners of the windscreen) each drop grows in and runs off on its own clock

## 2.50.1 - the wipers sweep over the glass

- Fixed: the wipers swung about the wrong axis (found from the blade's own shape, which lay along the glass), so they poked out of the windscreen into the air; the axis is now the windscreen's normal, and the blade is checked to lie in the plane it sweeps
- Fixed: raindrops on the side windows were stretched into slanted streaks (the pattern was flattened onto two axes); they are now round drops from two 3D lattices, many small and a few big, on every pane

## 2.50.0 - the autopilot in the wet, wipers that wipe

- Changed: the autopilot slows down on wet ground in proportion to the grip that is left (the square root of the wet grip factor, never below half), so the braking distance stays what it was; a soaked hairpin that ran off the road before is now taken cleanly
- Fixed: the raindrops on the glass vanished all at once; now each cell of the windscreen is cleared when the wiper blade actually passes over it (up and back) and the drops grow in again afterwards; the side windows, which the wipers do not reach, still clear once per cycle; the drops are smaller and less ring-like
- Fixed: when the rain stopped the wipers jumped to their park position; they now finish the wipe they are in and park

## 2.49.0 - the car gets wet

- Added: in the rain the Peugeot's paint darkens a little, its clear coat turns smoother and more reflective (a water film), and small water beads on the body glint (near the camera only)

## 2.48.0 - physically based foliage

- Changed: leaves and needles have their own lighting: the diffuse term wraps round the terminator (light scatters inside a crown), a leaf lit from behind glows in its own saturated colour (transmission), a faint waxy sheen that sharpens and darkens when the leaves are wet
- Changed: bark and rock are rough (roughness 0.92) and become darker and glossier in the rain

## 2.47.0 - physically based water and sky

- Changed: the sky's clouds are lit as a participating medium: optical depth grows with density (Beer-Lambert), the sun's light is attenuated by the density towards the sun and scattered forward (Henyey-Greenstein), so cloud edges glow silver and dense cores and bases are dark; crisper edges, smaller billows, a thin cirrus veil above; the sun's glow is a Mie forward-scattering lobe
- Changed: water uses water's real reflectance (F0 0.02, Fresnel), a roughness that follows the waves and the rain, the sky's blurred reflection, a GGX sun glint, finer waves that fade out with distance, and the headlights glitter on it
- Added: rain makes rings spread over the water
- Added: debug file `lookup` (with `gpuprof 1`) raises the camera target to look at the sky

## 2.46.0 - clear coat

- Added: the Peugeot's paint is lit as a base under a clear coat: a thin glossy layer (dielectric, roughness 0.07) on the smooth surface that reflects the sun as a sharp glint and the sky crisply, stronger at grazing angles, over the paint's softer satin reflection; the livery's normal map stays under the coat

## 2.45.0 - shadow quality option

- Added: Settings > Shadows: High (soft shadows with a blocker search, as before, the default), Medium (six taps turned per pixel, a fixed soft edge), Low (one filtered tap), Off (no shadow-map shadows; hills still shade the ground). Measured on a POCO F3 in daylight: High costs about 10 ms more in the main pass than the other three, which are close to each other (about 22 ms); Medium looks nearly like High, a little grainier

## 2.44.1 - profiling tools

- Added: the GPU profile (debug extra `gpuprof 1`) now times the shadow and the main pass on their own, and a file `skipmask` in the app's files directory leaves draw groups out while profiling (1 terrain, 2 models, 4 instances, 8 roads, 16 water, 32 glass and rain) to see what each costs
- Changed: soft shadows beyond 45 m use about half the taps, and surfaces facing away from the sun skip the shadow lookup (no visible difference)
- Measured (day, 3rd person, POCO F3): the main pass takes about 25 ms, of which the terrain about 11 ms, roads 4 ms, models 3 ms; the soft-shadow filter is about a third of the main pass; at night the headlight lighting adds 5 to 8 ms. The phone throttles when hot (the same scene ranged from 25 to 30 ms)

## 2.44.0 - micro-relief

- Added: normal maps for the Peugeot (paint details, tyres, rims, plastics from the model's own maps, in a half-size atlas next to the livery); they fade out with distance
- Added: micro-relief on the ground: the terrain's texture is read as a 3 cm height, the road gets aggregate (asphalt) and loose gravel with ruts (dirt), all fading out with distance, so low sun and headlights rake across the surface

## 2.43.0 - the dash comes alive

- Added: the Peugeot's instrument cluster and dials are backlit at night (they glow white against the dark cabin)
- Added: the four RPM shift LEDs light one by one, green, green, amber, red, as the revs rise with speed
- Changed: `rally_car.rcar` carries up to 15 meshes (the dash)

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
