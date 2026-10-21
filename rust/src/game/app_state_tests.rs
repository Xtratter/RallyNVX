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
#[test] fn the_tyres_are_reported_in_the_state_slots() {
    let mut w = World::new(7); let mut s = [0.0f32; STAT_LEN];
    w.vehicle.tyres[0].temp = 101.0; w.vehicle.tyres[3].temp = 33.0; w.vehicle.tyres[1].wear = 0.25;
    fill_state(&mut s, &w, 0, 60.0, 4.0, 7);
    assert_eq!((s[88], s[91], s[93]), (101.0, 33.0, 0.25));
    assert!(s[89] > 0.0 && s[90] > 0.0 && s[92] == 0.0 && s[94] == 0.0 && s[95] == 0.0);
}
#[test] fn the_heat_and_the_damage_are_reported_in_the_state_slots() {
    let mut w = World::new(7); let mut s = [0.0f32; STAT_LEN];
    w.drivetrain.coolant.temp = 97.0; w.drivetrain.clutch.temp = 210.0;
    w.vehicle.damage = crate::game::damage::Damage { body: 0.5, susp: [0.1, 0.2, 0.3, 0.4], engine: 0.25 };
    fill_state(&mut s, &w, 0, 60.0, 4.0, 7);
    assert_eq!((s[96], s[97], s[98], s[99]), (97.0, 210.0, 0.5, 0.25));
    assert_eq!([s[100], s[101], s[102], s[103]], [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(STAT_LEN, 104);
}
