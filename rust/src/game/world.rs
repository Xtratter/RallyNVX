//! Everything that happens in a level: terrain, props, the player's vehicle, the camera.
use std::collections::HashMap;

use super::airrun::{AirAction, AirRun};
use super::camera_follow::FollowCam;
use super::city::City;
use super::ground::WorldGround;
use super::flight::{Aircraft, AircraftKind, FlightInput};
use super::input::Input;
use super::missions::{ring_normal, Mission, MissionKind, GOAL_RADIUS, RING_RADIUS_DRONE, RING_RADIUS_HELI, ZONE_RADIUS};
use super::run::{MissionRun, Outcome, Phase};
use super::sling::Sling;
use super::daylight::{daylight, Daylight};
use super::tracks::Tracks;
use super::math::{v3, V3};
use super::props::{tile_props, Prop};
use super::terrain::{Terrain, TILE_SIZE};
use super::vehicle::{Vehicle, VehicleSpec};

pub const WORLD_LIMIT: f32 = 4000.0;

pub fn tile_of(x: f32) -> i32 { (x / TILE_SIZE).floor() as i32 }

pub struct World {
    pub terrain: Terrain,
    pub vehicle: Vehicle,
    pub cam: FollowCam,
    pub time: f32,
    pub mission: Option<MissionRun>,
    pub aircraft: Option<Aircraft>,
    pub sling: Option<Sling>,
    pub air: Option<AirRun>,
    pub cities: Vec<City>,
    pub tracks: Tracks,
    props: HashMap<(i32, i32), Vec<Prop>>,
}

impl World {
    pub fn new(seed: u64) -> World {
        let terrain = Terrain::new(seed);
        let vehicle = Vehicle::new(VehicleSpec::offroader(), v3(0.0, terrain.height(0.0, 0.0) + 1.1, 0.0), 0.0);
        let cam = FollowCam::new(&vehicle);
        let cities = terrain.sites().iter().map(|s| City::build(*s, seed)).collect();
        World { terrain, vehicle, cam, time: 0.0, mission: None, aircraft: None, sling: None, air: None, cities, tracks: Tracks::new(), props: HashMap::new() }
    }

    pub fn step(&mut self, inp: &Input, dt: f32) { self.step_with(inp, &FlightInput::default(), dt); }

    /// One physics step; `fi` drives the aircraft in air missions, `inp` the car otherwise.
    pub fn step_with(&mut self, inp: &Input, fi: &FlightInput, dt: f32) {
        if self.aircraft.is_some() { self.step_air(fi, dt); return; }
        let inp = inp.sanitized();
        if inp.reset { self.vehicle.reset_upright(&self.terrain); }
        let (tx, tz) = (tile_of(self.vehicle.pos.x), tile_of(self.vehicle.pos.z));
        self.ensure_props(tx, tz, 1);
        let near = self.nearby_props();
        self.vehicle.step(&self.terrain, &near, &inp, dt);
        let v = &mut self.vehicle;
        v.pos.x = v.pos.x.clamp(-WORLD_LIMIT, WORLD_LIMIT);
        v.pos.z = v.pos.z.clamp(-WORLD_LIMIT, WORLD_LIMIT);
        if v.pos.x.abs() >= WORLD_LIMIT { v.vel.x = 0.0; }
        if v.pos.z.abs() >= WORLD_LIMIT { v.vel.z = 0.0; }
        for c in &self.cities { c.resolve_hull(&mut self.vehicle.pos, &mut self.vehicle.vel, &mut self.vehicle.omega, self.vehicle.rot); }
        self.tracks.update(&self.vehicle, &self.terrain, dt);
        self.cam.update(&self.vehicle, &self.terrain, dt);
        if let Some(r) = self.mission.as_mut() { r.update(&self.vehicle, &inp, inp.reset, dt); }
        self.time += dt;
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
    pub fn daylight(&self) -> Daylight { daylight(self.terrain.seed, self.time) }
    /// Unit vector towards the sun.
    pub fn sun_dir(&self) -> V3 { self.daylight().sun }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum MarkerKind { Beacon, Ring, Next, AirRing, AirNext, Pad, Zone }
/// `normal` is the plane normal of air rings (zero for the other kinds).
#[derive(Clone, Copy, Debug)] pub struct Marker { pub kind: MarkerKind, pub pos: V3, pub radius: f32, pub normal: V3 }

/// One view over a ground run and an air run: what the HUD, the state array and the recorder read.
pub struct RunInfo<'a> {
    pub mission: &'a Mission, pub phase: Phase, pub time: f32, pub integrity: f32,
    pub next: usize, pub total: usize, pub outcome: Option<Outcome>, pub target: Option<V3>,
}

impl World {
    pub fn for_mission(m: Mission) -> World {
        let mut w = World::new(m.seed);
        if let Some(kind) = m.aircraft {
            let p0 = v3(m.start.x, w.terrain.height(m.start.x, m.start.z) + 3.0, m.start.z);
            let yaw = m.start_yaw.unwrap_or_else(|| m.checkpoints.first().map_or(0.0, |t| { let (dx, dz) = (t.x - p0.x, t.z - p0.z); if dx.hypot(dz) > 1.0 { dx.atan2(dz) } else { 0.0 } }));
            let a = Aircraft::new(kind, p0, yaw);
            let (back, up) = if kind == AircraftKind::Heli { (9.0, 3.0) } else { (6.0, 2.0) };
            w.cam = FollowCam::new_for(p0, a.forward(), back, up);
            if m.kind == MissionKind::Sling { if let Some(pa) = m.checkpoints.first() { w.sling = Some(Sling::on_ground(*pa + v3(0.0, 0.3, 0.0))); } }
            w.aircraft = Some(a);
            w.air = Some(AirRun::new(m));
            return w;
        }
        // start at the mission start (the origin plateau, or a street in the city) facing the start heading or the first target
        let pos = v3(m.start.x, w.terrain.height(m.start.x, m.start.z) + 1.1, m.start.z);
        let yaw = m.start_yaw.unwrap_or_else(|| m.checkpoints.first().map_or(0.0, |t| { let (dx, dz) = (t.x - pos.x, t.z - pos.z); if dx.hypot(dz) > 1.0 { dx.atan2(dz) } else { 0.0 } }));
        w.vehicle = Vehicle::new(VehicleSpec::offroader(), pos, yaw);
        w.cam = FollowCam::new(&w.vehicle);
        w.mission = Some(MissionRun::new(m));
        w
    }

    fn step_air(&mut self, fi: &FlightInput, dt: f32) {
        let fi = fi.sanitized();
        let pos = self.aircraft.as_ref().map_or(V3::ZERO, |a| a.pos);
        self.ensure_props(tile_of(pos.x), tile_of(pos.z), 1);
        let near = self.nearby_props_at(pos);
        let g = WorldGround { terrain: &self.terrain, cities: &self.cities };
        let a = self.aircraft.as_mut().unwrap();
        a.step(&g, &near, &fi, dt);
        if self.cities.iter().any(|c| c.touches_side(a.pos, a.spec.radius)) { a.status = super::flight::Status::Crashed; a.vel = V3::ZERO; }
        let (ap, av) = (a.pos, a.vel);
        let mut impact = None;
        if let Some(s) = self.sling.as_mut() {
            impact = s.step(ap - v3(0.0, 0.8, 0.0), av, &g, dt);
            for c in &self.cities {
                if let Some((push, n)) = c.push_out_sides(s.pos, 0.6) { s.pos = s.pos + push; let vn = s.vel.dot(n); if vn < 0.0 { s.vel = s.vel - n * vn; } }
            }
        }
        let a = *self.aircraft.as_ref().unwrap();
        let (back, up) = if a.kind == AircraftKind::Heli { (9.0, 3.0) } else { (6.0, 2.0) };
        self.cam.follow(a.pos, a.forward(), back, up, &self.terrain, dt);
        if let Some(r) = self.air.as_mut() {
            if r.update(&a, self.sling.as_ref(), impact, fi.active(), dt) == Some(AirAction::AttachCargo) {
                if let Some(s) = self.sling.as_mut() { s.attach(); }
            }
        }
        self.time += dt;
    }

    /// Puts the player at a street intersection near the centre of the first city (an aircraft 120 m above it). False without a city.
    pub fn place_in_city(&mut self) -> bool {
        let Some(c) = self.cities.first() else { return false };
        let s = c.site;
        let (x, z) = (s.centre.x - 24.0, s.centre.z - 24.0);       // the intersection of the 5th street each way
        if let Some(a) = self.aircraft.as_mut() { *a = Aircraft::new(a.kind, v3(x, s.centre.y + 120.0, z), 0.0); }
        else {
            self.vehicle = Vehicle::new(VehicleSpec::offroader(), v3(x, s.centre.y + 1.1, z), 0.0);
            self.cam = FollowCam::new(&self.vehicle);
        }
        true
    }

    /// Puts the player (the aircraft in an air mission, else the car) 60 m west of the first city facing it. False without a city.
    pub fn place_near_city(&mut self) -> bool {
        let Some(c) = self.cities.first() else { return false };
        let s = c.site;
        let (x, z) = (s.centre.x - s.half - 60.0, s.centre.z);
        let yaw = std::f32::consts::FRAC_PI_2;
        let g = self.terrain.height(x, z);
        if let Some(a) = self.aircraft.as_mut() { *a = Aircraft::new(a.kind, v3(x, g + 25.0, z), yaw); }
        else {
            self.vehicle = Vehicle::new(VehicleSpec::offroader(), v3(x, g + 1.1, z), yaw);
            self.cam = FollowCam::new(&self.vehicle);
        }
        true
    }

    /// Debug/test: puts the car (or the aircraft) on dry ground 20 m from the nearest lake, facing it. False without a lake.
    pub fn place_near_water(&mut self) -> bool {
        let t = self.terrain;
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
                if let Some(ac) = self.aircraft.as_mut() { *ac = Aircraft::new(ac.kind, v3(x, g + 25.0, z), yaw); }
                else { self.vehicle = Vehicle::new(VehicleSpec::offroader(), v3(x, g + 1.1, z), yaw); self.cam = FollowCam::new(&self.vehicle); }
                return true;
            }
        }
        false
    }

    /// Where the player is: the aircraft in air missions, else the car.
    pub fn player_pos(&self) -> V3 { self.aircraft.as_ref().map_or(self.vehicle.pos, |a| a.pos) }

    pub fn abandon_mission(&mut self) {
        if let Some(r) = self.mission.as_mut() { r.abandon(); }
        if let Some(r) = self.air.as_mut() { r.abandon(); }
    }

    pub fn run_info(&self) -> Option<RunInfo<'_>> {
        if let Some(r) = &self.mission {
            return Some(RunInfo { mission: r.mission(), phase: r.phase(), time: r.time(), integrity: r.integrity(), next: r.next_index(),
                total: r.mission().checkpoints.len(), outcome: r.outcome(), target: r.target() });
        }
        let r = self.air.as_ref()?;
        Some(RunInfo { mission: r.mission(), phase: r.phase(), time: r.time(), integrity: r.integrity(), next: r.next_index(),
            total: r.total(), outcome: r.outcome(), target: r.target(self.player_pos()) })
    }

    /// Rescue: the people still waiting, standing on a small circle around each pending site (ground level).
    pub fn survivors(&self) -> Vec<V3> {
        let Some(r) = self.air.as_ref().filter(|r| r.mission().kind == MissionKind::Rescue && r.phase() != Phase::Done) else { return Vec::new() };
        let mut out = Vec::new();
        for (site, n) in r.rescue_pending() {
            for k in 0..n {
                let a = k as f32 * std::f32::consts::TAU / n as f32 + 0.6;
                let (x, z) = (site.x + a.cos() * 4.6, site.z + a.sin() * 4.6);
                out.push(v3(x, self.terrain.height(x, z), z));
            }
        }
        out
    }

    /// Beacon at the goal; races also show the remaining checkpoints, the next one highlighted; air missions show rings,
    /// pads or zones. Empty when done.
    pub fn markers(&self) -> Vec<Marker> {
        let mk = |kind, pos, radius, normal| Marker { kind, pos, radius, normal };
        if let Some(r) = &self.air {
            if r.phase() == Phase::Done { return Vec::new(); }
            let m = r.mission();
            let mut out = Vec::new();
            match m.kind {
                MissionKind::Rings => {
                    let rad = if m.aircraft == Some(AircraftKind::Drone) { RING_RADIUS_DRONE } else { RING_RADIUS_HELI };
                    for (i, p) in m.checkpoints.iter().enumerate() {
                        if i < r.next_index() { continue; }
                        out.push(mk(if i == r.next_index() { MarkerKind::AirNext } else { MarkerKind::AirRing }, *p, rad, ring_normal(m, i)));
                    }
                }
                MissionKind::Sling => {
                    if r.next_index() == 0 { out.push(mk(MarkerKind::Pad, m.checkpoints[0], GOAL_RADIUS, V3::ZERO)); }
                    out.push(mk(MarkerKind::Pad, m.checkpoints[1], GOAL_RADIUS, V3::ZERO));
                    out.push(mk(MarkerKind::Beacon, m.checkpoints[1], GOAL_RADIUS, V3::ZERO));
                }
                MissionKind::Rescue => {
                    for (i, p) in m.checkpoints.iter().enumerate().skip(r.next_index()) {
                        out.push(mk(MarkerKind::Pad, *p, GOAL_RADIUS, V3::ZERO));
                        if i == r.next_index() { out.push(mk(MarkerKind::Beacon, *p, GOAL_RADIUS, V3::ZERO)); }
                    }
                }
                _ => {
                    for (p, done) in m.checkpoints.iter().zip(r.captured()) {
                        if !*done { out.push(mk(MarkerKind::Zone, *p, ZONE_RADIUS, V3::ZERO)); }
                    }
                }
            }
            return out;
        }
        let Some(r) = &self.mission else { return Vec::new() };
        if r.phase() == Phase::Done { return Vec::new(); }
        let m = r.mission();
        let mut out = Vec::new();
        if m.kind == MissionKind::Race {
            for (i, p) in m.checkpoints.iter().enumerate() {
                if i < r.next_index() { continue; }
                out.push(mk(if i == r.next_index() { MarkerKind::Next } else { MarkerKind::Ring }, *p, GOAL_RADIUS, V3::ZERO));
            }
        }
        out.push(mk(MarkerKind::Beacon, m.goal, GOAL_RADIUS, V3::ZERO));
        out
    }
}

/// What a render backend may read; gameplay never touches the GPU.
pub struct WorldView<'a> {
    pub terrain: &'a Terrain, pub vehicle: &'a Vehicle,
    pub cam_pos: V3, pub cam_target: V3,
    pub props: Vec<&'a Prop>, pub sun: V3, pub daylight: Daylight, pub survivors: Vec<V3>, pub markers: Vec<Marker>,
    pub aircraft: Option<Aircraft>, pub sling: Option<Sling>, pub time: f32, pub cities: &'a [City], pub tracks: &'a Tracks,
}

impl World {
    /// Loaded props of the tiles within `r` of the camera tile.
    pub fn view(&self, r: i32) -> WorldView<'_> {
        let (cx, cz) = (tile_of(self.cam.pos.x), tile_of(self.cam.pos.z));
        let mut props = Vec::new();
        for tz in cz - r..=cz + r { for tx in cx - r..=cx + r { props.extend(self.tile_props_ref(tx, tz).iter()); } }
        WorldView { terrain: &self.terrain, vehicle: &self.vehicle, cam_pos: self.cam.pos, cam_target: self.cam.target, props, sun: self.sun_dir(), daylight: self.daylight(), survivors: self.survivors(), markers: self.markers(),
            aircraft: self.aircraft, sling: self.sling, time: self.time, cities: &self.cities, tracks: &self.tracks }
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
