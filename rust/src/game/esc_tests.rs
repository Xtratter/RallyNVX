use super::*;

const L: f32 = 2.5;
fn none(e: &Esc) -> bool { e.brake.iter().all(|b| *b == 0.0) && e.cut == 0.0 && !e.active }

#[test] fn a_car_that_goes_where_it_is_steered_is_left_alone() {
    // the steady turn: the yaw rate is the one the steering asks for
    let steer = 0.05f32; let speed = 20.0f32;   // 0.4 rad/s, under what the tyres can hold (0.53)
    let target = -speed * steer.tan() / L;
    assert!(none(&control(target, steer, speed, L, 1.2, 1500.0)));
    assert!(none(&control(0.0, 0.0, speed, L, 1.2, 1500.0)), "straight on");
    assert!(none(&control(-1.0, 0.1, 3.0, L, 1.2, 1500.0)), "too slow to matter");
}
#[test] fn oversteer_brakes_the_outer_front_wheel_and_cuts_the_engine() {
    // a right turn (negative yaw), the tail steps out: the yaw is far above what the steering asks for
    let steer = 0.1f32; let speed = 20.0f32; let target = -speed * steer.tan() / L;
    let e = control(target * 2.5, steer, speed, L, 1.2, 1500.0);
    assert!(e.active && e.brake[1] > 100.0 && e.brake[0] == 0.0 && e.brake[2] == 0.0 && e.brake[3] == 0.0 && e.cut > 0.1, "a right turn: the outer (left) front wheel, index 1: {e:?}");
    // a left turn mirrors it: the right front, index 0
    let m = control(-target * 2.5, -steer, speed, L, 1.2, 1500.0);
    assert!(m.brake[0] > 100.0 && m.brake[1] == 0.0, "{m:?}");
}
#[test] fn understeer_brakes_the_inner_rear_wheel() {
    let steer = 0.12f32; let speed = 20.0f32; let target = -speed * steer.tan() / L;
    let e = control(target * 0.2, steer, speed, L, 1.2, 1500.0);   // the car barely turns: the front is pushing wide
    assert!(e.active && e.brake[2] > 100.0 && e.brake[0] == 0.0 && e.brake[1] == 0.0 && e.brake[3] == 0.0, "a right turn: the inner (right) rear wheel, index 2: {e:?}");
    let m = control(-target * 0.2, -steer, speed, L, 1.2, 1500.0);
    assert!(m.brake[3] > 100.0 && m.brake[2] == 0.0, "{m:?}");
}
#[test] fn a_spin_on_a_straight_is_caught_and_the_brake_is_capped() {
    let e = control(1.5, 0.0, 25.0, L, 1.2, 1500.0);                  // yawing left with the wheel straight
    assert!(e.active && e.brake[0] > 0.0 && e.brake[0] <= 1500.0, "{e:?}");
    let big = control(9.0, 0.3, 40.0, L, 1.2, 800.0);
    assert!(big.brake.iter().all(|b| *b <= 800.0) && big.cut <= 0.7 + 1e-6, "{big:?}");
}
#[test] fn the_target_is_limited_by_the_grip_and_garbage_does_nothing() {
    // a huge steering angle at speed asks for more yaw than the tyres can give: the target is capped, so the car that corners at the limit is not called an understeerer
    let speed = 25.0f32; let limit = 0.9 * 1.2 * 9.81 / speed;
    let e = control(-limit, 0.5, speed, L, 1.2, 1500.0);
    assert!(none(&e), "{e:?}");
    for &(y, s, v, l, m) in &[(f32::NAN, 0.1, 20.0, 2.5, 1.2), (1.0, f32::NAN, 20.0, 2.5, 1.2), (1.0, 0.1, f32::INFINITY, 2.5, 1.2), (1.0, 0.1, 20.0, 0.0, 1.2), (1.0, 0.1, 20.0, 2.5, f32::NAN)] {
        assert!(none(&control(y, s, v, l, m, 1500.0)), "{y} {s} {v} {l} {m}");
    }
}
