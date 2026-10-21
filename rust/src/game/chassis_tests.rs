use super::*;

#[test] fn the_bar_pushes_the_compressed_wheel_up_and_the_other_down_in_proportion() {
    assert_eq!(bar_force(20000.0, 1.5, 0.05, 0.05), 0.0);
    let f = bar_force(20000.0, 1.5, 0.08, 0.02);
    assert!(f > 0.0 && (bar_force(20000.0, 1.5, 0.02, 0.08) + f).abs() < 1e-3, "antisymmetric: {f}");
    assert!((bar_force(20000.0, 1.5, 0.10, 0.0) / bar_force(20000.0, 1.5, 0.05, 0.0) - 2.0).abs() < 1e-4);
    assert!((bar_force(40000.0, 1.5, 0.08, 0.02) / f - 2.0).abs() < 1e-4 && bar_force(0.0, 1.5, 0.1, 0.0) == 0.0);
    assert_eq!(bar_force(f32::NAN, 1.5, 0.1, 0.0), 0.0);
}
#[test] fn a_tyre_gains_less_grip_than_it_gains_load() {
    assert!((load_factor(3000.0, 3000.0) - 1.0).abs() < 1e-6);
    assert!(load_factor(4500.0, 3000.0) < 1.0 && load_factor(1500.0, 3000.0) > 1.0);
    // a pair of equal loads gives more grip than the same total split unevenly
    let even = 2.0 * 3000.0 * load_factor(3000.0, 3000.0);
    let uneven = 4500.0 * load_factor(4500.0, 3000.0) + 1500.0 * load_factor(1500.0, 3000.0);
    assert!(even > uneven, "{even} against {uneven}");
    assert!(load_factor(1e9, 3000.0) >= 0.6 && load_factor(0.0, 3000.0) <= 1.15 && load_factor(3000.0, 0.0).is_finite());
}
#[test] fn camber_has_its_best_near_one_degree_of_negative_and_the_roll_leans_the_outer_wheel_out() {
    let g = |c: f32| camber_grip(c, 0.0, 1.0);
    assert!(g(-1.0) > g(-3.0) && g(-1.0) > g(2.0) && g(-1.0) > 0.999);
    assert!((g(-4.0) - g(2.0)).abs() < 1e-5, "symmetric around -1");
    assert!(g(-30.0) >= 0.8 && g(30.0) >= 0.8, "the floor");
    // a wheel on the outside of a turn (the body rolls onto it) gains outward camber: with a static -2 it is nearer to the best at some roll, with a static -1 it is worse
    assert!(camber_grip(-2.0, 2.0, 1.0) > camber_grip(-2.0, 0.0, 1.0) - 1e-6);
    assert!(camber_grip(-1.0, 4.0, 1.0) < camber_grip(-1.0, 0.0, 1.0));
    // the inner wheel (roll towards the other side) leans in
    assert!(camber_grip(-1.0, 4.0, -1.0) < camber_grip(-1.0, 0.0, -1.0));
}
#[test] fn toe_turns_the_two_wheels_of_an_axle_the_opposite_way() {
    assert_eq!(wheel_toe(0.0, true), 0.0);
    assert!((wheel_toe(0.2, true) + wheel_toe(0.2, false)).abs() < 1e-9, "the wheels turn opposite ways");
    assert!(wheel_toe(-0.2, true) == -wheel_toe(0.2, true));
    assert!((wheel_toe(1.0, true).abs() - 0.5f32.to_radians()).abs() < 1e-6, "one degree of toe is half a degree on each wheel");
}
#[test] fn the_multipliers_of_the_menu_become_the_setup() {
    let c = Chassis::from_tuning(&[1.0; 6]);
    assert_eq!((c.arb_front, c.arb_rear), (BASE.arb_front, BASE.arb_rear));
    assert!((c.camber_front - BASE.camber_front).abs() < 1e-6 && (c.toe_front - BASE.toe_front).abs() < 1e-6);
    let s = Chassis::from_tuning(&[0.0, 2.0, 0.0, 2.0, 0.0, 2.0]);
    assert_eq!((s.arb_front, s.arb_rear), (0.0, 2.0 * BASE.arb_rear));
    assert!((s.camber_front - 0.0).abs() < 1e-6 && (s.camber_rear - 2.0 * BASE.camber_rear).abs() < 1e-6);
    assert!(s.toe_front < c.toe_front && s.toe_rear > c.toe_rear);
    let n = Chassis::from_tuning(&[f32::NAN, 9.0, -3.0, f32::INFINITY, 0.5, 0.5]);
    assert!(n.arb_front.is_finite() && n.arb_rear <= 2.0 * BASE.arb_rear + 1e-3 && n.camber_front.is_finite() && n.toe_rear.abs() <= 0.5);
}
