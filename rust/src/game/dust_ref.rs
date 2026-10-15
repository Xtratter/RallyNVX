//! One step of a dust particle, as the GPU runs it (`shaders/dust_gpu.wgsl`): the host-tested reference. `game/dust.rs` (the CPU particles) uses the
//! same functions, so both paths move the dust the same way.
use super::dust::{Particle, Phys};
use super::math::{v3, V3};

/// A dust particle: `life` in seconds, `size0` the size when it is born, `coarse` heavy grit (else fine dust), `seed` a stable random number.
#[derive(Clone, Copy, Debug)]
pub struct GP { pub pos: V3, pub vel: V3, pub age: f32, pub life: f32, pub size0: f32, pub color: [f32; 3], pub coarse: bool, pub seed: f32 }

/// Moves `p` for `dt` seconds in the air velocity `air` (Stokes drag: the exact relaxation towards it with the response time of its class, the settling
/// speed of its class on top) and meets the ground (`ground(x, z)`): grit is deposited, fine dust slides and slows. False when the particle is gone
/// (too old or deposited). A non-finite `air` counts as still air.
pub fn step(p: &mut GP, air: V3, ground: impl Fn(f32, f32) -> f32, ph: &Phys, dt: f32) -> bool {
    p.age += dt;
    if p.age >= p.life { return false; }
    let u = if air.is_finite() { air } else { V3::ZERO };
    let (tau, settle) = if p.coarse { (ph.tau_coarse, ph.settle_coarse) } else { (ph.tau_fine, ph.settle_fine) };
    let target = v3(u.x, u.y - settle, u.z);
    let k = 1.0 - (-dt / tau).exp();   // stable at any step
    p.vel = p.vel + (target - p.vel) * k;
    p.pos = p.pos + p.vel * dt;
    let g = ground(p.pos.x, p.pos.z) + 0.04;
    if p.pos.y < g {
        if p.coarse { return false; }
        p.pos.y = g;
        if p.vel.y < 0.0 { p.vel.y = 0.0; }
        let s = 1.0 - (2.5 * dt).min(1.0);
        p.vel.x *= s; p.vel.z *= s;
    }
    true
}

/// What the renderer needs of a particle: it grows as it spreads, fades in over the first 8 % of its life and out at the end.
pub fn render(p: &GP) -> Particle {
    let x = (p.age / p.life).clamp(0.0, 1.0);
    let fade_in = (x / 0.08).min(1.0);
    // turbulent diffusion spreads a puff like the square root of its age
    let size = p.size0 * (1.0 + if p.coarse { 0.8 } else { 1.7 } * p.age.sqrt() + 1.2 * x);
    Particle { pos: p.pos, size, color: p.color, alpha: (if p.coarse { 0.6 } else { 0.55 }) * fade_in * (1.0 - x).powf(1.4), seed: p.seed, age: x, kind: if p.coarse { 1.0 } else { 0.0 } }
}

/// How far (m) from the faces of the airflow box the grid's air blends into the outside air.
pub const EDGE: f32 = 1.5;

fn smooth(x: f32) -> f32 { let t = x.clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

/// The car's forward and right on the ground plane for a yaw (the car's forward is (sin yaw, 0, cos yaw), as `Vehicle::forward`).
fn axes(yaw: f32) -> (V3, V3) { let f = v3(yaw.sin(), 0.0, yaw.cos()); (f, v3(-f.z, 0.0, f.x)) }

/// A world position in the airflow box's own coordinates, metres from its corner: x across the car (the box's middle on the car's centre line),
/// y up from the road (`ground_y`, the terrain under the car), z along the car (the box's middle at the car's centre). `size` is the box in metres.
pub fn to_box(p: V3, car_pos: V3, yaw: f32, ground_y: f32, size: [f32; 3]) -> [f32; 3] {
    let (f, r) = axes(yaw);
    let q = p - car_pos;
    [q.dot(r) + size[0] * 0.5, p.y - ground_y, q.dot(f) + size[2] * 0.5]
}

/// A vector of the box's frame (x across, y up, z along the car) as a world vector.
pub fn from_box_dir(d: V3, yaw: f32) -> V3 { let (f, r) = axes(yaw); r * d.x + v3(0.0, d.y, 0.0) + f * d.z }

/// 1 inside the box, 0 outside, a smooth blend over `EDGE` metres from its faces (the road is no face: the box has no floor to leave through).
pub fn box_weight(c: [f32; 3], size: [f32; 3]) -> f32 {
    if c[1] < 0.0 { return 0.0; }
    let d = c[0].min(size[0] - c[0]).min(size[1] - c[1]).min(c[2]).min(size[2] - c[2]);
    smooth(d / EDGE)
}

/// The world air velocity at `p`: the box's air (the grid velocity is relative to the car, in the car's frame) inside the box, `outside` beyond it.
pub fn air_in_box(p: V3, car_pos: V3, car_vel: V3, yaw: f32, ground_y: f32, grid_v: V3, outside: V3, size: [f32; 3]) -> V3 {
    let w = box_weight(to_box(p, car_pos, yaw, ground_y, size), size);
    let inside = car_vel + from_box_dir(grid_v, yaw);
    outside + (inside - outside) * w
}

/// The small-scale turbulence the grid cannot carry: grows with the car's speed.
pub fn turbulence(p: V3, t: f32, speed: f32, ph: &Phys) -> V3 {
    let amp = ph.turbulence * (0.35 + 0.05 * speed);
    let n = v3((p.y * 0.9 + t * 1.7 + p.z * 0.35).sin() + 0.5 * (p.z * 1.9 - t * 2.3).sin(),
        0.6 * (p.x * 0.8 + t * 1.1 + p.z * 0.5).sin(),
        (p.y * 1.1 - t * 1.5 + p.x * 0.4).sin() + 0.5 * (p.x * 2.1 + t * 2.9).sin());
    n * amp
}

#[cfg(test)]
#[path = "dust_ref_tests.rs"]
mod tests;
