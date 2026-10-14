use super::*;
use crate::game::{math::v3, missions::{Mission, MissionKind}};
fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rt_meta_{}_{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap(); d.join("profile.json")
}
fn m(seed: u64, kind: MissionKind) -> Mission {
    Mission { kind, seed, difficulty: 1, start: v3(0., 0., 0.), goal: v3(0., 0., 100.), checkpoints: vec![v3(0., 0., 100.)], length_m: 100.0, time_limit: 100.0, reward_base: 100.0, aircraft: None, urban: false, start_yaw: None, normals: vec![] }
}
#[test] fn json_roundtrip_keeps_everything() {
    let mut p = Profile::new(1234);
    p.record(&m(5, MissionKind::Cargo), Medal::Silver, 42.5);
    p.round = 3;
    let q = Profile::from_json(&p.to_json()).unwrap();
    assert_eq!((q.money, q.round, q.profile_seed, q.version), (p.money, 3, 1234, PROFILE_VERSION));
    assert_eq!(q.best_medal(5, MissionKind::Cargo), Medal::Silver);
    assert_eq!(q.results[0].best_time, 42.5);
}
#[test] fn record_pays_by_improvement_and_keeps_best_time() {
    let mut p = Profile::new(1);
    assert_eq!(p.record(&m(9, MissionKind::Race), Medal::Bronze, 80.0), 100);
    assert_eq!(p.record(&m(9, MissionKind::Race), Medal::Gold, 50.0), 120);
    assert_eq!(p.record(&m(9, MissionKind::Race), Medal::Silver, 70.0), 75);
    assert_eq!(p.money, 295);
    assert_eq!((p.best_medal(9, MissionKind::Race), p.results[0].best_time), (Medal::Gold, 50.0));
    assert_eq!(p.best_medal(9, MissionKind::Cargo), Medal::None);
    assert_eq!(p.gold_count(), 1);
}
#[test] fn save_is_atomic_and_load_reads_it_back() {
    let path = tmp("save");
    let mut p = Profile::new(7); p.money = 99; p.save(&path).unwrap();
    assert!(!path.with_extension("json.tmp").exists());
    assert_eq!(Profile::load_or_new(&path, 1).money, 99);
}
#[test] fn missing_file_creates_a_fresh_profile_with_the_given_seed() {
    let p = Profile::load_or_new(&tmp("missing"), 0xABCDEF);
    assert_eq!((p.money, p.profile_seed), (0, 0xABCDEF));
}
#[test] fn corrupt_empty_truncated_and_newer_files_are_kept_as_bak() {
    for (name, content) in [("corrupt", "not json at all"), ("empty", ""), ("trunc", "{\"version\":1,\"money\":5,"), ("newer", "{\"version\":999,\"money\":5,\"profile_seed\":1,\"round\":0,\"results\":[]}"), ("wrongtype", "{\"version\":1,\"money\":\"lots\"}")] {
        let path = tmp(name);
        std::fs::write(&path, content).unwrap();
        let p = Profile::load_or_new(&path, 321);
        assert_eq!((p.money, p.profile_seed), (0, 321), "{name}");
        let bak = path.with_extension("json.bak");
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), content, "{name}: original must be kept");
    }
}
#[test] fn negative_and_huge_numbers_do_not_panic() {
    assert!(Profile::from_json("{\"version\":1,\"money\":-5,\"profile_seed\":1,\"round\":0,\"results\":[]}").is_err());
    assert!(Profile::from_json("{\"version\":1,\"money\":1e30,\"profile_seed\":1,\"round\":0,\"results\":[]}").is_err());
}
#[test] fn old_profiles_with_kind_ids_0_and_1_still_load() {
    let p = Profile::from_json("{\"version\":1,\"money\":10,\"profile_seed\":5,\"round\":2,\"results\":[{\"seed\":1,\"kind\":0,\"medal\":2,\"best_time\":30.0},{\"seed\":2,\"kind\":1,\"medal\":3,\"best_time\":20.0}]}").unwrap();
    assert_eq!((p.best_medal(1, MissionKind::Cargo), p.best_medal(2, MissionKind::Race)), (Medal::Silver, Medal::Gold));
}
#[test] fn air_kinds_round_trip() {
    let mut p = Profile::new(3);
    p.record(&m(7, MissionKind::Rings), Medal::Gold, 40.0);
    p.record(&m(8, MissionKind::Recon), Medal::Bronze, 90.0);
    p.record(&m(9, MissionKind::Sling), Medal::Silver, 70.0);
    let q = Profile::from_json(&p.to_json()).unwrap();
    assert_eq!((q.best_medal(7, MissionKind::Rings), q.best_medal(8, MissionKind::Recon), q.best_medal(9, MissionKind::Sling)), (Medal::Gold, Medal::Bronze, Medal::Silver));
    assert_eq!(q.best_medal(7, MissionKind::Race), Medal::None);
}
#[test] fn unknown_kind_id_6_is_rejected() {
    assert!(Profile::from_json("{\"version\":1,\"money\":1,\"profile_seed\":1,\"round\":0,\"results\":[{\"seed\":1,\"kind\":6,\"medal\":1,\"best_time\":1.0}]}").is_err());
}
#[test] fn rescue_kind_round_trips() {
    let mut p = Profile::new(3);
    p.record(&m(11, MissionKind::Rescue), Medal::Gold, 77.0);
    let q = Profile::from_json(&p.to_json()).unwrap();
    assert_eq!(q.best_medal(11, MissionKind::Rescue), Medal::Gold);
}

mod garage_profile {
    use super::*;
    use crate::game::garage::Upgrade;
    fn rich() -> Profile { let mut p = Profile::new(3); p.money = 20_000; p }
    #[test] fn a_new_profile_owns_the_offroader_only_and_has_it_selected() {
        let p = Profile::new(1);
        assert!(p.owns("offroader") && !p.owns("vulpes") && p.garage.selected == "offroader");
        assert_eq!(p.levels("offroader"), [0; 4]); assert_eq!(p.levels("vulpes"), [0; 4]);
        assert_eq!(p.selected_spec().price, 0);
    }
    #[test] fn buying_a_car_needs_money_and_charges_exactly_once() {
        let mut p = Profile::new(1); p.money = 2999;
        assert!(!p.buy_car("vulpes") && p.money == 2999 && !p.owns("vulpes"));
        p.money = 3500;
        assert!(p.buy_car("vulpes") && p.money == 500 && p.owns("vulpes"));
        assert!(!p.buy_car("vulpes") && p.money == 500, "buying again changes nothing");
        assert!(!p.buy_car("nope") && !p.buy_car("offroader"));
    }
    #[test] fn selecting_needs_ownership_and_changes_the_spec_used() {
        let mut p = rich();
        assert!(!p.select("vulpes") && p.garage.selected == "offroader");
        assert!(p.buy_car("vulpes") && p.select("vulpes"));
        assert_eq!(p.selected_spec().style, crate::game::vehicle::CarStyle::Rally);
        assert!(!p.select("nope"));
    }
    #[test] fn upgrades_are_atomic_capped_and_need_ownership() {
        let mut p = rich();
        assert!(!p.buy_upgrade("vulpes", Upgrade::Engine), "not owned");
        assert!(p.buy_upgrade("offroader", Upgrade::Engine) && p.money == 19_700 && p.levels("offroader") == [1, 0, 0, 0]);
        assert!(p.buy_upgrade("offroader", Upgrade::Engine) && p.buy_upgrade("offroader", Upgrade::Engine));
        assert_eq!((p.money, p.levels("offroader")), (20_000 - 300 - 600 - 1200, [3, 0, 0, 0]));
        let m = p.money; assert!(!p.buy_upgrade("offroader", Upgrade::Engine) && p.money == m, "capped at level 3");
        let mut poor = Profile::new(1); poor.money = 299;
        assert!(!poor.buy_upgrade("offroader", Upgrade::Tyres) && poor.money == 299 && poor.levels("offroader") == [0; 4]);
        poor.money = 300; assert!(poor.buy_upgrade("offroader", Upgrade::Tyres) && poor.money == 0);
        assert!(p.selected_spec().engine_force > Profile::new(1).selected_spec().engine_force);
    }
    #[test] fn the_garage_survives_a_json_round_trip() {
        let mut p = rich(); p.buy_car("vulpes"); p.select("vulpes"); p.buy_upgrade("vulpes", Upgrade::Brakes); p.buy_upgrade("offroader", Upgrade::Suspension);
        let q = Profile::from_json(&p.to_json()).unwrap();
        assert_eq!((q.garage.selected.as_str(), q.levels("vulpes"), q.levels("offroader"), q.money), ("vulpes", [0, 0, 0, 1], [0, 0, 1, 0], p.money));
        assert!(q.owns("vulpes"));
    }
    #[test] fn old_files_unknown_ids_and_unowned_selection_are_handled() {
        let old = Profile::from_json("{\"version\":1,\"money\":10,\"profile_seed\":5,\"round\":0,\"results\":[]}").unwrap();
        assert!(old.owns("offroader") && !old.owns("vulpes") && old.garage.selected == "offroader");
        let odd = Profile::from_json("{\"version\":1,\"money\":10,\"profile_seed\":5,\"round\":0,\"results\":[],\"garage\":{\"selected\":\"vulpes\",\"cars\":{\"offroader\":{\"engine\":9,\"tyres\":1},\"ghost\":{\"engine\":2}}}}").unwrap();
        assert_eq!(odd.garage.selected, "offroader", "not owned: falls back");
        assert_eq!(odd.levels("offroader"), [3, 1, 0, 0], "levels clamp to the cap, missing ones are 0");
        assert!(!odd.owns("ghost"));
        let mut text: serde_json::Value = serde_json::from_str(&odd.to_json()).unwrap();
        assert!(text["garage"]["cars"].get("ghost").is_none()); text["version"] = serde_json::json!(1);
    }
    #[test] fn api_helpers_act_on_a_profile_file_by_car_index() {
        let dir = std::env::temp_dir().join(format!("rt_garage_{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("p.json");
        let mut p = Profile::new(4); p.money = 4000; p.save(&path).unwrap();
        let info = crate::game::api::garage_info(&path, 1);
        assert_eq!(&info[..3], &[4000.0, 0.0, 2.0]);
        assert_eq!(&info[3..10], &[1.0, 0.0, 300.0, 0.0, 0.0, 0.0, 0.0]);         // off-roader: owned, price 0, upgrade 300
        assert_eq!(&info[10..13], &[0.0, 3000.0, 750.0]);                         // peugeot: not owned
        assert!(crate::game::api::garage_buy_car(&path, 1) && crate::game::api::garage_select(&path, 1));
        assert!(crate::game::api::garage_buy_upgrade(&path, 1, 0), "engine level 1 costs 750 and money is 1000");
        let info = crate::game::api::garage_info(&path, 1);
        assert_eq!((info[0], info[1], info[10], info[13]), (250.0, 1.0, 1.0, 1.0));
        assert!(!crate::game::api::garage_buy_car(&path, 9) && !crate::game::api::garage_select(&path, 9) && !crate::game::api::garage_buy_upgrade(&path, 9, 0) && !crate::game::api::garage_buy_upgrade(&path, 1, 9));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test] fn profiles_saved_with_the_old_car_id_keep_their_garage() {
    let old = Profile::from_json("{\"version\":1,\"money\":10,\"profile_seed\":5,\"round\":0,\"results\":[],\"garage\":{\"selected\":\"peugeot206\",\"cars\":{\"offroader\":{},\"peugeot206\":{\"engine\":2}}}}").unwrap();
    assert!(old.owns("vulpes") && old.garage.selected == "vulpes" && old.levels("vulpes") == [2, 0, 0, 0]);
}
