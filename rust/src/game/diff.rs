//! A differential: how a torque is shared between two shafts (the two wheels of an axle, or the front and the rear axle) and how the speed of its input follows the two outputs.
//! An open one gives each side its share whatever the speeds; a limited-slip one (a clutch pack pressed by ramps) moves torque from the faster side to the slower with a lock that is a preload
//! plus a share of the input torque (a different share on the power and on the coast); a viscous one moves it in proportion to the speed difference.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Open, Lsd, Viscous }

/// `split` is the share of the input torque that side A gets when nothing locks (0.5 for an axle; the front share for a centre diff). `preload` (N m) locks with no load,
/// `power` and `coast` are the shares of the input torque that lock on the power and on the coast (0..1), `visc` is N m per rad/s of the viscous coupling, `cap` the most the lock can move (N m).
#[derive(Clone, Copy, Debug)]
pub struct Diff { pub kind: Kind, pub split: f32, pub preload: f32, pub power: f32, pub coast: f32, pub visc: f32, pub cap: f32 }

/// How stiffly a limited-slip lock pulls the two sides together (N m per rad/s) before it saturates: it keeps the lock from chattering at a small speed difference.
const LOCK_K: f32 = 60.0;

/// The torque taken from side A and given to side B; positive when A is the faster side (the lock slows the faster side and drives the slower).
pub fn moved(d: &Diff, t_in: f32, dw: f32) -> f32 {
    let m = match d.kind {
        Kind::Open => return 0.0,
        Kind::Lsd => {
            let mag = (d.preload + t_in.abs() * if t_in >= 0.0 { d.power } else { d.coast }).clamp(0.0, d.cap.max(0.0));
            (LOCK_K * dw).clamp(-mag, mag)
        }
        Kind::Viscous => (d.visc * dw).clamp(-d.cap.abs(), d.cap.abs()),
    };
    if m.is_finite() { m } else { 0.0 }
}

/// The torques of side A and side B; they always add up to `t_in`.
pub fn split(d: &Diff, t_in: f32, w_a: f32, w_b: f32) -> (f32, f32) {
    let m = moved(d, t_in, w_a - w_b);
    (t_in * d.split - m, t_in * (1.0 - d.split) + m)
}

/// The speed of the input shaft: the share-weighted mean of the two outputs (the power in equals the power out for an open diff).
pub fn input_speed(d: &Diff, w_a: f32, w_b: f32) -> f32 { d.split * w_a + (1.0 - d.split) * w_b }

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
