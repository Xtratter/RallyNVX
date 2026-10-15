use super::*;
fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rt_meta_{}_{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap(); d.join("profile.json")
}
#[test] fn json_roundtrip_keeps_money_and_the_garage() {
    let mut p = Profile::new();
    p.money = 1234;
    p.garage.cars.push(("vulpes".to_string(), [1, 2, 3, 0, 0, 0, 0, 1]));
    let q = Profile::from_json(&p.to_json()).unwrap();
    assert_eq!((q.money, q.version), (1234, PROFILE_VERSION));
    assert_eq!(q.levels("vulpes"), [1, 2, 3, 0, 0, 0, 0, 1]);
}
#[test] fn save_is_atomic_and_load_reads_it_back() {
    let path = tmp("save");
    let mut p = Profile::new(); p.money = 99; p.save(&path).unwrap();
    assert!(!path.with_extension("json.tmp").exists());
    assert_eq!(Profile::load_or_new(&path).money, 99);
}
#[test] fn missing_file_creates_a_fresh_profile() {
    let path = tmp("missing");
    let p = Profile::load_or_new(&path);
    assert_eq!(p.money, 0);
    assert!(path.exists());
}
#[test] fn corrupt_empty_truncated_and_newer_files_are_kept_as_bak() {
    for (name, content) in [("corrupt", "not json at all"), ("empty", ""), ("trunc", "{\"version\":1,\"money\":5,"), ("newer", "{\"version\":999,\"money\":5,\"profile_seed\":1,\"round\":0,\"results\":[]}"), ("wrongtype", "{\"version\":1,\"money\":\"lots\"}")] {
        let path = tmp(name);
        std::fs::write(&path, content).unwrap();
        let p = Profile::load_or_new(&path);
        assert_eq!(p.money, 0, "{name}");
        let bak = path.with_extension("json.bak");
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), content, "{name}: original must be kept");
    }
}
#[test] fn negative_and_huge_numbers_do_not_panic() {
    assert!(Profile::from_json("{\"version\":1,\"money\":-5,\"profile_seed\":1,\"round\":0,\"results\":[]}").is_err());
    assert!(Profile::from_json("{\"version\":1,\"money\":1e30,\"profile_seed\":1,\"round\":0,\"results\":[]}").is_err());
}
#[test] fn old_profiles_with_mission_results_still_load_and_the_results_are_ignored() {
    let p = Profile::from_json("{\"version\":1,\"money\":10,\"profile_seed\":5,\"round\":2,\"results\":[{\"seed\":1,\"kind\":0,\"medal\":2,\"best_time\":30.0},{\"seed\":2,\"kind\":5,\"medal\":3,\"best_time\":20.0}]}").unwrap();
    assert_eq!(p.money, 10);
}
mod garage_profile {
    use super::*;
    use crate::game::garage::Upgrade;
    fn rich() -> Profile { let mut p = Profile::new(); p.money = 20_000; p }
    #[test] fn a_new_profile_owns_the_offroader_only_and_has_it_selected() {
        let p = Profile::new();
        assert!(p.owns("offroader") && !p.owns("vulpes") && p.garage.selected == "offroader");
        assert_eq!(p.levels("offroader"), default_levels()); assert_eq!(p.levels("vulpes"), default_levels());
        assert_eq!(p.selected_spec().price, 0);
    }
    #[test] fn buying_a_car_needs_money_and_charges_exactly_once() {
        let mut p = Profile::new(); p.money = 2999;
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
        assert!(p.buy_upgrade("offroader", Upgrade::Engine) && p.money == 19_700 && p.levels("offroader") == [1, 0, 0, 0, 0, 0, 0, 1]);
        assert!(p.buy_upgrade("offroader", Upgrade::Engine) && p.buy_upgrade("offroader", Upgrade::Engine));
        assert_eq!((p.money, p.levels("offroader")), (20_000 - 300 - 600 - 1200, [3, 0, 0, 0, 0, 0, 0, 1]));
        let m = p.money; assert!(!p.buy_upgrade("offroader", Upgrade::Engine) && p.money == m, "capped at level 3");
        let mut poor = Profile::new(); poor.money = 299;
        assert!(!poor.buy_upgrade("offroader", Upgrade::Tyres) && poor.money == 299 && poor.levels("offroader") == default_levels());
        poor.money = 300; assert!(poor.buy_upgrade("offroader", Upgrade::Tyres) && poor.money == 0);
        assert!(p.selected_spec().engine_force > Profile::new().selected_spec().engine_force);
    }
    #[test] fn the_garage_survives_a_json_round_trip() {
        let mut p = rich(); p.buy_car("vulpes"); p.select("vulpes"); p.buy_upgrade("vulpes", Upgrade::Brakes); p.buy_upgrade("offroader", Upgrade::Suspension);
        let q = Profile::from_json(&p.to_json()).unwrap();
        assert_eq!((q.garage.selected.as_str(), q.levels("vulpes"), q.levels("offroader"), q.money), ("vulpes", [0, 0, 0, 1, 0, 0, 0, 1], [0, 0, 1, 0, 0, 0, 0, 1], p.money));
        assert!(q.owns("vulpes"));
    }
    #[test] fn old_files_unknown_ids_and_unowned_selection_are_handled() {
        let old = Profile::from_json("{\"version\":1,\"money\":10,\"profile_seed\":5,\"round\":0,\"results\":[]}").unwrap();
        assert!(old.owns("offroader") && !old.owns("vulpes") && old.garage.selected == "offroader");
        let odd = Profile::from_json("{\"version\":1,\"money\":10,\"profile_seed\":5,\"round\":0,\"results\":[],\"garage\":{\"selected\":\"vulpes\",\"cars\":{\"offroader\":{\"engine\":9,\"tyres\":1},\"ghost\":{\"engine\":2}}}}").unwrap();
        assert_eq!(odd.garage.selected, "offroader", "not owned: falls back");
        assert_eq!(odd.levels("offroader"), [3, 1, 0, 0, 0, 0, 0, 1], "levels clamp to the cap, missing ones are 0");
        assert!(!odd.owns("ghost"));
        let mut text: serde_json::Value = serde_json::from_str(&odd.to_json()).unwrap();
        assert!(text["garage"]["cars"].get("ghost").is_none()); text["version"] = serde_json::json!(1);
    }
    #[test] fn api_helpers_act_on_a_profile_file_by_car_index() {
        let dir = std::env::temp_dir().join(format!("rt_garage_{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("p.json");
        let mut p = Profile::new(); p.money = 4000; p.save(&path).unwrap();
        let info = crate::game::api::garage_info(&path);
        assert_eq!(&info[..3], &[4000.0, 0.0, 2.0]);
        let f = crate::game::api::CAR_FIELDS;
        assert_eq!(&info[3..10], &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], "off-roader: owned, free, upgrades free");
        assert_eq!(&info[3 + f..3 + f + 3], &[0.0, 0.0, 0.0], "the Vulpes is not owned yet but costs nothing while the garage is free");
        assert!(crate::game::api::garage_buy_car(&path, 1) && crate::game::api::garage_select(&path, 1));
        assert!(crate::game::api::garage_buy_upgrade(&path, 1, 0), "upgrades cost nothing for now");
        let info = crate::game::api::garage_info(&path);
        assert_eq!((info[0], info[1], info[3 + f], info[3 + f + 3]), (4000.0, 1.0, 1.0, 1.0), "the balance is untouched");
        assert!(!crate::game::api::garage_buy_car(&path, 9) && !crate::game::api::garage_select(&path, 9) && !crate::game::api::garage_buy_upgrade(&path, 9, 0) && !crate::game::api::garage_buy_upgrade(&path, 1, 9));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

mod aero_parts {
    use super::*;
    use crate::game::garage::default_levels;

    #[test] fn aero_parts_are_bought_like_upgrades_and_the_wing_angle_is_free() {
        let mut p = Profile::new();
        p.money = 20_000;
        assert_eq!(p.levels("offroader")[7], 1, "the wing angle starts in the middle");
        assert!(p.buy_upgrade("offroader", Upgrade::Wing) && p.buy_upgrade("offroader", Upgrade::Splitter) && p.buy_upgrade("offroader", Upgrade::Underbody));
        assert_eq!(p.money, 20_000 - 3 * 300);
        assert_eq!(&p.levels("offroader")[4..7], &[1, 1, 1]);
        for _ in 0..2 { assert!(p.buy_upgrade("offroader", Upgrade::Wing)); }
        assert!(!p.buy_upgrade("offroader", Upgrade::Wing), "the cap is 3");
        let before = p.money;
        assert!(p.set_wing_angle("offroader", 2) && p.money == before && p.levels("offroader")[7] == 2);
        assert!(!p.set_wing_angle("offroader", 3) && !p.set_wing_angle("vulpes", 0), "bad angle or an unowned car");
        // the selected car carries the kit into its spec
        let spec = p.selected_spec();
        assert_eq!((spec.kit.wing, spec.kit.splitter, spec.kit.underbody, spec.kit.wing_angle), (3, 1, 1, 2));
    }
    #[test] fn profiles_round_trip_the_new_levels_and_an_old_profile_loads_with_defaults() {
        let mut p = Profile::new();
        p.garage.cars[0].1 = [1, 2, 3, 0, 1, 2, 3, 0];
        let q = Profile::from_json(&p.to_json()).unwrap();
        assert_eq!(q.levels("offroader"), [1, 2, 3, 0, 1, 2, 3, 0]);
        let old = r#"{"version":1,"money":5,"garage":{"selected":"offroader","cars":{"offroader":{"engine":2,"tyres":1,"suspension":0,"brakes":3}}}}"#;
        let o = Profile::from_json(old).unwrap();
        assert_eq!(o.levels("offroader"), [2, 1, 0, 3, 0, 0, 0, 1], "the new parts are 0 and the wing angle is in the middle");
        let odd = Profile::from_json(r#"{"version":1,"money":0,"garage":{"cars":{"offroader":{"wing":9,"wing_angle":7,"splitter":-1}}}}"#).unwrap();
        assert_eq!(odd.levels("offroader"), [0, 0, 0, 0, 0, 3, 0, 2], "clamped");
        assert_eq!(default_levels()[7], 1);
    }
}
