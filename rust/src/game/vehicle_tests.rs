use super::*;
const DT: f32 = 1.0 / 60.0;
fn run(v: &mut Vehicle, t: &Terrain, inp: Input, secs: f32) { for _ in 0..(secs / DT) as usize { v.step(t, &[], &inp, DT); } }
fn flat_car(y: f32) -> (Vehicle, Terrain) { (Vehicle::new(VehicleSpec::offroader(), v3(0., y, 0.), 0.0), Terrain::flat()) }
#[test] fn rests_on_flat_ground() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input::default(), 4.0);
    assert!(v.vel.len() < 0.15, "vel {}", v.vel.len());
    assert!(v.pos.y > 0.5 && v.pos.y < 0.8, "y {}", v.pos.y);
    assert!(v.up().y > 0.99);
}
#[test] fn accelerates_to_a_sane_top_speed() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 15.0);
    assert!(v.speed() > 18.0 && v.speed() < 40.0, "speed {}", v.speed());
    assert!(v.pos.y > 0.4 && v.pos.y < 1.1, "y {}", v.pos.y);
    assert!(v.up().y > 0.95);
}
#[test] fn brakes_to_a_stop() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 8.0);
    run(&mut v, &t, Input { brake: true, ..Default::default() }, 6.0);
    assert!(v.speed().abs() < 0.5, "speed {}", v.speed());
}
#[test] fn steering_turns_the_heading() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 3.0);
    let h0 = v.forward().x.atan2(v.forward().z);
    run(&mut v, &t, Input { throttle: 0.5, steer: 1.0, ..Default::default() }, 2.0);
    let h1 = v.forward().x.atan2(v.forward().z);
    assert!((h1 - h0).abs() > 0.35, "turned {}", (h1 - h0).abs());
}
#[test] fn handbrake_lets_the_rear_slide() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 4.0);
    run(&mut v, &t, Input { throttle: 0.3, steer: 1.0, handbrake: true, ..Default::default() }, 1.5);
    let side = v.vel.dot(v.right()).abs();
    assert!(side > 1.0, "sideways {side}");
}
#[test] fn lands_from_a_drop_without_exploding() {
    let (mut v, t) = flat_car(6.0);
    run(&mut v, &t, Input::default(), 4.0);
    assert!(v.pos.is_finite() && v.pos.y > 0.5 && v.pos.y < 0.8, "y {}", v.pos.y);
    assert!(v.vel.len() < 0.5);
}
#[test] fn big_steps_stay_finite() {
    let (mut v, t) = flat_car(1.1);
    for _ in 0..200 { v.step(&t, &[], &Input { throttle: 1.0, steer: 0.5, ..Default::default() }, 0.1); }
    assert!(v.pos.is_finite() && v.vel.is_finite() && v.omega.is_finite());
    assert!(v.pos.y > -0.5);
}
#[test] fn flipped_is_detected_and_reset_uprights() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.5);
    v.rot = v.rot.mul(Q::axis_angle(v.forward(), 2.0));
    assert!(v.flipped());
    v.vel = v3(3., 2., 1.); v.reset_upright(&t);
    assert!(!v.flipped() && v.up().y > 0.99 && v.vel.len() == 0.0 && v.pos.y > 0.9);
}
#[test] fn drives_over_real_terrain_without_nan() {
    let t = Terrain::new(13);
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., t.height(0., 0.) + 1.1, 0.), 0.0);
    run(&mut v, &t, Input { throttle: 1.0, steer: 0.2, ..Default::default() }, 20.0);
    assert!(v.pos.is_finite() && v.vel.is_finite());
    let g = t.height(v.pos.x, v.pos.z);
    assert!(v.pos.y > g - 0.5, "below ground: y {} g {}", v.pos.y, g);
}
#[test] fn rock_blocks_the_car() {
    let (mut v, t) = flat_car(1.1);
    let rock = Prop { kind: PropKind::Rock, pos: v3(0., 0., 12.), radius: 1.5, height: 2.1 };
    for _ in 0..(8.0 / DT) as usize { v.step(&t, &[rock], &Input { throttle: 1.0, ..Default::default() }, DT); }
    let d = (v.pos - rock.pos).len();
    assert!(d > rock.radius + 0.8, "inside rock: {d}");
}
#[test] fn positive_steer_turns_towards_the_right_hand_side() {
    // x right, y up, z forward is what the chase camera sees from behind with look_at (right-handed):
    // the car's right-hand side is forward x up = -X.
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 3.0);
    run(&mut v, &t, Input { throttle: 0.5, steer: 1.0, ..Default::default() }, 1.5);
    assert!(v.forward().x < -0.1, "forward.x {}", v.forward().x);
    assert!(v.right().x < -0.9 || v.right().dot(v.forward().cross(v.up())) > 0.99);
}
#[test] fn flipped_car_slides_to_a_stop() {
    let (mut v, t) = flat_car(0.6);
    v.rot = v.rot.mul(Q::axis_angle(v.forward(), std::f32::consts::PI));
    v.vel = v3(0., 0., 6.);
    run(&mut v, &t, Input::default(), 4.0);
    assert!(v.vel.len() < 0.5, "still sliding at {}", v.vel.len());
}
fn never_rolls(throttle_pre: f32, secs_pre: f32, steer: f32, thr: f32) -> (bool, f32) {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { throttle: throttle_pre, ..Default::default() }, secs_pre);
    let mut worst: f32 = 1.0;
    for _ in 0..(5.0 / DT) as usize {
        v.step(&t, &[], &Input { throttle: thr, steer, ..Default::default() }, DT);
        worst = worst.min(v.up().y);
    }
    (worst > 0.7, worst)
}
#[test] fn hard_turns_on_flat_ground_do_not_roll_the_car() {
    for (pre, secs) in [(0.4, 3.0), (0.7, 5.0), (1.0, 8.0), (1.0, 15.0)] {
        for steer in [1.0, -1.0] {
            for thr in [1.0, 0.0] {
                let (ok, worst) = never_rolls(pre, secs, steer, thr);
                assert!(ok, "rolled: pre {pre}/{secs}s steer {steer} throttle {thr}: up.y min {worst}");
            }
        }
    }
}
#[test] fn centre_of_mass_is_low_enough_to_slide_before_it_rolls() {
    let s = VehicleSpec::offroader();
    let h = s.rest + s.radius - s.wheels[0].y;                    // centre of mass above the ground at rest
    let half_track = s.wheels[0].x.abs();
    assert!(half_track / h > s.mu * 1.15, "rollover limit {} g vs grip {} g", half_track / h, s.mu);
}
#[test] fn water_slows_the_car_down() {
    let go = |t: &Terrain| { let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.1, 0.), 0.0); run(&mut v, t, Input { throttle: 1.0, ..Default::default() }, 8.0); v.speed() };
    let (dry, wet) = (go(&Terrain::flat()), go(&Terrain::flooded(1.0)));
    assert!(wet < dry * 0.7, "dry {dry} wet {wet}");
}
#[test] fn the_brake_amount_scales_the_brake_force_and_zero_is_the_legacy_full_brake() {
    let speed_after = |inp: Input| {
        let (mut v, t) = flat_car(1.1);
        run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 8.0);
        let before = v.speed();
        run(&mut v, &t, inp, 1.0);
        (before, v.speed())
    };
    let (_, full) = speed_after(Input { brake: true, ..Default::default() });
    let (_, one) = speed_after(Input { brake: true, brake_amt: 1.0, ..Default::default() });
    let (before, soft) = speed_after(Input { brake: true, brake_amt: 0.3, ..Default::default() });
    assert!((full - one).abs() < 0.01 * before, "full {full} vs amt 1.0 {one}");
    assert!(soft > full + 1.0 && soft < before, "soft {soft} full {full} before {before}");
}
#[test] fn a_light_brake_press_never_weakens_the_handbrake() {
    let speed_after = |inp: Input| {
        let (mut v, t) = flat_car(1.1);
        run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 3.0);
        let before = v.speed();
        run(&mut v, &t, inp, 0.5);
        (before, v.speed())
    };
    let (before, hand) = speed_after(Input { handbrake: true, ..Default::default() });
    let (_, both) = speed_after(Input { handbrake: true, brake: true, brake_amt: 0.15, ..Default::default() });
    assert!(hand < before - 1.0, "the handbrake must brake: {before} -> {hand}");
    assert!(both <= hand + 0.05, "handbrake alone {hand}, with a light brake press {both}");
}
#[test] fn reversing_without_the_brake_gets_well_above_walking_pace() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { throttle: -1.0, ..Default::default() }, 6.0);
    assert!(v.speed() < -5.0, "reverse speed {} m/s", v.speed());
}

mod peugeot {
    use super::*;
    fn spec(id: &str) -> VehicleSpec { VehicleSpec::by_id(id).unwrap() }
    fn car_of(id: &str) -> (Vehicle, Terrain) { (Vehicle::new(spec(id), v3(0., 1.0, 0.), 0.0), Terrain::flat()) }
    #[test] fn the_centre_of_mass_guard_holds_for_every_car() {
        for (id, s) in VehicleSpec::all() {
            let h = s.rest + s.radius - s.wheels[0].y;
            assert!(s.wheels[0].x.abs() / h > s.mu * 1.15, "{id}: rollover limit {} g vs grip {}", s.wheels[0].x.abs() / h, s.mu);
        }
    }
    #[test] fn the_peugeot_rests_on_flat_ground_and_out_accelerates_the_offroader() {
        let (mut p, t) = car_of("vulpes");
        run(&mut p, &t, Input::default(), 3.0);
        assert!(p.up().y > 0.99 && p.vel.len() < 0.2 && p.pos.y > 0.3 && p.pos.y < 0.8, "y {} vel {}", p.pos.y, p.vel.len());
        let go = |id: &str| { let (mut v, t) = car_of(id); run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 5.0); v.speed() };
        assert!(go("vulpes") > go("offroader") + 2.0, "peugeot {} offroader {}", go("vulpes"), go("offroader"));
    }
    #[test] fn the_peugeot_does_not_roll_in_hard_turns_and_reverses_properly() {
        for (pre, secs) in [(0.4, 3.0), (0.7, 5.0), (1.0, 8.0), (1.0, 15.0)] { for steer in [1.0, -1.0] { for thr in [1.0, 0.0] {
            let (mut v, t) = car_of("vulpes");
            run(&mut v, &t, Input { throttle: pre, ..Default::default() }, secs);
            let mut worst: f32 = 1.0;
            for _ in 0..(5.0 / DT) as usize { v.step(&t, &[], &Input { throttle: thr, steer, ..Default::default() }, DT); worst = worst.min(v.up().y); }
            assert!(worst > 0.7, "rolled: pre {pre}/{secs}s steer {steer} thr {thr}: {worst}");
        } } }
        let (mut v, t) = car_of("vulpes");
        run(&mut v, &t, Input { throttle: -1.0, ..Default::default() }, 6.0);
        assert!(v.speed() < -5.0, "reverse {}", v.speed());
    }
}
#[test] fn the_vulpes_accelerates_like_the_real_rally_car() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    let go = Input { throttle: 1.0, ..Default::default() };
    let mut secs = 0.0; let mut best = 0.0f32;
    while v.speed() < 100.0 / 3.6 && secs < 12.0 { v.step(&t, &[], &go, 1.0 / 120.0); secs += 1.0 / 120.0; best = best.max(v.speed()); }
    assert!(secs > 3.5 && secs < 4.4, "0-100 km/h took {secs} s (best speed {best})");
}

use crate::game::{grip::LooseMap, surface::Surface};

fn braking_distance(surface: Surface, spec: VehicleSpec) -> f32 {
    let t = Terrain::flat();
    let mut v = Vehicle::new(spec, v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(surface);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0;
    let z0 = v.pos.z;
    for _ in 0..600 { v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 120.0); if v.speed() < 0.3 { break; } }
    v.pos.z - z0
}
fn cornering_accel(surface: Surface, spec: VehicleSpec) -> f32 {
    let t = Terrain::flat();
    let mut v = Vehicle::new(spec, v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(surface);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    v.vel = v.forward() * 20.0;
    let mut best = 0.0f32;
    for i in 0..240 {
        v.step(&t, &[], &Input { steer: 0.35, throttle: 0.3, ..Default::default() }, 1.0 / 120.0);
        if i > 120 { best = best.max(v.vel.len() * v.omega.y.abs()); }
    }
    best
}

#[test] fn braking_distance_orders_the_surfaces() {
    for spec in [VehicleSpec::offroader(), VehicleSpec::by_id("vulpes").unwrap()] {
        let d = |s| braking_distance(s, spec);
        let (a, g, gr, sa) = (d(Surface::Asphalt), d(Surface::Gravel), d(Surface::Grass), d(Surface::Sand));
        assert!(a < g && g < gr && gr < sa, "asphalt {a:.1} gravel {g:.1} grass {gr:.1} sand {sa:.1}");
        assert!(sa > a * 1.6 && a > 10.0, "{a} {sa}");
    }
}
#[test] fn cornering_grip_orders_the_surfaces() {
    let spec = VehicleSpec::by_id("vulpes").unwrap();
    let (a, g, s) = (cornering_accel(Surface::Asphalt, spec), cornering_accel(Surface::Gravel, spec), cornering_accel(Surface::Sand, spec));
    assert!(a > g * 1.15 && g > s * 1.1, "asphalt {a:.1} gravel {g:.1} sand {s:.1}");
}
#[test] fn the_vulpes_spins_its_wheels_on_gravel() {
    let spec = VehicleSpec::by_id("vulpes").unwrap();
    let time_to = |surface: Surface| {
        let t = Terrain::flat();
        let mut v = Vehicle::new(spec, v3(0., 1.0, 0.), 0.0);
        v.force_surface = Some(surface);
        for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
        let go = Input { throttle: 1.0, ..Default::default() };
        let mut secs = 0.0;
        while v.speed() < 100.0 / 3.6 && secs < 15.0 { v.step(&t, &[], &go, 1.0 / 120.0); secs += 1.0 / 120.0; }
        secs
    };
    let (a, g) = (time_to(Surface::Asphalt), time_to(Surface::Gravel));
    assert!(a > 3.4 && a < 4.5, "asphalt {a}");
    assert!(g > a * 1.2, "gravel {g} vs asphalt {a}");
}
#[test] fn the_car_reports_its_surface_and_the_road_wears_the_loose_layer() {
    use crate::game::roads::{Road, RoadKind, RoadNet, RoadPt};
    let net = RoadNet::new(vec![Road { kind: RoadKind::Stage, pts: (0..=250).map(|i| RoadPt { pos: v3(0.0, 0.0, i as f32 * 4.0), width: 8.0, s: i as f32 * 4.0 }).collect() }]);
    let t = Terrain::flat().with_net(net);
    let mut map = LooseMap::new(t.roads().unwrap());
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0.0, 1.0, 10.0), 0.0);
    for _ in 0..120 { v.step_with(&t, &[], &Input::default(), 1.0 / 60.0, &mut map); }
    assert_eq!(v.surface, Surface::Gravel);
    v.vel = v.forward() * 15.0;
    for _ in 0..600 { v.step_with(&t, &[], &Input { throttle: 0.4, ..Default::default() }, 1.0 / 120.0, &mut map); }
    let mid = v.pos.z - 20.0;
    let on_line = (0..8).map(|i| map.loose_at(0, mid - i as f32, -v.spec.wheels[0].x)).fold(1.0f32, f32::min);
    assert!(on_line < 0.6, "the wheel line is packed: {on_line}");
    assert_eq!(map.loose_at(0, mid, 3.5), 1.0, "3.5 m to the side is untouched");
}

#[test] fn wheel_info_reports_sliding_but_not_rolling() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0);
    v.force_surface = Some(Surface::Gravel);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    assert!(v.wheels.iter().all(|w| w.grounded && w.slide == 0.0 && w.speed < 0.2), "standing");
    for _ in 0..30 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 120.0); }
    assert!(v.wheels.iter().any(|w| w.slide > 0.1), "flooring it on loose gravel spins the wheels: {:?}", v.wheels.iter().map(|w| w.slide).collect::<Vec<_>>());
    for _ in 0..360 { v.step(&t, &[], &Input { throttle: 0.2, ..Default::default() }, 1.0 / 120.0); }
    let cruise: f32 = v.wheels.iter().map(|w| w.slide).sum();
    assert!(cruise < 0.2 && v.wheels[0].speed > 3.0, "cruising does not slide: {cruise}, speed {}", v.wheels[0].speed);
    v.vel = v.forward() * 22.0;
    for _ in 0..90 { v.step(&t, &[], &Input { steer: 0.5, throttle: 0.3, ..Default::default() }, 1.0 / 120.0); }
    assert!(v.wheels.iter().any(|w| w.lat_slip > 2.0 || w.slide > 0.1), "a hard turn at 22 m/s slides");
    assert!(v.wheels.iter().all(|w| w.slide <= 1.5 && w.contact.is_finite() && w.speed.is_finite()));
}
#[test] fn an_airborne_wheel_is_not_grounded() {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 6.0, 0.), 0.0);
    v.step(&t, &[], &Input::default(), 1.0 / 60.0);
    assert!(v.wheels.iter().all(|w| !w.grounded));
}

#[test] fn a_car_flies_off_a_crest_and_lands_in_one_piece() {
    use crate::game::roads::{Road, RoadKind, RoadNet, RoadPt};
    // a straight road along +z with one asymmetric crest (take-off 40.5 m, landing 19.5 m, 1.2 m high) at z = 300
    let bump = |z: f32| { let d = z - 300.0; let half = if d < 0.0 { 40.5 } else { 19.5 }; if d.abs() < half { 1.2 * 0.5 * (1.0 + (std::f32::consts::PI * d / half).cos()) } else { 0.0 } };
    let pts = (0..=240).map(|i| { let z = -100.0 + i as f32 * 4.0; RoadPt { pos: v3(0.0, bump(z), z), width: 8.0, s: i as f32 * 4.0 } }).collect();
    let t = Terrain::flat().with_net(RoadNet::new(vec![Road { kind: RoadKind::Stage, pts }]));
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(0., 1.0, 0.), 0.0);
    v.vel = v3(0.0, 0.0, 30.0);
    let (mut air, mut longest, mut max_speed) = (0.0f32, 0.0f32, 0.0f32);
    let mut loose = crate::game::grip::LooseMap::new(t.roads().unwrap());
    for _ in 0..(14.0 / DT) as usize {
        let inp = Input { throttle: if v.speed() < 30.0 { 1.0 } else { 0.0 }, ..Default::default() };   // keeps about 30 m/s
        v.step_with(&t, &[], &inp, DT, &mut loose);
        assert!(v.pos.is_finite() && v.vel.is_finite(), "NaN");
        max_speed = max_speed.max(v.vel.len());
        if v.wheels.iter().all(|w| !w.grounded) { air += DT; longest = longest.max(air); } else { air = 0.0; }
    }
    assert!(longest >= 0.15, "the car never left the ground (longest air {longest} s)");
    assert!(max_speed < 45.0, "{max_speed}");
    assert!(v.pos.z > 330.0, "the car should be past the crest, z {}", v.pos.z);
    assert!(v.up().y > 0.5 && !v.flipped(), "up.y {}", v.up().y);
    assert!(v.wheels.iter().all(|w| w.grounded) || v.vel.len() < 1.0);
}

#[test] fn brake_heat_grows_when_braking_hard_cools_down_and_stays_in_range() {
    let t = Terrain::flat(); let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.2, 0.), 0.0);
    for _ in 0..180 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    assert!(v.brake_heat.iter().all(|h| *h == 0.0), "cold at rest");
    for _ in 0..600 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    let fast = v.vel.len(); assert!(fast > 10.0, "{fast}");
    for _ in 0..240 { v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 60.0); }
    let hot = v.brake_heat; assert!(hot.iter().all(|h| (0.0..=1.0).contains(h)) && hot[0] > 0.3, "a hard stop from speed heats the front discs well: {hot:?}");
    assert!(hot[0] >= hot[2], "the front brakes work harder");
    for _ in 0..1200 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    assert!(v.brake_heat.iter().all(|h| *h < hot[0] * 0.2 + 1e-3), "{:?} cooled from {:?}", v.brake_heat, hot);
}
