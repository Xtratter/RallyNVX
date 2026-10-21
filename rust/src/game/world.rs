//! Everything that happens in a level: terrain, props, the player's vehicle, the camera.
use std::collections::HashMap;

use super::camera_follow::FollowCam;
use super::dust::{Dust, Particle};
use super::wind;
use super::grip::LooseMap;
use super::carlights::{self, CarLights, Lamps};
use super::city::City;
use super::input::Input;
use super::daylight::{daylight, Daylight};
use super::tracks::Tracks;
use super::math::{v3, V3};
use super::props::{tile_props, Prop};
use super::terrain::{Terrain, TILE_SIZE};
use super::vehicle::{Vehicle, VehicleSpec};

pub const WORLD_LIMIT: f32 = 4000.0;

pub fn tile_of(x: f32) -> i32 { (x / TILE_SIZE).floor() as i32 }

/// A rally stage in progress: the runner, which stage it is (0 the special stage, 1 the city loop), the upside-down timer and
/// the last event for the HUD (code 1 split, 2 reset, 3 off road, 4 skipped lines, 5 finish, 6 go; its value and a serial).
pub struct RallyState { pub run: super::rally::Run, pub stage: u32, flip_t: f32, pub event: (u32, f32, u32), pub pace: super::pacenotes::Playback, pub note: (u32, String, String), pub note_clips: String }

/// Laps of the city loop in a rally.
pub const CITY_LAPS: usize = 2;

pub struct World {
    pub terrain: Terrain,
    pub vehicle: Vehicle,
    pub cam: FollowCam,
    /// The engine, the clutch and the gearbox (see `drivetrain`), and whether the driver chose the manual (the autopilot always drives in the automatic).
    pub drivetrain: super::drivetrain::Drivetrain, pub manual: bool,
    /// The aids that can be switched off and the tuning of the car's parts (see `assists`); `tuned` is the car with the tuning applied.
    pub assists: super::assists::Assists, pub tuning: super::assists::Tuning, tuned: (super::assists::Tuning, VehicleSpec), tuned_dirty: bool,
    pub time: f32,
    pub cities: Vec<City>,
    /// The loose layer of the special stage (worn by the wheels, reset with the level).
    pub loose: LooseMap,
    /// Tyre dust and spray.
    pub dust: Dust,
    /// The level's weather and how wet its ground is.
    pub climate: super::weather::Climate,
    /// How far the wind has carried the clouds (sky shader noise units).
    pub cloud_off: (f32, f32),
    pub tracks: Tracks,
    /// The car this world is built for (a restart or a placement helper makes the same car again).
    pub car_spec: VehicleSpec,
    /// Garage scene: the camera orbit angle (radians); `None` in play.
    orbit: Option<f32>,
    /// The autopilot while it drives (see `game/autopilot`).
    pub autopilot: Option<super::autopilot::Autopilot>,
    /// The rally stage being driven against the clock (`None` in free drive).
    pub rally: Option<RallyState>,
    /// Whether the signs along the stage are drawn (a setting).
    pub signs_on: bool,
    props: HashMap<(i32, i32), Vec<Prop>>,
}

impl World {
    pub fn new(seed: u64) -> World { World::new_with(seed, VehicleSpec::offroader()) }

    pub fn new_with(seed: u64, car: VehicleSpec) -> World { World::from_terrain(Terrain::new(seed), car, (0.0, 0.0), 0.0) }

    /// The game's world: roads generated for the level; the car spawns on the liaison facing along it.
    pub fn with_roads(seed: u64, car: VehicleSpec) -> World {
        let t0 = std::time::Instant::now();
        let terrain = Terrain::with_roads(seed);
        log::info!("roads generated in {} ms", t0.elapsed().as_millis());
        // the spawn: 44 m along the liaison (it starts 40 m behind the old spawn point), facing along it
        let (at, yaw) = terrain.roads().and_then(|n| n.roads.first()).filter(|r| r.pts.len() > 2).map_or(((0.0, 0.0), 0.0), |r| {
            let i = r.pts.iter().position(|p| p.s >= 44.0).unwrap_or(1).clamp(1, r.pts.len() - 2);
            let d = r.pts[i + 1].pos - r.pts[i - 1].pos;
            ((r.pts[i].pos.x, r.pts[i].pos.z), d.x.atan2(d.z))
        });
        World::from_terrain(terrain, car, at, yaw)
    }

    fn from_terrain(terrain: Terrain, car: VehicleSpec, at: (f32, f32), yaw: f32) -> World {
        let seed = terrain.seed;
        let vehicle = Vehicle::new(car, v3(at.0, terrain.height(at.0, at.1) + 1.1, at.1), yaw);
        let cam = FollowCam::new(&vehicle);
        let cities = terrain.sites().iter().map(|s| City::build(*s, seed)).collect();
        let loose = terrain.roads().map_or_else(LooseMap::empty, LooseMap::new);
        World { terrain, vehicle, cam, drivetrain: super::drivetrain::Drivetrain::new(), manual: false, assists: Default::default(), tuning: Default::default(), tuned: (Default::default(), car), tuned_dirty: true, time: 0.0, cities, loose, dust: Dust::new(seed), climate: super::weather::Climate::new(seed), cloud_off: (0.0, 0.0), tracks: Tracks::new(), car_spec: car, orbit: None, autopilot: None, rally: None, signs_on: true, props: HashMap::new() }
    }

    /// Swaps the car (a new spec) without regenerating the level: back at the same spot facing as before.
    /// Turns the autopilot on (it joins the stage from wherever the car is) or off; returns whether it is on afterwards.
    pub fn set_autopilot(&mut self, on: bool) -> bool { self.set_autopilot_style(if on { 1 } else { -1 }) }
    /// Style -1 off, 0 calm, 1 normal, 2 aggressive (slides allowed), 3 custom (the numbers set in the settings).
    pub fn set_autopilot_style(&mut self, style: i32) -> bool {
        self.autopilot = if style >= 0 { super::autopilot::Autopilot::start_with(&self.terrain, &self.vehicle, self.car_spec, style.min(3), self.rally.as_ref().map(|r| r.stage == 1)) } else { None };
        self.autopilot.is_some()
    }
    pub fn autopilot_on(&self) -> bool { self.autopilot.is_some() }

    /// Starts a rally stage (0 the special stage, 1 the city loop): the car is put before the start line facing along
    /// the road, and the countdown begins. Returns false when the level has no such stage.
    pub fn rally_start(&mut self, stage: u32) -> bool {
        use super::lines::{Line, LineKind};
        let mut pace_path: Vec<V3> = Vec::new();
        let mut junctions: Vec<(f32, bool)> = Vec::new();
        let (lines, at, yaw): (Vec<Line>, V3, f32) = if stage == 0 {
            let (Some(net), l) = (self.terrain.roads(), self.terrain.stage_lines()) else { return false };
            let Some(road) = net.roads.iter().find(|r| r.kind == super::roads::RoadKind::Stage) else { return false };
            if l.len() < 2 { return false; }
            let i = road.pts.iter().position(|p| p.s >= 8.0).unwrap_or(1).min(road.pts.len() - 2);
            let d = road.pts[i + 1].pos - road.pts[i].pos;
            let sides: Vec<&super::roads::Road> = net.roads.iter().filter(|r| r.kind == super::roads::RoadKind::Side).collect();
            pace_path = road.pts.iter().map(|p| p.pos).collect();
            junctions = super::signs::stage_junctions(road, &sides).iter().map(|j| (j.0, j.1)).collect();
            (l.to_vec(), road.pts[i].pos, d.x.atan2(d.z))
        } else {
            let Some(c) = self.terrain.city_loop() else { return false };
            let n = c.pts.len();
            let (a, b) = (c.pts[n.saturating_sub(4)], c.pts[n - 1]);
            let start = c.lines[0];
            let checks: Vec<Line> = c.lines.iter().skip(1).cloned().collect();
            let mut seq = vec![start];
            for lap in 0..CITY_LAPS {
                seq.extend(checks.iter().cloned());
                seq.push(Line { kind: if lap + 1 == CITY_LAPS { LineKind::Finish } else { LineKind::Check }, ..start });
            }
            // the lead-in to the line, then the laps
            pace_path = c.pts[n.saturating_sub(4)..].to_vec();
            for _ in 0..CITY_LAPS { pace_path.extend(c.pts[1..].iter().cloned()); }
            let d = b - a;
            (seq, a, d.x.atan2(d.z))
        };
        self.autopilot = None;
        self.vehicle = Vehicle::new(self.car_spec, v3(at.x, at.y + 1.1, at.z), yaw);
        self.reset_cam();
        if stage == 0 { self.loose = self.terrain.roads().map_or_else(LooseMap::empty, LooseMap::new); }
        junctions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let notes = super::pacenotes::generate(&pace_path, &junctions);
        self.rally = Some(RallyState { run: super::rally::Run::new(lines), stage, flip_t: 0.0, event: (0, 0.0, 0), pace: super::pacenotes::Playback::new(pace_path, notes), note: (0, String::new(), String::new()), note_clips: String::new() });
        true
    }
    pub fn rally_stop(&mut self) { self.rally = None; }

    /// A reset during a rally: the car goes back on the road (the nearest road point of the stage, the nearest loop
    /// point of the city) facing along it, and the run takes its time penalty.
    fn rally_reset(&mut self) {
        let Some(stage) = self.rally.as_ref().map(|r| r.stage) else { return };
        let p = self.vehicle.pos;
        let (at, yaw) = if stage == 0 {
            // the nearest point of the special stage itself (not of a side road)
            let stage = self.terrain.roads().and_then(|n| n.roads.iter().find(|r| r.kind == super::roads::RoadKind::Stage));
            match stage {
                Some(r) => {
                    let i = r.pts.iter().enumerate().min_by(|a, b| ((a.1.pos.x - p.x).hypot(a.1.pos.z - p.z)).partial_cmp(&((b.1.pos.x - p.x).hypot(b.1.pos.z - p.z))).unwrap()).map_or(0, |x| x.0);
                    let (a, b) = (r.pts[i].pos, r.pts[(i + 1).min(r.pts.len() - 1)].pos);
                    (a, (b.x - a.x).atan2(b.z - a.z))
                }
                None => (p, 0.0),
            }
        } else {
            let c = self.terrain.city_loop();
            let i = c.map_or(0, |c| c.pts.iter().enumerate().min_by(|a, b| ((a.1.x - p.x).hypot(a.1.z - p.z)).partial_cmp(&((b.1.x - p.x).hypot(b.1.z - p.z))).unwrap()).map_or(0, |x| x.0));
            c.map_or((p, 0.0), |c| { let (a, b) = (c.pts[i], c.pts[(i + 1).min(c.pts.len() - 1)]); (a, (b.x - a.x).atan2(b.z - a.z)) })
        };
        self.vehicle = Vehicle::new(self.car_spec, v3(at.x, at.y + 1.1, at.z), yaw);
        if let Some(r) = self.rally.as_mut() { r.run.reset_penalty(); r.flip_t = 0.0; }
    }

    /// Feeds the run with the car's position; turns its events into the HUD's last event.
    fn rally_step(&mut self, dt: f32) {
        let Some(stage) = self.rally.as_ref().map(|r| r.stage) else { return };
        let p = self.vehicle.pos;
        let off = if stage == 0 { self.terrain.road_edge_distance(p.x, p.z) } else { None };
        let flipped = self.vehicle.flipped();
        let upside_down_too_long = { let r = self.rally.as_mut().unwrap(); r.flip_t = if flipped { r.flip_t + dt } else { 0.0 }; r.flip_t > 3.0 };
        if upside_down_too_long { self.rally_reset(); }
        let r = self.rally.as_mut().unwrap();
        r.run.update(self.vehicle.pos, dt, off);
        if r.run.phase == super::rally::Phase::Running {
            if let Some(k) = r.pace.update(self.vehicle.pos, self.vehicle.speed(), dt) { r.note = (r.note.0 + 1, r.pace.text(k, true), r.pace.text(k, false)); r.note_clips = r.pace.clips(k).join(","); }
        }
        for e in r.run.drain_events() {
            use super::rally::Event::*;
            let (code, value) = match e { Split { time, .. } => (1, time), Reset => (2, super::rally::RESET_PENALTY), OffRoad => (3, super::rally::OFFROAD_PENALTY), Missed { lines } => (4, lines as f32 * super::rally::MISSED_PENALTY), Finish { total } => (5, total), Go => (6, 0.0) };
            r.event = (code, value, r.event.2 + 1);
        }
        if r.run.phase == super::rally::Phase::Finished { self.autopilot = None; }
    }

    /// The wind now (horizontal, m/s, gusts included).
    pub fn wind(&self) -> V3 { wind::wind_now(self.terrain.seed, self.time) }

    /// Time passes: the clouds drift downwind.
    fn advance_sky(&mut self, dt: f32) {
        let wd = self.wind();
        self.cloud_off.0 += wd.x * wind::CLOUD_DRIFT * dt;
        self.cloud_off.1 += wd.z * wind::CLOUD_DRIFT * dt;
    }

    pub fn swap_car(&mut self, car: VehicleSpec) {
        let f = self.vehicle.forward();
        let (x, z) = (self.vehicle.pos.x, self.vehicle.pos.z);
        self.car_spec = car; self.tuned_dirty = true;
        self.vehicle = Vehicle::new(car, v3(x, self.terrain.height(x, z) + 1.1, z), f.x.atan2(f.z));
        self.reset_cam();
    }

    /// Debug/test: puts the car on a road facing along it: 1 liaison 60 m in, 2 stage 60 m in, 3 stage middle, 4 stage
    /// 150 m before its end. False without roads.
    /// Debug/test: mode 1..4 = liaison start, stage start, middle, end, 5 = the city loop's start line; mode >= 10 = the stage at `mode * 10` m.
    pub fn place_on_road(&mut self, mode: i32) -> bool {
        if mode == 5 {   // the start/finish line of the city loop, facing along it
            let Some(l) = self.terrain.city_loop() else { return false };
            let (a, b) = (l.pts[0], l.pts[2]);
            let yaw = (b.x - a.x).atan2(b.z - a.z);
            self.vehicle = Vehicle::new(self.car_spec, v3(a.x, a.y + 1.1, a.z), yaw);
            self.reset_cam();
            return true;
        }
        let Some(net) = self.terrain.roads() else { return false };
        let len1 = net.roads.get(1).map_or(0.0, |r| r.length());
        let (ri, s) = match mode { m if m >= 10 => (1, m as f32 * 10.0), 1 => (0, 60.0), 2 => (1, 60.0), 3 => (1, len1 * 0.5), _ => (1, (len1 - 150.0).max(0.0)) };
        let Some(r) = net.roads.get(ri) else { return false };
        if r.pts.len() < 2 { return false; }
        let i = r.pts.iter().position(|p| p.s >= s).unwrap_or(r.pts.len() - 1).min(r.pts.len() - 2);
        let (a, b) = (r.pts[i].pos, r.pts[i + 1].pos);
        let yaw = (b.x - a.x).atan2(b.z - a.z);
        let g = self.terrain.height(a.x, a.z);
        self.vehicle = Vehicle::new(self.car_spec, v3(a.x, g + 1.1, a.z), yaw);
        self.reset_cam();
        true
    }

    /// A fresh camera behind the car (after a reset or a teleport) that keeps the player's view, distance and height.
    fn reset_cam(&mut self) {
        let (view, dist, height) = (self.cam.view as i32, self.cam.dist, self.cam.height);
        self.cam = FollowCam::new(&self.vehicle);
        self.cam.set_view(view, dist, height);
    }

    /// The test track: a bare flat sheet of tarmac (painted with a grid of lines every 50 m), no props, no water, no roads; the car on it from the start. For trying the sound,
    /// the drivetrain and the handling.
    pub fn test_track(car: VehicleSpec) -> World {
        let mut w = World::from_terrain(Terrain::flat(), car, (0.0, 0.0), 0.0);
        for _ in 0..120 { w.step(&Input::default(), 1.0 / 60.0); }
        w.vehicle.vel = V3::ZERO; w.vehicle.omega = V3::ZERO; w.drivetrain.reset();
        w.time = 0.0;
        w
    }

    /// The garage scene: the car settled on the flat spawn plateau, never driven; `step_garage` orbits the camera.
    pub fn garage(car: VehicleSpec) -> World {
        let mut w = World::new_with(7, car);
        for _ in 0..180 { w.step(&Input::default(), 1.0 / 60.0); }
        w.vehicle.vel = V3::ZERO; w.vehicle.omega = V3::ZERO;
        w.orbit = Some(0.0);
        w.step_garage(0.0);
        w
    }

    /// Garage scene time step: the camera circles the standing car once per 20 s, 6.5 m away and 1.6 m up.
    pub fn step_garage(&mut self, dt: f32) {
        let a = (self.orbit.unwrap_or(0.0) + dt * std::f32::consts::TAU / 20.0) % std::f32::consts::TAU;
        self.orbit = Some(a);
        let c = self.vehicle.pos;
        self.cam.pos = c + v3(a.cos() * 6.5, 1.6, a.sin() * 6.5);
        // look a little to the right of the car so it sits left of centre (the garage panel covers the right of the screen)
        let d = v3(-a.cos(), 0.0, -a.sin());
        let right = v3(-d.z, 0.0, d.x);
        self.cam.target = c + v3(0.0, 0.6, 0.0) + right * 1.8;
        self.advance_sky(dt); self.time += dt;
    }

    /// One physics step of the car.
    pub fn step(&mut self, inp: &Input, dt: f32) {
        let mut inp = inp.sanitized();
        let counting = self.rally.as_ref().map_or(false, |r| r.run.phase == super::rally::Phase::Countdown);
        if inp.reset {
            self.autopilot = None;
            if self.rally.is_some() { self.rally_reset(); inp.reset = false; } else { self.vehicle.reset_upright(&self.terrain); }
            self.drivetrain.reset();
        }
        if counting { self.autopilot = None; }
        if let Some(ap) = self.autopilot.as_mut() {
            let cmd = ap.drive(&self.vehicle, dt);
            inp = cmd.input.sanitized();
            if cmd.reset { self.vehicle.reset_upright(&self.terrain); }
            if let Some((p, yaw)) = cmd.teleport { self.vehicle = Vehicle::new(self.car_spec, p, yaw); }
            if cmd.done { self.autopilot = None; }
        }
        let (tx, tz) = (tile_of(self.vehicle.pos.x), tile_of(self.vehicle.pos.z));
        self.ensure_props(tx, tz, 1);
        let near = self.nearby_props();
        if counting { inp = Input { brake: true, handbrake: true, ..Default::default() }; }   // held at the start line until GO
        if self.autopilot.is_some() { if let Some(r) = self.rally.as_mut() { r.run.assisted = true; } }
        self.climate.step(dt);
        self.vehicle.wet = self.climate.weather().wet;
        // the aids and the tuning: the tuned car (the garage's car with its brakes, tyres, springs and downforce scaled), the drivetrain's options, ABS and traction control
        {
            if self.tuned_dirty || self.tuned.0 != self.tuning { self.tuned = (self.tuning, self.tuning.apply(&self.car_spec)); self.tuned_dirty = false; }
            self.vehicle.spec = self.tuned.1;
            let (a, t) = (self.assists, self.tuning);
            self.drivetrain.opts = super::drivetrain::DriveOpts { engine: t.engine, turbo: t.turbo, gearing: t.gearing, flat_shift: a.flat_shift, blip: a.blip, limiter: a.limiter, tc: a.tc };
            self.vehicle.abs_on = a.abs; self.vehicle.tc_on = a.tc;
        }
        // the drivetrain: the engine against the clutch and the gear, solved together with the wheels in the vehicle's substeps (the autopilot always drives the automatic)
        {
            use super::drivetrain::{DriveIn, Mode};
            let want = if self.manual && self.autopilot.is_none() && !counting { Mode::Manual } else { Mode::Auto };
            self.drivetrain.auto_reverse = self.autopilot.is_some();   // only the autopilot asks for the reverse with the sign of its throttle; the driver picks R by hand
            if self.autopilot.is_some() { self.drivetrain.reverse_sel = false; }
            if want != self.drivetrain.mode { self.drivetrain.set_mode(want); }
            let fwd = self.vehicle.vel.dot(self.vehicle.forward());
            let tc_slip = if self.drivetrain.gear < 0 { -self.vehicle.slip.iter().cloned().fold(0.0, f32::min) } else { self.vehicle.slip.iter().cloned().fold(0.0, f32::max) };
            let dtr = &mut self.drivetrain;
            dtr.begin(dt, &DriveIn { throttle: inp.throttle, speed: fwd, wheel_r: self.vehicle.spec.radius, airborne: self.vehicle.contacts == 0, brake: inp.brake,
                lat_g: self.vehicle.omega.dot(self.vehicle.up()) * fwd / 9.81, slope: self.vehicle.forward().y, tc_slip });
            // the reverse gear of the manual: the pedal asks for backwards (the lamps and the camera follow it)
            if dtr.gear < 0 && (want == Mode::Manual || dtr.reverse_sel) { inp.throttle = -inp.throttle.abs(); } else if want == Mode::Manual { inp.throttle = inp.throttle.abs(); }
            let iw = 4.0 * super::wheel::INERTIA;
            self.vehicle.step_driven(&self.terrain, &near, &inp, dt, &mut self.loose, Some(&mut |h, w, tyre| { let t = dtr.sub_coupled(h, (w[0] + w[1] + w[2] + w[3]) * 0.25, iw, tyre); [t * 0.25; 4] }));
            self.drivetrain.finish();
        }
        self.dust.set_gpu(super::dust::gpu_on(self.dust.level()));
        self.dust.update(&self.vehicle, &self.terrain, self.time, dt);
        let v = &mut self.vehicle;
        v.pos.x = v.pos.x.clamp(-WORLD_LIMIT, WORLD_LIMIT);
        v.pos.z = v.pos.z.clamp(-WORLD_LIMIT, WORLD_LIMIT);
        if v.pos.x.abs() >= WORLD_LIMIT { v.vel.x = 0.0; }
        if v.pos.z.abs() >= WORLD_LIMIT { v.vel.z = 0.0; }
        for c in &self.cities { c.resolve_hull(&mut self.vehicle.pos, &mut self.vehicle.vel, &mut self.vehicle.omega, self.vehicle.rot); }
        self.tracks.update(&self.vehicle, &self.terrain, dt);
        self.rally_step(dt);
        self.cam.update(&self.vehicle, &self.terrain, dt);
        self.advance_sky(dt); self.time += dt;
    }

    /// Loads props of tiles within `r` of (cx, cz) and drops tiles farther than `r + 2`.
    pub fn ensure_props(&mut self, cx: i32, cz: i32, r: i32) {
        for tz in cz - r..=cz + r { for tx in cx - r..=cx + r {
            let t = &self.terrain;
            self.props.entry((tx, tz)).or_insert_with(|| tile_props(t, tx, tz));
        } }
        self.props.retain(|&(tx, tz), _| (tx - cx).abs() <= r + 2 && (tz - cz).abs() <= r + 2);
    }

    pub fn tile_props_ref(&self, tx: i32, tz: i32) -> &[Prop] { self.props.get(&(tx, tz)).map_or(&[], |v| v.as_slice()) }

    pub fn nearby_props(&self) -> Vec<Prop> { self.nearby_props_at(self.vehicle.pos) }

    pub fn nearby_props_at(&self, p: V3) -> Vec<Prop> {
        let (cx, cz) = (tile_of(p.x), tile_of(p.z));
        let mut out = Vec::new();
        for tz in cz - 1..=cz + 1 { for tx in cx - 1..=cx + 1 { out.extend_from_slice(self.tile_props_ref(tx, tz)); } }
        out
    }

    /// Light and sky colours of the moment (the sun moves while the level runs).
    pub fn daylight(&self) -> Daylight { super::weather::apply(&daylight(self.terrain.seed, self.time), self.climate.weather()) }
    /// Unit vector towards the sun.
    pub fn sun_dir(&self) -> V3 { self.daylight().sun }
}

impl World {
    /// Puts the player at a street intersection near the centre of the first city. False without a city.
    pub fn place_in_city(&mut self) -> bool {
        let Some(c) = self.cities.first() else { return false };
        let s = c.site;
        let (x, z) = (s.centre.x - 24.0, s.centre.z - 24.0);       // the intersection of the 5th street each way
        self.vehicle = Vehicle::new(self.car_spec, v3(x, s.centre.y + 1.1, z), 0.0);
        self.reset_cam();
        true
    }

    /// Puts the player 60 m west of the first city facing it. False without a city.
    pub fn place_near_city(&mut self) -> bool {
        let Some(c) = self.cities.first() else { return false };
        let s = c.site;
        let (x, z) = (s.centre.x - s.half - 60.0, s.centre.z);
        let yaw = std::f32::consts::FRAC_PI_2;
        let g = self.terrain.height(x, z);
        self.vehicle = Vehicle::new(self.car_spec, v3(x, g + 1.1, z), yaw);
        self.reset_cam();
        true
    }

    /// Debug/test: puts the car on dry ground 20 m from the nearest lake, facing it. False without a lake.
    pub fn place_near_water(&mut self) -> bool {
        let t = &self.terrain;
        let Some(level) = t.water_level() else { return false };
        for ring in 1..=80 {
            let r = ring as f32 * 25.0;
            for d in 0..16 {
                let a = d as f32 * std::f32::consts::TAU / 16.0;
                let (wx, wz) = (a.cos() * r, a.sin() * r);
                if t.height(wx, wz) > level - 1.0 { continue; }
                let (x, z) = (wx - a.cos() * 22.0, wz - a.sin() * 22.0);
                if t.is_water(x, z, 0.3) || t.in_city(x, z, 0.0) { continue; }
                let yaw = (wx - x).atan2(wz - z);
                let g = t.height(x, z);
                self.vehicle = Vehicle::new(self.car_spec, v3(x, g + 1.1, z), yaw);
                self.reset_cam();
                return true;
            }
        }
        false
    }

    pub fn player_pos(&self) -> V3 { self.vehicle.pos }

}

/// What a render backend may read; gameplay never touches the GPU.
/// What the GPU dust needs of a frame: the particles the wheels emitted, the car's pose and velocity, the wind, the ground under the car.
pub struct DustGpuIn { pub emits: Vec<super::dust_ref::GP>, pub car_pos: V3, pub yaw: f32, pub car_vel: V3, pub wind: V3, pub ground_y: f32, pub cap: usize, pub time: f32, pub sun: V3, pub body: [f32; 3],
    /// The garage's aero parts strengthen the car's wake: 1 for the bare car.
    pub vortex_scale: f32,
    /// The GPU dust's option bits (`dust::OPT_*`).
    pub opts: u32,
    /// Where the body mesh is drawn: its origin and its three axes in the world (for the dust's exact-shape collisions).
    pub body_pose: (V3, [V3; 3]) }

pub struct WorldView<'a> {
    /// The camera's vertical field of view in radians.
    pub cam_fov: f32,
    /// The airflow debug view's input when it is on.
    pub airflow: Option<super::airflow::FlowIn>,
    /// The GPU dust's input of a frame when the particles live on the GPU.
    pub dust_gpu: Option<DustGpuIn>,
    pub signs: &'a [super::signs::Sign],
    pub terrain: &'a Terrain, pub vehicle: &'a Vehicle,
    pub cam_pos: V3, pub cam_target: V3,
    pub props: Vec<&'a Prop>, pub sun: V3, pub daylight: Daylight, pub weather: super::weather::Weather, pub lights: CarLights, pub lamps: Lamps, pub dust: Vec<Particle>, pub wind: V3, pub cloud_off: (f32, f32),
    pub time: f32, pub cities: &'a [City], pub tracks: &'a Tracks,
}

impl World {
    /// Loaded props of the tiles within `r` of the camera tile.
    pub fn view(&self, r: i32) -> WorldView<'_> {
        let (cx, cz) = (tile_of(self.cam.pos.x), tile_of(self.cam.pos.z));
        let mut props = Vec::new();
        for tz in cz - r..=cz + r { for tx in cx - r..=cx + r { props.extend(self.tile_props_ref(tx, tz).iter()); } }
        let dl = self.daylight();
        WorldView { cam_fov: self.cam.fov, dust_gpu: if self.dust.gpu() { Some(DustGpuIn { emits: self.dust.take_emits(), car_pos: self.vehicle.pos, yaw: self.vehicle.forward().x.atan2(self.vehicle.forward().z), car_vel: self.vehicle.vel, wind: self.dust.wind(), ground_y: self.terrain.height(self.vehicle.pos.x, self.vehicle.pos.z), cap: self.dust.gpu_cap(), time: self.time, body_pose: { let (q, o) = crate::raster::vehicle_models::car_body_pose(&self.vehicle); (o, [q.rotate(v3(1.0, 0.0, 0.0)), q.rotate(v3(0.0, 1.0, 0.0)), q.rotate(v3(0.0, 0.0, 1.0))]) }, opts: super::dust::opts(), sun: self.sun_dir(), body: [self.vehicle.spec.size.x * 0.5, self.vehicle.spec.size.z * 0.5, self.vehicle.spec.size.y], vortex_scale: 1.0 + 0.25 * self.vehicle.spec.kit.wing as f32 + 0.12 * self.vehicle.spec.kit.underbody as f32 + 0.1 * self.vehicle.spec.kit.splitter as f32 }) } else { None },
            airflow: if super::airflow::debug_on() || self.dust.gpu() { Some(super::airflow::flow_in(&self.vehicle, self.wind(), &super::airflow::cfg())) } else { None }, signs: if self.signs_on { self.terrain.signs() } else { &[] }, terrain: &self.terrain, vehicle: &self.vehicle, cam_pos: self.cam.pos, cam_target: self.cam.target, props, sun: self.sun_dir(), daylight: dl, weather: self.climate.weather(), lights: carlights::state(&self.vehicle, dl.night), lamps: carlights::lamps(&self.vehicle), dust: self.dust.particles(), wind: self.wind(), cloud_off: self.cloud_off,
            time: self.time, cities: &self.cities, tracks: &self.tracks }
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
