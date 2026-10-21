use super::*;

#[test] fn a_bump_and_a_soft_landing_do_nothing() {
    let mut d = Damage::default();
    d.hit(2.0, true, true); d.landing(5.0); d.hit(f32::NAN, true, false); d.landing(f32::NAN);
    assert_eq!(d, Damage::default());
}
#[test] fn a_front_left_crash_hurts_the_body_that_corner_the_other_front_one_a_little_and_the_engine() {
    let mut d = Damage::default(); d.hit(9.0, true, true);
    assert!(d.body > 0.4 && d.body < 1.0, "{d:?}");
    assert!(d.susp[1] > d.susp[0] && d.susp[0] > 0.0 && d.susp[2] == 0.0 && d.susp[3] == 0.0, "the left front is the wheel 1: {d:?}");
    assert!(d.engine > 0.2, "{d:?}");
    let mut r = Damage::default(); r.hit(9.0, false, false);
    assert!(r.susp[2] > r.susp[3] && r.susp[3] > 0.0 && r.susp[0] == 0.0 && r.engine < d.engine * 0.4, "a rear right hit: the wheel 2 first, and it spares the engine: {r:?}");
}
#[test] fn a_harder_hit_hurts_more_and_the_damage_adds_up_to_one_at_most() {
    let (mut a, mut b) = (Damage::default(), Damage::default());
    a.hit(5.0, true, false); b.hit(12.0, true, false);
    assert!(b.body > a.body && b.susp[1] > a.susp[1]);
    for _ in 0..20 { b.hit(15.0, true, true); b.landing(30.0); }
    assert!(b.body <= 1.0 && b.engine <= 1.0 && b.susp.iter().all(|s| *s <= 1.0 && *s >= 0.0), "{b:?}");
}
#[test] fn a_hard_landing_hurts_all_four_springs_and_repair_makes_it_new() {
    let mut d = Damage::default(); d.landing(14.0);
    assert!(d.susp.iter().all(|s| *s > 0.1) && d.susp.iter().all(|s| (s - d.susp[0]).abs() < 1e-6) && d.engine == 0.0, "{d:?}");
    d.hit(9.0, true, true); d.repair();
    assert_eq!(d, Damage::default());
}
#[test] fn the_damage_costs_power_drag_a_bent_wheel_and_a_soft_spring_within_limits() {
    let new = Damage::default();
    assert_eq!((new.power(), new.drag_factor(), new.spring(0), new.heading(0)), (1.0, 1.0, 1.0, 0.0));
    let mut w = Damage { body: 1.0, susp: [1.0; 4], engine: 1.0 };
    assert!((w.power() - 0.6).abs() < 1e-6 && (w.drag_factor() - 1.25).abs() < 1e-6 && (w.spring(1) - 0.7).abs() < 1e-6);
    assert!(w.heading(0) > 0.0 && w.heading(1) < 0.0 && w.heading(2) > 0.0 && w.heading(3) < 0.0, "the bent wheels toe out: the right ones (0, 2) to the right");
    assert!((w.heading(0) - 3.0f32.to_radians()).abs() < 1e-6);
    w.susp[1] = 0.0; assert_eq!(w.heading(1), 0.0);
    assert!(w.total() > 0.8 && Damage { body: 1.0, susp: [1.0; 4], engine: 1.0 }.total() > 0.99 && Damage::default().total() == 0.0);
}
