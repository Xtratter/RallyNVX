# Car controls and settings (v2.9) — design

Status: approved in chat (2026-10-14). Car only; the helicopter and drone keep their sticks and are postponed. Performance work and render mode A move to later releases.

## 1. Goal
The two sticks are a poor fit for driving. The car gets an on-screen steering wheel and gas/brake pedals (or tilt steering), plus a settings screen (graphics quality, steering mode and sensitivity, volume, pedal side, tilt calibration) and one-time hints. No sliders (user's design preference): cycling buttons and steps only.

## 2. Controls (ground vehicle only)
- Layout (landscape): wheel bottom-left (about 190 dp), gas pedal (tall) bottom-right, brake pedal to its left, handbrake button above the pedals, "pedals on the left" setting swaps the sides (wheel then goes right).
- Wheel: dragging around its centre turns it (accumulated angle, limit +-120 degrees), release returns it to the centre at 400 deg/s. `steer = shape(angle / 120)`, with a light response curve and sensitivity Low/Normal/High (gain 0.7 / 1.0 / 1.4), clamped to -1..1.
- Pedals: analog by touch height: `pressure = 0.15 + 0.85 * (y / height)` while touched, 0 otherwise. Gas -> `throttle = pressure`. Brake pedal: while moving forward faster than 3 km/h it brakes (`brake = true`, `brake_amt = pressure`); at or below 3 km/h it reverses (`throttle = -pressure`, no brake).
- Tilt mode (setting): roll of the phone about its long axis (landscape) steers; the pose at game start is zero, a "Calibrate" button in settings and a double tap on the indicator re-zero; dead zone 3 degrees, full lock at 30 / gain degrees, low-pass about 0.08 s. The wheel is replaced by a small steering indicator.
- Rust: `Input.brake_amt` (0 = legacy full brake, otherwise 0.05..1 scales the brake force); JNI `gameInput(throttle, steer, brake, handbrake, brakeAmt)`.
- Air missions keep `SticksLayer`.

## 3. Settings screen (`SettingsActivity`, M3 style)
Rows (label + a button that cycles the value): Graphics quality (Low/Medium/High, applies at the next game start), Steering (Wheel/Tilt), Sensitivity (Low/Normal/High), Volume (0/25/50/75/100 %), Pedals (Right/Left), Calibrate tilt (button, only meaningful in tilt mode), Reset hints. Opened from the main menu (replaces the quality button) and from the pause panel (the game applies the changes on return). Stored in the `prefs` SharedPreferences.
- Volume: `audio::set_volume(0..1)` master gain in the synth (applied before the soft clip), JNI `audioVolume(float)`, set from the setting at start and when it changes.
- Hints: a short banner once per control mode at the first game start: wheel "Drag the wheel to steer, pedals on the right: gas and brake / reverse" (RU too), tilt "Tilt the phone to steer; the pedals are on the right"; it fades after 6 s; "Reset hints" shows them again.

## 4. Testing
Kotlin unit tests (JVM, no Android): wheel angle from touch points, accumulation across the +-180 seam, limit, return to centre, steering curve and sensitivities; pedal pressure; brake/reverse switch at 3 km/h; tilt dead zone, scaling, calibration, smoothing; volume steps; settings cycle logic. Rust tests: analog brake force scales with `brake_amt` and legacy `brake_amt = 0` is unchanged; volume gain (silence at 0, half amplitude at 0.5, clamping, NaN). Device: install, open settings, change values, screenshots of the new layout in both pedal sides; the feel of the controls is judged by the user. One final review on the most capable model.
