//! Tyre dust and spray: a deterministic ring of particles emitted by the wheels (see the dust design).
use super::grip;
use super::math::{v3, V3};
use super::terrain::{hash01, Terrain};
use super::vehicle::Vehicle;

/// What the renderer needs of a particle.
#[derive(Clone, Copy, Debug)]
pub struct Particle { pub pos: V3, pub size: f32, pub color: [f32; 3], pub alpha: f32 }

#[derive(Clone, Copy, Debug)]
struct P { pos: V3, vel: V3, age: f32, life: f32, size0: f32, color: [f32; 3] }

pub struct Dust { seed: u64, rng: u32, parts: Vec<P>, head: usize, cap: usize, level: i32, acc: [f32; 4], wind: V3 }

/// The wind: a direction from the seed that wanders slowly and a speed between 0.6 and 1.0 of the table's `wind_speed`.
pub fn wind_at(seed: u64, time: f32) -> V3 {
    let t = if time.is_finite() { time } else { 0.0 };
    let a = hash01(seed ^ 0x71D, 1, 2) * std::f32::consts::TAU + 0.6 * (t * 0.011).sin() + 0.4 * (t * 0.0043 + 1.0).sin();
    let k = 0.7 + 0.3 * (t * 0.037 + 2.0).sin();
    let speed = grip::table().wind_speed * (0.6 + 0.4 * (0.5 + 0.5 * k)).min(1.0);
    v3(a.cos() * speed, 0.0, a.sin() * speed)
}

fn caps(level: i32) -> (usize, f32) {
    let t = grip::table();
    match level { 0 => (0, 0.0), 2 => (t.dust_cap_lots, 1.5), _ => (t.dust_cap_normal, 1.0) }
}

impl Dust {
    pub fn new(seed: u64) -> Dust {
        let (cap, _) = caps(1);
        Dust { seed, rng: (seed as u32) ^ 0x9E37_79B9 | 1, parts: Vec::new(), head: 0, cap, level: 1, acc: [0.0; 4], wind: V3::ZERO }
    }
    pub fn len(&self) -> usize { self.parts.len() }
    pub fn cap(&self) -> usize { self.cap }

    /// 0 off, 1 normal, 2 lots.
    pub fn set_level(&mut self, level: i32) {
        let level = level.clamp(0, 2);
        self.level = level;
        self.cap = caps(level).0;
        if self.parts.len() > self.cap { self.parts.clear(); self.head = 0; }
        if self.cap == 0 { self.parts = Vec::new(); self.head = 0; self.acc = [0.0; 4]; }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 17; self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }
    fn push(&mut self, p: P) {
        if self.cap == 0 { return; }
        if self.parts.len() < self.cap { self.parts.push(p); } else { self.parts[self.head] = p; self.head = (self.head + 1) % self.cap; }
    }

    pub fn update(&mut self, v: &Vehicle, t: &Terrain, time: f32, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 { return; }
        let dt = dt.min(0.1);
        self.wind = wind_at(self.seed, time);
        let table = grip::table();
        // motion and ageing
        let wind = self.wind;
        let mut i = 0;
        while i < self.parts.len() {
            let p = &mut self.parts[i];
            p.age += dt;
            if p.age >= p.life { self.parts.swap_remove(i); if self.head >= self.parts.len() { self.head = 0; } continue; }
            let k = (1.1 * dt).min(1.0);
            p.vel = p.vel + (wind - p.vel) * k;
            p.vel.y = p.vel.y * (1.0 - 0.8 * dt) + 0.15 * dt;
            p.pos = p.pos + p.vel * dt;
            let g = t.height(p.pos.x, p.pos.z) + 0.05;
            if p.pos.y < g { p.pos.y = g; if p.vel.y < 0.0 { p.vel.y = 0.0; } }
            i += 1;
        }
        // emission
        let (cap, factor) = caps(self.level);
        if cap == 0 { return; }
        for w in 0..4 {
            let info = v.wheels[w];
            if !info.grounded || !info.speed.is_finite() || !info.slide.is_finite() || !info.lat_slip.is_finite() || !info.contact.is_finite() { self.acc[w] = 0.0; continue; }
            let d = table.dust(info.surface);
            let roll = if info.speed > 2.0 { d.roll * 14.0 * (info.speed / 20.0).min(1.5) } else { 0.0 };
            let slide = d.slide * (60.0 * info.slide.min(1.5) + 25.0 * (info.lat_slip / 6.0).min(1.5));
            let rate = (roll + slide) * factor;
            self.acc[w] = (self.acc[w] + rate * dt).min(8.0);
            while self.acc[w] >= 1.0 {
                self.acc[w] -= 1.0;
                let (a, b, c, e, f, g2) = (self.rand(), self.rand(), self.rand(), self.rand(), self.rand(), self.rand());
                let wheel_vel = v.vel;
                let pos = v3(info.contact.x + (a - 0.5) * 0.4, info.contact.y + 0.1, info.contact.z + (b - 0.5) * 0.4);
                let vel = v3(wheel_vel.x * 0.25 + (c - 0.5) * 1.2, 1.2 + e * 1.0, wheel_vel.z * 0.25 + (f - 0.5) * 1.2);
                let shade = 0.9 + 0.2 * g2;
                self.push(P { pos, vel, age: 0.0, life: d.life * (0.75 + 0.5 * a), size0: d.size, color: [d.color[0] * shade, d.color[1] * shade, d.color[2] * shade] });
            }
        }
    }

    pub fn particles(&self) -> Vec<Particle> {
        self.parts.iter().map(|p| {
            let x = (p.age / p.life).clamp(0.0, 1.0);
            let fade_in = (x / 0.08).min(1.0);
            Particle { pos: p.pos, size: p.size0 * (1.0 + 2.5 * x.powf(0.6)), color: p.color, alpha: 0.55 * fade_in * (1.0 - x).powf(1.5) }
        }).collect()
    }
}

#[cfg(test)]
#[path = "dust_tests.rs"]
mod tests;
