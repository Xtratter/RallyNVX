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
#[test] fn unknown_kind_id_5_is_rejected() {
    assert!(Profile::from_json("{\"version\":1,\"money\":1,\"profile_seed\":1,\"round\":0,\"results\":[{\"seed\":1,\"kind\":5,\"medal\":1,\"best_time\":1.0}]}").is_err());
}
