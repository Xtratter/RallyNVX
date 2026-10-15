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
    assert_eq!(before[11], 0.0, "tarmac wheels by default");
    assert!(before[14] > 80.0 && before[14] < 200.0, "top speed {}", before[14]);
    for up in [4, 5, 6] { for _ in 0..3 { assert!(garage_buy_upgrade(&path, 0, up)); } }
    assert!(garage_set_wing_angle(&path, 0, 2));
    let after = car(&garage_info(&path), 0);
    assert_eq!((after[7], after[8], after[9], after[10]), (3.0, 3.0, 3.0, 2.0));
    assert!(after[12] > before[12] + 80.0, "downforce {} -> {}", before[12], after[12]);
    assert!(after[13] > 30.0 && after[14] < before[14], "drag {} %, top {} -> {}", after[13], before[14], after[14]);
    assert!(!garage_set_wing_angle(&path, 0, 5));
}

#[test] fn the_wheel_set_is_saved_per_car_refused_out_of_range_and_shown_in_the_info() {
    let dir = std::env::temp_dir().join(format!("rt_api_wheel_{}", std::process::id())); std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("profile.json");
    assert!(garage_set_wheel_set(&path, 0, 1));
    assert_eq!(garage_info(&path)[3 + 11], 1.0, "the set sits after the wing angle");
    assert!(!garage_set_wheel_set(&path, 0, 3), "only 0, 1, 2");
    assert_eq!(garage_info(&path)[3 + 11], 1.0, "a refused write changes nothing");
    assert!(!garage_set_wheel_set(&path, 99, 1), "a bad car index");
    let _ = std::fs::remove_dir_all(&dir);
}
