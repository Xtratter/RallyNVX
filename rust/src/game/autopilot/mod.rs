//! The autopilot: joins the special stage and drives it to the finish (see docs/superpowers/specs/2026-10-14-autopilot-design.md).
pub mod cfg;
pub mod join;
pub mod route;
pub mod streets;
#[cfg(test)]
mod stage_tests;

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
    /// Route progress (m) where the stage was joined from the field: the speed is kept tame for 80 m after it.
    joined_at: Option<f32>,
    /// How many times the recovery had to act (the tests want zero).
    pub recoveries: u32,
}

/// The speed limits for a car on a road of the given kind (loose gravel is the worst case on the stage).
pub fn limits_for(car: &VehicleSpec, surface: super::surface::Surface, v_max: f32, stop_at_end: bool) -> Limits {
    Limits { mu: car.mu * grip(surface, 1.0).mu, v_max: v_max.min(car.max_speed), accel: 0.5 * car.engine_force / car.mass, stop_at_end }
}

/// The route along a road (its centre pulled to the least-curvature line), ending in a standstill.
pub fn route_for_road(road: &Road, car: &VehicleSpec, c: &AutoCfg) -> Route {
    let centre: Vec<V3> = road.pts.iter().map(|p| p.pos).collect();
    let width: Vec<f32> = road.pts.iter().map(|p| p.width).collect();
    Route::build(&centre, &width, limits_for(car, road.kind.surface(), f32::MAX, true), c)
}

/// The whole drive: the liaison, the city streets between its end and the stage start, then the stage, as one route with
/// the grip of each part, ending in a standstill. `None` when the level has no stage.
pub fn chain_route(t: &super::terrain::Terrain, car: &VehicleSpec, c: &AutoCfg) -> Option<Route> {
    use super::roads::RoadKind;
    let net = t.roads()?;
    let stage = net.roads.iter().find(|r| r.kind == RoadKind::Stage)?;
    let mu_of = |s: super::surface::Surface| car.mu * grip(s, 1.0).mu;
    let (mut centre, mut width, mut mu): (Vec<V3>, Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new(), Vec::new());
    let mut push = |p: V3, w: f32, m: f32| {
        if centre.last().map_or(true, |q| (*q - p).len() > 1.5) { centre.push(p); width.push(w); mu.push(m); }
    };
    if let Some(l) = net.roads.iter().find(|r| r.kind == RoadKind::Liaison) {
        for p in &l.pts { push(p.pos, p.width, mu_of(l.kind.surface())); }
        let (a, b) = (l.pts.last()?.pos, stage.pts.first()?.pos);
        if let Some(site) = t.sites().into_iter().min_by(|x, y| ((x.centre - a).len()).partial_cmp(&(y.centre - a).len()).unwrap_or(std::cmp::Ordering::Equal)) {
            for p in streets::city_path(&site, a, b) { push(p, 12.0, mu_of(super::surface::Surface::Asphalt)); }
        }
    }
    for p in &stage.pts { push(p.pos, p.width, mu_of(stage.kind.surface())); }
    if centre.len() < 7 { return None; }
    let lim = Limits { mu: 1.0, v_max: car.max_speed, accel: 0.5 * car.engine_force / car.mass, stop_at_end: true };
    Some(Route::build_with(&centre, &width, &mu, lim, c))
}

impl Autopilot {
    /// An autopilot for a car that is already on (or next to) `route`.
    pub fn on_route(route: Route, car: VehicleSpec, v: &Vehicle) -> Autopilot {
        let idx = route.nearest(v.pos.x, v.pos.z).0;
        Autopilot { route, car, cfg: cfg(), idx, stuck: 0.0, next: None, joined_at: None, recoveries: 0 }
    }

    /// Starts from wherever the car is: on the road network it joins at the nearest point of the whole chain (liaison,
    /// the city streets, the stage), elsewhere it first drives a field route to the nearest road point. `None` when the
    /// level has no stage.
    pub fn start(t: &super::terrain::Terrain, v: &Vehicle, car: VehicleSpec) -> Option<Autopilot> {
        let c = cfg();
        let chain = chain_route(t, &car, &c)?;
        let (k, d) = chain.nearest(v.pos.x, v.pos.z);
        if d <= 15.0 { return Some(Autopilot::on_route(chain, car, v)); }
        let to = chain.p[k];
        let Some(field) = join::field_route(t, &car, (v.pos.x, v.pos.z), (to.x, to.z), &c) else { return Some(Autopilot::on_route(chain, car, v)) };
        let mut ap = Autopilot::on_route(field, car, v);
        ap.next = Some((chain, k));
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
            self.route = stage; self.idx = k; self.stuck = 0.0; self.joined_at = Some(self.route.s[k]);
            return self.drive(v, dt);
        }
        let remaining = self.route.length() - self.route.s[i];
        if remaining < 12.0 && speed.abs() < 1.5 { cmd.done = true; cmd.input = Input { brake: true, ..Default::default() }; return cmd; }

        // recovery: flipped, stuck, or far from the route
        // the profile's target a few points ahead (a crawl is kept until the last 12 m so the car does not stall short of the finish)
        let want = self.route.speed[(i + 4).min(self.route.len() - 1)].max(if remaining > 12.0 { 3.5 } else { 0.0 });
        let tame = self.joined_at.map_or(f32::MAX, |s0| if self.route.s[i] - s0 < 80.0 { self.cfg.field_speed * 1.25 } else { f32::MAX });
        let want = want.min(tame);
        if v.flipped() { cmd.reset = true; self.recoveries += 1; self.stuck = 0.0; }
        self.stuck = if speed.abs() < 1.0 && want > 3.0 { self.stuck + dt } else { 0.0 };
        if remaining < 25.0 && self.stuck > 2.0 { cmd.done = true; return cmd; }   // stopped just short of the line
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
