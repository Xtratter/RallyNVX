//! One wheel on the ground: its spin, the slip curve of the tyre's longitudinal grip, the brake (with ABS) and the drive torque.
//! The tyre is stiff (a small slip already gives a large force), so the wheel's equation is stepped semi-implicitly: the tyre force is linearised about the current slip
//! and its slope (kept positive, so the step is always damped) joins the wheel's inertia.
use super::surface::Surface;

/// Below this ground speed (m/s) the slip ratio's denominator stays at it, so the ratio is finite at a standstill.
const V0: f32 = 1.0;
/// A wheel's inertia with its tyre, rim, disc and the half-shaft's share, kg m2.
pub const INERTIA: f32 = 1.3;
/// The share of the brake force on the front axle.
pub const BRAKE_BIAS: f32 = 0.55;

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
    let mut w = i.w + i.h * (i.drive - r * f0) / (i.inertia + i.h * stiff);
    // the brake only ever slows the wheel towards a stop; ABS will not take the slip beyond the band that holds the peak of the grip
    let mut used = 0.0;
    if i.brake > 0.0 {
        let dw = i.h * i.brake / i.inertia;
        let band = 1.2 * i.kp * den;
        if w > 0.0 {
            let floor = if i.abs && fmax > 0.0 && i.vf > 0.0 { ((i.vf - band) / r).max(0.0).min(w) } else { 0.0 };
            let nw = (w - dw).max(floor); used = (w - nw) * i.inertia / i.h; w = nw;
        } else if w < 0.0 {
            let ceil = if i.abs && fmax > 0.0 && i.vf < 0.0 { ((i.vf + band) / r).min(0.0).max(w) } else { 0.0 };
            let nw = (w + dw).min(ceil); used = (nw - w) * i.inertia / i.h; w = nw;
        }
    }
    let k1 = (w * r - i.vf) / den;
    Out { w, fx: fmax * curve(k1, i.kp, i.slide), slip: k1, brake_used: used }
}

#[cfg(test)]
#[path = "wheel_tests.rs"]
mod tests;
