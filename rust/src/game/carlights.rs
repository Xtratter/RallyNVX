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

#[cfg(test)]
#[path = "carlights_tests.rs"]
mod tests;
