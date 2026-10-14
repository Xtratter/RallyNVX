use super::*;
use crate::game::math::v3;

/// Points every 4 m along a polyline of (length, turn in degrees: positive = to the car's left) pieces, heading +z.
fn path(pieces: &[(f32, f32)]) -> Vec<V3> {
    let (mut x, mut z, mut h) = (0.0f32, 0.0f32, 0.0f32);
    let mut pts = vec![v3(x, 0.0, z)];
    for &(len, turn) in pieces {
        let n = (len / 4.0).ceil().max(1.0) as usize; let dh = turn.to_radians() / n as f32;
        for _ in 0..n { h += dh; x += 4.0 * h.sin() * if true { 1.0 } else { -1.0 }; z += 4.0 * h.cos(); pts.push(v3(x, 0.0, z)); }
    }
    pts
}
fn corners(notes: &[Note]) -> Vec<Corner> { notes.iter().filter_map(|n| if let NoteKind::Corner(c) = n.kind { Some(c) } else { None }).collect() }

#[test] fn a_left_circle_of_40_m_is_left_2_and_the_mirror_right_2() {
    // heading +z, turning towards +x is the car's left; a radius of 40 m is an arc of length 40 * angle
    let left = generate(&path(&[(150.0, 0.0), (40.0 * 1.2, 68.75), (150.0, 0.0)]), &[]);
    let c = corners(&left);
    assert_eq!(c.len(), 1, "{left:?}");
    assert!(c[0].left && c[0].sev == 2, "{:?}", c[0]);
    let right = generate(&path(&[(150.0, 0.0), (40.0 * 1.2, -68.75), (150.0, 0.0)]), &[]);
    let c = corners(&right);
    assert!(!c[0].left && c[0].sev == 2, "{:?}", c[0]);
    assert_eq!(left[0].text(Some(120.0), false), "Left 2, 100");
    assert_eq!(left[0].text(Some(120.0), true), "Лево 2, 100");
}
#[test] fn a_tight_radius_is_a_hairpin_and_a_straight_has_only_the_finish() {
    let h = generate(&path(&[(100.0, 0.0), (12.0 * 3.14, 180.0), (100.0, 0.0)]), &[]);
    let c = corners(&h);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].sev, 0, "{c:?}");
    assert!(h[0].text(None, false).starts_with("Hairpin"), "{}", h[0].text(None, false));
    let s = generate(&path(&[(600.0, 0.0)]), &[]);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].kind, NoteKind::Finish);
}
#[test] fn an_s_bend_runs_into_the_second_corner_and_far_corners_stay_separate() {
    let s = generate(&path(&[(100.0, 0.0), (50.0, 60.0), (20.0, 0.0), (50.0, -60.0), (100.0, 0.0)]), &[]);
    let c = corners(&s);
    assert_eq!(c.len(), 1, "{s:?}");
    assert!(c[0].left && matches!(c[0].into, Some((false, _))), "{:?}", c[0]);
    assert!(s[0].text(None, false).contains("into right"), "{}", s[0].text(None, false));
    let far = generate(&path(&[(100.0, 0.0), (50.0, 60.0), (200.0, 0.0), (50.0, -60.0), (100.0, 0.0)]), &[]);
    assert_eq!(corners(&far).len(), 2);
}
#[test] fn tightening_and_opening_long_corners_are_called() {
    // a spiral: the radius shrinks along the arc (a turn rate that grows)
    let mut pieces = vec![(100.0, 0.0)];
    for k in 0..8 { pieces.push((10.0, 4.0 + 3.0 * k as f32)); }
    pieces.push((100.0, 0.0));
    let t = corners(&generate(&path(&pieces), &[]));
    assert!(t.iter().any(|c| c.tightens), "{t:?}");
    let mut pieces = vec![(100.0, 0.0)];
    for k in 0..8 { pieces.push((10.0, 28.0 - 3.0 * k as f32)); }
    pieces.push((100.0, 0.0));
    let o = corners(&generate(&path(&pieces), &[]));
    assert!(o.iter().any(|c| c.opens), "{o:?}");
    let l = corners(&generate(&path(&[(100.0, 0.0), (130.0, 90.0), (100.0, 0.0)]), &[]));
    assert!(l[0].long, "{l:?}");
}
#[test] fn crests_junctions_and_the_finish() {
    let mut p = path(&[(600.0, 0.0)]);
    for q in p.iter_mut() { let d = q.z - 300.0; if d.abs() < 20.0 { q.y = 1.6 * 0.5 * (1.0 + (std::f32::consts::PI * d / 20.0).cos()); } }
    let notes = generate(&p, &[(150.0, true), (450.0, false)]);
    let kinds: Vec<NoteKind> = notes.iter().map(|n| n.kind).collect();
    assert!(matches!(kinds[0], NoteKind::Junction { left: true }), "{kinds:?}");
    assert!(matches!(kinds[1], NoteKind::Crest { .. }), "{kinds:?}");
    assert!(matches!(kinds[2], NoteKind::Junction { left: false }) && matches!(kinds[3], NoteKind::Finish), "{kinds:?}");
    assert!((notes[1].s - 300.0).abs() < 12.0);
    assert_eq!(notes[0].text(None, true), "Перекрёсток слева");
    assert_eq!(notes[3].text(None, false), "Finish in 100");
    assert!(notes.windows(2).all(|w| w[0].s <= w[1].s));
}
#[test] fn the_generated_stages_get_a_sensible_set_of_notes_in_both_languages() {
    use crate::game::{roads::{gen::{generate_with, GenCfg}, RoadKind}, terrain::Terrain};
    for seed in 1..=6u64 {
        let t = Terrain::new(seed);
        let n = generate_with(seed, &t, GenCfg::coarse());
        let stage = n.roads.iter().find(|r| r.kind == RoadKind::Stage).unwrap();
        let pts: Vec<V3> = stage.pts.iter().map(|p| p.pos).collect();
        let notes = generate(&pts, &[]);
        assert!(notes.len() >= 20 && notes.len() <= 200, "seed {seed}: {} notes", notes.len());
        assert!(notes.windows(2).all(|w| w[0].s <= w[1].s) && notes.last().unwrap().kind == NoteKind::Finish);
        assert!(notes.iter().all(|x| x.s >= 0.0 && x.s <= stage.length() + 1.0));
        for (i, x) in notes.iter().enumerate() { let d = notes.get(i + 1).map(|y| y.s - x.s); assert!(!x.text(d, true).is_empty() && !x.text(d, false).is_empty()); }
    }
}
#[test] fn playback_calls_every_note_once_in_order_between_80_and_140_m_ahead() {
    let p = path(&[(300.0, 0.0), (50.0, 60.0), (300.0, 0.0), (50.0, -60.0), (300.0, 0.0)]);
    let notes = generate(&p, &[]);
    assert!(notes.len() >= 3);
    for speed in [20.0f32, 30.0] {
        let mut pb = Playback::new(p.clone(), notes.clone());
        let mut called: Vec<(usize, f32)> = Vec::new();
        for (i, q) in p.iter().enumerate() {
            let mut t = 0.0;
            while t < 4.0 / speed { if let Some(k) = pb.update(*q, speed, 0.05) { called.push((k, notes[k].s - pb.progress())); } t += 0.05; }
            let _ = i;
        }
        assert_eq!(called.iter().map(|c| c.0).collect::<Vec<_>>(), (0..notes.len()).collect::<Vec<_>>(), "speed {speed}: every note once, in order");
        for (k, ahead) in &called { assert!(*ahead <= (5.0 * speed).max(80.0) + 8.0 && *ahead >= -1.0, "speed {speed}: note {k} called {ahead} m ahead"); }
    }
}

#[test] fn clips_match_the_text_and_exist_in_the_clip_list() {
    let list: serde_json::Value = serde_json::from_str(include_str!("../../../tools/voices/clips.json")).unwrap();
    let has = |lang: &str, id: &str| list[lang][id].is_string();
    let c = Corner { left: true, sev: 3, tightens: true, opens: false, long: true, into: Some((false, 0)), arc: 50.0 };
    let n = Note { s: 0.0, kind: NoteKind::Corner(c) };
    assert_eq!(n.clips(Some(150.0)), vec!["L3", "long", "tightens", "into", "HR", "d150"]);
    assert_eq!(n.clips(Some(40.0)), vec!["L3", "long", "tightens", "into", "HR"]);
    assert_eq!(Note { s: 0.0, kind: NoteKind::Finish }.clips(Some(500.0)), vec!["finish"]);
    // every clip any note can ask for is in the list, in both languages
    let mut kinds = vec![NoteKind::Crest { big: false }, NoteKind::Crest { big: true }, NoteKind::Junction { left: true }, NoteKind::Junction { left: false }, NoteKind::Finish];
    for left in [true, false] { for sev in 0..=5u8 { for (t, o, l) in [(false, false, false), (true, false, false), (false, true, false), (false, false, true)] {
        kinds.push(NoteKind::Corner(Corner { left, sev, tightens: t, opens: o, long: l, into: Some((!left, sev)), arc: 50.0 }));
    } } }
    for k in kinds { for d in [None, Some(30.0), Some(75.0), Some(120.0), Some(260.0), Some(450.0), Some(5000.0)] {
        for id in (Note { s: 0.0, kind: k }).clips(d) { assert!(has("ru", &id) && has("en", &id), "clip {id} is missing from clips.json"); }
    } }
}
