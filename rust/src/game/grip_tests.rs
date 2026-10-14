use super::*;
use crate::game::{math::v3, roads::{Road, RoadKind, RoadNet, RoadPt}, surface::Surface, terrain::Terrain};

fn stage(len: f32) -> RoadNet {
    let n = (len / 4.0) as usize;
    RoadNet::new(vec![Road { kind: RoadKind::Stage, pts: (0..=n).map(|i| RoadPt { pos: v3(0.0, 0.0, i as f32 * 4.0), width: 8.0, s: i as f32 * 4.0 }).collect() }])
}

#[test] fn the_table_orders_the_surfaces_and_stays_in_bounds() {
    let g = |s| grip(s, 1.0).mu;
    assert!(g(Surface::Asphalt) > g(Surface::Rock) && g(Surface::Rock) > g(Surface::Dirt) && g(Surface::Dirt) > g(Surface::Grass) && g(Surface::Grass) > g(Surface::Sand));
    for s in [Surface::Grass, Surface::Dirt, Surface::Sand, Surface::Rock, Surface::Gravel, Surface::Asphalt, Surface::Water] {
        for l in [0.0, 0.5, 1.0, 2.0, -1.0, f32::NAN] { let k = grip(s, l); assert!((0.3..=1.0).contains(&k.mu) && (0.3..=1.0).contains(&k.lateral) && (0.9..=3.5).contains(&k.rolling), "{s:?} {l}"); }
    }
    assert!(grip(Surface::Gravel, 0.0).mu > grip(Surface::Gravel, 1.0).mu, "packed gravel grips better than loose");
    assert!((grip(Surface::Gravel, 0.0).mu - 0.78).abs() < 1e-5 && (grip(Surface::Gravel, 1.0).mu - 0.62).abs() < 1e-5);
}
#[test] fn the_map_starts_loose_and_wear_packs_the_line_not_the_sides() {
    let net = stage(400.0);
    let mut m = LooseMap::new(&net);
    assert_eq!(m.loose_at(0, 100.0, 0.0), 1.0);
    for _ in 0..60 { m.wear(0, 100.0, 0.0, 20.0, 1.0 / 240.0); }   // a quarter of a second at 20 m/s on one spot
    assert!(m.loose_at(0, 100.0, 0.0) < 0.5, "{}", m.loose_at(0, 100.0, 0.0));
    assert_eq!(m.loose_at(0, 100.0, 1.4), 1.0, "1.4 m to the side is untouched");
    assert_eq!(m.loose_at(0, 140.0, 0.0), 1.0, "40 m along is untouched");
    for _ in 0..2000 { m.wear(0, 100.0, 0.0, 30.0, 1.0 / 60.0); }
    assert!(m.loose_at(0, 100.0, 0.0) >= 0.0);
}
#[test] fn the_map_survives_odd_queries() {
    let net = stage(100.0);
    let mut m = LooseMap::new(&net);
    for (s, side) in [(-5.0, 0.0), (1e9, 0.0), (50.0, 99.0), (50.0, -99.0), (f32::NAN, 0.0), (50.0, f32::NAN)] {
        assert_eq!(m.loose_at(0, s, side), 1.0, "({s}, {side})");
        m.wear(0, s, side, 20.0, 0.01);
    }
    m.wear(0, 50.0, 0.0, f32::NAN, 0.01); m.wear(0, 50.0, 0.0, f32::INFINITY, 0.01);
    assert!((0.0..=1.0).contains(&m.loose_at(0, 50.0, 0.0)));
    assert_eq!(m.loose_at(7, 50.0, 0.0), 1.0, "an unknown road");
    assert_eq!(LooseMap::empty().loose_at(0, 1.0, 0.0), 1.0);
}
#[test] fn a_liaison_road_has_no_loose_layer() {
    let net = RoadNet::new(vec![Road { kind: RoadKind::Liaison, pts: (0..=25).map(|i| RoadPt { pos: v3(0.0, 0.0, i as f32 * 4.0), width: 8.0, s: i as f32 * 4.0 }).collect() }]);
    let mut m = LooseMap::new(&net);
    m.wear(0, 50.0, 0.0, 20.0, 1.0);
    assert_eq!(m.loose_at(0, 50.0, 0.0), 1.0);
}
#[test] fn ground_at_reports_surface_looseness_and_the_gravel_road_cell() {
    let t = Terrain::flat().with_net(stage(400.0));
    let mut m = LooseMap::new(t.roads().unwrap());
    let (s, loose, road) = ground_at(&t, &m, 1.0, 100.0);
    assert_eq!((s, loose), (Surface::Gravel, 1.0));
    let (ri, along, side) = road.expect("on the gravel stage");
    assert!(ri == 0 && (along - 100.0).abs() < 1.0 && (side - (-1.0)).abs() < 1e-3 || (side - 1.0).abs() < 1e-3, "side {side}");
    m.wear(0, 100.0, side, 20.0, 0.5);
    assert!(ground_at(&t, &m, 1.0, 100.0).1 < 1.0);
    let (s2, l2, r2) = ground_at(&t, &m, 300.0, 100.0);
    assert!(s2 == Surface::Asphalt && l2 == 1.0 && r2.is_none(), "the flat test terrain far from the road is neutral asphalt");
}

#[test] fn the_embedded_file_holds_the_documented_numbers() {
    let t = GripTable::from_json(EMBEDDED).expect("parses");
    let g = |s, l| t.grip(s, l);
    assert_eq!((g(Surface::Asphalt, 1.0).mu, g(Surface::Asphalt, 1.0).lateral, g(Surface::Asphalt, 1.0).rolling), (1.0, 1.0, 1.0));
    assert_eq!((g(Surface::Gravel, 0.0).mu, g(Surface::Gravel, 1.0).mu, g(Surface::Gravel, 1.0).lateral, g(Surface::Gravel, 1.0).rolling), (0.78, 0.62, 0.75, 1.3));
    assert_eq!((g(Surface::Dirt, 1.0).mu, g(Surface::Grass, 1.0).mu, g(Surface::Sand, 1.0).mu, g(Surface::Rock, 1.0).mu, g(Surface::Water, 1.0).mu), (0.70, 0.60, 0.50, 0.80, 0.55));
    assert_eq!(g(Surface::Sand, 1.0).rolling, 3.0);
    assert_eq!(t.wear_per_metre, 0.5);
    assert_eq!(table(), t, "the active table is the embedded one unless an override was loaded");
}
#[test] fn editing_the_json_changes_the_table_and_bad_files_say_what_is_wrong() {
    let edited = EMBEDDED.replace("\"mu\": 0.60", "\"mu\": 0.45");
    assert_ne!(edited, EMBEDDED);
    assert_eq!(GripTable::from_json(&edited).unwrap().grip(Surface::Grass, 1.0).mu, 0.45);
    let e = GripTable::from_json("not json").unwrap_err(); assert!(e.contains("JSON"), "{e}");
    let e = GripTable::from_json("{}").unwrap_err(); assert!(e.contains("asphalt") || e.contains("gravel") || e.contains("grass"), "{e}");
    let e = GripTable::from_json(&EMBEDDED.replace("\"mu\": 0.50", "\"mu\": 9.0")).unwrap_err(); assert!(e.contains("sand") && e.contains("mu"), "{e}");
    let e = GripTable::from_json(&EMBEDDED.replace("\"wear_per_metre\": 0.5", "\"wear_per_metre\": \"fast\"")).unwrap_err(); assert!(e.contains("wear_per_metre"), "{e}");
    let e = GripTable::from_json(&EMBEDDED.replace("\"mu_loose\": 0.62,", "")).unwrap_err(); assert!(e.contains("mu_loose"), "{e}");
}
#[test] fn an_override_file_is_loaded_but_a_missing_or_broken_one_changes_nothing() {
    let dir = std::env::temp_dir().join(format!("rt_surf_{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
    let before = table();
    load_override(&dir.join("nope.json"));
    std::fs::write(dir.join("bad.json"), "{ nonsense").unwrap(); load_override(&dir.join("bad.json"));
    assert_eq!(table(), before);
    std::fs::write(dir.join("same.json"), EMBEDDED).unwrap(); load_override(&dir.join("same.json"));   // the same numbers: other tests stay valid
    assert_eq!(table(), before);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test] fn the_dust_numbers_are_parsed_and_checked() {
    let t = GripTable::from_json(EMBEDDED).unwrap();
    let g = t.dust(Surface::Gravel); let a = t.dust(Surface::Asphalt); let s = t.dust(Surface::Sand); let gr = t.dust(Surface::Grass);
    assert!(g.roll > 0.3 && a.roll == 0.0 && a.slide > 0.0 && a.slide < g.slide, "tarmac only smokes when sliding");
    assert!(s.roll > g.roll && s.size > g.size && s.life > g.life && gr.roll < 0.15, "sand is the dustiest, grass barely");
    for k in [Surface::Grass, Surface::Dirt, Surface::Sand, Surface::Rock, Surface::Gravel, Surface::Asphalt, Surface::Water] {
        let d = t.dust(k);
        assert!(d.color.iter().all(|c| (0.0..=1.0).contains(c)) && (0.0..=1.0).contains(&d.roll) && (0.0..=2.0).contains(&d.slide) && (0.2..=3.0).contains(&d.size) && (0.3..=6.0).contains(&d.life), "{k:?}");
    }
    assert_eq!((t.dust_cap_normal, t.dust_cap_lots), (400, 800));
    assert!((t.wind_speed - 2.5).abs() < 1e-6 && (t.wind_gust - 0.35).abs() < 1e-6);
    let e = GripTable::from_json(&EMBEDDED.replace("\"life\": 2.6", "\"life\": 60.0")).unwrap_err(); assert!(e.contains("gravel") && e.contains("life"), "{e}");
    let e = GripTable::from_json(&EMBEDDED.replace("\"cap_normal\": 400,", "")).unwrap_err(); assert!(e.contains("cap_normal"), "{e}");
}
