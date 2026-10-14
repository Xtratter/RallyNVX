use super::*;
use crate::game::{input::Input, world::World};
#[test] fn free_drive_state_carries_speed_seed_and_money() {
    let w = World::new(7); let mut s = [9.0f32; STAT_LEN];
    fill_state(&mut s, &w, 55, 60.0, 4.0, 7);
    assert_eq!((s[0], s[3], s[16], s[2]), (60.0, 4.0, 55.0, 7.0));
    assert!(s[4..16].iter().chain(&s[17..25]).all(|v| *v == 0.0), "the unused slots stay zero");
}
#[test] fn signed_forward_speed_is_reported_for_the_brake_pedal() {
    let mut w = World::new(7); let mut s = [0.0f32; STAT_LEN];
    let gas = Input { throttle: 1.0, ..Default::default() };
    for _ in 0..120 { w.step(&gas, 1.0 / 60.0); }
    fill_state(&mut s, &w, 0, 60.0, 4.0, 7);
    assert!(s[25] > 5.0 && (s[25] - s[1]).abs() < 1.0, "forward {} total {}", s[25], s[1]);
    w.vehicle.vel = w.vehicle.forward() * -6.0;
    fill_state(&mut s, &w, 0, 60.0, 4.0, 7);
    assert!(s[25] < -20.0, "backwards must be negative: {}", s[25]);
}
#[test] fn the_surface_under_the_car_is_reported() {
    let mut w = World::with_roads(4, crate::game::vehicle::VehicleSpec::offroader());
    let mut s = [0.0f32; STAT_LEN];
    assert!(w.place_on_road(1));
    for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); }
    fill_state(&mut s, &w, 0, 60.0, 4.0, 4);
    assert_eq!(s[28], 5.0, "tarmac on the liaison");
    assert!(w.place_on_road(2));
    for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); }
    fill_state(&mut s, &w, 0, 60.0, 4.0, 4);
    assert_eq!(s[28], 4.0, "gravel on the stage");
}
