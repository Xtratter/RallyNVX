//! The handling of the car with the aids on, pinned so the wheel model of 2.74.0 can be checked against 2.72.0.
use crate::game::{input::Input, math::v3, surface::Surface, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}, world::World};

fn time_to(speed: f32) -> f32 {
    let mut w = World::test_track(VehicleSpec::default_car());
    for k in 0..60 * 30 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); if w.vehicle.vel.len() >= speed { return k as f32 / 60.0; } }
    99.0
}
fn stop(speed: f32) -> f32 {
    let mut w = World::test_track(VehicleSpec::default_car());
    for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); }
    let f = w.vehicle.forward(); w.vehicle.vel = f * speed;
    let p0 = w.vehicle.pos;
    for _ in 0..60 * 15 { w.step(&Input { brake: true, brake_amt: 1.0, ..Default::default() }, 1.0 / 60.0); if w.vehicle.vel.len() < 0.2 { break; } }
    (w.vehicle.pos - p0).len()
}
fn braking(surface: Surface) -> f32 {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0); v.force_surface = Some(surface);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0; let z0 = v.pos.z;
    for _ in 0..600 { v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 120.0); if v.speed() < 0.3 { break; } }
    v.pos.z - z0
}
fn cornering(surface: Surface) -> f32 {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0); v.force_surface = Some(surface);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0; let mut best = 0.0f32;
    for i in 0..240 { v.step(&t, &[], &Input { steer: 0.35, ..Default::default() }, 1.0 / 120.0); if i > 120 { best = best.max(v.vel.len() * v.omega.y.abs()); } }
    best
}
fn near(got: f32, want: f32, tol: f32, what: &str) { assert!((got / want - 1.0).abs() <= tol, "{what}: {got} against the anchor {want} (+-{:.0} %)", tol * 100.0); }

#[test] #[ignore] fn print_anchors() {
    eprintln!("ANCHOR t22 {:.3} t27.8 {:.3} stop33 {:.2}", time_to(22.0), time_to(27.8), stop(33.0));
    for s in [Surface::Asphalt, Surface::Gravel, Surface::Sand] { eprintln!("ANCHOR {:?} brake20 {:.2} corner {:.2}", s, braking(s), cornering(s)); }
}
#[test] fn anchors_launch() {
    near(time_to(22.0), 4.45, 0.06, "0 to 22 m/s"); near(time_to(27.8), 6.50, 0.06, "0 to 27.8 m/s");
}
#[test] fn anchors_braking() {
    near(stop(33.0), 56.81, 0.09, "stop from 33 m/s");
    near(braking(Surface::Asphalt), 19.92, 0.09, "asphalt 20 m/s"); near(braking(Surface::Gravel), 27.06, 0.10, "gravel 20 m/s"); near(braking(Surface::Sand), 33.47, 0.10, "sand 20 m/s");
}
#[test] fn anchors_cornering() {
    near(cornering(Surface::Asphalt), 10.27, 0.08, "asphalt"); near(cornering(Surface::Gravel), 6.79, 0.08, "gravel"); near(cornering(Surface::Sand), 5.08, 0.08, "sand");
}
