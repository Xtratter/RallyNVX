//! Cargo on a rope under the helicopter: a point mass on a one-sided spring-damper (taut only beyond the rest length).
use super::math::{v3, V3};
use super::terrain::Terrain;

pub const ROPE: f32 = 8.0;

const K: f32 = 60.0;
const C: f32 = 6.0;
const DRAG: f32 = 0.3;
const SUB: usize = 4;

#[derive(Clone, Copy, Debug)]
pub struct Sling { pub pos: V3, pub vel: V3, pub attached: bool }

impl Sling {
    pub fn on_ground(pos: V3) -> Sling { Sling { pos, vel: V3::ZERO, attached: false } }
    pub fn attach(&mut self) { self.attached = true; }
    pub fn relative_speed(&self, attach_vel: V3) -> f32 { (self.vel - attach_vel).len() }
    pub fn rope_taut(&self, attach: V3) -> bool { (attach - self.pos).len() >= ROPE - 0.05 }

    /// Advances the cargo; returns the speed of a real ground impact (> 0.5 m/s into the ground) in this step.
    pub fn step(&mut self, attach: V3, attach_vel: V3, t: &Terrain, dt: f32) -> Option<f32> {
        let h = dt / SUB as f32;
        let mut impact: Option<f32> = None;
        for _ in 0..SUB {
            let mut force = v3(0., -9.81, 0.) - self.vel * DRAG;
            if self.attached {
                let d = attach - self.pos; let dist = d.len();
                if dist > ROPE && dist > 1e-4 {
                    let n = d * (1.0 / dist);
                    let f = (K * (dist - ROPE) + C * (attach_vel - self.vel).dot(n)).max(0.0);
                    force = force + n * f;
                }
            }
            self.vel = self.vel + force * h;
            self.pos = self.pos + self.vel * h;
            let g = t.height(self.pos.x, self.pos.z) + 0.3;
            if self.pos.y < g {
                let n = t.normal(self.pos.x, self.pos.z);
                let into = -self.vel.dot(n);                   // speed into the ground; sliding along it is not an impact
                if into > 0.5 { impact = Some(impact.map_or(into, |p: f32| p.max(into))); }
                self.pos.y = g;
                if into > 0.0 { self.vel = self.vel + n * into; }
                self.vel.x *= 0.8; self.vel.z *= 0.8;
            }
        }
        // hard cap: a teleported attach point must not stretch the rope without bound
        if self.attached {
            let d = attach - self.pos; let dist = d.len();
            if dist > ROPE * 1.5 {
                self.pos = attach - d * (1.0 / dist) * (ROPE * 1.5); self.vel = attach_vel;
                let g = t.height(self.pos.x, self.pos.z) + 0.3;
                if self.pos.y < g { self.pos.y = g; }
            }
        }
        impact
    }
}

#[cfg(test)]
#[path = "sling_tests.rs"]
mod tests;
