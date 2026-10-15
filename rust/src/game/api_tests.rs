use super::*;
fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rt_api_{}_{}", std::process::id(), name)); let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap(); d.join("profile.json")
}
#[test] fn garage_info_has_the_documented_layout_and_creates_the_profile() {
    let path = dir("a");
    let a = garage_info(&path);
    let cars = VehicleSpec::all().len();
    assert_eq!(a.len(), 3 + CAR_FIELDS * cars);
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

#[test] fn garage_info_carries_the_aero_fields_and_the_numbers_react_to_the_parts() {
    let d = std::env::temp_dir().join(format!("rt_api_aero_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap();
    let path = d.join("profile.json");
    let info = garage_info(&path);
    let n = info[2] as usize;
    assert_eq!(info.len(), 3 + n * CAR_FIELDS);
    let car = |info: &[f32], i: usize| info[3 + i * CAR_FIELDS..3 + (i + 1) * CAR_FIELDS].to_vec();
    let before = car(&info, 0);
    assert_eq!((before[7], before[8], before[9], before[10]), (0.0, 0.0, 0.0, 1.0), "no parts, the wing angle in the middle");
    assert!(before[13] > 80.0 && before[13] < 200.0, "top speed {}", before[13]);
    for up in [4, 5, 6] { for _ in 0..3 { assert!(garage_buy_upgrade(&path, 0, up)); } }
    assert!(garage_set_wing_angle(&path, 0, 2));
    let after = car(&garage_info(&path), 0);
    assert_eq!((after[7], after[8], after[9], after[10]), (3.0, 3.0, 3.0, 2.0));
    assert!(after[11] > before[11] + 80.0, "downforce {} -> {}", before[11], after[11]);
    assert!(after[12] > 30.0 && after[13] < before[13], "drag {} %, top {} -> {}", after[12], before[13], after[13]);
    assert!(!garage_set_wing_angle(&path, 0, 5));
}
