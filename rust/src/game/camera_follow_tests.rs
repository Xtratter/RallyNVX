use super::*;
use crate::game::{input::Input, vehicle::*};
#[test] fn camera_stays_above_terrain_and_behind() {
    let t = Terrain::new(6);
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., t.height(0., 0.) + 1.1, 0.), 0.0);
    let mut c = FollowCam::new(&v);
    for i in 0..1200 {
        v.step(&t, &[], &Input { throttle: 1.0, steer: if (i / 200) % 2 == 0 { 0.4 } else { -0.4 }, ..Default::default() }, 1.0 / 60.0);
        c.update(&v, &t, 1.0 / 60.0);
        assert!(c.pos.y >= t.height(c.pos.x, c.pos.z) + 0.89, "step {i}");
        assert!(c.pos.is_finite() && c.target.is_finite());
    }
    assert!((c.pos - v.pos).len() > 3.0 && (c.pos - v.pos).len() < 14.0);
}
#[test] fn follow_generalises_the_chase_camera() {
    let t = Terrain::flat();
    let mut c = FollowCam::new_for(v3(0., 30., 0.), v3(0., 0., 1.), 6.0, 2.0);
    let mut p = v3(0., 30., 0.);
    for _ in 0..180 { p = p + v3(0., 0., 0.3); c.follow(p, v3(0., 0., 1.), 6.0, 2.0, &t, 1.0 / 60.0); }
    assert!((c.pos.z - (p.z - 6.0)).abs() < 4.0 && (c.pos.y - 32.0).abs() < 0.5, "{:?} vs {:?}", c.pos, p);
    let mut low = FollowCam::new_for(v3(0., 0.5, 0.), v3(0., 0., 1.), 6.0, 2.0);
    for _ in 0..60 { low.follow(v3(0., 0.5, 0.), v3(0., 0., 1.), 6.0, -5.0, &t, 1.0 / 60.0); }
    assert!(low.pos.y >= 0.89);
}

fn settled_car() -> (Vehicle, Terrain, FollowCam) {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    let mut c = FollowCam::new(&v);
    for _ in 0..120 { c.update(&v, &t, 1.0 / 60.0); }
    (v, t, c)
}
fn behind(c: &FollowCam, v: &Vehicle) -> f32 { -(c.pos - v.pos).dot(v.forward()) }

#[test] fn the_default_view_is_behind_the_car() {
    let (v, _, c) = settled_car();
    assert!(behind(&c, &v) > 7.0, "behind {}", behind(&c, &v));
    assert!(c.pos.y > v.pos.y + 2.0);
}
#[test] fn dragging_orbits_the_camera_around_the_car_and_tilts_it() {
    let (v, t, mut c) = settled_car();
    c.drag(std::f32::consts::FRAC_PI_2, 0.0);
    for _ in 0..120 { c.update(&v, &t, 1.0 / 60.0); }
    assert!(behind(&c, &v).abs() < 1.5, "a quarter turn puts the camera at the side: behind {}", behind(&c, &v));
    assert!((c.pos - v.pos).dot(v.right()).abs() > 7.0);
    let h0 = c.pos.y;
    c.set_dragging(false); c.user_yaw = 0.0;
    c.drag(0.0, -0.5);
    for _ in 0..120 { c.update(&v, &t, 1.0 / 60.0); }
    assert!(c.pos.y < h0 - 1.0, "dragging down lowers the camera: {} vs {h0}", c.pos.y);
    c.drag(0.0, -5.0); c.drag(0.0, 5.0); c.drag(0.0, 5.0);
    assert!(c.user_pitch <= 1.05 && c.user_pitch >= -0.25, "pitch is limited: {}", c.user_pitch);
}
#[test] fn the_camera_returns_behind_when_the_car_drives_forward_unless_the_finger_is_down() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    let mut c = FollowCam::new(&v);
    c.drag(1.5, 0.4);
    for _ in 0..180 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); c.update(&v, &t, 1.0 / 60.0); }
    assert!(v.speed() > 5.0 && c.user_yaw.abs() > 1.4, "the finger is still down: the offset stays ({})", c.user_yaw);
    c.set_dragging(false);
    for _ in 0..180 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); c.update(&v, &t, 1.0 / 60.0); }
    assert!(c.user_yaw.abs() < 0.05 && c.user_pitch.abs() < 0.05, "released and moving: back behind ({}, {})", c.user_yaw, c.user_pitch);
    assert!(behind(&c, &v) > 5.0);
}
#[test] fn a_parked_car_keeps_the_players_view() {
    let (v, t, mut c) = settled_car();
    c.drag(2.0, 0.3); c.set_dragging(false);
    for _ in 0..600 { c.update(&v, &t, 1.0 / 60.0); }
    assert!((c.user_yaw - 2.0).abs() < 1e-3, "{}", c.user_yaw);
}
#[test] fn reversing_swings_the_camera_to_the_front_and_forward_driving_swings_it_back() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    let mut c = FollowCam::new(&v);
    for _ in 0..60 { c.update(&v, &t, 1.0 / 60.0); }
    let rev = Input { throttle: -1.0, ..Default::default() };
    for _ in 0..30 { v.step(&t, &[], &rev, 1.0 / 60.0); c.update(&v, &t, 1.0 / 60.0); }
    assert!(behind(&c, &v) > 3.0, "0.5 s into reversing the swing has barely started or waits: {}", behind(&c, &v));
    for _ in 0..150 { v.step(&t, &[], &rev, 1.0 / 60.0); c.update(&v, &t, 1.0 / 60.0); }
    assert!(v.speed() < -1.0, "rolling backwards: {}", v.speed());
    assert!(behind(&c, &v) < -5.0, "the camera is in front of the car: {}", behind(&c, &v));
    let view = (c.target - c.pos).dot(v.forward());
    assert!(view < -3.0, "and looks backwards: {view}");
    let go = Input { throttle: 1.0, ..Default::default() };
    for _ in 0..360 { v.step(&t, &[], &go, 1.0 / 60.0); c.update(&v, &t, 1.0 / 60.0); }
    assert!(v.speed() > 2.0 && behind(&c, &v) > 5.0, "back behind: speed {} behind {}", v.speed(), behind(&c, &v));
}
#[test] fn a_short_reverse_blip_does_not_swing_the_camera() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    let mut c = FollowCam::new(&v);
    for _ in 0..60 { c.update(&v, &t, 1.0 / 60.0); }
    for _ in 0..4 { v.step(&t, &[], &Input { throttle: -1.0, ..Default::default() }, 1.0 / 60.0); c.update(&v, &t, 1.0 / 60.0); }
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); c.update(&v, &t, 1.0 / 60.0); }
    assert!(behind(&c, &v) > 6.0, "{}", behind(&c, &v));
}
#[test] fn the_orbit_never_goes_under_the_terrain_and_ignores_nan() {
    let t = Terrain::new(4);
    let v = Vehicle::new(VehicleSpec::offroader(), v3(0., t.height(0., 0.) + 1.1, 0.), 0.0);
    let mut c = FollowCam::new(&v);
    c.drag(f32::NAN, 1.0); assert_eq!(c.user_pitch, 0.0);
    c.drag(0.0, -9.0);
    for _ in 0..120 { c.update(&v, &t, 1.0 / 60.0); assert!(c.pos.y >= t.height(c.pos.x, c.pos.z) + 0.89 && c.pos.is_finite()); }
}
