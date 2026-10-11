//! Arcade helicopter / drone: tilt-driven acceleration, auto-hover, soft landing vs crash.
use super::math::{v3, Q, V3};
use super::props::{Prop, PropKind};
use super::ground::Ground;
use super::world::WORLD_LIMIT;


#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum AircraftKind { Heli, Drone }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Status { Flying, Landed, Crashed }

#[derive(Clone, Copy, Debug, Default)]
pub struct FlightInput { pub yaw: f32, pub lift: f32, pub pitch: f32, pub roll: f32 }
impl FlightInput {
    pub fn sanitized(self) -> FlightInput {
        let f = |v: f32| if v.is_finite() { v.clamp(-1.0, 1.0) } else { 0.0 };
        FlightInput { yaw: f(self.yaw), lift: f(self.lift), pitch: f(self.pitch), roll: f(self.roll) }
    }
    /// Any stick beyond the dead zone (starts the mission clock).
    pub fn active(&self) -> bool { [self.yaw, self.lift, self.pitch, self.roll].iter().any(|v| v.is_finite() && v.abs() > 0.05) }
}

#[derive(Clone, Copy, Debug)]
pub struct AircraftSpec {
    pub max_tilt: f32, pub tilt_lag: f32, pub max_speed: f32, pub drag: f32, pub vmax: f32, pub vacc: f32, pub yaw_rate: f32, pub yaw_lag: f32, pub radius: f32,
    /// Flight rules shared by all aircraft (from the `flight` section): ceiling in metres, the highest soft touchdown
    /// speed along the ground normal (m/s), the largest soft tilt (rad) and the ground friction (1/s).
    pub ceiling: f32, pub soft_speed: f32, pub soft_tilt: f32, pub ground_damp: f32,
}
impl AircraftSpec {
    /// From `vehicles.json` (see docs/vehicles.md).
    pub fn of(kind: AircraftKind) -> AircraftSpec { super::params::get().aircraft(kind) }
}

#[derive(Clone, Copy, Debug)]
pub struct Aircraft {
    pub kind: AircraftKind, pub spec: AircraftSpec,
    pub pos: V3, pub vel: V3,
    /// Heading angle: forward = (sin yaw, 0, cos yaw). Pitch is nose-down positive, roll is right-down positive.
    pub yaw: f32, pub pitch: f32, pub roll: f32, pub yaw_rate: f32,
    pub status: Status,
}

const G: f32 = 9.81;

impl Aircraft {
    pub fn new(kind: AircraftKind, pos: V3, yaw: f32) -> Aircraft {
        Aircraft { kind, spec: AircraftSpec::of(kind), pos, vel: V3::ZERO, yaw, pitch: 0.0, roll: 0.0, yaw_rate: 0.0, status: Status::Flying }
    }
    pub fn forward(&self) -> V3 { v3(self.yaw.sin(), 0.0, self.yaw.cos()) }
    pub fn right(&self) -> V3 { v3(-self.yaw.cos(), 0.0, self.yaw.sin()) }
    pub fn speed(&self) -> f32 { self.vel.len() }
    pub fn rotation(&self) -> Q {
        Q::axis_angle(v3(0., 1., 0.), self.yaw).mul(Q::axis_angle(v3(1., 0., 0.), self.pitch)).mul(Q::axis_angle(v3(0., 0., 1.), self.roll))
    }

    pub fn step(&mut self, t: &dyn Ground, props: &[Prop], inp: &FlightInput, dt: f32) {
        if self.status == Status::Crashed { return; }
        let inp = inp.sanitized();
        let s = self.spec;
        let k = |lag: f32| 1.0 - (-dt / lag).exp();
        // heading: a positive input turns right, which decreases the heading angle
        self.yaw_rate += (-inp.yaw * s.yaw_rate - self.yaw_rate) * k(s.yaw_lag);
        self.yaw += self.yaw_rate * dt;
        // tilt
        self.pitch += (inp.pitch * s.max_tilt - self.pitch) * k(s.tilt_lag);
        self.roll += (inp.roll * s.max_tilt - self.roll) * k(s.tilt_lag);
        // horizontal motion
        let (f, r) = (self.forward(), self.right());
        let acc = f * (G * self.pitch.tan()) + r * (G * self.roll.tan());
        let mut vh = v3(self.vel.x, 0.0, self.vel.z);
        vh = vh + acc * dt - vh * (s.drag * dt);
        let sp = vh.len();
        if sp > s.max_speed { vh = vh * (s.max_speed / sp); }
        // vertical motion
        let mut vy = self.vel.y + (inp.lift * s.vmax - self.vel.y).clamp(-s.vacc * dt, s.vacc * dt);
        if self.pos.y >= s.ceiling && vy > 0.0 { vy = 0.0; }
        self.vel = v3(vh.x, vy, vh.z);
        self.pos = self.pos + self.vel * dt;
        // world wall
        let lim = WORLD_LIMIT - 20.0;
        if self.pos.x.abs() > lim { self.pos.x = self.pos.x.clamp(-lim, lim); if self.pos.x * self.vel.x > 0.0 { self.vel.x = 0.0; } }
        if self.pos.z.abs() > lim { self.pos.z = self.pos.z.clamp(-lim, lim); if self.pos.z * self.vel.z > 0.0 { self.vel.z = 0.0; } }
        // props
        for p in props {
            let hit = match p.kind {
                PropKind::Rock => (self.pos - (p.pos + v3(0., p.radius * 0.5, 0.))).len() < p.radius + s.radius,
                PropKind::Tree => self.pos.y < p.pos.y + p.height && (self.pos.x - p.pos.x).hypot(self.pos.z - p.pos.z) < p.radius + s.radius,
            };
            if hit { self.status = Status::Crashed; self.vel = V3::ZERO; return; }
        }
        // ground (the small epsilon keeps a landed aircraft in contact despite float rounding)
        let g = t.height(self.pos.x, self.pos.z);
        if self.pos.y - g < 0.3 + 1e-3 {
            let n = t.normal(self.pos.x, self.pos.z);
            let into = -self.vel.dot(n);                       // speed into the ground along its normal (also covers hillsides)
            let tilt = self.pitch.abs().max(self.roll.abs());
            let landed = self.status == Status::Landed;
            if into > s.soft_speed || (tilt > s.soft_tilt && !landed) { self.status = Status::Crashed; self.vel = V3::ZERO; return; }
            if landed {                                         // sitting on its skids: the stick cannot tip it over
                let m = s.soft_tilt;
                self.pitch = self.pitch.clamp(-m, m); self.roll = self.roll.clamp(-m, m);
            }
            self.pos.y = g + 0.3;
            if into > 0.0 { self.vel = self.vel + n * into; }
            let damp = (1.0 - s.ground_damp * dt).max(0.0);
            self.vel.x *= damp; self.vel.z *= damp;
            self.status = Status::Landed;
        } else if (self.status == Status::Landed && self.vel.y > 0.0) || self.pos.y - g > 0.5 {
            self.status = Status::Flying;
        }
    }
}

#[cfg(test)]
#[path = "flight_tests.rs"]
mod tests;
