use super::*;
use crate::game::{flight::{Aircraft, AircraftKind}, math::v3};
fn near(a: V3, b: V3) -> bool { (a - b).len() < 1e-3 }
#[test] fn align_y_maps_up_to_the_direction() {
    for d in [v3(0., 1., 0.), v3(1., 0., 0.), v3(0., -1., 0.), v3(1., 2., -3.).norm()] { assert!(near(align_y(d).rotate(v3(0., 1., 0.)), d), "{d:?}"); }
}
#[test] fn the_helicopter_has_spinning_rotor_blades_that_move_with_time() {
    let a = Aircraft::new(AircraftKind::Heli, v3(10., 20., 30.), 0.5);
    let (p0, p1) = (aircraft_parts(&a, 0.0), aircraft_parts(&a, 0.05));
    assert!(p0.len() >= 8 && p0.len() == p1.len());
    assert!(p0.iter().zip(&p1).any(|(x, y)| !near(x.rot.rotate(v3(1., 0., 0.)), y.rot.rotate(v3(1., 0., 0.)))), "blades must rotate");
    assert!(p0.iter().all(|p| (p.pos - a.pos).len() < 8.0 && p.scale.x > 0.0 && p.scale.y > 0.0 && p.scale.z > 0.0 && p.pos.is_finite()));
}
#[test] fn the_drone_has_four_rotors_and_follows_its_attitude() {
    let mut a = Aircraft::new(AircraftKind::Drone, v3(0., 10., 0.), 0.0);
    assert!(aircraft_parts(&a, 0.0).len() >= 9);
    let y0 = aircraft_parts(&a, 0.0)[0].pos.y;
    a.pitch = 0.5; a.pos = v3(0., 12., 0.);
    assert!(aircraft_parts(&a, 0.0)[0].pos.y > y0);
}
#[test] fn rings_have_16_glowing_cubes_on_the_circle_in_the_plane() {
    let n = v3(0., 0., 1.); let c = v3(5., 30., 5.);
    let parts = ring_parts(c, n, 8.0, false);
    assert_eq!(parts.len(), 16);
    for p in &parts { assert!(p.glow && p.mesh == 0); assert!(((p.pos - c).len() - 8.0).abs() < 1e-3); assert!((p.pos - c).dot(n).abs() < 1e-3); }
    assert_ne!(ring_parts(c, n, 8.0, true)[0].color, parts[0].color);
    let up = ring_parts(c, v3(0., 1., 0.), 8.0, false);                       // degenerate basis (normal parallel to up)
    assert!(up.iter().all(|p| p.pos.is_finite() && ((p.pos - c).len() - 8.0).abs() < 1e-3));
}
#[test] fn rope_connects_the_two_points() {
    let p = rope_part(v3(0., 10., 0.), v3(3., 4., 0.));
    assert!(near(p.pos, v3(1.5, 7.0, 0.)));
    assert!((p.scale.y - (3f32 * 3.0 + 36.0).sqrt()).abs() < 1e-3);
    assert!(near(p.rot.rotate(v3(0., 1., 0.)), (v3(3., 4., 0.) - v3(0., 10., 0.)).norm()));
}
#[test] fn pads_zones_and_cargo() {
    let pad = pad_part(v3(1., 2., 3.), 6.0);
    assert!(pad.glow && pad.scale.x == 12.0 && pad.scale.y < 1.0);
    let z = zone_parts(v3(0., 50., 0.), 3.0, 40.0);
    assert_eq!(z.len(), 25);
    assert!(z.iter().take(24).all(|p| (p.pos.x.hypot(p.pos.z) - 40.0).abs() < 1e-3 && (p.pos.y - 3.5).abs() < 1.0));
    assert!(z[24].scale.y > 40.0);
    assert!(!cargo_part(v3(0., 1., 0.)).glow);
}
#[test] fn aircraft_and_markers_are_never_facades() {
    let a = Aircraft::new(AircraftKind::Heli, v3(0., 10., 0.), 0.0);
    assert!(aircraft_parts(&a, 0.0).iter().all(|p| !p.facade));
    assert!(ring_parts(v3(0., 10., 0.), v3(0., 0., 1.), 8.0, false).iter().all(|p| !p.facade));
}
