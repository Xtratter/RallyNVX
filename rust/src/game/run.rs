//! State machine of one mission attempt: timer, cargo integrity, checkpoints, outcome. No wall clock.
use super::input::Input;
use super::math::V3;
use super::medals::{grade_cargo, grade_race, Medal};
use super::missions::{Mission, MissionKind, GOAL_RADIUS};
use super::vehicle::Vehicle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Phase { Ready, Running, Done }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Failure { TimeUp, CargoDestroyed, Abandoned, Crashed }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome { Passed { medal: Medal, time: f32, integrity: f32 }, Failed(Failure) }

pub struct MissionRun {
    m: Mission, phase: Phase, time: f32, integrity: f32, next: usize,
    prev_vel: V3, air_time: f32, outcome: Option<Outcome>, first: bool,
}

impl MissionRun {
    pub fn new(m: Mission) -> MissionRun {
        MissionRun { m, phase: Phase::Ready, time: 0.0, integrity: 100.0, next: 0, prev_vel: V3::ZERO, air_time: 0.0, outcome: None, first: true }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn outcome(&self) -> Option<Outcome> { self.outcome }
    pub fn time(&self) -> f32 { self.time }
    pub fn time_left(&self) -> f32 { (self.m.time_limit - self.time).max(0.0) }
    pub fn integrity(&self) -> f32 { self.integrity }
    pub fn next_index(&self) -> usize { self.next }
    pub fn mission(&self) -> &Mission { &self.m }
    pub fn target(&self) -> Option<V3> { if self.phase == Phase::Done { None } else { self.m.checkpoints.get(self.next).copied() } }

    pub fn abandon(&mut self) {
        if self.phase != Phase::Done { self.finish(Outcome::Failed(Failure::Abandoned)); }
    }

    fn finish(&mut self, o: Outcome) { self.phase = Phase::Done; self.outcome = Some(o); }

    pub fn update(&mut self, v: &Vehicle, inp: &Input, reset_pressed: bool, dt: f32) {
        if self.phase == Phase::Done { return; }
        if self.phase == Phase::Ready {
            if inp.throttle.abs() > 0.05 { self.phase = Phase::Running; } else { self.prev_vel = v.vel; self.first = false; return; }
        }
        self.time += dt;
        let cargo = self.m.kind == MissionKind::Cargo;
        if cargo {
            // a reset zeroes the velocity: that is not an impact or a landing, it costs only its own 5
            if reset_pressed { self.air_time = 0.0; }
            else if !self.first {
                let dv = (v.vel.x - self.prev_vel.x).hypot(v.vel.z - self.prev_vel.z);
                if dv > 6.0 { self.integrity -= (dv - 6.0) * 2.5; }
            }
            if reset_pressed { /* handled above */ } else if v.contacts == 0 { self.air_time += dt; } else {
                if self.air_time > 0.4 && self.prev_vel.y < -7.0 { self.integrity -= (-self.prev_vel.y - 7.0) * 3.0; }
                self.air_time = 0.0;
            }
            if v.flipped() { self.integrity -= 8.0 * dt; }
            if reset_pressed { self.integrity -= 5.0; }
        }
        self.first = false;
        self.prev_vel = v.vel;
        // goal first: reaching it in the step the clock runs out still counts
        if let Some(p) = self.m.checkpoints.get(self.next) {
            if (v.pos - *p).len() <= GOAL_RADIUS && (!cargo || self.integrity > 0.0) {
                self.next += 1;
                if self.next >= self.m.checkpoints.len() {
                    let medal = if cargo { grade_cargo(self.time, self.m.time_limit, self.integrity) } else { grade_race(self.time, self.m.time_limit) };
                    self.finish(Outcome::Passed { medal, time: self.time, integrity: self.integrity.max(0.0) });
                    return;
                }
            }
        }
        if cargo && self.integrity <= 0.0 { self.integrity = 0.0; self.finish(Outcome::Failed(Failure::CargoDestroyed)); return; }
        if self.time > self.m.time_limit { self.finish(Outcome::Failed(Failure::TimeUp)); }
    }
}

/// Angle to `to` relative to the heading (hx, hz) as seen from above: 0 ahead, positive = to the right.
/// With x right-handed (x is left of a car heading +z), the right-hand side of heading h is (-hz, hx).
pub fn bearing(hx: f32, hz: f32, from: V3, to: V3) -> f32 {
    let (dx, dz) = (to.x - from.x, to.z - from.z);
    let fwd = hx * dx + hz * dz;
    let right = -hz * dx + hx * dz;
    right.atan2(fwd)
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
