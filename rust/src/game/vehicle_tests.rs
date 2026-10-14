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
