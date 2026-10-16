//! Turns the game world into `AudioParams` and one-shot `Event`s by comparing each frame with the previous one.
use super::{AudioParams, Event, Machine, Scene, Surface, N_SRC, SRC_BAY, SRC_BODY, SRC_EXHAUST, SRC_WHEEL};
use crate::game::{math::v3, world::World};

pub struct Director {
    started: bool, time: f32, seed: u64,
    grounded: bool, wet: bool,
    hspeed: f32, vy: f32, air_time: f32, quiet_until: f32,
}

impl Default for Director { fn default() -> Self { Director::new() } }

impl Director {
    pub fn new() -> Director {
        Director { started: false, time: 0.0, seed: 0, grounded: true, wet: false, hspeed: 0.0, vy: 0.0, air_time: 0.0, quiet_until: 0.0 }
    }

    /// Where the listener (the camera) and the sound sources of the car are: the exhaust at the tail, the engine bay at the nose, the four tyres where they touch the ground.
    pub fn scene(&self, w: &World) -> Scene {
        let v = &w.vehicle;
        let size = v.spec.size;
        let at = |x: f32, y: f32, z: f32| v.pos + v.rot.rotate(v3(x, y, z));
        let mut sc = Scene::default();
        sc.listener = w.cam.pos;
        let g = w.cam.target - w.cam.pos;
        sc.fwd = if g.len() > 1e-3 { g.norm() } else { v3(0.0, 0.0, 1.0) };
        let mut src = [v.pos; N_SRC];
        src[SRC_EXHAUST] = at(0.30, -0.25, -0.5 * size.z);
        src[SRC_BAY] = at(0.0, 0.05, 0.30 * size.z);
        src[SRC_BODY] = v.pos;
        for k in 0..4 {
            let wi = &v.wheels[k];
            src[SRC_WHEEL + k] = if wi.grounded { wi.contact } else { v.pos + v.rot.rotate(v.spec.wheels[k]) };
            sc.wheel_on[k] = wi.grounded as u8 as f32;
            sc.wheel_slip[k] = if wi.grounded { ((wi.lat_slip - 1.5) / 3.0).clamp(0.0, 1.0) } else { 0.0 };
        }
        sc.src = src;
        sc
    }

    /// `throttle` is the player's throttle (-1..1), `active` is false while paused. Never emits events when inactive
    /// or on the first update of a new world (a level change, a restart).
    pub fn update(&mut self, w: &World, throttle: f32, active: bool) -> (AudioParams, Vec<Event>) {
        let throttle = if throttle.is_finite() { throttle.clamp(-1.0, 1.0) } else { 0.0 };
        let mut ev = Vec::new();
        let fresh = !self.started || w.terrain.seed != self.seed || w.time < self.time - 1e-4;
        let dt = if fresh { 0.0 } else { (w.time - self.time).clamp(0.0, 0.5) };
        self.started = true; self.seed = w.terrain.seed; self.time = w.time;
        let mut p = AudioParams { active, ..Default::default() };
        let v = &w.vehicle;
        let speed = v.vel.len();
        let frac = (speed / v.spec.max_speed.max(1.0)).clamp(0.0, 1.0);
        let grounded = v.contacts > 0;
        let wet = w.terrain.is_water(v.pos.x, v.pos.z, 0.0);
        let (hspeed, vy) = (v.vel.x.hypot(v.vel.z), v.vel.y);
        p.machine = Machine::Car;
        p.rpm = (0.18 + 0.6 * frac.powf(0.7) + 0.2 * throttle.abs()).min(1.0);
        p.load = (0.2 + 0.7 * throttle.abs()).min(1.0);
        p.roll = if grounded { frac.sqrt() } else { 0.0 };
        p.surface = if wet { Surface::Water } else { Surface::Ground };
        p.slip = if grounded { ((v.vel.dot(v.right()).abs() - 2.0) / 6.0).clamp(0.0, 1.0) } else { 0.0 };
        // the wind you hear is the apparent wind: the level's wind minus the car's own velocity (a tail wind is quieter)
        let air = w.wind() - v3(v.vel.x, 0.0, v.vel.z);
        p.wind = (air.len() / 34.0).clamp(0.0, 1.0) * 0.42;
        // a teleport (the Reset button zeroes the velocity and lifts the car) must not sound like a crash
        let teleport = hspeed == 0.0 && self.hspeed > 6.0;
        if teleport { self.quiet_until = w.time + 1.0; self.air_time = 0.0; }
        if !grounded { self.air_time += dt; }
        let quiet = w.time < self.quiet_until;
        if !fresh && active && !quiet {
            if !self.grounded && grounded && self.vy < -3.0 && self.air_time >= 0.3 { ev.push(Event::Touchdown((self.vy.abs() / 10.0).clamp(0.0, 1.0))); }
            if !self.wet && wet && hspeed > 2.0 { ev.push(Event::Splash((hspeed / 20.0).clamp(0.0, 1.0))); }
            if self.hspeed - hspeed > 6.0 { ev.push(Event::Crash); }
        }
        if grounded { self.air_time = 0.0; }
        self.grounded = grounded; self.wet = wet; self.hspeed = hspeed;
        // keep the last airborne vertical speed for the touchdown strength
        if !grounded { self.vy = vy; } else if !fresh && grounded && self.vy > -0.001 { self.vy = vy; }
        (p, ev)
    }
}

#[cfg(test)]
#[path = "director_tests.rs"]
mod tests;
