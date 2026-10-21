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
    assert!(a > g * 1.15 && g > s * 1.05, "asphalt {a:.1} gravel {g:.1} sand {s:.1}");
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

#[test] fn the_right_wheel_set_corners_harder_on_its_surface() {
    // a car on gravel and on asphalt with each set: the peak lateral grip follows the set
    fn peak_side_accel(set: u8, surface: crate::game::surface::Surface) -> f32 {
        let t = Terrain::flat();
        let mut spec = VehicleSpec::by_id("vulpes").unwrap(); spec.wheel_set = set;
        let mut v = Vehicle::new(spec, v3(0., 1.2, 0.), 0.0);
        v.force_surface = Some(surface);
        for _ in 0..180 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
        for _ in 0..360 { v.step(&t, &[], &Input { throttle: 0.6, ..Default::default() }, 1.0 / 60.0); }
        let mut peak = 0.0f32;
        for _ in 0..240 {
            let before = v.vel;
            v.step(&t, &[], &Input { throttle: 0.2, steer: 1.0, ..Default::default() }, 1.0 / 60.0);
            let a = (v.vel - before) * 60.0;
            peak = peak.max(a.dot(v.right()).abs());
        }
        peak
    }
    use crate::game::surface::Surface::{Asphalt, Gravel};
    let (g_on_gravel, t_on_gravel) = (peak_side_accel(1, Gravel), peak_side_accel(0, Gravel));
    assert!(g_on_gravel > t_on_gravel, "gravel tyres corner harder on gravel: {g_on_gravel} vs {t_on_gravel}");
    let (t_on_asph, g_on_asph) = (peak_side_accel(0, Asphalt), peak_side_accel(1, Asphalt));
    assert!(t_on_asph > g_on_asph, "tarmac tyres corner harder on asphalt: {t_on_asph} vs {g_on_asph}");
}

#[test] fn wet_ground_makes_the_car_brake_later_and_slide_sooner() {
    use crate::game::surface::Surface::Asphalt;
    fn stop(wet: f32) -> f32 {
        let t = Terrain::flat();
        let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.2, 0.), 0.0);
        v.force_surface = Some(Asphalt); v.wet = wet;
        for _ in 0..180 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
        for _ in 0..600 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
        let x0 = v.pos;
        for _ in 0..900 { if v.vel.len() < 0.5 { break; } v.step(&t, &[], &Input { brake: true, ..Default::default() }, 1.0 / 60.0); }
        (v.pos - x0).len()
    }
    let (dry, soaked) = (stop(0.0), stop(1.0));
    assert!(soaked > dry * 1.1, "the wet road lengthens the braking: {dry} m dry, {soaked} m soaked");
}

/// A spot of the level's terrain where the ground falls away ahead of the car at roughly `grade` (rise over run) and is smooth enough to park on.
fn slope_spot(t: &Terrain, grade: f32) -> Option<(V3, f32)> {
    for j in -40..=40 { for i in -40..=40 {
        let (x, z) = (i as f32 * 12.0, j as f32 * 12.0);
        let (gx, gz) = ((t.height(x + 2.0, z) - t.height(x - 2.0, z)) / 4.0, (t.height(x, z + 2.0) - t.height(x, z - 2.0)) / 4.0);
        let g = gx.hypot(gz);
        if (g - grade).abs() < 0.04 && !t.is_water(x, z, 2.0) {
            // the car looks downhill: along (-gx, -gz)
            let yaw = (-gx).atan2(-gz);
            // the ground must be about as steep a little way across and further down, so it does not level off under the car
            let (fx, fz) = (x - gx / g * 6.0, z - gz / g * 6.0);
            if (t.height(fx, fz) - t.height(x, z)).abs() > grade * 6.0 * 0.6 { return Some((v3(x, t.height(x, z) + 1.1, z), yaw)); }
        }
    } }
    None
}
#[test] fn the_handbrake_holds_the_car_on_a_slope_that_it_rolls_down_without_it() {
    let t = Terrain::new(7);
    let (pos, yaw) = slope_spot(&t, 0.18).expect("a slope of about 10 degrees");
    let drift = |inp: Input| {
        let mut v = Vehicle::new(VehicleSpec::offroader(), pos, yaw);
        run(&mut v, &t, inp, 1.5);   // it settles onto its suspension
        let p0 = v.pos;
        run(&mut v, &t, inp, 6.0);
        ((v.pos - p0).len(), v.vel.len())
    };
    let (free, free_v) = drift(Input::default());
    let (held, held_v) = drift(Input { handbrake: true, ..Default::default() });
    assert!(free > 6.0, "it rolls down without the handbrake: {free} m at {free_v} m/s");
    assert!(held < 0.7 && held_v < 0.3, "the handbrake holds it: {held} m, {held_v} m/s");
}
#[test] fn a_locked_rear_wheel_gives_no_drive_and_the_front_wheels_keep_theirs() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input::default(), 1.0);
    run(&mut v, &t, Input { throttle: 1.0, handbrake: true, ..Default::default() }, 3.0);
    let (mut free, _) = flat_car(1.1);
    run(&mut free, &t, Input::default(), 1.0);
    run(&mut free, &t, Input { throttle: 1.0, ..Default::default() }, 3.0);
    assert!(v.speed() < free.speed() * 0.6, "the locked rear tyres drag: {} against {}", v.speed(), free.speed());
    assert!(v.parked && !v.braking, "parked, not braking");
}

#[test] fn a_coasting_car_keeps_its_wheels_rolling_with_the_ground() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input::default(), 1.0);
    v.vel = v.forward() * 15.0;
    run(&mut v, &t, Input::default(), 1.0);
    for k in 0..4 { assert!((v.w[k] * v.spec.radius - v.speed()).abs() < 0.8, "wheel {k}: {} m/s against the car's {}", v.w[k] * v.spec.radius, v.speed()); }
}
#[test] fn without_abs_the_brake_locks_the_wheels_and_with_it_they_keep_turning() {
    let go = |abs: bool| { let (mut v, t) = flat_car(1.1); v.abs_on = abs; run(&mut v, &t, Input::default(), 1.0); v.vel = v.forward() * 25.0; run(&mut v, &t, Input::default(), 0.3);
        run(&mut v, &t, Input { brake: true, ..Default::default() }, 0.6); (v.w, v.slip, v.speed()) };
    let (w_off, _, _) = go(false); let (w_on, slip_on, speed_on) = go(true);
    assert!(w_off.iter().any(|w| w.abs() < 1.0), "without ABS some wheel locks: {w_off:?}");
    assert!(w_on.iter().all(|w| *w > 20.0) && speed_on > 5.0, "turning: {w_on:?} at {speed_on} m/s");
    assert!(slip_on.iter().all(|s| *s < 0.0 && *s > -2.5), "ABS holds the slip near the peak: {slip_on:?}");
}
#[test] fn the_handbrake_locks_the_rear_wheels_and_the_front_ones_keep_turning() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input::default(), 1.0); v.vel = v.forward() * 12.0; run(&mut v, &t, Input::default(), 0.3);
    run(&mut v, &t, Input { handbrake: true, ..Default::default() }, 0.6);
    assert!(v.w[2].abs() < 1.0 && v.w[3].abs() < 1.0, "rear locked: {:?}", v.w);
    assert!(v.w[0] > 5.0 && v.w[1] > 5.0, "front free: {:?}", v.w);
}
#[test] fn a_locked_car_on_a_slope_stays_put_without_jitter() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input { handbrake: true, brake: true, ..Default::default() }, 1.0);
    let p = v.pos; let mut max_speed = 0.0f32;
    for _ in 0..600 { v.step(&t, &[], &Input { handbrake: true, brake: true, ..Default::default() }, DT); max_speed = max_speed.max(v.vel.x.hypot(v.vel.z)); }
    assert!((v.pos - p).len() < 0.02 && max_speed < 0.05, "moved {} m, peak speed {}", (v.pos - p).len(), max_speed);
}
#[test] fn standing_starts_are_smooth_forward_and_reverse() {
    for sign in [1.0f32, -1.0] {
        let (mut v, t) = flat_car(1.1);
        run(&mut v, &t, Input::default(), 1.0);
        let mut last = 0.0f32; let mut worst_jump = 0.0f32;
        for _ in 0..180 {
            v.drive_torque = Some([sign * 500.0; 4]);
            v.step(&t, &[], &Input { throttle: sign, ..Default::default() }, DT);
            let s = v.speed(); worst_jump = worst_jump.max((s - last).abs()); last = s;
            assert!(v.w.iter().all(|w| w.is_finite() && w.abs() < 400.0), "{:?}", v.w);
        }
        assert!(sign * v.speed() > 3.0 && worst_jump < 0.5, "sign {sign}: {} m/s, the largest jump in one step {worst_jump}", v.speed());
    }
}
#[test] fn landing_with_spinning_wheels_is_stable() {
    let (mut v, t) = flat_car(3.0);
    v.drive_torque = Some([900.0; 4]);
    for _ in 0..90 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, DT); }
    assert!(v.pos.y.is_finite() && v.vel.len() < 60.0 && v.w.iter().all(|w| w.is_finite() && w.abs() < 800.0), "{:?} {:?}", v.vel, v.w);
}
#[test] fn the_wheel_slip_shows_in_the_info_when_driving_hard_on_gravel() {
    let (mut v, t) = flat_car(1.1); v.force_surface = Some(crate::game::surface::Surface::Gravel);
    run(&mut v, &t, Input::default(), 1.0);
    v.drive_torque = Some([1400.0; 4]);
    run(&mut v, &t, Input { throttle: 1.0, ..Default::default() }, 0.4);
    // the lighter front wheels spin first (the weight moves back under power)
    assert!(v.slip[0] > 1.0 && v.slip[1] > 1.0, "spinning on gravel: {:?}", v.slip);
    assert!(v.wheels[0].slide > 0.0 && v.wheels[1].slide > 0.0, "the dust sees it: {:?}", v.wheels.iter().map(|w| w.slide).collect::<Vec<_>>());
}

#[test] fn the_brake_bias_decides_which_axle_locks_first() {
    let go = |bias: f32| {
        let (mut v, t) = flat_car(1.1); v.abs_on = false; v.brake_bias = bias;
        run(&mut v, &t, Input::default(), 1.0); v.vel = v.forward() * 25.0; run(&mut v, &t, Input::default(), 0.3);
        run(&mut v, &t, Input { brake: true, brake_amt: 0.6, ..Default::default() }, 0.35);
        (v.w[0].abs() + v.w[1].abs()) * 0.5 - (v.w[2].abs() + v.w[3].abs()) * 0.5   // front spin minus rear spin
    };
    let (front_heavy, rear_heavy) = (go(0.85), go(0.35));
    assert!(front_heavy < rear_heavy - 10.0, "with 85 % on the front the front slows more (front - rear spin {front_heavy}), with 35 % the rear does ({rear_heavy})");
}

/// A steady turn at 25 m/s with the speed held; returns the lateral acceleration (speed times yaw rate) after settling, for a car whose tyres are set up by `prep`.
fn turn_grip(prep: impl Fn(&mut Vehicle)) -> f32 {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), DT); }
    prep(&mut v);
    v.vel = v.forward() * 25.0;
    let mut best = 0.0f32;
    for i in 0..240 {
        let s = v3(v.vel.x, 0.0, v.vel.z).len(); if s > 1.0 { let k = 25.0 / s; v.vel = v3(v.vel.x * k, v.vel.y, v.vel.z * k); }
        v.step(&t, &[], &Input { steer: 0.35, ..Default::default() }, DT);
        if i > 120 { best = best.max(v.vel.len() * v.omega.y.abs()); }
    }
    best
}
#[test] fn a_hard_slalom_heats_the_tyres_and_wears_them_a_little() {
    let (mut v, t) = flat_car(1.1);
    run(&mut v, &t, Input::default(), 1.0);
    v.vel = v.forward() * 20.0;
    let start = v.tyres[0].temp;
    for k in 0..60 * 12 { v.step(&t, &[], &Input { steer: if (k / 60) % 2 == 0 { 1.0 } else { -1.0 }, throttle: 0.4, ..Default::default() }, DT); }
    // the front tyres do the steering work and heat first; the rear ones follow and wear less
    assert!(v.tyres[0].temp > start + 8.0 && v.tyres[1].temp > start + 8.0, "heated from {start}: {:?}", v.tyres.map(|y| y.temp));
    assert!(v.tyres[0].wear > 0.0 && v.tyres.iter().all(|y| y.wear < 0.1), "worn a little: {:?}", v.tyres.map(|y| y.wear));
    assert!(v.tyres[0].wear > v.tyres[2].wear, "the front wears more: {:?}", v.tyres.map(|y| y.wear));
}
#[test] fn a_hot_tyre_cools_on_a_straight_faster_than_standing_and_neither_wears() {
    let cooled = |moving: bool| {
        let (mut v, t) = flat_car(1.1);
        run(&mut v, &t, Input::default(), 1.0);
        for y in v.tyres.iter_mut() { y.temp = 130.0; }
        for _ in 0..60 * 10 { if moving { let f = v.forward() * 25.0; v.vel = v3(f.x, v.vel.y, f.z); } v.step(&t, &[], &Input::default(), DT); }
        (v.tyres[0].temp, v.tyres[0].wear)
    };
    let ((still, w0), (rolling, w1)) = (cooled(false), cooled(true));
    assert!(still < 130.0 && rolling < still - 5.0, "standing {still}, rolling {rolling}");
    assert!(w0 < 1e-6 && w1 < 1e-3, "a car that rolls straight or stands hardly wears: {w0} {w1}");
}
#[test] fn worn_cold_and_wrongly_inflated_tyres_corner_worse() {
    let fresh = turn_grip(|_| {});
    let worn = turn_grip(|v| for y in v.tyres.iter_mut() { y.wear = 0.9; });
    let cold = turn_grip(|v| for y in v.tyres.iter_mut() { y.temp = 10.0; });
    let flat = turn_grip(|v| v.tyre_bar = 0.8);
    let hard = turn_grip(|v| v.tyre_bar = 1.25);
    assert!(worn < fresh * 0.92, "worn {worn} against fresh {fresh}");
    assert!(cold < fresh * 0.92, "cold {cold} against warm {fresh}");
    assert!(flat < fresh * 0.99 && hard < fresh * 0.99, "the default pressure is the best: {flat} {fresh} {hard}");
}

#[test] fn the_throttle_does_not_spin_the_wheels_of_a_car_in_the_air() {
    let (mut v, t) = flat_car(6.0);                       // dropped from six metres with the gas floored
    let mut worst = 0.0f32;
    for _ in 0..30 { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, DT); worst = worst.max(v.w.iter().cloned().fold(0.0, f32::max)); }
    assert!(v.contacts == 0 && worst < 5.0, "airborne: wheels turn at {worst} rad/s with nothing to push against");
}

/// A steady turn at 25 m/s (the speed held) for a car with the given setup: (yaw rate, the body's roll as the sine of the angle, speed times yaw rate).
fn steady_turn(setup: crate::game::chassis::Chassis) -> (f32, f32, f32) {
    let t = Terrain::flat();
    let mut v = Vehicle::new(VehicleSpec::by_id("vulpes").unwrap(), v3(0., 1.0, 0.), 0.0);
    v.chassis = setup;
    for _ in 0..120 { v.step(&t, &[], &Input::default(), DT); }
    v.vel = v.forward() * 25.0;
    let (mut yaw, mut roll, mut lat) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..300 {
        let s = v3(v.vel.x, 0.0, v.vel.z).len(); if s > 1.0 { let k = 25.0 / s; v.vel = v3(v.vel.x * k, v.vel.y, v.vel.z * k); }
        v.step(&t, &[], &Input { steer: 0.35, ..Default::default() }, DT);
        if i > 200 { yaw = yaw.max(v.omega.y.abs()); roll = roll.max(v.right().y.abs()); lat = lat.max(v.vel.len() * v.omega.y.abs()); }
    }
    (yaw, roll, lat)
}
#[test] fn stiffer_anti_roll_bars_roll_the_body_less() {
    use crate::game::chassis::BASE;
    let soft = steady_turn(crate::game::chassis::Chassis { arb_front: 0.0, arb_rear: 0.0, ..BASE });
    let stiff = steady_turn(crate::game::chassis::Chassis { arb_front: 3.0 * BASE.arb_front, arb_rear: 3.0 * BASE.arb_rear, ..BASE });
    assert!(stiff.1 < soft.1 * 0.9, "the roll with the bars {} against without {}", stiff.1, soft.1);
}
#[test] fn the_front_bar_understeers_and_the_rear_bar_oversteers() {
    use crate::game::chassis::BASE;
    let front = steady_turn(crate::game::chassis::Chassis { arb_front: 3.0 * BASE.arb_front, arb_rear: 0.0, ..BASE });
    let rear = steady_turn(crate::game::chassis::Chassis { arb_front: 0.0, arb_rear: 3.0 * BASE.arb_rear, ..BASE });
    assert!(front.0 < rear.0 * 0.99, "yaw rate with the stiff front bar {} against the stiff rear bar {}", front.0, rear.0);
}
#[test] fn camber_has_an_optimum_and_the_default_is_near_it() {
    use crate::game::chassis::{Chassis, BASE};
    let at = |c: f32| steady_turn(Chassis { camber_front: c, camber_rear: c, ..BASE }).2;
    let (best, flat, extreme) = (at(-1.5), at(2.0), at(-7.0));
    assert!(best > flat * 1.005 && best > extreme * 1.005, "camber -1.5 {best}, +2 {flat}, -7 {extreme}");
}
#[test] fn toe_out_costs_tyre_work_on_a_straight() {
    use crate::game::chassis::{Chassis, BASE};
    let heat = |toe: f32| {
        let (mut v, t) = flat_car(1.1); v.chassis = Chassis { toe_front: toe, toe_rear: toe, ..BASE };
        run(&mut v, &t, Input::default(), 1.0);
        for y in v.tyres.iter_mut() { y.temp = 40.0; y.wear = 0.0; }
        for _ in 0..60 * 8 { let f = v.forward() * 25.0; v.vel = v3(f.x, v.vel.y, f.z); v.step(&t, &[], &Input::default(), DT); }
        (v.tyres.iter().map(|y| y.temp).sum::<f32>() / 4.0, v.tyres.iter().map(|y| y.wear).sum::<f32>())
    };
    let (straight, out) = (heat(0.0), heat(1.5));
    assert!(out.0 > straight.0 + 0.3 && out.1 > straight.1 * 1.5, "straight {straight:?}, toe-out {out:?}");
}
#[test] #[ignore] fn print_chassis_effects() {
    use crate::game::chassis::{Chassis, BASE};
    let row = |l: &str, c: Chassis| { let (y, r, a) = steady_turn(c); eprintln!("CH {l:28} yaw {y:.3} roll {r:.4} v*yaw {a:.2}"); };
    row("as built", BASE);
    row("no bars", Chassis { arb_front: 0.0, arb_rear: 0.0, ..BASE });
    row("bars x3", Chassis { arb_front: 3.0 * BASE.arb_front, arb_rear: 3.0 * BASE.arb_rear, ..BASE });
    row("front bar x3, rear 0", Chassis { arb_front: 3.0 * BASE.arb_front, arb_rear: 0.0, ..BASE });
    row("front bar 0, rear x3", Chassis { arb_front: 0.0, arb_rear: 3.0 * BASE.arb_rear, ..BASE });
    for c in [2.0, 0.0, -1.5, -3.0, -6.0] { row(&format!("camber {c}"), Chassis { camber_front: c, camber_rear: c, ..BASE }); }
    row("toe-out 1.0 both", Chassis { toe_front: 1.0, toe_rear: 1.0, ..BASE });
    row("toe-in 1.0 both", Chassis { toe_front: -1.0, toe_rear: -1.0, ..BASE });
}
