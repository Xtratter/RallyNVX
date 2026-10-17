//! Turns the game world into `AudioParams` and one-shot `Event`s by comparing each frame with the previous one.
use super::{AudioParams, Event, Machine, Scene, Surface, N_SRC, SRC_BAY, SRC_BODY, SRC_EXHAUST, SRC_WHEEL};
use super::acoustics::{environment, ground_reflection, occlusion, sphere_dirs, surface_alpha, surface_reflection, trace, Cabin, Kind, Obstacle, RayResult, Shape};
use super::reverb::Env;
use super::SrcExtra;
use crate::game::props::PropKind;
use crate::game::{math::v3, world::World};

pub struct Director {
    started: bool, time: f32, seed: u64,
    grounded: bool, wet: bool,
    hspeed: f32, vy: f32, air_time: f32, quiet_until: f32,
    /// The rays cast round the listener, a share of them renewed every frame (the tail and the echoes follow what they met), and where the renewing is.
    rays: Vec<RayResult>, ray_next: usize, ray_seed: u64,
}

/// Rays cast round the listener, and how far they look, m.
const RAYS: usize = 64;
const RAY_RANGE: f32 = 150.0;

impl Default for Director { fn default() -> Self { Director::new() } }

impl Director {
    pub fn new() -> Director {
        Director { started: false, time: 0.0, seed: 0, grounded: true, wet: false, hspeed: 0.0, vy: 0.0, air_time: 0.0, quiet_until: 0.0, rays: Vec::new(), ray_next: 0, ray_seed: u64::MAX }
    }

    /// Where the listener (the camera) and the sound sources of the car are: the exhaust at the tail, the engine bay at the nose, the four tyres where they touch the ground.
    pub fn scene(&mut self, w: &World) -> Scene {
        let v = &w.vehicle;
        let size = v.spec.size;
        let at = |x: f32, y: f32, z: f32| v.pos + v.rot.rotate(v3(x, y, z));
        let mut sc = Scene::default();
        sc.listener = w.cam.pos;
        let g = w.cam.target - w.cam.pos;
        sc.fwd = if g.len() > 1e-3 { g.norm() } else { v3(0.0, 0.0, 1.0) };
        let mut src = [v.pos; N_SRC];
        src[SRC_EXHAUST] = at(0.30, -0.25, -0.5 * size.z);
        src[SRC_BAY] = at(0.0, 0.05, 0.30 * size.z);
        src[SRC_BODY] = v.pos;
        for k in 0..4 {
            let wi = &v.wheels[k];
            src[SRC_WHEEL + k] = if wi.grounded { wi.contact } else { v.pos + v.rot.rotate(v.spec.wheels[k]) };
            sc.wheel_on[k] = wi.grounded as u8 as f32;
            sc.wheel_slip[k] = if wi.grounded { ((wi.lat_slip - 1.5) / 3.0).clamp(0.0, 1.0) } else { 0.0 };
        }
        sc.src = src;
        // what stands between the sources and the ears: the ground, the rocks, the crowns of the trees, the houses, and the car's own body
        let t = &w.terrain; let water = t.water_level();
        let ground = |x: f32, z: f32| { let h = t.height(x, z); water.map_or(h, |l| h.max(l)) };
        let character = |x: f32, z: f32| if water.map_or(false, |l| t.height(x, z) < l) { surface_reflection(crate::game::surface::Surface::Water) } else { surface_reflection(crate::game::surface::surface_at(t, x, z)) };
        let mut obs: Vec<Obstacle> = Vec::new();
        for centre in [v.pos, sc.listener] {
            for p in w.nearby_props_at(centre) {
                match p.kind {
                    PropKind::Rock => obs.push(Obstacle { shape: Shape::Sphere { c: p.pos + v3(0.0, p.radius * 0.5, 0.0), r: p.radius }, kind: Kind::Hard }),
                    PropKind::Tree => obs.push(Obstacle { shape: Shape::Sphere { c: p.pos + v3(0.0, p.height + 0.5, 0.0), r: 2.2 }, kind: Kind::Foliage }),
                }
            }
            for c in &w.cities { for b in c.buildings_near(centre, 60.0) { obs.push(Obstacle { shape: Shape::Aabb { min: b.min, max: b.max }, kind: Kind::Wall }); } }
        }
        let cabin = Self::cabin(w);
        let inside = cabin.contains(sc.listener);
        // the loss through the car's body (dB at 250 Hz, 1 kHz, 4 kHz) from each source to a listener inside: the engine bay is behind the firewall, the tyres under the floor
        let through = |k: usize| if k == SRC_BAY { [6.0, 10.0, 18.0] } else if k == SRC_EXHAUST { [9.0, 15.0, 26.0] } else { [10.0, 16.0, 26.0] };
        for k in 0..N_SRC {
            let o = occlusion(sc.listener, sc.src[k], &ground, &obs, Some(&cabin), through(k));
            let (gain, hz) = o.gain_and_cutoff();
            let echo = if inside { None } else { ground_reflection(sc.listener, sc.src[k], &ground, &character).map(|r| (r.image, r.coeff, r.hz)) };
            sc.extra[k] = SrcExtra { gain, hz, extra_delay: o.extra_delay, echo };
        }
        if inside { sc.wind_gain = 0.45; sc.wind_hz = 1500.0; }
        // the environment: rays round the listener (a quarter of them each frame) say how long the tail rings and where the walls are
        if self.rays.is_empty() { self.rays = sphere_dirs(RAYS).into_iter().map(|dir| RayResult { dir, hit: None }).collect(); }
        if self.ray_seed != t.seed { self.ray_seed = t.seed; for r in self.rays.iter_mut() { r.hit = None; } }
        if inside {
            sc.env = Env { rt60: 0.12, level: 0.18, damp_hz: 3500.0, ..Env::default() };   // the cabin: a small, upholstered room
        } else {
            let galpha = |x: f32, z: f32| if water.map_or(false, |l| t.height(x, z) < l) { surface_alpha(crate::game::surface::Surface::Water) } else { surface_alpha(crate::game::surface::surface_at(t, x, z)) };
            for _ in 0..RAYS / 4 {
                let i = self.ray_next; self.ray_next = (i + 1) % RAYS;
                let dir = self.rays[i].dir;
                self.rays[i].hit = trace(sc.listener, dir, RAY_RANGE, &ground, &galpha, &obs);
            }
            sc.env = environment(&self.rays, sc.fwd);
        }
        sc
    }

    /// The car's cabin as a box in the car's frame: between the axles, from the floor to the roof.
    pub fn cabin(w: &World) -> Cabin {
        let v = &w.vehicle; let s = &v.spec;
        let floor = s.wheels[0].y - s.rest - s.radius;   // the road, in the car's frame
        Cabin { centre: v.pos, x: v.rot.rotate(v3(1.0, 0.0, 0.0)), y: v.rot.rotate(v3(0.0, 1.0, 0.0)), z: v.rot.rotate(v3(0.0, 0.0, 1.0)),
            min: v3(-0.46 * s.size.x, floor + 0.35 * s.size.y, -0.28 * s.size.z), max: v3(0.46 * s.size.x, floor + s.size.y, 0.10 * s.size.z) }
    }

    /// `throttle` is the player's throttle (-1..1), `active` is false while paused. Never emits events when inactive
    /// or on the first update of a new world (a level change, a restart).
    pub fn update(&mut self, w: &World, throttle: f32, active: bool) -> (AudioParams, Vec<Event>) {
        let throttle = if throttle.is_finite() { throttle.clamp(-1.0, 1.0) } else { 0.0 };
        let mut ev = Vec::new();
        let fresh = !self.started || w.terrain.seed != self.seed || w.time < self.time - 1e-4;
        let dt = if fresh { 0.0 } else { (w.time - self.time).clamp(0.0, 0.5) };
        self.started = true; self.seed = w.terrain.seed; self.time = w.time;
        let mut p = AudioParams { active, ..Default::default() };
        let v = &w.vehicle;
        let speed = v.vel.len();
        let frac = (speed / v.spec.max_speed.max(1.0)).clamp(0.0, 1.0);
        let grounded = v.contacts > 0;
        let wet = w.terrain.is_water(v.pos.x, v.pos.z, 0.0);
        let (hspeed, vy) = (v.vel.x.hypot(v.vel.z), v.vel.y);
        p.machine = Machine::Car;
        // the engine: the game's drivetrain decides the revs, the throttle and the spark cut; the engine model on the audio thread follows them (locked to the revs) and makes the sound
        let d = w.drivetrain.last;
        p.drive_rpm = d.rpm; p.clutch = 1.0; p.cut = d.cut && !d.stalled; p.als = w.assists.als; p.no_bov = !w.assists.bov; p.stalled = d.stalled;
        p.load = d.throttle;
        p.load = throttle.abs();
        p.roll = if grounded { frac.sqrt() } else { 0.0 };
        p.surface = if wet { Surface::Water } else { Surface::Ground };
        p.slip = if grounded { ((v.vel.dot(v.right()).abs() - 2.0) / 6.0).clamp(0.0, 1.0) } else { 0.0 };
        // the wind you hear is the apparent wind: the level's wind minus the car's own velocity (a tail wind is quieter)
        let air = w.wind() - v3(v.vel.x, 0.0, v.vel.z);
        p.wind = (air.len() / 34.0).clamp(0.0, 1.0) * 0.42;
        // a teleport (the Reset button zeroes the velocity and lifts the car) must not sound like a crash
        let teleport = hspeed == 0.0 && self.hspeed > 6.0;
        if teleport { self.quiet_until = w.time + 1.0; self.air_time = 0.0; }
        if !grounded { self.air_time += dt; }
        let quiet = w.time < self.quiet_until;
        if !fresh && active && !quiet {
            if !self.grounded && grounded && self.vy < -3.0 && self.air_time >= 0.3 { ev.push(Event::Touchdown((self.vy.abs() / 10.0).clamp(0.0, 1.0))); }
            if !self.wet && wet && hspeed > 2.0 { ev.push(Event::Splash((hspeed / 20.0).clamp(0.0, 1.0))); }
            if self.hspeed - hspeed > 6.0 { ev.push(Event::Crash); }
        }
        if grounded { self.air_time = 0.0; }
        self.grounded = grounded; self.wet = wet; self.hspeed = hspeed;
        // keep the last airborne vertical speed for the touchdown strength
        if !grounded { self.vy = vy; } else if !fresh && grounded && self.vy > -0.001 { self.vy = vy; }
        (p, ev)
    }
}

#[cfg(test)]
#[path = "director_tests.rs"]
mod tests;
