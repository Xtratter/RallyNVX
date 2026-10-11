use super::*;
use crate::game::{input::Input, math::v3, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};
const DT: f32 = 1.0 / 60.0;
fn drive(secs: f32, throttle: f32) -> (Vehicle, Tracks) {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0); let mut tr = Tracks::new();
    for _ in 0..(secs / DT) as usize { v.step(&t, &[], &Input { throttle, ..Default::default() }, DT); tr.update(&v, &t, DT); }
    (v, tr)
}
#[test] fn driving_leaves_segments_about_every_35_cm_per_wheel() {
    let (v, tr) = drive(2.5, 1.0);
    let dist = v.pos.z; assert!(dist > 10.0 && dist < 40.0, "{dist}");
    let expect = (dist / SPACING) as usize * 4;
    assert!(tr.len() > expect * 6 / 10 && tr.len() <= MAX_TRACKS, "{} segments for {dist} m (expected about {expect})", tr.len());
    for s in tr.iter() { assert!((s.dir.len() - 1.0).abs() < 1e-3 && (s.normal.len() - 1.0).abs() < 1e-3 && s.pos.y > 0.0 && s.pos.y < 0.2); }
}
#[test] fn nothing_at_rest_or_in_the_air() {
    let (_, tr) = drive(3.0, 0.0); assert_eq!(tr.len(), 0);
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0); let mut tr = Tracks::new();
    v.vel = v3(0., 0., 20.0);
    for _ in 0..30 { v.comp = [0.0; 4]; tr.update(&v, &t, DT); }
    assert_eq!(tr.len(), 0, "wheels in the air leave no tracks");
}
#[test] fn the_ring_buffer_caps_at_256_and_keeps_the_newest() {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 0.9, 0.), 0.0); let mut tr = Tracks::new();
    v.vel = v3(0., 0., 20.0); v.comp = [0.1; 4];
    for i in 0..600 { v.pos.z = i as f32 * 0.4; tr.update(&v, &t, DT); }
    assert_eq!(tr.len(), MAX_TRACKS);
    let newest = tr.iter().map(|s| s.pos.z).fold(f32::MIN, f32::max);
    assert!(newest > 200.0, "the newest segment survives: {newest}");
}
#[test] fn rock_slopes_get_no_tracks() {
    use crate::game::materials::rock_dominant;
    assert!(rock_dominant(3.0, v3(0.7, 0.55, 0.0).norm(), 0.0, 0.0));
    assert!(!rock_dominant(3.0, v3(0.0, 1.0, 0.0), 0.0, 0.0));
}
#[test] fn track_rotation_lies_on_the_slope_and_follows_the_heading() {
    let n = v3(0.3, 0.9, 0.1).norm(); let d = v3(0.0, 0.0, 1.0);
    let q = track_rot(d, n);
    assert!((q.rotate(v3(0., 1., 0.)) - n).len() < 1e-3);
    let f = q.rotate(v3(0., 0., 1.));
    assert!(f.dot(n).abs() < 1e-3 && f.dot(d) > 0.9, "{f:?}");
}
#[test] fn no_tracks_are_left_in_water() {
    let t = Terrain::flooded(0.5); let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0); let mut tr = Tracks::new();
    v.vel = v3(0., 0., 12.0);
    for _ in 0..120 { v.comp = [0.1; 4]; v.pos.z += 0.2; tr.update(&v, &t, DT); }
    assert_eq!(tr.len(), 0);
}
