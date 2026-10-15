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

#[cfg(test)]
#[path = "dust_ref_tests.rs"]
mod tests;
