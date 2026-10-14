# Pace notes (R2, v2.23.0) - design

Second cycle of the rally mode (after the core, before the championship). Approved in chat on 2026-10-14: text on the screen and voice through the system speech synthesizer, in Russian and English.

## Goal

In a rally the co-driver tells what comes next ("Left 3 tightens, 120. Crest. Right 5. Junction left. Finish in 100") as text and as speech, early enough to react.

## Design

- `game/pacenotes.rs` (pure): `generate(path, junctions) -> Vec<Note>`; `Note { s, kind }` with `Corner { left, sev, tightens, opens, long, into: Option<(left, sev)> }`, `Hairpin`-like severity 0, `Crest { big }`, `Junction { left }`, `Finish`. Corners come from the radius over the path (three points 12 m apart): a corner starts below 200 m of radius and ends above 220 m or when the direction flips; kept when it turns by 20 degrees or more; severity by the smallest radius (hairpin < 18 m, 1: 18-30, 2: 30-45, 3: 45-65, 4: 65-95, 5: 95-140, faster ones are not called); tightens/opens by the radius of the last third against the first third (0.75 / 1.35); long above 80 m of arc; a corner followed by another within 40 m is announced as "into" and the second one is not announced alone. Crests come from the height profile (second difference over 20 m below -0.0035; "big" below -0.008, called a jump), junctions from the side roads (left or right of the travel direction), the finish 140 m before the end. `Note::text(dist_to_next, ru)` gives the sentence; the distance to the next note is added when over 60 m, rounded to 10 m (under 100), 50 m (under 300) or 100 m.
- Playback (`World.rally.notes`): the path of a stage (the special stage's road points; the city: the loop twice) and its notes; the car's progress is the nearest path point in a window around the last one; a note is announced when its start is within `max(80 m, 5 s of the current speed)` ahead, at most one per 1.5 s, once each, in order. The announced note's text (Russian and English) goes to a shared slot; state slots `[42]` serial of the last note and `[43]` the metres to the next one.
- JNI `gameNoteText(ru)` returns the last note's text; Kotlin shows it in a bar (top left under the status island) and speaks it with `TextToSpeech` (locale of the device: ru or en; text only when no engine is available).
- Setting "Pace notes": Off / Text / Text and voice (default); in the settings screen; only in rallies.

## Testing

Pure: a left circle of 40 m radius is "Left 2" and the mirror is "Right 2"; radius 12 is a hairpin; a straight has only the finish; an S-bend gives "into"; a tightening spiral says "tightens", an opening one "opens"; a long arc is "long"; a crest bump is a crest and a tall one a jump; junctions are announced with the right side; for seeds 1-8 the special stage has 25-140 notes with increasing positions, non-empty strings in both languages and the finish last; the city loop gets notes too. Playback: a car driving the path at 20 m/s hears every note once, in order, between 80 m and 140 m before it; at 30 m/s earlier; the spacing is at least 1.5 s. Kotlin: nothing pure; the setting row and the bar are checked on the device (the voice by ear).

## Risks

A text read at speed: short phrases; `TextToSpeech` may be slow to initialise (the notes are text-only until ready); speech language packs may be missing (text only).
