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

// ---- views, distance and height ----
fn settle_in(view: i32, dist: f32, height: f32) -> (Vehicle, Terrain, FollowCam) {
    let (v, t, mut c) = settled_car();
    c.set_view(view, dist, height);
    for _ in 0..240 { c.update(&v, &t, 1.0 / 60.0); }
    (v, t, c)
}
#[test] fn views_are_numbered_and_a_wild_index_is_clamped() {
    assert_eq!(CamView::from_index(0), CamView::Chase);
    assert_eq!(CamView::from_index(4), CamView::Orbit);
    assert_eq!(CamView::from_index(-3), CamView::Chase);
    assert_eq!(CamView::from_index(99), CamView::Orbit);
    let (_, _, mut c) = settled_car();
    c.set_view(99, 50.0, -50.0);
    assert_eq!(c.view, CamView::Orbit);
    assert!(c.dist <= 2.5 && c.dist >= 0.5 && c.height >= -3.0 && c.height <= 6.0, "{} {}", c.dist, c.height);
    c.set_view(0, f32::NAN, f32::INFINITY);
    assert!(c.dist.is_finite() && c.height.is_finite());
}
#[test] fn the_chase_distance_scales_and_the_height_offset_raises_the_camera() {
    let (v, _, near) = settle_in(0, 0.5, 0.0);
    let (_, _, far) = settle_in(0, 2.0, 0.0);
    assert!(behind(&far, &v) > behind(&near, &v) + 6.0, "{} vs {}", behind(&far, &v), behind(&near, &v));
    let (_, _, low) = settle_in(0, 1.0, 0.0);
    let (_, _, high) = settle_in(0, 1.0, 4.0);
    assert!(high.pos.y > low.pos.y + 2.5, "{} vs {}", high.pos.y, low.pos.y);
}
#[test] fn the_roof_view_is_higher_and_steeper_than_the_chase() {
    let (v, _, chase) = settle_in(0, 1.0, 0.0);
    let (_, _, roof) = settle_in(3, 1.0, 0.0);
    assert!(roof.pos.y > chase.pos.y + 4.0, "{} vs {}", roof.pos.y, chase.pos.y);
    assert!(behind(&roof, &v) < behind(&chase, &v) + 1.0);
}
#[test] fn the_hood_view_sits_on_the_bonnet_and_looks_ahead() {
    let (v, _, c) = settle_in(1, 1.0, 0.0);
    let off = c.pos - v.pos;
    let ahead = off.dot(v.forward());
    assert!(ahead > 0.0 && ahead < v.spec.size.z * 0.5, "on the bonnet: {ahead} m ahead of the centre");
    assert!(off.dot(v.right()).abs() < 0.2, "on the centre line");
    assert!(c.pos.y > v.pos.y - 0.2 && c.pos.y < v.pos.y + 1.4, "height {}", c.pos.y - v.pos.y);
    let dir = (c.target - c.pos).norm();
    assert!(dir.dot(v.forward()) > 0.97, "looks along the car: {}", dir.dot(v.forward()));
}
#[test] fn the_bumper_view_is_lower_and_further_forward_than_the_hood() {
    let (v, _, hood) = settle_in(1, 1.0, 0.0);
    let (_, _, bumper) = settle_in(2, 1.0, 0.0);
    assert!(bumper.pos.y < hood.pos.y - 0.2, "{} vs {}", bumper.pos.y, hood.pos.y);
    assert!((bumper.pos - v.pos).dot(v.forward()) > (hood.pos - v.pos).dot(v.forward()) + 0.3);
}
#[test] fn in_the_hood_view_the_distance_is_the_zoom_and_the_height_tilts_the_gaze() {
    let (_, _, wide) = settle_in(1, 0.6, 0.0);
    let (_, _, tele) = settle_in(1, 1.8, 0.0);
    assert!(tele.fov < wide.fov - 0.2, "zoomed in has a narrower field: {} vs {}", tele.fov, wide.fov);
    assert!(wide.fov <= 1.45 && tele.fov >= 0.5);
    let (_, _, level) = settle_in(1, 1.0, 0.0);
    let (_, _, up) = settle_in(1, 1.0, 1.0);
    assert!(up.target.y > level.target.y + 1.0, "{} vs {}", up.target.y, level.target.y);
}
#[test] fn the_orbit_view_keeps_the_players_angle_while_the_car_drives_on() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    let mut c = FollowCam::new(&v);
    c.set_view(4, 1.0, 0.0);
    c.drag(1.2, 0.0); c.set_dragging(false);
    for _ in 0..360 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); c.update(&v, &t, 1.0 / 60.0); }
    assert!(v.speed() > 5.0);
    assert!(c.user_yaw.abs() > 1.0, "the orbit view does not swing back behind: {}", c.user_yaw);
}
#[test] fn every_view_stays_finite_and_the_chase_views_stay_above_the_ground() {
    let t = Terrain::new(9);
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., t.height(0., 0.) + 1.1, 0.), 0.0);
    for view in 0..5 {
        let mut c = FollowCam::new(&v);
        c.set_view(view, 0.5, -3.0);
        for i in 0..400 {
            v.step(&t, &[], &Input { throttle: 1.0, steer: if (i / 100) % 2 == 0 { 0.3 } else { -0.3 }, ..Default::default() }, 1.0 / 60.0);
            c.update(&v, &t, 1.0 / 60.0);
            assert!(c.pos.is_finite() && c.target.is_finite() && c.fov.is_finite(), "view {view} step {i}");
            if matches!(c.view, CamView::Chase | CamView::Roof | CamView::Orbit) { assert!(c.pos.y >= t.height(c.pos.x, c.pos.z) + 0.89, "view {view} step {i}"); }
        }
    }
}
