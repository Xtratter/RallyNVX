//! The autopilot: joins the special stage and drives it to the finish (see docs/superpowers/specs/2026-10-14-autopilot-design.md).
pub mod cfg;
pub mod join;
pub mod model;
pub mod train;
pub mod job;
pub mod nn;
pub mod sense;
pub mod learn;
pub mod es;
pub mod route;
pub mod shoulder;
pub mod streets;
#[cfg(test)]
mod stage_tests;
#[cfg(test)]
mod suite_tests;

use self::cfg::{cfg, cfg_style, AutoCfg};
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

/// How often Max Attack used each rally technique in a drive (for the diagnosis and the tests): the Scandinavian flick (`pendulum`), the handbrake turn, lift-off rotation (`lift`),
/// the seconds of left-foot braking (`trail_s`) and of a held power slide (`slide_s`), the jumps (airborne for over 0.3 s) and the metres cut beyond the road's edge (`cut_m`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Techniques { pub pendulum: u32, pub handbrake: u32, pub lift: u32, pub trail_s: f32, pub slide_s: f32, pub jumps: u32, pub cut_m: f32 }

/// The seconds between two measurements of the car while it drives.
const REMEASURE_S: f32 = 12.0;

/// The gain of the line's sideways offset in the steering (1/s of the offset in metres over the speed).
const STANLEY: f32 = 0.7;
const K_MAX: f32 = 1.0;

pub struct Autopilot {
    route: Route,
    car: VehicleSpec,
    cfg: AutoCfg,
    style: i32,
    idx: usize,
    stuck: f32,
    /// The handbrake flick in progress (seconds left) and the time until the next one may start.
    flick: f32, cooldown: f32,
    /// Max Attack's pendulum (the Scandinavian flick): seconds of steering away from the coming bend left, the side of that bend (+1 right) and the time until the next one may start.
    pend: f32, pend_dir: f32, pend_cool: f32,
    /// Lift-off rotation: seconds of lifted throttle left and the time until the next one; the technique counters and the seconds in the air.
    lift: f32, lift_cool: f32, pub tech: Techniques, air_t: f32,
    /// The sensors' feature vector of this step (set by `sense`), the car's velocity of the step before, whether the samples recorded are the sensors' (not the route's) and the last command.
    /// A measurement of the car's present state under way on another thread, and the seconds since the last one: the models are measured again every 12 s (the tyres warm up, wear, the road dries).
    remeasure: Option<std::sync::mpsc::Receiver<model::Models>>, since_measure: f32,
    sensor: Option<Vec<f32>>, prev_vel: V3, sensor_mode: bool, last_cmd: (f32, f32), mem: Vec<[f32; sense::MEM]>, mem_step: u32,
    /// The stage route and where to join it, while a field route is being driven.
    next: Option<(Route, usize)>,
    /// Route progress (m) where the stage was joined from the field: the speed is kept tame for 80 m after it.
    joined_at: Option<f32>,
    /// How many times the recovery had to act (the tests want zero).
    pub recoveries: u32,
    /// What the car measured of itself, and how much of the planned cornering grip it dares to use (`k`, 0.6..1: it falls while the car slides or nears the road's edge, and
    /// recovers while all is calm); `k_planned` is the value the speed plan was last made with.
    models: Option<model::Models>, k: f32, k_planned: f32,
    /// A neural policy that drives instead of the geometric controller (the controller still runs: it gives the labels and takes over when the car is lost), the seconds of the takeover left, and the recorded (features, expert action) pairs.
    policy: Option<std::sync::Arc<nn::Mlp>>, nn_hold: f32, takeover_time: f32, record: bool, samples: Vec<(Vec<f32>, [f32; nn::O])>, rec_step: u32,
}

/// The speed limits for a car on a road of the given kind (loose gravel is the worst case on the stage).
/// The share of the planned speed the autopilot keeps on ground that is `wet` (0..1) soaked: the square root of the grip left, because a corner's speed goes with the root of the
/// grip (and the braking distance, which goes with speed squared over grip, stays what it was). Never below half.
pub fn wet_speed_scale(surface: super::surface::Surface, wet: f32) -> f32 {
    super::grip::wet_factor(surface, wet).0.max(0.0).sqrt().clamp(0.5, 1.0)
}

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
    let lim = Limits { mu: 1.0, v_max: car.max_speed * c.speed_cap, accel: 0.5 * car.engine_force / car.mass, stop_at_end: true };
    // Max Attack may use the shoulder where the ground allows
    let ext = if c.attack > 0.5 { let half: Vec<f32> = width.iter().map(|w| w * 0.5).collect(); Some(shoulder::extents(t, &centre, &half)) } else { None };
    Some(Route::build_ext(&centre, &width, &mu, lim, c, ext.as_deref()))
}

/// The city loop from its point `k` through `laps` laps, ending on the start/finish line.
pub fn loop_route(l: &super::superspecial::SuperSpecial, k: usize, car: &VehicleSpec, c: &AutoCfg) -> Route {
    let mut centre: Vec<V3> = l.pts[k..].to_vec();
    // a car just before the line (a stage start) has all its laps ahead of it; elsewhere the rest of the lap counts as one
    let before_line = l.pts.len() - k <= 20;
    let full = if before_line { c.laps } else { c.laps.saturating_sub(1) };
    for _ in 0..full { centre.extend(l.pts[1..].iter().cloned()); }
    centre.extend(l.pts[1..8].iter().cloned());   // a little past the line, so the finish is crossed, not just reached
    if centre.len() < 7 { centre.extend(l.pts[1..8].iter().cloned()); }
    let mu = vec![car.mu * grip(super::surface::Surface::Asphalt, 1.0).mu; centre.len()];
    let lim = Limits { mu: 1.0, v_max: car.max_speed * c.speed_cap, accel: 0.5 * car.engine_force / car.mass, stop_at_end: true };
    Route::build_with(&centre, &vec![12.0; centre.len()], &mu, lim, c)
}

impl Autopilot {
    /// An autopilot for a car that is already on (or next to) `route`.
    pub fn on_route(route: Route, car: VehicleSpec, v: &Vehicle) -> Autopilot { Autopilot::on_route_styled(route, car, v, 1) }
    /// The same with a driving style: 0 calm, 1 normal, 2 aggressive.
    pub fn on_route_styled(route: Route, car: VehicleSpec, v: &Vehicle, style: i32) -> Autopilot { Autopilot::on_route_cfg(route, car, v, style, cfg_style(style)) }
    /// The same with the numbers given (the self-training tries many sets at once, so they cannot come from the global style).
    pub fn on_route_cfg(route: Route, car: VehicleSpec, v: &Vehicle, style: i32, cfg: AutoCfg) -> Autopilot {
        let idx = route.nearest(v.pos.x, v.pos.z).0;
        Autopilot { route, car, cfg, style: style.clamp(0, 6), idx, stuck: 0.0, flick: 0.0, cooldown: 0.0, pend: 0.0, pend_dir: 0.0, pend_cool: 0.0, lift: 0.0, lift_cool: 0.0, tech: Techniques::default(), air_t: 0.0, remeasure: None, since_measure: 0.0, sensor: None, prev_vel: V3::ZERO, sensor_mode: false, last_cmd: (0.0, 0.0), mem: Vec::new(), mem_step: 0, next: None, joined_at: None, recoveries: 0, models: None, k: 1.0, k_planned: 1.0, policy: None, nn_hold: 0.0, takeover_time: 0.0, record: false, samples: Vec::new(), rec_step: 0 }
    }

    /// Starts from wherever the car is: on the road network it joins at the nearest point of the whole chain (liaison,
    /// the city streets, the stage), elsewhere it first drives a field route to the nearest road point. `None` when the
    /// level has no stage.
    pub fn start(t: &super::terrain::Terrain, v: &Vehicle, car: VehicleSpec, style: i32) -> Option<Autopilot> { Autopilot::start_with(t, v, car, style, None) }

    /// As `start`; `force_loop` decides between the city loop (`Some(true)`) and the road chain (`Some(false)`) instead of
    /// the distance (a rally stage knows which one it is).
    pub fn start_with(t: &super::terrain::Terrain, v: &Vehicle, car: VehicleSpec, style: i32, force_loop: Option<bool>) -> Option<Autopilot> { Autopilot::start_cfg(t, v, car, style, cfg_style(style), force_loop) }

    /// As `start_with`, with the numbers given.
    pub fn start_cfg(t: &super::terrain::Terrain, v: &Vehicle, car: VehicleSpec, style: i32, c: AutoCfg, force_loop: Option<bool>) -> Option<Autopilot> {
        let chain = chain_route(t, &car, &c);
        // on the city loop: its laps (the start/finish line is the end)
        if let Some(l) = t.city_loop() {
            let (k, dl) = l.pts.iter().enumerate().map(|(i, p)| (i, (p.x - v.pos.x).hypot(p.z - v.pos.z))).fold((0, f32::INFINITY), |a, b| if b.1 < a.1 { b } else { a });
            let dc = chain.as_ref().map_or(f32::INFINITY, |r| r.nearest(v.pos.x, v.pos.z).1);
            if force_loop.unwrap_or(dl <= 20.0 && dl <= dc) { return Some(Autopilot::on_route_cfg(loop_route(l, k, &car, &c), car, v, style, c)); }
        }
        let chain = chain?;
        let (k, d) = chain.nearest(v.pos.x, v.pos.z);
        if d <= 15.0 { return Some(Autopilot::on_route_cfg(chain, car, v, style, c)); }
        let to = chain.p[k];
        let Some(field) = join::field_route(t, &car, (v.pos.x, v.pos.z), (to.x, to.z), &c) else { return Some(Autopilot::on_route_cfg(chain, car, v, style, c)) };
        let mut ap = Autopilot::on_route_cfg(field, car, v, style, c);
        ap.next = Some((chain, k));
        Some(ap)
    }

    /// One step: the geometric controller's command; with a policy set the net's command instead (the controller takes over for two seconds when the car is 6 m off the line or flipped); with recording on, the (features, controller's action) pair is kept (every third step).
    pub fn drive(&mut self, v: &Vehicle, dt: f32) -> AutoCmd {
        let mut cmd = self.drive_expert(v, dt);
        if (self.policy.is_none() && !self.record) || cmd.teleport.is_some() || cmd.reset || cmd.done { return cmd; }
        let sensor_net = self.policy.as_ref().map_or(self.sensor_mode, |p| p.n_in != nn::F);
        let f: Vec<f32> = if sensor_net { match &self.sensor { Some(s) => s.clone(), None => return cmd } } else { self.features(v).to_vec() };
        if self.record {
            self.rec_step += 1;
            if self.rec_step % 3 == 0 {
                let i = cmd.input;
                let accel = if i.brake { -(i.brake_amt.max(0.1)) } else { i.throttle };
                self.samples.push((f.clone(), [i.steer, accel, if i.handbrake { 1.0 } else { -1.0 }]));
            }
        }
        if let Some(p) = self.policy.clone() {
            self.nn_hold = (self.nn_hold - dt).max(0.0);
            if self.nn_hold > 0.0 { self.takeover_time += dt; }
            if self.route.cross(self.idx, v.pos.x, v.pos.z) > 6.0 || v.flipped() { self.nn_hold = 2.0; }
            if self.nn_hold <= 0.0 {
                let o = p.forward(&f);
                cmd.input = Input { steer: o[0], throttle: o[1].max(0.0), brake: o[1] < -0.1, brake_amt: if o[1] < -0.1 { (-o[1]).clamp(0.1, 1.0) } else { 0.0 }, handbrake: o[2] > 0.3, ..Default::default() }.sanitized();
            }
        }
        self.last_cmd = (cmd.input.throttle, if cmd.input.brake { cmd.input.brake_amt.max(0.1) } else { 0.0 });
        cmd
    }

    /// Counts the time since the car was last measured; `true` when it is time to measure it again (and no measurement is under way).
    pub fn tick_remeasure(&mut self, dt: f32) -> bool {
        if self.models.is_none() { return false; }
        self.since_measure += dt;
        self.since_measure >= REMEASURE_S && self.remeasure.is_none()
    }
    /// Measures the car again on another thread from a copy of its present state; `drive` takes the result when it is ready.
    pub fn start_remeasure(&mut self, setup: super::world::ProbeSetup) {
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new().name("autopilot-measure".into()).spawn(move || { let _ = tx.send(model::Models::measure_setup(&setup)); });
        if spawned.is_ok() { self.remeasure = Some(rx); }
        self.since_measure = 0.0;
    }
    /// Takes a finished measurement and plans the speeds again with it (the daring `k` as it is).
    fn poll_remeasure(&mut self) {
        let Some(rx) = self.remeasure.as_ref() else { return };
        match rx.try_recv() {
            Ok(m) => {
                self.remeasure = None;
                let mut c = self.cfg; c.lat_factor *= self.k * self.k;
                self.route.replan(&m, &c);
                if let Some((r, _)) = self.next.as_mut() { r.replan(&m, &c); }
                self.models = Some(m); self.k_planned = self.k;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(_) => { self.remeasure = None; }
        }
    }

    /// Whether the net (or the recording) needs the sensors' vector this step.
    pub fn wants_sensors(&self) -> bool { self.policy.as_ref().map_or(self.sensor_mode, |p| p.n_in != nn::F) }
    /// Reads the sensors for this step (call before `drive`): the rays and the car's own metrics, with the route's coarse direction hints.
    pub fn sense(&mut self, t: &super::terrain::Terrain, v: &Vehicle, mut tel: sense::Telemetry, dt: f32) {
        tel.throttle = self.last_cmd.0; tel.brake = self.last_cmd.1;
        let (fwd, right) = (v.forward(), v.right());
        let mut hint = [0.0f32; 4];
        for (k, d) in [40.0f32, 100.0].iter().enumerate() {
            let (_, p) = self.route.ahead(self.idx, *d);
            let (dx, dz) = (p.x - v.pos.x, p.z - v.pos.z); let l = dx.hypot(dz).max(1.0);
            hint[2 * k] = (dx * right.x + dz * right.z) / l; hint[2 * k + 1] = (dx * fwd.x + dz * fwd.z) / l;
        }
        // the short memory: one set of key values every 0.1 s (kept for 0.6 s); the features carry how they changed over 0.2 s and 0.5 s
        let mut now = sense::key_values(v, &tel);
        if dt > 1e-4 { let acc = (v.vel - self.prev_vel) * (1.0 / dt); now[2] = ((acc.x * right.x + acc.z * right.z) / 10.0).clamp(-1.5, 1.5); now[3] = ((acc.x * fwd.x + acc.z * fwd.z) / 10.0).clamp(-1.5, 1.5); }
        self.mem_step += 1;
        if self.mem_step % 6 == 1 { self.mem.push(now); if self.mem.len() > 7 { self.mem.remove(0); } }
        let mut memory = [0.0f32; 2 * sense::MEM];
        for (slot, back) in [2usize, 5].iter().enumerate() {
            if let Some(old) = self.mem.len().checked_sub(1 + back).and_then(|i| self.mem.get(i)) { for k in 0..sense::MEM { memory[slot * sense::MEM + k] = (now[k] - old[k]).clamp(-2.0, 2.0); } }
        }
        self.sensor = Some(sense::features(t, v, &tel, self.prev_vel, dt, hint, &memory));
        self.prev_vel = v.vel;
    }
    /// Makes the recorded samples the sensors' (for a net that reads them) instead of the route's.
    pub fn set_sensor_mode(&mut self, on: bool) { self.sensor_mode = on; }

    /// Sets (or removes) the neural policy and turns recording on or off.
    pub fn set_policy(&mut self, policy: Option<std::sync::Arc<nn::Mlp>>, record: bool) { self.policy = policy; self.record = record; self.nn_hold = 0.0; }
    /// The daring (0.6..1) the autopilot dares to use of the planned cornering grip now.
    pub fn daring(&self) -> f32 { self.k }
    /// The rally techniques used so far.
    pub fn techniques(&self) -> Techniques { self.tech }
    /// The cornering grip (m/s^2) of the latest measurement on gravel, at its plateau; `None` before there is a measurement.
    pub fn measured_grip(&self) -> Option<f32> { self.models.as_ref().map(|m| m.list.last().map_or(0.0, |x| x.1.lateral_plateau())) }
    /// The seconds the controller has driven in place of the policy (a measure of how often the net got lost).
    pub fn takeover_time(&self) -> f32 { self.takeover_time }
    /// The recorded pairs since the last call.
    pub fn take_samples(&mut self) -> Vec<(Vec<f32>, [f32; nn::O])> { std::mem::take(&mut self.samples) }

    /// What the net sees: the speed, how the car sits on the line (offset, heading, yaw rate, side slip), the bend ahead as the chord's side at six distances, the planned speed at those distances and here,
    /// the road's width and where the car is on it, the grip, the daring and whether it is on the ground.
    pub fn features(&self, v: &Vehicle) -> [f32; nn::F] {
        const D: [f32; 6] = [5.0, 10.0, 16.0, 24.0, 36.0, 52.0];
        let i = self.idx.min(self.route.len() - 1);
        let right = v.right(); let fwd = v.forward();
        let mut f = [0.0f32; nn::F];
        let speed = v.vel.len();
        let (vf, vr) = (v.vel.x * fwd.x + v.vel.z * fwd.z, v.vel.x * right.x + v.vel.z * right.z);
        f[0] = speed / 30.0;
        let pi = self.route.p[i];
        f[1] = (((pi.x - v.pos.x) * right.x + (pi.z - v.pos.z) * right.z) / 4.0).clamp(-1.5, 1.5);
        let (_, t5) = self.route.ahead(i, 6.0);
        f[2] = ((t5.x - pi.x) * right.x + (t5.z - pi.z) * right.z).atan2(((t5.x - pi.x) * fwd.x + (t5.z - pi.z) * fwd.z).max(0.1)).clamp(-1.5, 1.5);
        f[3] = (v.omega.dot(v.up()) / 2.0).clamp(-2.0, 2.0);
        f[4] = vr.atan2(vf.abs().max(1.0)).clamp(-1.5, 1.5);
        for (k, d) in D.iter().enumerate() {
            let (j, t) = self.route.ahead(i, *d);
            let (dx, dz) = (t.x - v.pos.x, t.z - v.pos.z); let ld = dx.hypot(dz).max(1.0);
            f[5 + k] = ((dx * right.x + dz * right.z) / ld).clamp(-1.5, 1.5);
            f[11 + k] = self.route.speed[j] / 30.0;
        }
        f[17] = self.route.speed[i] / 30.0;
        let half = self.route.half[i].max(1.0);
        let c = self.route.c[i];
        f[18] = (((c.x - v.pos.x) * right.x + (c.z - v.pos.z) * right.z) / half).clamp(-1.5, 1.5);
        f[19] = half / 10.0;
        f[20] = self.route.mu[i];
        f[21] = self.k;
        f[22] = if v.contacts >= 3 { 1.0 } else { 0.0 };
        f[23] = (v.steer / self.car.steer_max.max(0.1)).clamp(-1.5, 1.5);
        f
    }

    /// Plans the speeds again with what the car measured of itself (see `model`): the route and, when a field route is being driven, the stage behind it.
    pub fn apply_models(&mut self, models: &model::Models) {
        self.models = Some(models.clone()); self.k = 1.0; self.k_planned = 1.0;
        self.route.replan(models, &self.cfg);
        if let Some((r, _)) = self.next.as_mut() { r.replan(models, &self.cfg); }
    }

    /// The car's daring: down while it slides or nears the road's edge, slowly back up while all is calm; the speed plan follows (the corners' speeds go with the root of the grip used).
    fn adapt(&mut self, v: &Vehicle, i: usize, d: f32, dt: f32) {
        let Some(m) = self.models.clone() else { return };
        let slip = v.wheels.iter().filter(|w| w.grounded).map(|w| w.lat_slip.abs()).fold(0.0f32, f32::max);
        let room = self.route.room(i, v.pos.x, v.pos.z);
        let at = self.cfg.attack;   // Max Attack tolerates more: bigger slides, wandering off the line, the edge itself
        let ok = 1.0 + 2.0 * self.cfg.slide + at;
        let wide = self.route.half[i] > 1.0 && d > 1.2 + 2.0 * at;   // far off the planned line: it runs wide
        if slip > ok || wide || room.map_or(false, |r| r < 0.8 - 1.2 * at) { self.k -= 0.9 * dt; }
        else if slip < 0.6 * ok && d < 0.7 + at { self.k += 0.03 * dt; }
        self.k = self.k.clamp(0.6, K_MAX);
        if (self.k - self.k_planned).abs() > 0.03 {
            self.k_planned = self.k;
            let mut c = self.cfg; c.lat_factor *= self.k * self.k;
            self.route.replan(&m, &c);
            if let Some((r, _)) = self.next.as_mut() { r.replan(&m, &c); }
        }
    }

    /// 0 calm, 1 normal, 2 aggressive, 3 the player's own numbers, 4 the learned ones, 5 Max Attack, 6 the neural net.
    pub fn style(&self) -> i32 { self.style }
    pub fn route(&self) -> &Route { &self.route }
    pub fn progress(&self) -> f32 { self.route.s[self.idx] }

    fn drive_expert(&mut self, v: &Vehicle, dt: f32) -> AutoCmd {
        self.poll_remeasure();
        let wet_scale = wet_speed_scale(v.surface, v.wet);
        let mut cmd = AutoCmd::default();
        let (lo, hi) = (self.idx.saturating_sub(15), self.idx + 80);
        let (mut i, mut d) = self.route.nearest_in(v.pos.x, v.pos.z, lo, hi);
        if d > 15.0 { let all = self.route.nearest(v.pos.x, v.pos.z); i = all.0; d = all.1; }
        self.idx = i;
        let cross = self.route.cross(i, v.pos.x, v.pos.z);
        self.adapt(v, i, cross, dt);
        if self.cfg.attack > 0.5 { if let Some((r, shoulder)) = self.route.room_split(i, v.pos.x, v.pos.z) { if r - shoulder < 0.0 && shoulder > 0.0 { self.tech.cut_m += v.speed().abs() * dt; } } }
        let speed = v.speed();
        if self.next.is_some() && self.route.length() - self.route.s[i] < 14.0 {
            let (stage, k) = self.next.take().unwrap();
            self.route = stage; self.idx = k; self.stuck = 0.0; self.joined_at = Some(self.route.s[k]);
            return self.drive_expert(v, dt);
        }
        let remaining = self.route.length() - self.route.s[i];
        if remaining < 12.0 && speed.abs() < 1.5 { cmd.done = true; cmd.input = Input { brake: true, ..Default::default() }; return cmd; }

        // recovery: flipped, stuck, or far from the route
        // the profile's target a few points ahead (a crawl is kept until the last 12 m so the car does not stall short of the finish)
        let want = self.route.speed[(i + 4).min(self.route.len() - 1)].max(if remaining > 12.0 { 3.5 } else { 0.0 });
        let tame = self.joined_at.map_or(f32::MAX, |s0| if self.route.s[i] - s0 < 80.0 { self.cfg.field_speed * 1.25 } else { f32::MAX });
        let want = want.min(tame) * wet_scale;
        // off the line: slow down in proportion to how far into the road's edge the car has drifted
        let half = self.route.half[i];
        let want = if half > 1.0 && d > (0.55 + 0.6 * self.cfg.attack) * half { want * (1.0 - (d - 0.55 * half) / (0.6 * half)).clamp(0.45, 1.0) } else { want };
        // well beyond the shoulder (more than a metre past what the ground allows): slow right down and come back
        let want = if self.cfg.attack > 0.5 && self.route.room(i, v.pos.x, v.pos.z).map_or(false, |r| r < -1.0) { want * 0.5 } else { want };
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
        let mut look = self.cfg.look_base + self.cfg.look_gain * v.vel.len();
        // in a tight bend the look-ahead point must not lie far round the arc: a chord over a hairpin cuts the inside
        if self.models.is_some() { let (j, _) = self.route.ahead(i, look); let km = self.route.kraw[i..=j].iter().cloned().fold(0.0f32, f32::max); if km > 1e-3 { look = look.min((0.7 / km).max(3.5)); } }
        let (_, t) = self.route.ahead(i, look);
        let (dx, dz) = (t.x - v.pos.x, t.z - v.pos.z);
        let ld = dx.hypot(dz).max(1.0);
        let right = v.right();
        let lat = dx * right.x + dz * right.z;
        let wheelbase = (self.car.wheels[0].z - self.car.wheels[2].z).abs().max(1.0);
        let long = dx * v.forward().x + dz * v.forward().z;
        let (vf, vr) = (v.vel.x * v.forward().x + v.vel.z * v.forward().z, v.vel.x * right.x + v.vel.z * right.z);
        // slide assist: aim the velocity (not the nose) at the target and add the drift angle as counter-steer
        let beta = if self.cfg.slide > 0.0 && v.vel.len() > 5.0 && vf > 0.0 { vr.atan2(vf) * self.cfg.slide } else { 0.0 };
        let alpha = lat.atan2(long.max(0.1)) - beta;
        // with a model also the line's sideways offset from the car steers (Stanley's term: the stronger the slower), so a car drifting wide comes back before the next bend
        let pi = self.route.p[i];
        let e_line = ((pi.x - v.pos.x) * right.x + (pi.z - v.pos.z) * right.z).clamp(-5.0, 5.0);
        let stanley = if self.models.is_some() { (e_line.signum() * (e_line.abs() - 0.8).max(0.0) * STANLEY).atan2(2.0 + v.vel.len()) } else { 0.0 };
        let delta = (2.0 * wheelbase * alpha.sin() / ld).atan() + beta + stanley;
        let max_a = self.car.steer_max / (1.0 + v.vel.len() / self.car.steer_speed_ref);
        let mut steer = (delta / max_a).clamp(-1.0, 1.0);
        if self.cfg.attack > 0.5 {
            // the Scandinavian flick: before a fast bend the car is thrown the other way for a moment, so the rear swings out and the turn-in is already a slide
            self.pend_cool = (self.pend_cool - dt).max(0.0);
            if self.pend > 0.0 { self.pend -= dt; steer = (-self.pend_dir * 0.5).clamp(-1.0, 1.0); }
            else if self.pend_cool <= 0.0 && speed > 15.0 && speed < 20.0 && v.contacts >= 3 && steer.abs() < 0.35 && self.route.room(i, v.pos.x, v.pos.z).map_or(true, |r| r > 2.5) {
                let d0 = 3.0 + 0.7 * speed;
                let (j, _) = self.route.ahead(i, d0); let (j2, t2) = self.route.ahead(i, d0 + 22.0);
                let k_ahead = self.route.kraw[j..=j2].iter().cloned().fold(0.0f32, f32::max);
                if k_ahead > 0.015 && k_ahead < 0.04 {
                    self.pend = 0.18; self.pend_cool = 5.0; self.tech.pendulum += 1;
                    self.pend_dir = ((t2.x - v.pos.x) * right.x + (t2.z - v.pos.z) * right.z).signum();
                }
            }
            if v.contacts == 0 { steer *= 0.2; }   // in the air the wheels stay straight for the landing
        }

        if self.cfg.attack > 0.5 {
            // a jump: counted when the car has been in the air for 0.3 s
            if v.contacts == 0 { self.air_t += dt; if self.air_t > 0.3 && self.air_t - dt <= 0.3 { self.tech.jumps += 1; } } else { self.air_t = 0.0; }
            // lift-off rotation: a short lift of the throttle at the turn-in of a medium bend moves the weight to the front and lets the rear swing round (the flick covers 15 to 20 m/s)
            self.lift_cool = (self.lift_cool - dt).max(0.0);
            if self.lift > 0.0 { self.lift -= dt; }
            else if self.lift_cool <= 0.0 && self.pend <= 0.0 && speed > 20.0 && speed < 32.0 && v.contacts >= 3 && steer.abs() < 0.3 && self.route.room(i, v.pos.x, v.pos.z).map_or(true, |r| r > 2.0) {
                let d0 = 2.0 + 0.45 * speed;
                let (j, _) = self.route.ahead(i, d0); let (j2, _) = self.route.ahead(i, d0 + 18.0);
                let k_ahead = self.route.kraw[j..=j2].iter().cloned().fold(0.0f32, f32::max);
                if k_ahead > 0.01 && k_ahead < 0.03 { self.lift = 0.3; self.lift_cool = 4.0; self.tech.lift += 1; }
            }
            // a power slide is held with the throttle: on while the car is not yet sideways, eased when the angle grows too large
            let side = v.wheels.iter().filter(|w| w.grounded).map(|w| w.lat_slip.abs()).fold(0.0f32, f32::max);
            if speed > 10.0 && side > 2.0 && v.contacts >= 3 { self.tech.slide_s += dt; }
        }
        // speed: the profile's target a few points ahead, throttle and brake from the error
        let e = want - speed;
        let (throttle, brake, amt) = if self.cfg.attack > 0.5 {
            // Max Attack: flat out until the plan says brake, then hard
            let air = v.contacts == 0;
            let slip_angle = vr.atan2(vf.abs().max(1.0)).abs();
            let thr = if air { 0.4 } else if self.lift > 0.0 { 0.1 } else if slip_angle > 0.5 && steer.abs() > 0.2 { 0.1 } else if slip_angle > 0.3 && steer.abs() > 0.2 { 0.4 } else { 1.0 };
            if e > -0.5 { (thr, false, 0.0) }
            // braking into a bend with the throttle kept a little open (left-foot braking): the rear stays light and the car rotates
            else if e < -1.2 { if steer.abs() > 0.2 && speed > 8.0 { self.tech.trail_s += dt; } (if steer.abs() > 0.2 && speed > 8.0 { 0.3 } else { 0.0 }, true, (-e * 0.4).clamp(0.2, 1.0) * if steer.abs() > 0.2 { 0.8 } else { 1.0 }) } else { (0.0, false, 0.0) }
        } else if e > 0.3 { ((0.25 + 0.35 * e).clamp(0.0, 1.0), false, 0.0) }
            else if e < -0.7 { (0.0, true, (-e * 0.3).clamp(0.1, 1.0)) } else { (0.0, false, 0.0) };
        // handbrake flick: into a tight bend taken above the grip limit the rear is let go for a moment so the car rotates
        self.cooldown = (self.cooldown - dt).max(0.0);
        let mut handbrake = false;
        if self.cfg.slide > 0.0 {
            if self.flick > 0.0 { self.flick -= dt; handbrake = true; }
            else if self.cooldown <= 0.0 && speed > 6.0 && speed < 18.0 + 8.0 * self.cfg.attack && steer.abs() > 0.8 - 0.2 * self.cfg.attack {
                // only in a hairpin the car cannot turn by itself: the bend ahead is tight, the steering is at its lock and the car turns less than the bend asks (understeer)
                let (j, _) = self.route.ahead(i, 10.0 + 0.8 * speed);
                let k_max = self.route.kraw[i..=j].iter().cloned().fold(0.0, f32::max);
                let need = speed * k_max; let yaw = v.omega.dot(v.up()).abs();
                if k_max > 0.045 - 0.015 * self.cfg.attack && yaw < (0.7 + 0.15 * self.cfg.attack) * need && vf > 0.0 { self.flick = self.cfg.flick_time; self.cooldown = 3.0; handbrake = true; self.tech.handbrake += 1; }
            }
        }
        if handbrake { cmd.input = Input { throttle: 0.0, steer, brake: false, handbrake: true, ..Default::default() }; return cmd; }
        cmd.input = Input { throttle, steer, brake, brake_amt: amt, ..Default::default() };
        cmd
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
