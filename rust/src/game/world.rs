//! Everything that happens in a level: terrain, props, the player's vehicle, the camera.
use std::collections::HashMap;

use super::camera_follow::FollowCam;
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
    pub tracks: Tracks,
    /// The car this world is built for (a restart or a placement helper makes the same car again).
    pub car_spec: VehicleSpec,
    /// Garage scene: the camera orbit angle (radians); `None` in play.
    orbit: Option<f32>,
    props: HashMap<(i32, i32), Vec<Prop>>,
}

impl World {
    pub fn new(seed: u64) -> World { World::new_with(seed, VehicleSpec::offroader()) }

    pub fn new_with(seed: u64, car: VehicleSpec) -> World {
        let terrain = Terrain::new(seed);
        let vehicle = Vehicle::new(car, v3(0.0, terrain.height(0.0, 0.0) + 1.1, 0.0), 0.0);
        let cam = FollowCam::new(&vehicle);
        let cities = terrain.sites().iter().map(|s| City::build(*s, seed)).collect();
        World { terrain, vehicle, cam, time: 0.0, cities, tracks: Tracks::new(), car_spec: car, orbit: None, props: HashMap::new() }
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
        self.time += dt;
    }

    /// One physics step of the car.
    pub fn step(&mut self, inp: &Input, dt: f32) {
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
    pub props: Vec<&'a Prop>, pub sun: V3, pub daylight: Daylight, pub lights: CarLights, pub lamps: Lamps,
    pub time: f32, pub cities: &'a [City], pub tracks: &'a Tracks,
}

impl World {
    /// Loaded props of the tiles within `r` of the camera tile.
    pub fn view(&self, r: i32) -> WorldView<'_> {
        let (cx, cz) = (tile_of(self.cam.pos.x), tile_of(self.cam.pos.z));
        let mut props = Vec::new();
        for tz in cz - r..=cz + r { for tx in cx - r..=cx + r { props.extend(self.tile_props_ref(tx, tz).iter()); } }
        let dl = self.daylight();
        WorldView { terrain: &self.terrain, vehicle: &self.vehicle, cam_pos: self.cam.pos, cam_target: self.cam.target, props, sun: self.sun_dir(), daylight: dl, lights: carlights::state(&self.vehicle, dl.night), lamps: carlights::lamps(&self.vehicle),
            time: self.time, cities: &self.cities, tracks: &self.tracks }
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
