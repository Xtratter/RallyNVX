//! The rally stage runner: countdown, clock, check lines, penalties and the finish (pure logic; the world feeds it the
//! car's position every step). See docs/superpowers/specs/2026-10-14-rally-core-design.md.
use super::lines::Line;
use super::math::V3;

pub const COUNTDOWN: f32 = 3.5;
pub const RESET_PENALTY: f32 = 10.0;
pub const OFFROAD_PENALTY: f32 = 5.0;
pub const OFFROAD_TIME: f32 = 3.0;
pub const OFFROAD_DIST: f32 = 1.0;
pub const MISSED_PENALTY: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Countdown, Running, Finished }

/// Things that happened since the last `drain_events`, for the HUD.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event { Go, Split { index: usize, time: f32 }, Reset, OffRoad, Missed { lines: usize }, Finish { total: f32 } }

pub struct Run {
    pub phase: Phase,
    countdown: f32,
    /// The clock (from GO) and the penalties added to it, in seconds.
    pub time: f32,
    pub penalty: f32,
    /// The lines to cross in order: the start line (a first line of the kind `Start` is just passed), the checks and, last, the finish.
    lines: Vec<Line>,
    next: usize,
    pub splits: Vec<f32>,
    off_t: f32,
    armed: bool,
    /// The autopilot drove (part of) the run: it is not recorded.
    pub assisted: bool,
    prev: Option<V3>,
    events: Vec<Event>,
}

impl Run {
    pub fn new(lines: Vec<Line>) -> Run {
        assert!(!lines.is_empty(), "a run needs at least a finish line");
        Run { phase: Phase::Countdown, countdown: COUNTDOWN, time: 0.0, penalty: 0.0, lines, next: 0, splits: Vec::new(), off_t: 0.0, armed: true, assisted: false, prev: None, events: Vec::new() }
    }

    pub fn total(&self) -> f32 { self.time + self.penalty }
    /// The digit to show during the countdown (3, 2, 1), 0 at GO.
    pub fn countdown_digit(&self) -> u32 { if self.phase == Phase::Countdown { (self.countdown - 0.5).ceil().clamp(0.0, 3.0) as u32 } else { 0 } }
    pub fn next_line(&self) -> usize { self.next }
    pub fn line_count(&self) -> usize { self.lines.len() }
    pub fn drain_events(&mut self) -> Vec<Event> { std::mem::take(&mut self.events) }

    /// The car was put somewhere else (a reset): crossings are not judged across the jump.
    pub fn teleported(&mut self) { self.prev = None; }

    /// A reset of the car during the run costs `RESET_PENALTY` seconds.
    pub fn reset_penalty(&mut self) {
        if self.phase == Phase::Running { self.penalty += RESET_PENALTY; self.events.push(Event::Reset); }
        self.teleported();
    }

    /// One physics step: the car is at `pos`; `off_edge` is how far its centre is outside the road edge in metres
    /// (negative on the road), `None` where leaving the road does not count (the city stage).
    pub fn update(&mut self, pos: V3, dt: f32, off_edge: Option<f32>) {
        match self.phase {
            Phase::Finished => {}
            Phase::Countdown => {
                self.countdown -= dt;
                if self.countdown <= 0.0 { self.phase = Phase::Running; self.events.push(Event::Go); }
                self.prev = Some(pos);
            }
            Phase::Running => {
                self.time += dt;
                if let Some(e) = off_edge {
                    if e > OFFROAD_DIST { self.off_t = (self.off_t + dt).min(OFFROAD_TIME); } else { self.off_t = (self.off_t - dt * 2.0).max(0.0); if self.off_t == 0.0 { self.armed = true; } }
                    if self.off_t >= OFFROAD_TIME && self.armed { self.armed = false; self.penalty += OFFROAD_PENALTY; self.events.push(Event::OffRoad); }
                }
                if let Some(prev) = self.prev {
                    let last = self.lines.len() - 1;
                    for k in self.next..(self.next + 3).min(self.lines.len()) {
                        if !self.lines[k].crossed(prev, pos) { continue; }
                        if k > self.next { let n = k - self.next; self.penalty += MISSED_PENALTY * n as f32; self.events.push(Event::Missed { lines: n }); }
                        if k == 0 && self.lines[0].kind == super::lines::LineKind::Start { self.next = 1; break; }   // the start line itself: nothing to record
                        if k == last {
                            self.phase = Phase::Finished;
                            self.events.push(Event::Finish { total: self.total() });
                        } else {
                            self.splits.push(self.time);
                            self.events.push(Event::Split { index: k, time: self.time });
                        }
                        self.next = k + 1;
                        break;
                    }
                }
                self.prev = Some(pos);
            }
        }
    }
}

#[cfg(test)]
#[path = "rally_tests.rs"]
mod tests;
