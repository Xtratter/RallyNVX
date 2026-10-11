# Procedural sound (v2.8) — design

Status: approved in chat (2026-10-12). First of three polish releases: **v2.8 Sound**, v2.9 Settings + performance, v2.10 Render mode A (hybrid soft shadows/AO). Render mode B stays out of the plan.

## 1. Goal
Engine, rotor, rolling, water, wind and mission sounds, all synthesised by code (no files). Mono, 44.1 kHz, 16-bit. No music, no 3D positioning, no desktop audio, no volume slider (mute toggle only; the slider comes with the v2.9 settings).

## 2. Architecture
- `rust/src/audio/` (pure Rust, no new dependencies): `dsp.rs` (xorshift noise, one-pole filters, smoothing), `synth.rs` (`Synth`: `set_params(&AudioParams)`, `push(Event)`, `render(&mut [i16])`, `set_muted(bool)`), `director.rs` (`Director::update(&World, throttle) -> (AudioParams, Vec<Event>)`).
- `AudioParams`: machine (None/Car/Heli/Drone), `rpm` 0..1, `load` 0..1, `roll` 0..1 (rolling noise by speed and ground contact), `surface` (Ground/Water), `slip` 0..1, `wind` 0..1, `active` (false while paused or in menus: fades out).
- `Event`: `Touchdown(strength)`, `Crash`, `Splash(strength)`, `Checkpoint`, `Board`, `Success`, `Fail`.
- Glue: a global `Mutex<Synth>`; `Game::frame` runs the director after the steps and pushes params and events (while paused it sends `active = false`); JNI `Native.audioRender(ShortArray)` fills a buffer, `Native.audioMute(Boolean)`. Kotlin `AudioEngine`: `AudioTrack` (MODE_STREAM, PCM16 mono 44.1 kHz) on a daemon thread that loops `audioRender` and writes; started in `GameActivity` surface creation, stopped on surface destroy and `onPause`, restarted on resume; audio focus requested (loss = pause the track); a mute button in the pause panel (EN+RU) persisted in `SharedPreferences`.

## 3. Sounds
- Car: engine = detuned saw/square stack with 6 harmonics, fundamental `25 + 110*rpm` Hz, amplitude from load; rolling = low-passed noise (ground 1.5 kHz, water 0.6 kHz with a slow gurgle modulation) scaled by `roll`; tyre squeal = band-passed noise 900-1400 Hz scaled by slip.
- Helicopter: rotor thump (blade passes 13-17 Hz as noise bursts plus a 55 Hz tone) and whoosh; drone: four detuned tones 300-420 Hz plus faint noise; both scale with `rpm`/`load`. Landed aircraft idle at lower rpm.
- Wind: low-passed noise scaled by `wind` (altitude and speed).
- Events (one-shot voices, at most 8 at once): touchdown thump, crash noise burst, splash burst, checkpoint chime (two tones), boarding tick, success arpeggio (C-E-G), fail (two falling notes).
- Master: smoothing of all parameters (about 80 ms), soft clip, 50 ms fade for mute/inactive.

## 4. Director rules
Car: `rpm` from speed/`max_speed` and throttle; `load` from throttle and slope; `roll` = speed fraction when at least one wheel touches the ground; `surface` Water when a wheel is below the lake; `slip` from the lateral velocity. Aircraft: `rpm` 0.45 landed, else 0.6 plus speed and climb terms; `wind` grows with height and speed. Events by comparing with the previous frame: touchdown (car airborne to grounded with `vy < -3`, or aircraft Flying to Landed; strength by speed), crash (aircraft Crashed, or a horizontal speed loss over 6 m/s in a frame), splash (a wheel enters water at speed over 2), checkpoint (`next_index` increases while running), board (rescue people aboard increases), success/fail (the outcome appears, once).

## 5. Testing
Host tests: synth output finite and within range under random parameter sweeps; a Goertzel check that the engine fundamental follows rpm; muted/inactive output fades to silence; event voices appear then decay and the pool is capped; same inputs, same buffer; director: params for each machine, each event once (no repeats while a condition persists), paused/free-drive behaviour. Device: install, start a mission, check no crash, no underrun spam in logcat, process CPU; sound itself is judged by the user. One final review on the most capable model.
