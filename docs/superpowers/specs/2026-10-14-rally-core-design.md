# Rally mode core (R1, v2.22.0) - design

First of three cycles of the rally mode (R2 pace notes with voice, R3 championship). Design approved in chat on 2026-10-14 (user choices: championship later, pace notes with text and voice later, penalties in time for resets and leaving the road).

## Goal

A "Rally" entry beside "Free drive": pick a rally (a fixed seed, so records mean something), drive its two stages (the special stage, then the city super-special, 2 laps) against the clock with a countdown, check lines, penalties, results and personal records. Free drive stays untouched.

## Not in this cycle

Pace notes (R2), the championship, points, ghosts and opponents (R3), car damage, a service park, the liaison drive between stages (skipped: the car is put on the next start).

## Design

- `game/lines.rs`: `Line { pos, dir, width, kind: Start | Check | Finish }` and the crossing test. `superspecial.rs` lines become `Line`s; `roads::stage_lines(stage)` makes the stage's lines: start at s = 25 m, a check line every 1000 m, the finish 12 m before the end. `Terrain` holds the stage lines next to the city loop; `roadgen` draws them all (start and finish checkered, checks yellow), width = the road's width + 2 m.
- `game/rally.rs` (pure, host-tested): `Run { phase: Countdown(t) | Running | Finished, time, penalty, expected lines, next, splits, off_t, assisted, events }`. `Run::update(pos, dt, reset, off_edge)`: the countdown is 3.5 s ("3, 2, 1, GO"); the clock runs from GO; a line is crossed when the car's step crosses its plane inside its width; crossing a later line than the next one skips the lines in between: +30 s each; a reset is +10 s; staying more than 1 m outside the road edge for 3 s is +5 s (once, armed again after the car is back on the road; not on the city stage); the finish ends the run. Events (split, penalty, missed, finish) are queued for the HUD.
- `World.rally: Option<RallyState>`: `rally_start(stage)` puts the car 17 m before the stage's start line facing along the road (the city loop: 12 m before its line), builds the expected lines (the stage: its lines; the city: 2 laps of its check lines, the start line as the lap line, the finish on the second pass of the start line), and starts the countdown. During the countdown the car is held (brake and handbrake) and the autopilot is ignored; the Reset button during a run puts the car back on the road (the stage: the nearest road point; the city: the nearest loop point) with +10 s; a car upside down for 3 s is put back the same way (+10 s). `rally_stop()` ends the mode. The autopilot may be used; the run is then marked `assisted` and not recorded.
- State slots `[30..41]` (see `fill_state`): phase (0 none, 1 countdown, 2 running, 3 finished), time, penalty, the countdown digit, stage, next line, lines total, the last event code and its serial, its value, assisted, the final total.
- Kotlin: `RallyMenuActivity` (a list of 4 rallies "Rally 1..4" with fixed seeds and the records), the Rally button on the start screen, in `GameActivity` a timer, a big countdown, an event toast, a results panel (time, penalties, total, record, Next stage / Retry / Exit); records in `SharedPreferences` (`rec_<rally>_<stage>` and `rec_<rally>_total`). `RallyFormat.time(seconds)` formats `m:ss.t` (unit test).
- Debug: the intent extra `rally` N (1-based) with `rallystage` 0/1 starts a stage directly.

## Testing

Pure (`rally_tests.rs`): the countdown length and GO; the clock; crossing in the right order, from the wrong side (no crossing), outside the width (no); a skipped line costs 30 s each; reset +10 s; the off-road rule (3 s, once, re-arming) and none on the city stage; the finish; events in order; `assisted` blocks the record flag. World (host): a run with the autopilot on the special stage of seed 21 finishes with the expected number of splits and no penalty; on the city loop two laps finish on the second start-line pass; the car is held during the countdown; the Reset button puts the car on the road with +10 s. Kotlin unit: `RallyFormat`. Device: the countdown, the HUD, the results panel, fps.

## Risks

Check lines are drawn only on the stage road and the city streets, a run outside them would never be timed (a stage's lines are tied to the road, so the run cannot be cheated by cutting across the field except by the +30 s skip rule).
