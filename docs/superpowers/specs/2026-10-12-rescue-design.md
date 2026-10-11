# Rescue missions (v2.7) — design

Status: approved in chat (2026-10-12). Builds on v2.6 (flight, sling missions, generated models, lakes).

## 1. Goal
Add a helicopter **rescue** mission: fly to 1-3 survivor sites, land, wait while the people board, fly to the base pad, land, wait while they unload. The user chose "the helicopter picks people up and takes them to the base" for the first version.
Out of scope: urban/rooftop rescue, ground evacuation, fire or obstacles, winch, dialogue, drone rescue.

## 2. Rules
- Kind `Rescue`, air mission, aircraft always the helicopter. `Mission.checkpoints` = survivor sites then the base pad (last).
- Sites: difficulty 1/2/3 gives 1/2/3 sites. Each has 1-3 people, at most 4 in total (capacity 4). Site 1 is 200-500 m from the start, each next site 150-400 m from the previous, the base 300-700 m from the last site; all points on dry, gentle ground with `normal.y >= 0.97` outside cities (`point_ok` plus the stricter slope), at least 60 m apart.
- Boarding: the helicopter is `Status::Landed` with its position within 6 m (horizontal) of the current site for 3 s (progress 0..1); the dwell resets when it takes off or leaves the circle. Unloading at the base: the same with 2 s. People board only the current target, sites are done in order.
- Integrity (people's comfort, 100): a touchdown vertical speed above 1.2 m/s costs `(v - 1.2) * 12` per landing (the helicopter already crashes at 3 m/s, so the review moved the threshold down); at 0 the mission fails (`Failure::CargoDestroyed`, shown as "passengers hurt"). Crash and time up fail as usual.
- Time limit: `1.5 * length / 14 + 25 + 12 * sites + 8` seconds. Medal: `grade_cargo(time, limit, integrity)`. Reward base: `reward_base_for(length, difficulty) + 50 * people`.
- List: the sixth slot rotates by refresh round: `round % 3` = 0 Sling (urban), 1 Recon, 2 Rescue. Slot 5 stays one card; list size stays six, the menu array unchanged (kind id 5).

## 3. Presentation
- Markers: a Pad ring (radius 6 m) at each remaining site and at the base, a Beacon on the current target.
- Survivors: a generated person model (orange vest, 1.7 m) stands on each pending site in a small circle (1-3 figures); figures vanish when boarded. New model slot in the model pipeline.
- HUD: timer "sites done/total" and people aboard ("people N"), boarding progress bar (state array `[23]` people aboard, `[24]` dwell progress 0..1), integrity bar as for cargo. Card/briefing/result texts in EN+RU for kind 5.

## 4. Testing
Host tests: point generation (determinism, dry, gentle, not in city, spacing, counts per difficulty, people <= 4), the state machine scripted with synthetic aircraft states (boarding, dwell reset, order, unload, hard touchdown, crash, time), `roll` rotation and list shape, profile round trip with kind 5, state array values, markers and survivors. Device: edit `files/profile.json` round to 2 to get the rescue card, check the cards, flight, landing and boarding, fps >= 60. One final review on the most capable model.
