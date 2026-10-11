use super::*;
#[test] fn cargo_gold_needs_speed_and_integrity() {
    assert_eq!(grade_cargo(60.0, 100.0, 90.0), Medal::Gold);
    assert_eq!(grade_cargo(60.1, 100.0, 100.0), Medal::Silver);
    assert_eq!(grade_cargo(50.0, 100.0, 89.9), Medal::Silver);
    assert_eq!(grade_cargo(80.0, 100.0, 60.0), Medal::Silver);
    assert_eq!(grade_cargo(80.1, 100.0, 100.0), Medal::Bronze);
    assert_eq!(grade_cargo(50.0, 100.0, 59.9), Medal::Bronze);
    assert_eq!(grade_cargo(100.0, 100.0, 1.0), Medal::Bronze);
}
#[test] fn race_grades_by_time_only() {
    assert_eq!(grade_race(60.0, 100.0), Medal::Gold);
    assert_eq!(grade_race(60.1, 100.0), Medal::Silver);
    assert_eq!(grade_race(80.0, 100.0), Medal::Silver);
    assert_eq!(grade_race(80.1, 100.0), Medal::Bronze);
}
#[test] fn reward_multipliers_round() {
    assert_eq!(reward(100.0, Medal::None), 0);
    assert_eq!(reward(100.0, Medal::Bronze), 100);
    assert_eq!(reward(100.0, Medal::Silver), 150);
    assert_eq!(reward(100.0, Medal::Gold), 220);
    assert_eq!(reward(133.0, Medal::Gold), 293);
}
#[test] fn payout_first_improvement_and_replay() {
    assert_eq!(payout(100.0, Medal::Silver, Medal::None), 150);
    assert_eq!(payout(100.0, Medal::Gold, Medal::Silver), 70);
    assert_eq!(payout(100.0, Medal::Gold, Medal::Gold), 110);
    assert_eq!(payout(100.0, Medal::Bronze, Medal::Gold), 50);
}
