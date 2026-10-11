use super::*;
use crate::game::{math::v3, terrain::Terrain};
const DT: f32 = 1.0 / 60.0;
fn rope() -> f32 { crate::game::params::get().sling.rope }
fn hang(attach: V3) -> Sling { let mut s = Sling::on_ground(attach - v3(0., rope(), 0.)); s.attach(); s }

#[test] fn an_unattached_cargo_rests_on_the_ground_and_ignores_the_aircraft() {
    let t = Terrain::flat(); let mut s = Sling::on_ground(v3(5., 0., 5.));
    for _ in 0..120 { s.step(v3(5., 3., 5.), V3::ZERO, &t, DT); }
    assert!((s.pos.x - 5.0).abs() < 0.01 && s.pos.y < 0.5 && s.vel.len() < 0.1);
}
#[test] fn hangs_below_a_hovering_aircraft_at_rope_length() {
    let t = Terrain::flat(); let a = v3(0., 50., 0.); let mut s = hang(a);
    for _ in 0..(5.0 / DT) as usize { s.step(a, V3::ZERO, &t, DT); }
    let d = (a - s.pos).len();
    assert!(d >= rope() - 0.01 && d < rope() + 0.4, "rope {d}");
    assert!(s.pos.x.abs() < 0.2 && s.pos.z.abs() < 0.2);
    assert!(s.rope_taut(a));
}
#[test] fn follows_a_moving_aircraft() {
    let t = Terrain::flat(); let mut a = v3(0., 50., 0.); let mut s = hang(a);
    for _ in 0..(12.0 / DT) as usize { a.x += 10.0 * DT; s.step(a, v3(10., 0., 0.), &t, DT); }
    assert!((a.x - s.pos.x).abs() < 4.0, "lag {}", a.x - s.pos.x);
    assert!((a - s.pos).len() < rope() + 0.5);
}
#[test] fn swinging_damps_out() {
    let t = Terrain::flat(); let a = v3(0., 50., 0.); let mut s = hang(a);
    s.pos = a + v3(5.0, -6.0, 0.0);
    for _ in 0..(25.0 / DT) as usize { s.step(a, V3::ZERO, &t, DT); }
    assert!(s.pos.x.abs() < 0.4 && s.vel.len() < 0.3, "x {} v {}", s.pos.x, s.vel.len());
}
#[test] fn ground_impact_reports_its_speed_once() {
    let t = Terrain::flat(); let mut s = hang(v3(0., 8.5, 0.));
    s.vel = v3(0., -12.0, 0.);
    let mut hits = Vec::new();
    for _ in 0..120 { if let Some(v) = s.step(v3(0., 8.5, 0.), V3::ZERO, &t, DT) { hits.push(v); } }
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0] > 5.0, "{}", hits[0]);
}
#[test] fn a_teleporting_attach_point_keeps_the_cargo_finite_and_near() {
    let t = Terrain::flat(); let mut s = hang(v3(0., 50., 0.));
    for _ in 0..600 { s.step(v3(500., 90., -300.), V3::ZERO, &t, DT); }
    assert!(s.pos.is_finite() && s.vel.is_finite());
    assert!((v3(500., 90., -300.) - s.pos).len() < rope() + 1.0);
}
#[test] fn relative_speed_and_slack_rope() {
    let t = Terrain::flat(); let a = v3(0., 50., 0.); let mut s = hang(a);
    for _ in 0..300 { s.step(a, V3::ZERO, &t, DT); }
    assert!(s.relative_speed(V3::ZERO) < 0.2);
    assert!((s.relative_speed(v3(10., 0., 0.)) - 10.0).abs() < 0.3);
    assert!(!s.rope_taut(a + v3(0., -5.0, 0.)), "slack when the attach point is closer than the rope length");
}
#[test] fn dragging_over_hills_is_not_a_fall() {
    let t = Terrain::new(3);
    let mut a = v3(600., t.height(600., -500.) + 9.0, -500.); let mut s = hang(a);
    let mut worst: f32 = 0.0;
    for _ in 0..(12.0 / DT) as usize {
        a.x += 10.0 * DT; a.y = t.height(a.x, a.z) + 7.5;                  // the aircraft follows the terrain 7.5 m above it
        if let Some(v) = s.step(a, v3(10., 0., 0.), &t, DT) { worst = worst.max(v); }
    }
    assert!(worst < 5.0, "cargo never fell that fast, reported {worst}");
}
#[test] fn skimming_the_ground_is_not_an_impact() {
    let t = Terrain::flat();
    let mut a = v3(0., 8.5, 0.); let mut s = hang(a);
    let mut hits = 0;
    for _ in 0..(6.0 / DT) as usize { a.x += 10.0 * DT; if s.step(a, v3(10., 0., 0.), &t, DT).is_some() { hits += 1; } }
    assert_eq!(hits, 0, "dragging along the ground at 10 m/s is not a fall");
}
