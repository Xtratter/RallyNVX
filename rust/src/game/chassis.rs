//! The suspension setup: the anti-roll bars, the camber and the toe of the two axles. The bar twists as the body rolls and pushes the compressed wheel up and the other one down, which
//! moves more of the cornering load onto the axle with the stiffer bar; a tyre gains less grip than it gains load, so that axle loses grip: the bars shift the car's balance. The camber is
//! the lean of the wheel: the tyre grips best a little leaned in, and the body's roll leans the outer wheel the other way. The toe turns the two wheels of an axle towards each other
//! (toe-in) or away (toe-out): it costs rolling drag and heat, a toe-out front turns in sharper, a toe-in rear steadies the tail.

/// The setup: the bars' stiffness (N m/rad), the static camber (degrees, negative leans the top in) and the toe (degrees of the axle, positive is toe-out) of each axle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chassis { pub arb_front: f32, pub arb_rear: f32, pub camber_front: f32, pub camber_rear: f32, pub toe_front: f32, pub toe_rear: f32 }

/// The car as built.
pub const BASE: Chassis = Chassis { arb_front: 20000.0, arb_rear: 12000.0, camber_front: -1.5, camber_rear: -1.0, toe_front: 0.0, toe_rear: -0.1 };

impl Default for Chassis { fn default() -> Chassis { BASE } }

impl Chassis {
    /// From the six multipliers of the car menu (bar front, bar rear, camber front, camber rear, toe front, toe rear; 1 = as built): the bars and the camber scale, the toe moves by 0.4 degrees
    /// per unit (more than 1 is more toe-out). Garbage reads as 1; the range is 0 to 2.
    pub fn from_tuning(m: &[f32; 6]) -> Chassis {
        let k = |i: usize| if m[i].is_finite() { m[i].clamp(0.0, 2.0) } else { 1.0 };
        Chassis { arb_front: BASE.arb_front * k(0), arb_rear: BASE.arb_rear * k(1), camber_front: BASE.camber_front * k(2), camber_rear: BASE.camber_rear * k(3),
            toe_front: BASE.toe_front + (k(4) - 1.0) * 0.4, toe_rear: BASE.toe_rear + (k(5) - 1.0) * 0.4 }
    }
}

/// The force (N) the bar puts on the wheel whose spring is compressed by `ca` (m) when the other wheel of the axle is compressed by `cb`: up on the more compressed one (and the same
/// down on the other); `k` the bar's stiffness in N m/rad, `track` the axle's width in m.
pub fn bar_force(k: f32, track: f32, ca: f32, cb: f32) -> f32 {
    if !(k.is_finite() && track.is_finite() && ca.is_finite() && cb.is_finite()) || track < 0.3 { return 0.0; }
    k / (track * track) * (ca - cb)
}

/// How much of the load a tyre turns into grip: a tyre with more than its static load `fs0` gives less grip per newton, one with less gives more (0.6 to 1.15).
pub fn load_factor(fs: f32, fs0: f32) -> f32 {
    if !(fs.is_finite() && fs0.is_finite()) || fs0 < 1.0 { return 1.0; }
    (1.0 - 0.12 * (fs / fs0 - 1.0)).clamp(0.6, 1.15)
}

/// The grip of a tyre at its camber: the effective camber is the static one plus 0.6 of the body's roll (degrees, towards the left) with the sign of the side (+1 for a left wheel, -1 for a
/// right one); the best is -1 degree, 0.8 at the worst.
pub fn camber_grip(static_deg: f32, roll_left_deg: f32, side: f32) -> f32 {
    if !(static_deg.is_finite() && roll_left_deg.is_finite()) { return 1.0; }
    let eff = static_deg + 0.6 * roll_left_deg * side;
    (1.0 - 0.006 * (eff + 1.0).powi(2)).clamp(0.8, 1.0)
}

/// The angle (radians) a wheel's toe adds to its steering: half of the axle's toe, the left wheel turned one way and the right wheel the other (toe-out turns the left one to the left).
pub fn wheel_toe(toe_deg: f32, left: bool) -> f32 {
    if !toe_deg.is_finite() { return 0.0; }
    let a = (toe_deg.clamp(-3.0, 3.0) * 0.5).to_radians();
    if left { -a } else { a }
}

#[cfg(test)]
#[path = "chassis_tests.rs"]
mod tests;
