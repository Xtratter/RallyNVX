//! The autopilot: joins the special stage and drives it to the finish (see docs/superpowers/specs/2026-10-14-autopilot-design.md).
pub mod cfg;
pub mod join;
pub mod route;

use self::cfg::{cfg, AutoCfg};
use self::route::{Limits, Route};
use super::grip::grip;
use super::input::Input;
use super::math::{v3, V3};
use super::roads::Road;
use super::vehicle::{Vehicle, VehicleSpec};

/// What the autopilot asks of the world for one step: the car's input, an upright reset in place, a teleport back onto
/// the route (position and yaw) and, when the finish is reached, `done`.
#[derive(Clone, Copy, Debug, Default)]
pub struct AutoCmd { pub input: Input, pub reset: bool, pub teleport: Option<(V3, f32)>, pub done: bool }

pub struct Autopilot {
    route: Route,
    car: VehicleSpec,
    cfg: AutoCfg,
    idx: usize,
    stuck: f32,
    /// The stage route and where to join it, while a field route is being driven.
    next: Option<(Route, usize)>,
    /// How many times the recovery had to act (the tests want zero).
    pub recoveries: u32,
}

/// The speed limits for a car on a road of the given kind (loose gravel is the worst case on the stage).
pub fn limits_for(car: &VehicleSpec, surface: super::surface::Surface, v_max: f32, stop_at_end: bool) -> Limits {
    Limits { mu: car.mu * grip(surface, 1.0).mu, v_max: v_max.min(car.max_speed * 0.95), accel: 0.5 * car.engine_force / car.mass, stop_at_end }
}

/// The route along a road (its centre pulled to the least-curvature line), ending in a standstill.
pub fn route_for_road(road: &Road, car: &VehicleSpec, c: &AutoCfg) -> Route {
    let centre: Vec<V3> = road.pts.iter().map(|p| p.pos).collect();
    let width: Vec<f32> = road.pts.iter().map(|p| p.width).collect();
    Route::build(&centre, &width, limits_for(car, road.kind.surface(), f32::MAX, true), c)
}

impl Autopilot {
    /// An autopilot for a car that is already on (or next to) `route`.
    pub fn on_route(route: Route, car: VehicleSpec, v: &Vehicle) -> Autopilot {
        let idx = route.nearest(v.pos.x, v.pos.z).0;
        Autopilot { route, car, cfg: cfg(), idx, stuck: 0.0, next: None, recoveries: 0 }
    }

    /// Starts from wherever the car is: on the stage it joins at the nearest point, elsewhere it first drives a field
    /// route to the nearest stage point. `None` when the level has no stage.
    pub fn start(t: &super::terrain::Terrain, v: &Vehicle, car: VehicleSpec) -> Option<Autopilot> {
        let stage = t.roads()?.roads.iter().find(|r| r.kind == super::roads::RoadKind::Stage)?;
        let c = cfg();
        let stage_route = route_for_road(stage, &car, &c);
        let (k, d) = stage_route.nearest(v.pos.x, v.pos.z);
        if d <= 15.0 { return Some(Autopilot::on_route(stage_route, car, v)); }
        let to = stage_route.p[k];
        let Some(field) = join::field_route(t, &car, (v.pos.x, v.pos.z), (to.x, to.z), &c) else { return Some(Autopilot::on_route(stage_route, car, v)) };
        let mut ap = Autopilot::on_route(field, car, v);
        ap.next = Some((stage_route, k));
        Some(ap)
    }

    pub fn route(&self) -> &Route { &self.route }
    pub fn progress(&self) -> f32 { self.route.s[self.idx] }

    pub fn drive(&mut self, v: &Vehicle, dt: f32) -> AutoCmd {
        let mut cmd = AutoCmd::default();
        let (lo, hi) = (self.idx.saturating_sub(15), self.idx + 80);
        let (mut i, mut d) = self.route.nearest_in(v.pos.x, v.pos.z, lo, hi);
        if d > 15.0 { let all = self.route.nearest(v.pos.x, v.pos.z); i = all.0; d = all.1; }
        self.idx = i;
        let speed = v.speed();
        if self.next.is_some() && self.route.length() - self.route.s[i] < 14.0 {
            let (stage, k) = self.next.take().unwrap();
            self.route = stage; self.idx = k; self.stuck = 0.0;
            return self.drive(v, dt);
        }
        let remaining = self.route.length() - self.route.s[i];
        if remaining < 12.0 && speed.abs() < 1.5 { cmd.done = true; cmd.input = Input { brake: true, ..Default::default() }; return cmd; }

        // recovery: flipped, stuck, or far from the route
        let want = self.route.speed[(i + 4).min(self.route.len() - 1)];
        if v.flipped() { cmd.reset = true; self.recoveries += 1; self.stuck = 0.0; }
        self.stuck = if speed.abs() < 1.0 && want > 3.0 { self.stuck + dt } else { 0.0 };
        if self.stuck > self.cfg.stuck_time || d > 25.0 {
            let p = self.route.p[i]; let (_, q) = self.route.ahead(i, 8.0);
            cmd.teleport = Some((v3(p.x, p.y + 1.1, p.z), (q.x - p.x).atan2(q.z - p.z)));
            self.recoveries += 1; self.stuck = 0.0;
            return cmd;
        }

        // steering: pure pursuit to the point `look` metres ahead
        let look = self.cfg.look_base + self.cfg.look_gain * v.vel.len();
        let (_, t) = self.route.ahead(i, look);
        let (dx, dz) = (t.x - v.pos.x, t.z - v.pos.z);
        let ld = dx.hypot(dz).max(1.0);
        let right = v.right();
        let lat = dx * right.x + dz * right.z;
        let wheelbase = (self.car.wheels[0].z - self.car.wheels[2].z).abs().max(1.0);
        let delta = (2.0 * wheelbase * lat / (ld * ld)).atan();
        let max_a = self.car.steer_max / (1.0 + v.vel.len() / self.car.steer_speed_ref);
        let steer = (delta / max_a).clamp(-1.0, 1.0);

        // speed: the profile's target a few points ahead, throttle and brake from the error
        let e = want - speed;
        let (throttle, brake, amt) = if e > 0.3 { ((0.25 + 0.35 * e).clamp(0.0, 1.0), false, 0.0) }
            else if e < -0.7 { (0.0, true, (-e * 0.3).clamp(0.1, 1.0)) } else { (0.0, false, 0.0) };
        cmd.input = Input { throttle, steer, brake, brake_amt: amt, ..Default::default() };
        cmd
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
