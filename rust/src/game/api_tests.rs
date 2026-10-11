use super::*;
fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rt_api_{}_{}", std::process::id(), name)); let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap(); d.join("profile.json")
}
#[test] fn menu_info_has_the_documented_layout_and_persists_the_seed() {
    let path = dir("a");
    let a = menu_info(&path, 0x1234, false);
    assert_eq!(a.len(), 36);
    assert_eq!((a[0], a[1], a[2], a[3]), (0.0, 0.0, 4660.0, 0.0));
    assert_eq!((a[4], a[12], a[20], a[28]), (0.0, 1.0, 0.0, 1.0));
    let b = menu_info(&path, 0x9999, false);
    assert_eq!(b, a);
}
#[test] fn refresh_changes_the_list_and_is_stored() {
    let path = dir("b");
    let a = menu_info(&path, 5, false);
    let b = menu_info(&path, 5, true);
    assert_eq!(b[1], 1.0);
    assert_ne!(&a[4..], &b[4..]);
    assert_eq!(menu_info(&path, 5, false), b);
}
