//! Everything that happens in a level: terrain, props, the player's vehicle, the camera.
use std::collections::HashMap;

use super::camera_follow::FollowCam;
use super::input::Input;
use super::missions::{Mission, MissionKind, GOAL_RADIUS};
use super::run::{MissionRun, Phase};
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
    props: HashMap<(i32, i32), Vec<Prop>>,
}

impl World {
    pub fn new(seed: u64) -> World {
        let terrain = Terrain::new(seed);
        let vehicle = Vehicle::new(VehicleSpec::offroader(), v3(0.0, terrain.height(0.0, 0.0) + 1.1, 0.0), 0.0);
        let cam = FollowCam::new(&vehicle);
        World { terrain, vehicle, cam, time: 0.0, mission: None, props: HashMap::new() }
    }

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

    pub fn nearby_props(&self) -> Vec<Prop> {
        let (cx, cz) = (tile_of(self.vehicle.pos.x), tile_of(self.vehicle.pos.z));
        let mut out = Vec::new();
        for tz in cz - 1..=cz + 1 { for tx in cx - 1..=cx + 1 { out.extend_from_slice(self.tile_props_ref(tx, tz)); } }
        out
    }

    /// Unit vector towards the sun.
    pub fn sun_dir(&self) -> V3 { v3(0.45, 0.75, 0.35).norm() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum MarkerKind { Beacon, Ring, Next }
#[derive(Clone, Copy, Debug)] pub struct Marker { pub kind: MarkerKind, pub pos: V3, pub radius: f32 }

impl World {
    pub fn for_mission(m: Mission) -> World {
        let mut w = World::new(m.seed);
        // start facing the first target, camera behind
        if let Some(t) = m.checkpoints.first() {
            let (dx, dz) = (t.x - w.vehicle.pos.x, t.z - w.vehicle.pos.z);
            if dx.hypot(dz) > 1.0 {
                w.vehicle = Vehicle::new(VehicleSpec::offroader(), w.vehicle.pos, dx.atan2(dz));
                w.cam = FollowCam::new(&w.vehicle);
            }
        }
        w.mission = Some(MissionRun::new(m));
        w
    }

    /// Beacon at the goal; races also show the remaining checkpoints, the next one highlighted. Empty when done.
    pub fn markers(&self) -> Vec<Marker> {
        let Some(r) = &self.mission else { return Vec::new() };
        if r.phase() == Phase::Done { return Vec::new(); }
        let m = r.mission();
        let mut out = Vec::new();
        if m.kind == MissionKind::Race {
            for (i, p) in m.checkpoints.iter().enumerate() {
                if i < r.next_index() { continue; }
                out.push(Marker { kind: if i == r.next_index() { MarkerKind::Next } else { MarkerKind::Ring }, pos: *p, radius: GOAL_RADIUS });
            }
        }
        out.push(Marker { kind: MarkerKind::Beacon, pos: m.goal, radius: GOAL_RADIUS });
        out
    }
}

/// What a render backend may read; gameplay never touches the GPU.
pub struct WorldView<'a> {
    pub terrain: &'a Terrain, pub vehicle: &'a Vehicle,
    pub cam_pos: V3, pub cam_target: V3,
    pub props: Vec<&'a Prop>, pub sun: V3, pub markers: Vec<Marker>,
}

impl World {
    /// Loaded props of the tiles within `r` of the camera tile.
    pub fn view(&self, r: i32) -> WorldView<'_> {
        let (cx, cz) = (tile_of(self.cam.pos.x), tile_of(self.cam.pos.z));
        let mut props = Vec::new();
        for tz in cz - r..=cz + r { for tx in cx - r..=cx + r { props.extend(self.tile_props_ref(tx, tz).iter()); } }
        WorldView { terrain: &self.terrain, vehicle: &self.vehicle, cam_pos: self.cam.pos, cam_target: self.cam.target, props, sun: self.sun_dir(), markers: self.markers() }
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
