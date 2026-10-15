//! Car lights: which lamps are lit (the night factor and the driver's input) and where they sit on the body.
use super::math::{v3, V3};
use super::vehicle::{CarStyle, Vehicle};
use std::sync::atomic::{AtomicU8, Ordering};

/// The headlights switch on when the night factor passes this.
pub const HEAD_ON_NIGHT: f32 = 0.25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CarLights { pub head: bool, pub brake: bool, pub reverse: bool }

#[derive(Clone, Copy, Debug)]
pub struct Lamps { pub head: [V3; 2], pub tail: [V3; 2], pub rev: [V3; 2], pub head_dir: V3 }

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
        tail: [at(-x, -half_l), at(x, -half_l)],
        rev: [at(-x * 0.55, -half_l), at(x * 0.55, -half_l)],
        head_dir: v.rot.rotate(v3(0.0, -0.05, 1.0).norm()),
    }
}

/// Fallback lamp points (the Peugeot asset, measured once) in case the asset cannot be read: body frame, metres from the ground under the car, x across, z forward.
const RALLY_FALLBACK: crate::raster::rcar::LampPoints = crate::raster::rcar::LampPoints { head: [0.569, 0.634, 1.693], tail: [0.611, 0.747, -1.654], rev: [0.584, 0.818, -1.651] };

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
        tail: [at(p.tail, -1.0), at(p.tail, 1.0)],
        rev: [at(p.rev, -1.0), at(p.rev, 1.0)],
        head_dir: v.rot.rotate(v3(0.0, -0.05, 1.0).norm()),
    }
}

#[cfg(test)]
#[path = "carlights_tests.rs"]
mod tests;
