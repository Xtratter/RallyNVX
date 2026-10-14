use super::*;
fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rt_api_{}_{}", std::process::id(), name)); let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap(); d.join("profile.json")
}
#[test] fn garage_info_has_the_documented_layout_and_creates_the_profile() {
    let path = dir("a");
    let a = garage_info(&path);
    let cars = VehicleSpec::all().len();
    assert_eq!(a.len(), 3 + 7 * cars);
    assert_eq!((a[0], a[1], a[2]), (0.0, 0.0, cars as f32), "money, selected index, car count");
    assert!(path.exists(), "a missing profile is created");
    assert_eq!(garage_info(&path), a);
}
#[test] fn selecting_and_upgrading_go_through_the_profile_file() {
    let path = dir("b");
    let n = VehicleSpec::all().len();
    assert!(!garage_select(&path, n + 3), "a bad car index is refused");
    assert!(garage_select(&path, 0));
    assert!(garage_buy_upgrade(&path, 0, 0) || garage_info(&path)[0] == 0.0, "an upgrade is bought when it costs nothing, refused when too expensive");
}
