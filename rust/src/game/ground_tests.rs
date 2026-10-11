use super::*;
use crate::game::{city::{Building, City, CitySite}, flight::{Aircraft, AircraftKind, FlightInput, Status}, math::{v3, V3}, sling::Sling, terrain::Terrain};
fn city() -> City {
    City::from_buildings(CitySite { centre: v3(0., 0., 0.), half: 264.0 }, vec![Building { min: v3(-10., 0., -10.), max: v3(10., 40., 10.), color: 0 }])
}
const DT: f32 = 1.0 / 60.0;
#[test] fn roofs_raise_the_ground_and_have_an_up_normal() {
    let t = Terrain::flat(); let cs = [city()]; let g = WorldGround { terrain: &t, cities: &cs };
    assert_eq!(g.height(0.0, 0.0), 40.0);
    assert_eq!(g.height(50.0, 50.0), 0.0);
    assert!((g.normal(0.0, 0.0).y - 1.0).abs() < 1e-6);
    let tn: &dyn Ground = &t; assert_eq!(tn.height(3.0, 3.0), 0.0);
}
#[test] fn an_aircraft_lands_softly_on_a_roof_and_takes_off_again() {
    let t = Terrain::flat(); let cs = [city()]; let g = WorldGround { terrain: &t, cities: &cs };
    let mut a = Aircraft::new(AircraftKind::Heli, v3(0.0, 48.0, 0.0), 0.0);
    for _ in 0..(8.0 / DT) as usize { a.step(&g, &[], &FlightInput { lift: -0.3, ..Default::default() }, DT); }
    assert_eq!(a.status, Status::Landed, "{:?}", a.status);
    assert!((a.pos.y - 40.3).abs() < 0.05, "{}", a.pos.y);
    for _ in 0..(3.0 / DT) as usize { a.step(&g, &[], &FlightInput { lift: 1.0, ..Default::default() }, DT); }
    assert!(a.status == Status::Flying && a.pos.y > 44.0);
}
#[test] fn flying_off_the_roof_edge_is_not_a_crash() {
    let t = Terrain::flat(); let cs = [city()]; let g = WorldGround { terrain: &t, cities: &cs };
    let mut a = Aircraft::new(AircraftKind::Drone, v3(0.0, 41.0, 0.0), 0.0);
    for _ in 0..(6.0 / DT) as usize { a.step(&g, &[], &FlightInput { pitch: 0.5, ..Default::default() }, DT); }
    assert_ne!(a.status, Status::Crashed, "leaving the roof must not count as a crash");
}
#[test] fn the_sling_cargo_rests_on_a_roof() {
    let t = Terrain::flat(); let cs = [city()]; let g = WorldGround { terrain: &t, cities: &cs };
    let mut s = Sling::on_ground(v3(0.0, 41.0, 0.0));
    for _ in 0..180 { s.step(v3(0.0, 80.0, 0.0), V3::ZERO, &g, DT); }
    assert!((s.pos.y - 40.3).abs() < 0.05 && s.vel.len() < 0.1, "{:?}", s.pos);
}
