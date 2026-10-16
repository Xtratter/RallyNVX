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

mod physics {
    use super::*;

    fn still() -> Air { Air { wind: V3::ZERO, car_pos: v3(0.0, 0.7, 0.0), car_vel: V3::ZERO, fwd: v3(0., 0., 1.), right: v3(-1., 0., 0.), half: v3(1.0, 0.7, 2.1), time: 0.0 } }
    fn moving() -> Air { Air { car_vel: v3(0.0, 0.0, 20.0), ..still() } }

    #[test] fn heavy_grit_falls_faster_than_fine_dust_and_both_relax_to_the_air() {
        let ph = phys();
        let u = air_velocity(v3(30.0, 5.0, 30.0), &still(), 0.0, &ph);   // far from the car, in the wind-free air
        let fall = |tau: f32, settle: f32| { let mut vy = 0.0f32; for _ in 0..120 { let k = 1.0 - (-(1.0 / 60.0) / tau).exp(); vy += ((u.y - settle) - vy) * k; } vy };
        let (fine, coarse) = (fall(ph.tau_fine, ph.settle_fine), fall(ph.tau_coarse, ph.settle_coarse));
        assert!(coarse < fine - 0.5, "coarse {coarse} fine {fine}");
        assert!((fine - (u.y - ph.settle_fine)).abs() < 0.05, "a fine puff follows the air after a few response times");
    }
    #[test] fn the_wake_drags_the_air_along_behind_the_moving_car_and_dies_away() {
        let (ph, a) = (phys(), moving());
        let near = air_velocity(v3(0.0, 0.8, -2.1 - 2.5), &a, 0.0, &ph);   // 2.5 m behind the rear bumper, on the centre line
        let far = air_velocity(v3(0.0, 0.8, -2.1 - 45.0), &a, 0.0, &ph);
        let side = air_velocity(v3(-9.0, 0.8, -2.1 - 2.5), &a, 0.0, &ph);
        let ahead = air_velocity(v3(0.0, 0.8, 12.0), &a, 0.0, &ph);
        assert!(near.z > 8.0, "the air behind the car is dragged forward: {}", near.z);
        assert!(near.z > far.z * 3.0 + 1.0 && near.z > side.z * 2.0 + 1.0 && near.z > ahead.z * 3.0 + 1.0, "near {} far {} side {} ahead {}", near.z, far.z, side.z, ahead.z);
        assert!(near.y > far.y, "the wake lifts the air");
    }
    #[test] fn behind_the_car_the_air_flows_in_towards_the_centre_line() {
        let (ph, a) = (phys(), moving());
        for lx in [-1.2f32, 1.2] {
            let calm = air_velocity(v3(-lx, 0.8, -2.1 - 3.0), &Air { car_vel: V3::ZERO, ..a }, 0.0, &ph);
            let wake = air_velocity(v3(-lx, 0.8, -2.1 - 3.0), &a, 0.0, &ph);   // the right vector is -x: local lateral lx is world x = -lx
            let inward = (wake.x - calm.x) * if lx > 0.0 { 1.0 } else { -1.0 };
            assert!(inward > 0.0, "lateral {lx}: the air should flow towards the centre line ({inward})");
        }
    }
    #[test] fn the_wind_is_weaker_near_the_ground_and_a_calm_world_stays_calm() {
        let ph = phys();
        let windy = Air { wind: v3(5.0, 0.0, 0.0), ..still() };
        let low = air_velocity(v3(40.0, 0.1, 40.0), &windy, 0.0, &ph).x;
        let high = air_velocity(v3(40.0, 6.0, 40.0), &windy, 0.0, &ph).x;
        assert!(high > low + 1.0 && high < 8.0, "low {low} high {high}");
        let calm = air_velocity(v3(40.0, 3.0, 40.0), &still(), 0.0, &ph);
        assert!(calm.len() < 2.0 * ph.turbulence + 0.5, "turbulence alone is gentle: {}", calm.len());
        assert!(air_velocity(v3(1.0, 1.0, -4.0), &moving(), 0.0, &ph).is_finite());
    }
    #[test] fn fresh_puffs_are_thrown_back_grit_never_goes_below_the_ground_and_speeds_stay_bounded() {
        let d = run(Surface::Gravel, 0.4, 0.3, 3.0);
        let fwd = v3(0.0, 0.0, 1.0);   // the car faces +z at the start; its heading drifts a little with the steering
        let fresh: Vec<&P> = d.parts.iter().filter(|p| p.age < 0.15).collect();
        assert!(fresh.len() > 3, "{} fresh puffs", fresh.len());
        assert!(fresh.iter().map(|p| p.vel.dot(fwd)).sum::<f32>() / (fresh.len() as f32) < 3.0, "fresh dust is not carried forward faster than walking pace");
        assert!(d.parts.iter().all(|p| p.pos.y >= 0.0 && p.vel.is_finite() && p.vel.len() < 40.0), "ground and speed limits");
        assert!(d.parts.iter().any(|p| p.coarse) && d.parts.iter().any(|p| !p.coarse), "both kinds are emitted");
    }
    #[test] fn the_numbers_parse_and_bad_ones_are_named() {
        assert!(phys().tau_coarse > phys().tau_fine && phys().settle_coarse > phys().settle_fine);
        let bad = grip::EMBEDDED.replace("\"wake_len\": 9.0", "\"wake_len\": 0.0");
        assert!(Phys::from_json(&bad).unwrap_err().contains("wake_len"));
        assert!(Phys::from_json("{}").unwrap_err().contains("dust_physics"));
    }
}

#[test]
fn the_gpu_dust_is_for_normal_and_lots_with_the_switch_or_the_debug_flag() {
    for level in 0..=3 {
        let on = matches!(level, 1 | 2);
        assert_eq!(gpu_wanted(true, false, level), on, "switch, level {level}");
        assert_eq!(gpu_wanted(false, true, level), on, "debug, level {level}");
        assert!(!gpu_wanted(false, false, level), "off, level {level}");
    }
}
#[test]
fn in_gpu_mode_the_wheels_only_emit() {
    use crate::game::{terrain::Terrain, vehicle::{Vehicle, VehicleSpec}, math::v3};
    let t = Terrain::new(7);
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0.0, 40.0, 0.0), 0.0);
    v.vel = v3(0.0, 0.0, 25.0);
    let mut d = Dust::new(7);
    d.set_gpu(true);
    for i in 0..60 { v.pos = v3(0.0, t.height(0.0, 0.0) + 1.0, 0.0); d.update(&v, &t, i as f32 / 60.0, 1.0 / 60.0); }
    assert_eq!(d.len(), 0, "no CPU particles in GPU mode");
    assert!(d.particles().is_empty());
    let n = d.take_emits().len();
    assert!(n <= MAX_EMITS);
    assert!(d.take_emits().is_empty(), "taking empties the queue");
    d.set_gpu(false);
    assert_eq!(d.len(), 0);
}

#[test]
fn the_cars_wake_scours_the_road_behind_it_even_without_slip() {
    use crate::game::{input::Input, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}, math::v3};
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0.0, 1.2, 0.0), 0.0);
    v.force_surface = Some(crate::game::surface::Surface::Dirt);
    for _ in 0..180 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 25.0;
    let mut d = Dust::new(5);
    d.set_gpu(true);
    let (fwd, hl) = (v.forward(), v.spec.size.z * 0.5);
    let mut behind = 0;
    for i in 0..120 {
        d.update(&v, &t, i as f32 / 60.0, 1.0 / 60.0);
        for p in d.take_emits() {
            let back = -(p.pos - v.pos).dot(fwd) - hl;
            if back > 0.25 && back < 4.5 && p.vel.len() < 6.0 { behind += 1; }
        }
        v.pos = v.pos + v.vel * (1.0 / 60.0);   // the car drives on (the wheels keep their last contact data)
    }
    assert!(behind > 20, "the wake picked up {behind} particles behind the car");
}

/// The GPU dust's emits over a few seconds of driving on a surface with the ground this wet.
fn gpu_emits(surface: Surface, wet: f32) -> Vec<P> {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(surface);
    let mut d = Dust::new(7);
    d.set_gpu(true);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 18.0; v.wet = wet;
    let mut out = Vec::new();
    for i in 0..180 { v.step(&t, &[], &Input { throttle: 0.4, steer: if (i / 40) % 2 == 0 { 0.3 } else { -0.3 }, ..Default::default() }, 1.0 / 60.0); d.update(&v, &t, i as f32 / 60.0, 1.0 / 60.0); out.extend(d.take_emits()); }
    out
}
#[test] fn on_wet_gravel_the_wheels_throw_spray_and_mud_instead_of_dust() {
    let dry = gpu_emits(Surface::Gravel, 0.0);
    assert!(!dry.is_empty() && dry.iter().all(|p| !(p.smoke && p.coarse)), "dry: dust, no spray");
    let wet = gpu_emits(Surface::Gravel, 1.0);
    let spray = wet.iter().filter(|p| p.smoke && p.coarse).count();
    let mud = wet.iter().filter(|p| p.speck && p.coarse).count();
    let dust = wet.iter().filter(|p| !p.smoke && !p.speck).count();
    assert!(spray > wet.len() / 4 && mud > wet.len() / 10, "spray {spray}, mud {mud} of {}", wet.len());
    assert!(dust < wet.len() / 5, "only a thin share of dust is left: {dust} of {}", wet.len());
    assert!(wet.iter().filter(|p| p.smoke && p.coarse).all(|p| p.color[2] > p.color[0] - 0.05), "the mist is pale and cold");
    let specks: Vec<_> = wet.iter().filter(|p| p.speck).collect();
    assert!(specks.iter().filter(|p| p.color.iter().all(|c| *c < 0.5)).count() * 10 > specks.len() * 8, "the mud is dark");
}
#[test] fn wet_asphalt_gives_spray_and_no_mud() {
    let dry = gpu_emits(Surface::Asphalt, 0.0);
    let wet = gpu_emits(Surface::Asphalt, 1.0);
    assert!(wet.iter().any(|p| p.smoke && p.coarse), "spray");
    assert!(wet.iter().all(|p| !(p.speck && p.coarse)), "no mud on a road");
    assert!(wet.len() * 2 >= dry.len(), "the road is not silent when wet: {} vs {}", wet.len(), dry.len());
}
