//! State machine of one air mission (rings, sling delivery, recon). All time comes from `dt`.
use super::flight::{Aircraft, AircraftKind, Status};
use super::math::V3;
use super::medals::{grade_cargo, grade_race};
use super::missions::{ring_normal, Mission, MissionKind, RING_RADIUS_DRONE, RING_RADIUS_HELI, ZONE_RADIUS};
use super::run::{Failure, Outcome, Phase};
use super::sling::Sling;

#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum AirAction { AttachCargo }

pub struct AirRun {
    m: Mission, phase: Phase, time: f32, outcome: Option<Outcome>, integrity: f32,
    next: usize, captured: Vec<bool>, zone: Option<usize>, dwell: f32,
    attached: bool, hover: f32, prev: Option<V3>,
}

impl AirRun {
    pub fn new(m: Mission) -> AirRun {
        let n = m.checkpoints.len();
        AirRun { m, phase: Phase::Ready, time: 0.0, outcome: None, integrity: 100.0, next: 0, captured: vec![false; n], zone: None, dwell: 0.0, attached: false, hover: 0.0, prev: None }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn outcome(&self) -> Option<Outcome> { self.outcome }
    pub fn time(&self) -> f32 { self.time }
    pub fn time_left(&self) -> f32 { (self.m.time_limit - self.time).max(0.0) }
    pub fn integrity(&self) -> f32 { self.integrity }
    pub fn mission(&self) -> &Mission { &self.m }
    pub fn total(&self) -> usize { if self.m.kind == MissionKind::Sling { 2 } else { self.m.checkpoints.len() } }
    pub fn next_index(&self) -> usize {
        match self.m.kind { MissionKind::Recon => self.captured.iter().filter(|c| **c).count(), MissionKind::Sling => self.attached as usize, _ => self.next }
    }
    pub fn captured(&self) -> &[bool] { &self.captured }
    pub fn target(&self, from: V3) -> Option<V3> {
        if self.phase == Phase::Done { return None; }
        match self.m.kind {
            MissionKind::Rings => self.m.checkpoints.get(self.next).copied(),
            MissionKind::Sling => self.m.checkpoints.get(self.attached as usize).copied(),
            _ => self.m.checkpoints.iter().zip(&self.captured).filter(|(_, c)| !**c).map(|(p, _)| *p)
                .min_by(|a, b| (*a - from).len().partial_cmp(&(*b - from).len()).unwrap()),
        }
    }
    pub fn abandon(&mut self) { if self.phase != Phase::Done { self.finish(Outcome::Failed(Failure::Abandoned)); } }
    fn finish(&mut self, o: Outcome) { self.phase = Phase::Done; self.outcome = Some(o); }
    fn pass(&mut self) {
        let medal = if self.m.kind == MissionKind::Sling { grade_cargo(self.time, self.m.time_limit, self.integrity) } else { grade_race(self.time, self.m.time_limit) };
        self.finish(Outcome::Passed { medal, time: self.time, integrity: self.integrity.max(0.0) });
    }

    pub fn update(&mut self, a: &Aircraft, sling: Option<&Sling>, impact: Option<f32>, active: bool, dt: f32) -> Option<AirAction> {
        if self.phase == Phase::Done { return None; }
        if self.phase == Phase::Ready {
            if active { self.phase = Phase::Running; } else { self.prev = Some(a.pos); return None; }
        }
        self.time += dt;
        let prev = self.prev.unwrap_or(a.pos);
        self.prev = Some(a.pos);
        if a.status == Status::Crashed { self.finish(Outcome::Failed(Failure::Crashed)); return None; }
        let mut action = None;
        match self.m.kind {
            MissionKind::Rings => {
                if let Some(p) = self.m.checkpoints.get(self.next).copied() {
                    let n = ring_normal(&self.m, self.next);
                    let (d0, d1) = ((prev - p).dot(n), (a.pos - p).dot(n));
                    if d0 < 0.0 && d1 >= 0.0 {
                        let q = prev + (a.pos - prev) * (-d0 / (d1 - d0));
                        let r = if a.kind == AircraftKind::Drone { RING_RADIUS_DRONE } else { RING_RADIUS_HELI };
                        if (q - p).len() <= r { self.next += 1; if self.next >= self.m.checkpoints.len() { self.pass(); return None; } }
                    }
                }
            }
            MissionKind::Recon => {
                let inside = self.m.checkpoints.iter().enumerate().find(|(i, p)| !self.captured[*i] && (a.pos - **p).len() <= ZONE_RADIUS).map(|(i, _)| i);
                match inside {
                    Some(i) if self.zone == Some(i) => self.dwell += dt,
                    Some(i) => { self.zone = Some(i); self.dwell = dt; }
                    None => { self.zone = None; self.dwell = 0.0; }
                }
                if let (Some(i), true) = (self.zone, self.dwell >= 2.0) { self.captured[i] = true; self.zone = None; self.dwell = 0.0; }
                if self.captured.iter().all(|c| *c) { self.pass(); return None; }
            }
            MissionKind::Sling => {
                if let Some(s) = sling {
                    if !self.attached {
                        let above = a.pos.y - s.pos.y;
                        let close = (a.pos.x - s.pos.x).hypot(a.pos.z - s.pos.z) < 3.0 && above < 4.0 && above > 0.0;
                        self.hover = if close { self.hover + dt } else { 0.0 };
                        if self.hover >= 1.5 { self.attached = true; action = Some(AirAction::AttachCargo); }
                    } else {
                        if let Some(v) = impact { if v > 6.0 { self.integrity -= (v - 6.0) * 2.5; } }
                        if s.relative_speed(a.vel) > 12.0 { self.integrity -= 4.0 * dt; }
                        let b = self.m.checkpoints[1];
                        let in_circle = (s.pos.x - b.x).hypot(s.pos.z - b.z) <= 6.0 && s.pos.y - b.y < 1.0 && s.vel.len() < 2.0;
                        if self.integrity > 0.0 && in_circle { self.pass(); return None; }
                        if self.integrity <= 0.0 { self.integrity = 0.0; self.finish(Outcome::Failed(Failure::CargoDestroyed)); return None; }
                    }
                }
            }
            _ => {}
        }
        if self.time > self.m.time_limit { self.finish(Outcome::Failed(Failure::TimeUp)); }
        action
    }
}

#[cfg(test)]
#[path = "airrun_tests.rs"]
mod tests;
