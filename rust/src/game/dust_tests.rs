use super::*;
use crate::game::{input::Input, math::v3, surface::Surface, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};

fn run(surface: Surface, throttle: f32, steer: f32, secs: f32) -> Dust {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(surface);
    let mut d = Dust::new(7);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 18.0;
    let steps = (secs * 60.0) as usize;
    for i in 0..steps { v.step(&t, &[], &Input { throttle, steer, ..Default::default() }, 1.0 / 60.0); d.update(&v, &t, i as f32 / 60.0, 1.0 / 60.0); }
    d
}

#[test] fn a_standing_car_raises_no_dust() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(Surface::Sand);
    let mut d = Dust::new(1);
    for i in 0..300 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); d.update(&v, &t, i as f32 / 60.0, 1.0 / 60.0); }
    assert_eq!(d.len(), 0);
}
#[test] fn rolling_dust_follows_the_surface() {
    let n = |s| run(s, 0.4, 0.0, 3.0).len();
    let (gravel, sand, dirt, grass, asphalt) = (n(Surface::Gravel), n(Surface::Sand), n(Surface::Dirt), n(Surface::Grass), n(Surface::Asphalt));
    assert!(gravel > 10 && sand > gravel && gravel > grass && dirt > grass, "gravel {gravel} sand {sand} dirt {dirt} grass {grass}");
    assert_eq!(asphalt, 0, "tarmac does not raise dust while rolling");
}
#[test] fn a_hard_slide_smokes_on_tarmac_and_throws_more_on_gravel() {
    let (asph, grav) = (run(Surface::Asphalt, 0.3, 0.6, 3.0).len(), run(Surface::Gravel, 0.3, 0.6, 3.0).len());
    assert!(asph > 0, "a slide on tarmac gives some smoke");
    assert!(grav > asph, "gravel {grav} vs tarmac {asph}");
}
#[test] fn the_cap_the_levels_and_the_ring_hold() {
    let mut d = run(Surface::Sand, 1.0, 0.5, 6.0);
    assert!(d.len() <= 400 && d.len() > 100, "{}", d.len());
    d.set_level(2);
    let big = { let mut d2 = Dust::new(3); d2.set_level(2); d2 };
    assert!(big.cap() == 800 && d.cap() == 800);
    d.set_level(0);
    assert_eq!((d.len(), d.cap()), (0, 0), "off frees everything");
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(Surface::Sand); v.vel = v.forward() * 20.0;
    for i in 0..60 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); d.update(&v, &t, i as f32 / 60.0, 1.0 / 60.0); }
    assert_eq!(d.len(), 0, "level 0 emits nothing");
}
#[test] fn particles_age_out_grow_fade_and_drift_with_the_wind() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(Surface::Gravel);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0;
    let mut d = Dust::new(5);
    for i in 0..120 { v.step(&t, &[], &Input { throttle: 0.4, ..Default::default() }, 1.0 / 60.0); d.update(&v, &t, i as f32 / 60.0, 1.0 / 60.0); }
    let early = d.particles();
    assert!(!early.is_empty() && early.iter().all(|p| p.pos.is_finite() && p.size > 0.0 && (0.0..=1.0).contains(&p.alpha)));
    let (small, big) = (early.iter().map(|p| p.size).fold(f32::MAX, f32::min), early.iter().map(|p| p.size).fold(0.0, f32::max));
    assert!(big > small * 1.5, "older puffs are bigger: {small} .. {big}");
    // stop the car: every particle is gone after its longest life
    v.vel = v.vel * 0.0; v.force_surface = Some(Surface::Gravel);
    for i in 0..(7 * 60) { v.vel = v.vel * 0.0; v.step(&t, &[], &Input::default(), 1.0 / 60.0); d.update(&v, &t, 2.0 + i as f32 / 60.0, 1.0 / 60.0); }
    assert_eq!(d.len(), 0, "the cloud settles within a few seconds");
}
#[test] fn odd_inputs_never_poison_the_cloud() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(Surface::Sand);
    let mut d = Dust::new(2);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0;
    v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0);
    d.update(&v, &t, 0.0, 0.0);
    d.update(&v, &t, 1.0, f32::NAN);
    d.update(&v, &t, f32::NAN, 5.0);
    d.update(&v, &t, 2.0, -1.0);
    for w in &mut v.wheels { w.slide = f32::NAN; w.speed = f32::INFINITY; w.contact = v3(f32::NAN, 0., 0.); }
    d.update(&v, &t, 3.0, 1.0 / 60.0);
    assert!(d.particles().iter().all(|p| p.pos.is_finite() && p.size.is_finite() && p.alpha.is_finite()));
    assert!(d.len() <= 400);
}
