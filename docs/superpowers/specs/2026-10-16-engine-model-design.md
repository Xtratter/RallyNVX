# Engine model for the sound (block B of the spatial audio) (design)

Goal: the engine's sound comes from a simulated engine, not from a harmonic stack: the pulses of the exhaust and the intake follow from the gas in the cylinders, the valves, the combustion and the crank, so the throttle, the revs, the rev limiter, the gear shifts and the starting all sound the way they would. A port of the core ideas of [engine-sim](https://github.com/ange-yaghi/engine-sim) (MIT) to Rust, in its own terms and with its own numbers (no code copied, no scripts, no GUI).

## What is ported
- **Gas volumes**: moles, thermal energy, a fuel/O2/inert mixture; the pressure from the energy; the adiabatic change of volume; heat to the walls.
- **Orifice flow** between volumes (isentropic, subsonic and choked, limited so that it never overshoots the pressure equilibrium); enthalpy and composition travel with the gas.
- **Cylinder**: the slider-crank volume, valve lifts by crank angle, a Wiebe burn from the spark, the torque on the crank by virtual work.
- **Engine**: the crank's inertia, friction, a starter, a clutch to the driveline, an idle governor, a fuel cut at the limiter.
- **Pipes**: the intake plenum and the exhaust runners and collector are gas volumes; the exhaust and the intake pipes are waveguides (a delay line with an open end that reflects, so the pipe rings at its own resonances).

## What is not
Engine-sim's rigid-body constraint solver (the crank is one degree of freedom), its script language and its recorded impulse responses (the environment's impulse comes from the rays of block A).

## Stages
- **B1** the engine core with host tests (idle, starting, the torque curve, the firing frequency).
- **B2** the sound: the pulses through the waveguides into the exhaust and intake sources of the spatial audio, the gearbox of the Director, the clutch, the starter, the limiter; the old harmonic stack goes.
- **B3** turbo and boost, the blow-off valve and anti-lag pops, mechanical noise, tuning of the levels.

## Budget
About 90 000 fluid steps per second of audio (44.1 kHz x 2); a step is a handful of orifice flows per cylinder. It runs on the audio thread; a host test measures the time per second of sound.
