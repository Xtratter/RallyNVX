use super::*;

#[test] fn the_tank_starts_where_it_is_told_and_the_mass_follows_the_fuel() {
    let f = Fuel::with(35.0); assert!((f.mass_delta()).abs() < 1e-3, "half a tank is the car's listed mass");
    assert!((Fuel::with(TANK_L).mass_delta() - 35.0 * KG_PER_L).abs() < 1e-3 && (Fuel::with(0.0).mass_delta() + 35.0 * KG_PER_L).abs() < 1e-3);
    assert_eq!(Fuel::with(500.0).litres, TANK_L); assert_eq!(Fuel::with(-3.0).litres, 0.0); assert_eq!(Fuel::with(f32::NAN).litres, 0.0);
}
#[test] fn the_flow_follows_the_power_and_an_overrun_burns_nothing() {
    let idle = Fuel::flow(0.0, 950.0, false);
    assert!(idle > 0.3e-3 / 3600.0 * 1000.0 && idle < 3.0 / 3600.0, "idle burns about 1 to 2 litres an hour: {} l/s", idle);
    let full = Fuel::flow(170_000.0, 5000.0, false);
    assert!(full > 60.0 / 3600.0 && full < 110.0 / 3600.0, "full power burns 60 to 110 l/h: {} l/s", full);
    assert!(Fuel::flow(80_000.0, 4000.0, false) > idle * 10.0 && full > Fuel::flow(80_000.0, 4000.0, false));
    assert_eq!(Fuel::flow(-20_000.0, 4000.0, true), 0.0, "the fuel is cut on an overrun");
    assert!(Fuel::flow(-20_000.0, 900.0, true) > 0.0, "but not below the idle revs");
    assert!(Fuel::flow(f32::NAN, f32::NAN, false) >= 0.0 && Fuel::flow(f32::NAN, f32::NAN, false).is_finite());
}
#[test] fn burning_empties_the_tank_to_zero_and_never_below() {
    let mut f = Fuel::with(0.01);
    for _ in 0..600 { f.burn(1.0 / 60.0, 170_000.0, 5000.0, false); }
    assert_eq!(f.litres, 0.0); assert!(f.empty());
    let mut g = Fuel::with(40.0); for _ in 0..60 * 60 { g.burn(1.0 / 60.0, 60_000.0, 4000.0, false); }
    assert!(g.litres < 40.0 && g.litres > 30.0 && !g.empty(), "a minute at 60 kW: {}", g.litres);
}
