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

pub struct World {
    pub terrain: Terrain,
    pub vehicle: Vehicle,
    pub cam: FollowCam,
    pub time: f32,
    pub cities: Vec<City>,
    /// The loose layer of the special stage (worn by the wheels, reset with the level).
    pub loose: LooseMap,
    /// Tyre dust and spray.
    pub dust: Dust,
    /// How far the wind has carried the clouds (sky shader noise units).
    pub cloud_off: (f32, f32),
    pub tracks: Tracks,
    /// The car this world is built for (a restart or a placement helper makes the same car again).
    pub car_spec: VehicleSpec,
    /// Garage scene: the camera orbit angle (radians); `None` in play.
    orbit: Option<f32>,
    /// The autopilot while it drives (see `game/autopilot`).
    pub autopilot: Option<super::autopilot::Autopilot>,
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
        World { terrain, vehicle, cam, time: 0.0, cities, loose, dust: Dust::new(seed), cloud_off: (0.0, 0.0), tracks: Tracks::new(), car_spec: car, orbit: None, autopilot: None, props: HashMap::new() }
    }

    /// Swaps the car (a new spec) without regenerating the level: back at the same spot facing as before.
    /// Turns the autopilot on (it joins the stage from wherever the car is) or off; returns whether it is on afterwards.
    pub fn set_autopilot(&mut self, on: bool) -> bool { self.set_autopilot_style(if on { 1 } else { -1 }) }
    /// Style -1 off, 0 calm, 1 normal, 2 aggressive (slides allowed), 3 custom (the numbers set in the settings).
    pub fn set_autopilot_style(&mut self, style: i32) -> bool {
        self.autopilot = if style >= 0 { super::autopilot::Autopilot::start(&self.terrain, &self.vehicle, self.car_spec, style.min(3)) } else { None };
        self.autopilot.is_some()
    }
    pub fn autopilot_on(&self) -> bool { self.autopilot.is_some() }

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
        self.car_spec = car;
        self.vehicle = Vehicle::new(car, v3(x, self.terrain.height(x, z) + 1.1, z), f.x.atan2(f.z));
        self.cam = FollowCam::new(&self.vehicle);
    }

    /// Debug/test: puts the car on a road facing along it: 1 liaison 60 m in, 2 stage 60 m in, 3 stage middle, 4 stage
    /// 150 m before its end. False without roads.
    /// Debug/test: mode 1..4 = liaison start, stage start, middle, end; mode >= 10 = the stage at `mode * 10` m.
    pub fn place_on_road(&mut self, mode: i32) -> bool {
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
        self.cam = FollowCam::new(&self.vehicle);
        true
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
        if inp.reset { self.autopilot = None; self.vehicle.reset_upright(&self.terrain); }
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
        self.vehicle.step_with(&self.terrain, &near, &inp, dt, &mut self.loose);
        self.dust.update(&self.vehicle, &self.terrain, self.time, dt);
        let v = &mut self.vehicle;
        v.pos.x = v.pos.x.clamp(-WORLD_LIMIT, WORLD_LIMIT);
        v.pos.z = v.pos.z.clamp(-WORLD_LIMIT, WORLD_LIMIT);
        if v.pos.x.abs() >= WORLD_LIMIT { v.vel.x = 0.0; }
        if v.pos.z.abs() >= WORLD_LIMIT { v.vel.z = 0.0; }
        for c in &self.cities { c.resolve_hull(&mut self.vehicle.pos, &mut self.vehicle.vel, &mut self.vehicle.omega, self.vehicle.rot); }
        self.tracks.update(&self.vehicle, &self.terrain, dt);
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
    pub fn daylight(&self) -> Daylight { daylight(self.terrain.seed, self.time) }
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
        self.cam = FollowCam::new(&self.vehicle);
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
        self.cam = FollowCam::new(&self.vehicle);
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
                self.cam = FollowCam::new(&self.vehicle);
                return true;
            }
        }
        false
    }

    pub fn player_pos(&self) -> V3 { self.vehicle.pos }

}

/// What a render backend may read; gameplay never touches the GPU.
pub struct WorldView<'a> {
    pub terrain: &'a Terrain, pub vehicle: &'a Vehicle,
    pub cam_pos: V3, pub cam_target: V3,
    pub props: Vec<&'a Prop>, pub sun: V3, pub daylight: Daylight, pub lights: CarLights, pub lamps: Lamps, pub dust: Vec<Particle>, pub wind: V3, pub cloud_off: (f32, f32),
    pub time: f32, pub cities: &'a [City], pub tracks: &'a Tracks,
}

impl World {
    /// Loaded props of the tiles within `r` of the camera tile.
    pub fn view(&self, r: i32) -> WorldView<'_> {
        let (cx, cz) = (tile_of(self.cam.pos.x), tile_of(self.cam.pos.z));
        let mut props = Vec::new();
        for tz in cz - r..=cz + r { for tx in cx - r..=cx + r { props.extend(self.tile_props_ref(tx, tz).iter()); } }
        let dl = self.daylight();
        WorldView { terrain: &self.terrain, vehicle: &self.vehicle, cam_pos: self.cam.pos, cam_target: self.cam.target, props, sun: self.sun_dir(), daylight: dl, lights: carlights::state(&self.vehicle, dl.night), lamps: carlights::lamps(&self.vehicle), dust: self.dust.particles(), wind: self.wind(), cloud_off: self.cloud_off,
            time: self.time, cities: &self.cities, tracks: &self.tracks }
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
