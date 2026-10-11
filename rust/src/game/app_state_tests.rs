use super::*;
use crate::game::{input::Input, math::v3, missions::{Mission, MissionKind}, world::World};
fn mission() -> Mission {
    Mission { kind: MissionKind::Cargo, seed: 42, difficulty: 1, start: v3(0., 0., 0.), goal: v3(0., 0., 300.), checkpoints: vec![v3(0., 0., 300.)], length_m: 300.0, time_limit: 100.0, reward_base: 100.0 }
}
fn gas() -> Input { Input { throttle: 1.0, ..Default::default() } }
#[test] fn free_drive_state_has_phase_zero() {
    let w = World::new(7); let mut s = [9.0f32; STAT_LEN];
    fill_state(&mut s, &w, 55, 0, 60.0, 4.0, 7, 0);
    assert_eq!((s[0], s[3], s[4], s[16], s[2]), (60.0, 4.0, 0.0, 55.0, 7.0));
    assert_eq!(s[12], 0.0);
}
#[test] fn mission_state_reports_time_distance_and_bearing() {
    let mut w = World::for_mission(mission());
    w.step(&gas(), 1.0 / 60.0);
    let mut s = [0.0f32; STAT_LEN];
    fill_state(&mut s, &w, 0, 0, 60.0, 4.0, 42, 0);
    assert_eq!(s[4], 2.0);
    assert!(s[6] == 100.0 && s[7] == 100.0 && s[5] > 0.0);
    assert!(s[8] > 280.0 && s[8] < 301.0, "distance {}", s[8]);
    assert!(s[9].abs() < 0.2, "target is straight ahead, bearing {}", s[9]);
    assert_eq!((s[10], s[11], s[17]), (0.0, 1.0, 0.0));
}
#[test] fn done_state_carries_result_medal_and_payout() {
    let mut w = World::for_mission(mission());
    w.step(&gas(), 1.0 / 60.0);
    w.vehicle.pos = v3(0., 0.9, 299.0);
    w.step(&gas(), 1.0 / 60.0);
    let mut s = [0.0f32; STAT_LEN];
    fill_state(&mut s, &w, 220, 220, 60.0, 4.0, 42, 0);
    assert_eq!((s[4], s[12], s[13], s[14]), (3.0, 1.0, 3.0, 220.0));
    w.mission.as_mut().unwrap().abandon();
    fill_state(&mut s, &w, 220, 220, 60.0, 4.0, 42, 0);
    assert_eq!(s[12], 1.0);
}
#[test] fn failure_reasons_are_encoded() {
    let mut w = World::for_mission(mission());
    w.mission.as_mut().unwrap().abandon();
    let mut s = [0.0f32; STAT_LEN];
    fill_state(&mut s, &w, 0, 0, 0.0, 0.0, 42, 0);
    assert_eq!((s[12], s[15]), (2.0, 3.0));
}
#[test] fn run_counter_is_reported() {
    let w = World::new(1); let mut s = [0.0f32; STAT_LEN];
    fill_state(&mut s, &w, 0, 0, 0.0, 0.0, 1, 7);
    assert_eq!(s[18], 7.0);
}
