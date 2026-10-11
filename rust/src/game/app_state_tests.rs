use super::*;
use crate::game::{input::Input, math::v3, missions::{Mission, MissionKind}, world::World};
fn mission() -> Mission {
    Mission { kind: MissionKind::Cargo, seed: 42, difficulty: 1, start: v3(0., 0., 0.), goal: v3(0., 0., 300.), checkpoints: vec![v3(0., 0., 300.)], length_m: 300.0, time_limit: 100.0, reward_base: 100.0, aircraft: None, urban: false, start_yaw: None }
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
use crate::game::flight::{AircraftKind, FlightInput};
fn air_mission(kind: MissionKind, ac: AircraftKind, pts: Vec<crate::game::math::V3>) -> Mission {
    let goal = *pts.last().unwrap();
    Mission { kind, seed: 42, difficulty: 1, start: v3(0., 0., 0.), goal, checkpoints: pts, length_m: 300.0, time_limit: 100.0, reward_base: 100.0, aircraft: Some(ac), urban: false, start_yaw: None }
}
#[test] fn air_state_reports_altitude_speed_kind_and_target_height() {
    let mut w = World::for_mission(air_mission(MissionKind::Rings, AircraftKind::Heli, vec![v3(0., 33., 300.)]));
    for _ in 0..30 { w.step_with(&Input::default(), &FlightInput { pitch: 1.0, ..Default::default() }, 1.0 / 60.0); }
    let mut s = [0.0f32; STAT_LEN];
    fill_state(&mut s, &w, 0, 0, 60.0, 4.0, 42, 3);
    assert_eq!((s[20], s[17], s[18], s[4]), (1.0, 2.0, 3.0, 2.0));
    assert!(s[19] > 2.5 && s[19] < 3.5, "altitude {}", s[19]);
    assert!(s[1] > 3.0, "aircraft speed in km/h, got {}", s[1]);
    assert!(s[22] > 25.0 && s[22] < 35.0, "target height above the aircraft {}", s[22]);
    assert!(s[8] > 250.0 && s[8] < 302.0 && s[9].abs() < 0.3, "d {} b {}", s[8], s[9]);
}
#[test] fn crash_is_failure_code_4() {
    let mut w = World::for_mission(air_mission(MissionKind::Rings, AircraftKind::Drone, vec![v3(0., 33., 300.)]));
    w.step_with(&Input::default(), &FlightInput { pitch: 0.5, ..Default::default() }, 1.0 / 60.0);
    w.aircraft.as_mut().unwrap().status = crate::game::flight::Status::Crashed;
    w.step_with(&Input::default(), &FlightInput { pitch: 0.5, ..Default::default() }, 1.0 / 60.0);
    let mut s = [0.0f32; STAT_LEN];
    fill_state(&mut s, &w, 0, 0, 0.0, 0.0, 42, 0);
    assert_eq!((s[12], s[15], s[20]), (2.0, 4.0, 2.0));
}
#[test] fn sling_state_has_the_rope_flag() {
    let mut w = World::for_mission(air_mission(MissionKind::Sling, AircraftKind::Heli, vec![v3(0., 0., 60.), v3(0., 0., 400.)]));
    let mut s = [0.0f32; STAT_LEN];
    fill_state(&mut s, &w, 0, 0, 0.0, 0.0, 42, 0);
    assert_eq!((s[17], s[21]), (3.0, 0.0));
    w.sling.as_mut().unwrap().attach();
    w.aircraft.as_mut().unwrap().pos = v3(0., 40., 60.);
    w.sling.as_mut().unwrap().pos = v3(0., 31.0, 60.);
    w.step_with(&Input::default(), &FlightInput { yaw: 0.2, ..Default::default() }, 1.0 / 60.0);
    fill_state(&mut s, &w, 0, 0, 0.0, 0.0, 42, 0);
    assert_eq!(s[21], 1.0, "taut rope");
}
