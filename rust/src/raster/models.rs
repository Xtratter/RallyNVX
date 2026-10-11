//! Pure model builders for the air part of the scene: aircraft, rope, cargo, rings, pads and zones as primitive parts.
use crate::game::flight::{Aircraft, AircraftKind};
use crate::game::math::{v3, Q, V3};

/// One primitive: `mesh` indexes the shared meshes (0 cube, 1 cylinder, 2 cone, 3 sphere).
#[derive(Clone, Copy, Debug)]
pub struct Part { pub mesh: usize, pub rot: Q, pub pos: V3, pub scale: V3, pub color: [f32; 3], pub glow: bool }

const CUBE: usize = 0;
const CYL: usize = 1;
const SPHERE: usize = 3;

fn part(mesh: usize, rot: Q, pos: V3, scale: V3, color: [f32; 3], glow: bool) -> Part { Part { mesh, rot, pos, scale, color, glow } }

/// Rotation taking +Y to `dir` (a unit vector).
pub fn align_y(dir: V3) -> Q {
    let up = v3(0., 1., 0.);
    if dir.y > 0.9999 { return Q::IDENT; }
    if dir.y < -0.9999 { return Q::axis_angle(v3(1., 0., 0.), std::f32::consts::PI); }
    Q::axis_angle(up.cross(dir), dir.y.clamp(-1.0, 1.0).acos())
}

pub fn aircraft_parts(a: &Aircraft, t: f32) -> Vec<Part> {
    let rot = a.rotation();
    let at = |p: V3| a.pos + rot.rotate(p);
    let spin = |extra: f32| rot.mul(Q::axis_angle(v3(0., 1., 0.), t * 25.0 + extra));
    let mut out = Vec::new();
    match a.kind {
        AircraftKind::Heli => {
            out.push(part(SPHERE, rot, at(V3::ZERO), v3(2.0, 1.8, 4.2), [0.95, 0.45, 0.10], false));
            out.push(part(CUBE, rot, at(v3(0., 0.3, -3.6)), v3(0.35, 0.35, 4.0), [0.55, 0.55, 0.58], false));
            out.push(part(CUBE, rot, at(v3(0., 0.9, -5.4)), v3(0.1, 1.2, 0.8), [0.55, 0.55, 0.58], false));
            for sx in [-0.9, 0.9] { out.push(part(CUBE, rot, at(v3(sx, -1.2, 0.)), v3(0.12, 0.12, 3.4), [0.2, 0.2, 0.22], false)); }
            out.push(part(CYL, rot, at(v3(0., 1.2, 0.)), v3(0.15, 0.6, 0.15), [0.2, 0.2, 0.22], false));
            for extra in [0.0, std::f32::consts::FRAC_PI_2] { out.push(part(CUBE, spin(extra), at(v3(0., 1.55, 0.)), v3(11.0, 0.05, 0.35), [0.1, 0.1, 0.12], false)); }
            out.push(part(CUBE, rot.mul(Q::axis_angle(v3(1., 0., 0.), t * 30.0)), at(v3(0.15, 0.9, -5.6)), v3(0.05, 1.4, 0.15), [0.1, 0.1, 0.12], false));
        }
        AircraftKind::Drone => {
            out.push(part(CUBE, rot, at(V3::ZERO), v3(0.6, 0.18, 0.6), [0.15, 0.15, 0.18], false));
            for (i, (sx, sz)) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)].into_iter().enumerate() {
                let ang = if sx * sz > 0.0 { 0.785 } else { -0.785 };
                let diag = rot.mul(Q::axis_angle(v3(0., 1., 0.), ang));
                out.push(part(CUBE, diag, at(v3(sx * 0.32, 0., sz * 0.32)), v3(0.9, 0.05, 0.05), [0.3, 0.3, 0.33], false));
                let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
                out.push(part(CYL, spin(dir * 0.3), at(v3(sx * 0.62, 0.08, sz * 0.62)), v3(0.7, 0.02, 0.7), [0.9, 0.9, 0.95], false));
            }
            out.push(part(SPHERE, rot, at(v3(0., -0.12, 0.2)), v3(0.18, 0.18, 0.18), [0.05, 0.05, 0.08], false));
        }
    }
    out
}

pub fn rope_part(from: V3, to: V3) -> Part {
    let d = to - from;
    part(CYL, align_y(d.norm()), (from + to) * 0.5, v3(0.06, d.len(), 0.06), [0.15, 0.12, 0.1], false)
}

pub fn cargo_part(pos: V3) -> Part { part(CUBE, Q::IDENT, pos, v3(1.2, 1.2, 1.2), [0.55, 0.38, 0.2], false) }

pub fn ring_parts(centre: V3, normal: V3, radius: f32, next: bool) -> Vec<Part> {
    let n = normal.norm();
    let mut u = n.cross(v3(0., 1., 0.));
    if u.len() < 1e-3 { u = n.cross(v3(1., 0., 0.)); }
    let u = u.norm(); let v = n.cross(u);
    let color = if next { [1.0, 0.35, 0.85] } else { [0.25, 0.75, 1.0] };
    (0..16).map(|i| {
        let a = i as f32 / 16.0 * std::f32::consts::TAU;
        part(CUBE, Q::IDENT, centre + (u * a.cos() + v * a.sin()) * radius, v3(1.6, 1.6, 1.6), color, true)
    }).collect()
}

pub fn pad_part(pos: V3, radius: f32) -> Part { part(CYL, Q::IDENT, pos + v3(0., 0.15, 0.), v3(radius * 2.0, 0.3, radius * 2.0), [0.3, 1.0, 0.4], true) }

pub fn zone_parts(pos: V3, ground_y: f32, radius: f32) -> Vec<Part> {
    let mut out: Vec<Part> = (0..24).map(|i| {
        let a = i as f32 / 24.0 * std::f32::consts::TAU;
        part(CUBE, Q::IDENT, v3(pos.x + radius * a.cos(), ground_y + 0.5, pos.z + radius * a.sin()), v3(2.0, 2.0, 2.0), [0.2, 0.9, 0.9], true)
    }).collect();
    let h = (pos.y - ground_y).max(1.0);
    out.push(part(CYL, Q::IDENT, v3(pos.x, ground_y + h * 0.5, pos.z), v3(1.0, h, 1.0), [0.2, 0.9, 0.9], true));
    out
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
