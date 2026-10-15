//! Car lights: which lamps are lit (the night factor and the driver's input) and where they sit on the body.
use super::math::{v3, V3};
use super::vehicle::{CarStyle, Vehicle};
use std::sync::atomic::{AtomicU8, Ordering};

/// The headlights switch on when the night factor passes this.
pub const HEAD_ON_NIGHT: f32 = 0.25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CarLights { pub head: bool, pub brake: bool, pub reverse: bool }

#[derive(Clone, Copy, Debug)]
pub struct Lamps { pub head: [V3; 2], pub pos: [V3; 2], pub tail: [V3; 2], pub rev: [V3; 2], pub head_dir: V3 }

/// Debug/test: 0 headlights only, 1 + brake, 2 + reverse; `u8::MAX` = normal behaviour. Set over JNI only.
static DEBUG: AtomicU8 = AtomicU8::new(u8::MAX);
pub fn set_debug(mode: Option<u8>) { DEBUG.store(mode.filter(|m| *m <= 2).unwrap_or(u8::MAX), Ordering::Relaxed); }

pub fn state(v: &Vehicle, night: f32) -> CarLights { state_with(DEBUG.load(Ordering::Relaxed), v, night) }

pub fn state_with(mode: u8, v: &Vehicle, night: f32) -> CarLights {
    match mode {
        0 => CarLights { head: true, brake: false, reverse: false },
        1 => CarLights { head: true, brake: true, reverse: false },
        2 => CarLights { head: true, brake: false, reverse: true },
        _ => CarLights { head: night > HEAD_ON_NIGHT, brake: v.braking, reverse: v.reversing },
    }
}

/// Lamp positions in the world: front and rear at +-(length/2 - 0.08) from the centre, x at +-0.34 of the width, at a
/// lamp height above the ground that depends on the body style.
pub fn lamps(v: &Vehicle) -> Lamps {
    let s = &v.spec;
    if s.style == CarStyle::Rally { return rally_lamps(v); }
    let half_l = s.size.z * 0.5 - 0.08;
    let x = s.size.x * 0.34;
    let h = match s.style { CarStyle::Suv => 0.95, CarStyle::Rally => 0.62 };
    let y = s.wheels[0].y - s.rest - s.radius + h;
    let at = |lx: f32, lz: f32| v.pos + v.rot.rotate(v3(lx, y, lz));
    Lamps {
        head: [at(-x, half_l), at(x, half_l)],
        pos: [at(-x, half_l), at(x, half_l)],
        tail: [at(-x, -half_l), at(x, -half_l)],
        rev: [at(-x * 0.55, -half_l), at(x * 0.55, -half_l)],
        head_dir: v.rot.rotate(v3(0.0, -0.05, 1.0).norm()),
    }
}

/// Fallback lamp points (the Peugeot asset, measured once) in case the asset cannot be read: body frame, metres from the ground under the car, x across, z forward.
const RALLY_FALLBACK: crate::raster::rcar::LampPoints = crate::raster::rcar::LampPoints { head: [0.53, 0.62, 1.72], pos: [0.68, 0.68, 1.61], tail: [0.611, 0.747, -1.654], rev: [0.584, 0.818, -1.651] };

/// The lamps of a rally-style car: the lamp units found on the car's mesh (`rcar::lamp_points`; the asset is 1.91 x 1.50 x 3.97 m and the points scale with the spec's
/// size like the mesh), placed on the body where it is drawn. The next car model brings its own mesh and its lamps follow it.
fn rally_lamps(v: &Vehicle) -> Lamps {
    let s = &v.spec;
    let p = crate::raster::rally_model::lamp_points().unwrap_or(RALLY_FALLBACK);
    let (rot, origin) = crate::raster::vehicle_models::car_body_pose(v);   // where the body mesh is drawn, so the lamps sit on the mesh
    let k = [s.size.x / 1.91, s.size.y / 1.50, s.size.z / 3.97];
    let at = |p: [f32; 3], side: f32| origin + rot.rotate(v3(side * p[0] * k[0], p[1] * k[1], p[2] * k[2]));
    Lamps {
        head: [at(p.head, -1.0), at(p.head, 1.0)],
        pos: [at(p.pos, -1.0), at(p.pos, 1.0)],
        tail: [at(p.tail, -1.0), at(p.tail, 1.0)],
        rev: [at(p.rev, -1.0), at(p.rev, 1.0)],
        head_dir: v.rot.rotate(v3(0.0, -0.05, 1.0).norm()),
    }
}

/// One headlight's low beam at point `p` (0..1): `fwd` the beam axis (slightly down), `right` across it. Wide sideways (about 35 degrees each way), a sharp upper
/// cut-off at the axis that rises towards the right (right-hand traffic: it lights the verge), a brighter middle, 1/d^2 fall-off with a far limit. Mirrored in `raster.wgsl::beam_one`.
pub fn beam_gain(lamp: V3, fwd: V3, right: V3, p: V3) -> f32 {
    let d = p - lamp;
    let dist = d.len();
    if !(dist > 0.05) { return 0.0; }
    let l = d * (1.0 / dist);
    let (z, x) = (l.dot(fwd), l.dot(right));
    if z <= 0.05 { return 0.0; }
    let y = l.dot(fwd.cross(right));
    let (h, v) = ((x / z).atan(), (y / z).atan());
    let cut = 0.07 * smooth(0.0, 0.25, h);
    let gv = 1.0 - smooth(cut, cut + 0.02, v);
    let gh = 1.0 - smooth(0.35, 0.65, h.abs());
    let hot = 0.6 + 0.4 * (-(h / 0.2) * (h / 0.2)).exp();
    let att = 1.0 / (1.0 + (dist / 18.0) * (dist / 18.0));
    gv * gh * hot * att * (1.0 - dist / 90.0).clamp(0.0, 1.0)
}
fn smooth(a: f32, b: f32, x: f32) -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

/// The beam of both lamps at `p`, the mean of the two.
pub fn beam(l: &Lamps, right: V3, p: V3) -> f32 { 0.5 * (beam_gain(l.head[0], l.head_dir, right, p) + beam_gain(l.head[1], l.head_dir, right, p)) }

/// How much of a lamp's glow shows from the camera: full along the lamp's axis, faint from the side, nothing from behind.
pub fn glow_factor(axis: V3, to_cam: V3) -> f32 { let c = axis.dot(to_cam); if c <= 0.0 { 0.0 } else { c.powf(2.5) } }

/// The glows to draw: for every lit lamp its position, colour and strength (0..1, the viewing angle's `glow_factor`). Front lamps shine along `fwd`, rear lamps along `-fwd`.
pub fn glows(l: &CarLights, lamps: &Lamps, cam: V3, fwd: V3) -> Vec<(V3, [f32; 3], f32)> {
    let mut out = Vec::new();
    let mut add = |p: V3, col: [f32; 3], axis: V3, k: f32| out.push((p, col, k * glow_factor(axis, (cam - p).norm())));
    if l.head {
        for p in lamps.head { add(p, [1.0, 0.95, 0.8], fwd, 1.0); }
        for p in lamps.pos { add(p, [1.0, 0.95, 0.8], fwd, 0.45); }
    }
    if l.brake || l.head { for p in lamps.tail { add(p, [1.0, 0.05, 0.03], -fwd, if l.brake { 1.0 } else { 0.3 }); } }
    if l.reverse { for p in lamps.rev { add(p, [1.0, 1.0, 0.95], -fwd, 0.8); } }
    out
}

#[cfg(test)]
#[path = "carlights_tests.rs"]
mod tests;
