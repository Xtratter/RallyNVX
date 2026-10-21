//! The electronic stability control: it compares the yaw rate the car has with the one its steering asks for (the bicycle model: `v tan(steer) / wheelbase`, limited by what the tyres can
//! hold) and brakes single wheels against the difference, as real systems do: against an oversteer (the tail steps out) the outer front wheel, against an understeer (the nose pushes wide) the
//! inner rear wheel (the wheels are 0 front right, 1 front left, 2 rear right, 3 rear left: the right side is -X in the car's frame); in both it cuts the engine. It does nothing below 8 m/s. Yaw rate is positive to the left, the steering positive to the right.

/// The yaw rate error (rad/s) the control lives with, the brake per rad/s of the error beyond it (N m) and the largest cut of the engine.
const THRESH: f32 = 0.12;
const GAIN: f32 = 2500.0;
const MAX_CUT: f32 = 0.7;

/// What the control asks for: the extra brake torque of each wheel (FL, FR, RL, RR; N m), the share of the engine's torque to cut (0..1), whether it is acting.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Esc { pub brake: [f32; 4], pub cut: f32, pub active: bool }

pub fn control(yaw: f32, steer: f32, speed: f32, wheelbase: f32, mu: f32, brake_max: f32) -> Esc {
    let ok = [yaw, steer, speed, wheelbase, mu, brake_max].iter().all(|v| v.is_finite()) && wheelbase > 0.5 && mu > 0.0 && brake_max > 0.0;
    if !ok || speed < 8.0 { return Esc::default(); }
    let limit = 0.9 * mu * 9.81 / speed;
    let target = (-speed * steer.tan() / wheelbase).clamp(-limit, limit);
    let over = yaw.signum() * (yaw - target);                      // the yaw beyond what is asked, along its own direction
    let mut e = Esc::default();
    if over > THRESH {
        let m = over - THRESH;
        // the outer front wheel of the way the car is turning: a left turn (positive yaw) has the right wheel outside (the wheels 0 and 2 are the right ones, 1 and 3 the left ones)
        let outer = if yaw > 0.0 { 0 } else { 1 };
        e.brake[outer] = (GAIN * m).min(brake_max); e.cut = (m / 0.5).clamp(0.0, 1.0) * MAX_CUT; e.active = true;
    } else if target.abs() > 0.05 {
        let under = target.signum() * (target - yaw);              // the yaw short of what is asked
        if under > THRESH {
            let m = under - THRESH;
            let inner = if target > 0.0 { 3 } else { 2 };          // the inner rear wheel of the asked turn (a left turn: the left rear one)
            e.brake[inner] = (GAIN * m).min(brake_max); e.cut = (m / 0.5).clamp(0.0, 1.0) * MAX_CUT; e.active = true;
        }
    }
    e
}

#[cfg(test)]
#[path = "esc_tests.rs"]
mod tests;
