//! Turns the game world into `AudioParams` and one-shot `Event`s by comparing each frame with the previous one.
use super::{AudioParams, Event, Machine, Surface};
use crate::game::{flight::{AircraftKind, Status}, missions::MissionKind, run::{Outcome, Phase}, world::World};

pub struct Director {
    started: bool, time: f32, seed: u64,
    grounded: bool, wet: bool, status: Status, next: usize, loaded: u8, reported: bool,
    hspeed: f32, vy: f32, air_time: f32, quiet_until: f32,
}

impl Default for Director { fn default() -> Self { Director::new() } }

impl Director {
    pub fn new() -> Director {
        Director { started: false, time: 0.0, seed: 0, grounded: true, wet: false, status: Status::Flying, next: 0, loaded: 0, reported: false, hspeed: 0.0, vy: 0.0, air_time: 0.0, quiet_until: 0.0 }
    }

    /// `throttle` is the player's throttle (-1..1), `active` is false while paused. Never emits events when inactive
    /// or on the first update of a new world (a level change, a restart).
    pub fn update(&mut self, w: &World, throttle: f32, active: bool) -> (AudioParams, Vec<Event>) {
        let throttle = if throttle.is_finite() { throttle.clamp(-1.0, 1.0) } else { 0.0 };
        let mut ev = Vec::new();
        let fresh = !self.started || w.terrain.seed != self.seed || w.time < self.time - 1e-4;
        let dt = if fresh { 0.0 } else { (w.time - self.time).clamp(0.0, 0.5) };
        self.started = true; self.seed = w.terrain.seed; self.time = w.time;
        let info = w.run_info();
        let (next, phase, outcome) = info.as_ref().map_or((0, Phase::Ready, None), |r| (r.next, r.phase, r.outcome));
        let kind = info.as_ref().map(|r| r.mission.kind);
        let loaded = w.air.as_ref().map_or(0, |a| a.rescue_loaded());
        let mut p = AudioParams { active, ..Default::default() };
        let v = &w.vehicle;
        let (grounded, wet, status, hspeed, vy);
        if let Some(a) = &w.aircraft {
            let ground = w.terrain.height(a.pos.x, a.pos.z);
            let speed = a.vel.len();
            p.machine = if a.kind == AircraftKind::Heli { Machine::Heli } else { Machine::Drone };
            p.rpm = if a.status == Status::Landed { 0.45 } else { (0.6 + 0.25 * (speed / 35.0).min(1.0) + 0.15 * (a.vel.y.abs() / 6.0).min(1.0)).min(1.0) };
            p.load = if a.status == Status::Landed { 0.2 } else { 0.5 };
            p.wind = ((a.pos.y - ground) / 120.0).clamp(0.0, 1.0) * 0.5 + (speed / 60.0).clamp(0.0, 0.5);
            (grounded, wet, status, hspeed, vy) = (a.status == Status::Landed, false, a.status, a.vel.x.hypot(a.vel.z), a.vel.y);
        } else {
            let speed = v.vel.len();
            let frac = (speed / v.spec.max_speed.max(1.0)).clamp(0.0, 1.0);
            let on_ground = v.contacts > 0;
            let in_water = w.terrain.is_water(v.pos.x, v.pos.z, 0.0);
            p.machine = Machine::Car;
            p.rpm = (0.18 + 0.6 * frac.powf(0.7) + 0.2 * throttle.abs()).min(1.0);
            p.load = (0.2 + 0.7 * throttle.abs()).min(1.0);
            p.roll = if on_ground { frac.sqrt() } else { 0.0 };
            p.surface = if in_water { Surface::Water } else { Surface::Ground };
            p.slip = if on_ground { ((v.vel.dot(v.right()).abs() - 2.0) / 6.0).clamp(0.0, 1.0) } else { 0.0 };
            p.wind = (speed / 40.0).clamp(0.0, 1.0) * 0.4;
            (grounded, wet, status, hspeed, vy) = (on_ground, in_water, Status::Flying, v.vel.x.hypot(v.vel.z), v.vel.y);
        }
        // a teleport (the Reset button zeroes the velocity and lifts the car) must not sound like a crash
        let teleport = w.aircraft.is_none() && hspeed == 0.0 && self.hspeed > 6.0;
        if teleport { self.quiet_until = w.time + 1.0; self.air_time = 0.0; }
        if !grounded { self.air_time += dt; }
        let quiet = w.time < self.quiet_until;
        if !fresh && active && !quiet {
            if w.aircraft.is_some() {
                if self.status == Status::Flying && status == Status::Landed { ev.push(Event::Touchdown((self.vy.abs() / 3.0).clamp(0.0, 1.0))); }
                if self.status != Status::Crashed && status == Status::Crashed { ev.push(Event::Crash); }
            } else {
                if !self.grounded && grounded && self.vy < -3.0 && self.air_time >= 0.3 { ev.push(Event::Touchdown((self.vy.abs() / 10.0).clamp(0.0, 1.0))); }
                if !self.wet && wet && hspeed > 2.0 { ev.push(Event::Splash((hspeed / 20.0).clamp(0.0, 1.0))); }
                if self.hspeed - hspeed > 6.0 { ev.push(Event::Crash); }
            }
            if kind == Some(MissionKind::Rescue) {
                if loaded > self.loaded { ev.push(Event::Board); }
            } else if next > self.next && phase == Phase::Running { ev.push(Event::Checkpoint); }
            if let (Some(o), false) = (outcome, self.reported) {
                ev.push(match o { Outcome::Passed { .. } => Event::Success, Outcome::Failed(_) => Event::Fail });
            }
        }
        if grounded { self.air_time = 0.0; }
        if outcome.is_some() { p.machine = Machine::None; }   // the engine stops under the result panel
        if fresh { self.reported = outcome.is_some(); } else if outcome.is_some() { self.reported = true; }
        self.grounded = grounded; self.wet = wet; self.status = status; self.next = next; self.loaded = loaded; self.hspeed = hspeed;
        // keep the last airborne vertical speed for the touchdown strength
        if !grounded || w.aircraft.is_some() && status == Status::Flying { self.vy = vy; } else if !fresh && grounded && self.vy > -0.001 { self.vy = vy; }
        (p, ev)
    }
}

#[cfg(test)]
#[path = "director_tests.rs"]
mod tests;
