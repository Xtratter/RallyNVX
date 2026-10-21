//! One wheel on the ground: its spin, the slip curve of the tyre's longitudinal grip, the brake (with ABS) and the drive torque.
//! The tyre is stiff (a small slip already gives a large force), so the wheel's equation is stepped semi-implicitly: the tyre force is linearised about the current slip
//! and its slope (kept positive, so the step is always damped) joins the wheel's inertia.
use super::surface::Surface;

/// Below this ground speed (m/s) the slip ratio's denominator stays at it, so the ratio is finite at a standstill.
const V0: f32 = 1.0;
/// A wheel's inertia with its tyre, rim, disc and the half-shaft's share, kg m2.
pub const INERTIA: f32 = 1.3;
/// The share of the brake force on the front axle.
pub const BRAKE_BIAS: f32 = 0.62;

/// The slip ratio at the peak of the grip and the share of the peak that is kept when the tyre slides, per surface: tarmac peaks early and holds well, loose ground peaks later.
pub fn slip_params(s: Surface) -> (f32, f32) {
    match s {
        Surface::Asphalt => (0.07, 0.85), Surface::Gravel => (0.12, 0.88), Surface::Dirt => (0.13, 0.85), Surface::Grass => (0.14, 0.80),
        Surface::Sand => (0.16, 0.80), Surface::Rock => (0.10, 0.85), Surface::Water => (0.10, 0.60),
    }
}

/// The force in shares of the peak (odd in the slip): up to the peak a parabola (slope 2/kp at zero), then it falls to `slide`.
pub fn curve(k: f32, kp: f32, slide: f32) -> f32 {
    let a = (k.abs() / kp.max(1e-3)).min(30.0);
    let g = if a <= 1.0 { a * (2.0 - a) } else { slide + (1.0 - slide) * (-(a - 1.0) * 1.2).exp() };
    if k < 0.0 { -g } else { g }
}

/// The magnitude of the curve's slope per unit of slip, never below 0.4/kp (so the implicit step stays damped past the peak).
pub fn slope(k: f32, kp: f32, slide: f32) -> f32 {
    let a = (k.abs() / kp.max(1e-3)).min(30.0);
    let d = if a <= 1.0 { 2.0 * (1.0 - a) } else { 1.2 * (1.0 - slide) * (-(a - 1.0) * 1.2).exp() };
    d.max(0.4) / kp.max(1e-3)
}

/// One wheel's step: its spin `w` (rad/s, positive rolls forward), the ground speed of the contact along the wheel's heading `vf`, the grip it has (`fmax`, N; 0 in the air),
/// the drive torque from the driveline (positive pushes the car forward), the brake torque available (>= 0), ABS, the step `h` and the inertia.
#[derive(Clone, Copy, Debug)]
pub struct In { pub w: f32, pub vf: f32, pub radius: f32, pub fmax: f32, pub kp: f32, pub slide: f32, pub drive: f32, pub brake: f32, pub abs: bool, pub h: f32, pub inertia: f32 }

/// The new spin, the force on the car along the heading (N), the slip ratio and the brake torque that really acted.
#[derive(Clone, Copy, Debug)]
pub struct Out { pub w: f32, pub fx: f32, pub slip: f32, pub brake_used: f32 }

pub fn step(i: &In) -> Out {
    let r = i.radius.max(0.05);
    let fmax = i.fmax.max(0.0);
    let den = i.vf.abs().max(V0);
    let k0 = (i.w * r - i.vf) / den;
    let f0 = fmax * curve(k0, i.kp, i.slide);
    let stiff = r * r * fmax * slope(k0, i.kp, i.slide) / den;
    let denom = i.inertia + i.h * stiff;
    let tq = i.drive - r * f0;                         // the torque on the wheel apart from the brake
    let w_free = i.w + i.h * tq / denom;
    let (mut w, mut used) = (w_free, 0.0);
    if i.brake > 0.0 {
        // the brake is dry friction inside the same implicit step (or the tyre would damp it away): it opposes the spin with up to `brake`, never turns the wheel round,
        // and holds a wheel that stands still if the other torques are below it
        let still = i.w.abs() < 1e-6;
        let (w_b, locked) = if still && tq.abs() <= i.brake { (0.0, true) } else {
            let dir = if still { tq.signum() } else { i.w.signum() };
            let wb = i.w + i.h * (tq - dir * i.brake) / denom;
            if wb * dir < 0.0 { (0.0, true) } else { (wb, false) }
        };
        w = w_b;
        // ABS will not let the wheel fall behind the ground beyond the band that holds the peak of the grip
        if i.abs && fmax > 0.0 {
            let band = 1.0 * i.kp * den;
            if i.vf > 0.0 { w = w.max(((i.vf - band) / r).max(0.0).min(w_free)); }
            else if i.vf < 0.0 { w = w.min(((i.vf + band) / r).min(0.0).max(w_free)); }
        }
        let span = w_free - w_b;
        used = if span.abs() > 1e-9 { i.brake * ((w_free - w) / span).clamp(0.0, 1.0) } else if locked { i.brake.min(tq.abs()) } else { 0.0 };
    }
    let k1 = (w * r - i.vf) / den;
    Out { w, fx: fmax * curve(k1, i.kp, i.slide), slip: k1, brake_used: used }
}

#[cfg(test)]
#[path = "wheel_tests.rs"]
mod tests;
