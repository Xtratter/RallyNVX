use super::*;

fn spec(id: &str) -> VehicleSpec { VehicleSpec::by_id(id).unwrap() }

#[test] fn level_zero_is_the_identity_and_only_the_documented_fields_change() {
    let s = spec("offroader");
    let z = apply(&s, [0; NLEVELS]);
    assert_eq!((z.engine_force, z.max_speed, z.mu, z.spring, z.damper, z.brake_force), (s.engine_force, s.max_speed, s.mu, s.spring, s.damper, s.brake_force));
    let close = |a: f32, b: f32| (a - b).abs() < 1e-3 * b.abs().max(1.0);
    for (lv, e, ms, mu, su, br) in [(1u8, 1.08, 1.03, 1.05, 1.1, 1.2), (2, 1.16, 1.06, 1.10, 1.2, 1.4), (3, 1.25, 1.09, 1.15, 1.3, 1.6)] {
        let a = apply(&s, [lv, 0, 0, 0, 0, 0, 0, 1]); assert!(close(a.engine_force, s.engine_force * e) && close(a.max_speed, s.max_speed * ms), "engine {lv}");
        assert_eq!((a.mu, a.spring, a.brake_force), (s.mu, s.spring, s.brake_force), "engine touches only its own fields");
        let a = apply(&s, [0, lv, 0, 0, 0, 0, 0, 1]); assert!(close(a.mu, s.mu * mu), "tyres {lv}"); assert_eq!((a.engine_force, a.brake_force), (s.engine_force, s.brake_force));
        let a = apply(&s, [0, 0, lv, 0, 0, 0, 0, 1]); assert!(close(a.spring, s.spring * su) && close(a.damper, s.damper * su), "suspension {lv}"); assert_eq!(a.mu, s.mu);
        let a = apply(&s, [0, 0, 0, lv, 0, 0, 0, 1]); assert!(close(a.brake_force, s.brake_force * br), "brakes {lv}"); assert_eq!(a.spring, s.spring);
        assert_eq!((a.mass, a.size.x, a.rest, a.radius, a.wheels[0].x, a.style), (s.mass, s.size.x, s.rest, s.radius, s.wheels[0].x, s.style), "geometry never changes");
    }
    for up in Upgrade::ALL { for lv in 0..3u8 { let (mut a, mut b) = ([0u8; NLEVELS], [0u8; NLEVELS]); a[up.index()] = lv; b[up.index()] = lv + 1;
        let (x, y) = (apply(&s, a), apply(&s, b));
        assert!(y.engine_force >= x.engine_force && y.mu >= x.mu && y.spring >= x.spring && y.brake_force >= x.brake_force && y.max_speed >= x.max_speed, "{up:?} {lv}"); } }
    assert_eq!(apply(&s, [9; 8]).engine_force, apply(&s, [3, 3, 3, 3, 3, 3, 3, 2]).engine_force, "levels above the cap clamp");
}
#[test] fn fully_upgraded_cars_still_slide_before_they_roll() {
    for (id, s) in VehicleSpec::all() {
        let a = apply(&s, [3, 3, 3, 3, 0, 0, 0, 1]);
        let h = a.rest + a.radius - a.wheels[0].y;
        assert!(a.wheels[0].x.abs() / h > a.mu, "{id}: rollover limit {} g vs upgraded grip {}", a.wheels[0].x.abs() / h, a.mu);
    }
}
#[test] fn upgrade_prices_are_one_two_and_four_times_the_base() {
    let s = spec("offroader");
    assert_eq!((upgrade_price(&s, 0), upgrade_price(&s, 1), upgrade_price(&s, 2), upgrade_price(&s, 3)), (Some(300), Some(600), Some(1200), None));
    let p = spec("vulpes");
    assert_eq!((upgrade_price(&p, 0), upgrade_price(&p, 2)), (Some(750), Some(3000)));
}
