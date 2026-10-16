//! Tyre dust and spray: a deterministic ring of particles emitted by the wheels with a physical motion (see the dust designs):
//! each puff is fine (follows the air, hangs) or coarse (heavy, ballistic, falls out); it relaxes to the local air velocity with
//! a response time (Stokes drag), settles at its own speed, feels the wind, the car's wake (air dragged behind the body, an
//! inflow towards its centre line and a lift), turbulence that grows with the speed, and the ground (coarse grit is deposited,
//! fine dust slides). The wheels throw the dust back, up and away from a slide.
use super::grip;
use super::math::{v3, V3};
use super::terrain::Terrain;
use super::wind;
use super::vehicle::Vehicle;

/// What the renderer needs of a particle: `age` 0..1 of its life, `kind` 0 fine / 1 coarse, `seed` a stable random number.
#[derive(Clone, Copy, Debug)]
pub struct Particle { pub pos: V3, pub size: f32, pub color: [f32; 3], pub alpha: f32, pub seed: f32, pub age: f32, pub kind: f32 }

use super::dust_ref::{self, GP as P};

/// The numbers of `surfaces.json` -> `dust_physics`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phys { pub tau_fine: f32, pub tau_coarse: f32, pub settle_fine: f32, pub settle_coarse: f32, pub coarse_share: f32, pub wake_gain: f32, pub wake_len: f32, pub turbulence: f32, pub throw: f32,
    /// GPU dust: the plume behind the car rises (m/s of lift per m/s of car speed), up to about `plume_height` metres.
    pub plume_lift: f32, pub plume_height: f32,
    /// GPU dust: the peak swirl speed of the car's trailing vortices as a share of its speed.
    pub vortex: f32,
    /// GPU dust: the vortex street behind the car, as a share of its speed.
    pub street: f32 }

impl Phys {
    pub fn from_json(text: &str) -> Result<Phys, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("surfaces file is not valid JSON: {e}"))?;
        let f = v.get("dust_physics").filter(|o| o.is_object()).ok_or("surfaces: missing object \"dust_physics\"")?;
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = f.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("dust_physics: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("dust_physics: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        Ok(Phys { tau_fine: num("tau_fine", 0.02, 3.0)?, tau_coarse: num("tau_coarse", 0.02, 5.0)?, settle_fine: num("settle_fine", 0.0, 3.0)?, settle_coarse: num("settle_coarse", 0.0, 6.0)?,
            coarse_share: num("coarse_share", 0.0, 1.0)?, wake_gain: num("wake_gain", 0.0, 3.0)?, wake_len: num("wake_len", 1.0, 60.0)?, turbulence: num("turbulence", 0.0, 5.0)?, throw: num("throw", 0.0, 2.0)?,
            plume_lift: f.get("plume_lift").and_then(|x| x.as_f64()).filter(|x| (0.0..=1.0).contains(x)).map_or(0.0, |x| x as f32), plume_height: f.get("plume_height").and_then(|x| x.as_f64()).filter(|x| (0.5..=20.0).contains(x)).map_or(4.0, |x| x as f32),
            vortex: f.get("vortex").and_then(|x| x.as_f64()).filter(|x| (0.0..=1.0).contains(x)).map_or(0.0, |x| x as f32),
            street: f.get("street").and_then(|x| x.as_f64()).filter(|x| (0.0..=1.0).contains(x)).map_or(0.0, |x| x as f32) })
    }
}

pub fn phys() -> Phys {
    static CELL: std::sync::OnceLock<Phys> = std::sync::OnceLock::new();
    *CELL.get_or_init(|| Phys::from_json(grip::EMBEDDED).expect("the embedded surfaces.json has a bad dust_physics block"))
}

/// The car as the air sees it.
#[derive(Clone, Copy, Debug)]
pub struct Air { pub wind: V3, pub car_pos: V3, pub car_vel: V3, pub fwd: V3, pub right: V3, pub half: V3, pub time: f32 }

fn horizontal(v: V3) -> V3 { v3(v.x, 0.0, v.z) }

/// The air velocity at `p`: the wind (weaker near the ground), the car's wake and the turbulence.
pub fn air_velocity(p: V3, a: &Air, ground_y: f32, ph: &Phys) -> V3 {
    let h = (p.y - ground_y).max(0.0);
    let mut u = a.wind * (0.35 + 0.65 * (h / 4.0).min(1.0));
    let q = p - a.car_pos;
    let (lz, lx, ly) = (q.dot(a.fwd), q.dot(a.right), q.y);
    let cv = horizontal(a.car_vel);
    let speed = cv.len();
    // the wake behind the rear bumper: the air is dragged along, drawn in towards the centre line and lifted
    let behind = -lz - a.half.z;
    let reach = ph.wake_len * (0.35 + (speed / 20.0).min(1.2));
    let mut w = 0.0;
    if behind > -1.5 && speed > 1.0 {
        let b = behind.max(0.0);
        let sx = a.half.x * (1.0 + 0.12 * b) + 0.2;
        let sy = a.half.y * 1.1 + 0.12 * b + 0.2;
        let (gx, gy) = (lx / sx, (ly - a.half.y * 0.3) / sy);
        w = ph.wake_gain * (-b / reach).exp() * (-(gx * gx)).exp() * (-(gy * gy)).exp() * (1.0 + behind.min(0.0) / 1.5).clamp(0.0, 1.0);
        u = u + cv * (0.85 * w) + a.right * (-lx.signum() * speed * 0.12 * w) + v3(0.0, speed * 0.08 * w, 0.0);
    }
    // the body pushes the air aside as it passes
    if lz.abs() < a.half.z + 2.0 && lx.abs() < a.half.x + 2.5 && speed > 1.0 {
        let k = (-((lx.abs() - a.half.x).max(0.0) / 1.2).powi(2)).exp() * (1.0 - (lz.abs() / (a.half.z + 2.0)).min(1.0));
        u = u + a.right * (lx.signum() * speed * 0.12 * k);
    }
    // turbulence: grows with the speed and in the wake
    let t = a.time;
    let amp = ph.turbulence * (0.35 + 0.05 * speed + 0.25 * w.min(1.0) * speed * 0.2);
    let n = v3((p.y * 0.9 + t * 1.7 + p.z * 0.35).sin() + 0.5 * (p.z * 1.9 - t * 2.3).sin(),
        0.6 * (p.x * 0.8 + t * 1.1 + p.z * 0.5).sin(),
        (p.y * 1.1 - t * 1.5 + p.x * 0.4).sin() + 0.5 * (p.x * 2.1 + t * 2.9).sin());
    u + n * amp
}

pub struct Dust { seed: u64, rng: u32, parts: Vec<P>, head: usize, cap: usize, level: i32, acc: [f32; 4], wind: V3,
    /// GPU mode: the particles live on the GPU (`raster/dust_gpu.rs`); the wheels' new ones wait here until the renderer takes them.
    gpu: bool, pending: std::cell::Cell<Vec<P>>,
    /// The road pickup behind the car (GPU dust): how many particles the car's wake owes the road.
    pick: f32,
    /// The puffs the wheels shed (GPU dust): a cluster of particles that share a throw and a roll, so the cloud billows in lumps.
    puffs: [Puff; 4] }

#[derive(Clone, Copy, Default)]
struct Puff { centre: V3, vel: V3, axis: V3, omega: f32, left: u32 }

/// The most emits a frame hands to the GPU (more are dropped; one frame of the busiest slide emits far fewer).
pub const MAX_EMITS: usize = 4096;

/// How a surface's dust looks and lives on the GPU: how warm and light the loose-soil dust is (`tint` 0..1: asphalt has none), how long fine dust stays (seconds: real
/// clouds behind a vehicle take half a minute to a minute to clear in still air, Goossens et al.; the fine fraction hardly settles), and whether it is tyre smoke or
/// spray rather than dust (asphalt, water), and what share of the emits are kept (grass: a turf holds the soil, little dust rises; Cowherd/EPA, Arizona DEQ).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuLook { pub tint: f32, pub life: f32, pub smoke: bool, pub amount: f32 }

pub fn gpu_look(s: super::surface::Surface) -> GpuLook {
    use super::surface::Surface::*;
    match s {
        Gravel => GpuLook { tint: 1.0, life: 16.0, smoke: false, amount: 1.0 },
        Dirt => GpuLook { tint: 1.0, life: 14.0, smoke: false, amount: 1.0 },
        Sand => GpuLook { tint: 1.0, life: 20.0, smoke: false, amount: 1.0 },
        Grass => GpuLook { tint: 0.6, life: 5.0, smoke: false, amount: 0.25 },
        Rock => GpuLook { tint: 0.5, life: 8.0, smoke: false, amount: 1.0 },
        Asphalt => GpuLook { tint: 0.0, life: 4.0, smoke: true, amount: 1.0 },
        Water => GpuLook { tint: 0.0, life: 3.0, smoke: true, amount: 1.0 },
    }
}

/// The colour of a GPU dust particle of a surface: the soil's colour lifted towards a light tan and warmed (the dust of a dirt road is paler and warmer than the road).
pub fn gpu_colour(base: [f32; 3], look: GpuLook) -> [f32; 3] {
    let t = grip::table();
    [0, 1, 2].map(|i| {
        let warm = ((base[i] + ([0.88f32, 0.74, 0.56][i] - base[i]) * t.dust_gpu_lift) * t.dust_gpu_tint[i]).min(1.0);
        base[i] + (warm - base[i]) * look.tint
    })
}

static GPU_DEBUG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Debug/test: the GPU dust without the Settings switch.
pub fn set_gpu_debug(on: bool) { GPU_DEBUG.store(on, std::sync::atomic::Ordering::Relaxed); }
pub fn gpu_debug() -> bool { GPU_DEBUG.load(std::sync::atomic::Ordering::Relaxed) }

/// The GPU dust's options (Settings and the in-drive panel), each on its own bit: particle pressure (PBD), the car's exact shape, collisions with the world, the settled layer.
pub const OPT_PBD: u32 = 1;
pub const OPT_SHAPE: u32 = 2;
pub const OPT_WORLD: u32 = 4;
pub const OPT_DEPOSIT: u32 = 8;
/// The defaults: all but the heaviest (the pressure).
pub const OPTS_DEFAULT: u32 = OPT_SHAPE | OPT_WORLD | OPT_DEPOSIT;
static OPTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(OPTS_DEFAULT);
pub fn set_opts(mask: u32) { OPTS.store(mask & 15, std::sync::atomic::Ordering::Relaxed); }
pub fn opts() -> u32 { OPTS.load(std::sync::atomic::Ordering::Relaxed) }

static AERO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The Settings switch "Aerodynamics" (the app sets it at start; the host tests leave it off, so they run the CPU particles).
pub fn set_aero(on: bool) { AERO.store(on, std::sync::atomic::Ordering::Relaxed); }

/// Whether the particles run on the GPU, driven by the airflow: the switch (or the debug flag) is on and the level is Normal or Lots.
pub fn gpu_wanted(aero: bool, debug: bool, level: i32) -> bool { (aero || debug) && matches!(level, 1 | 2) }
pub fn gpu_on(level: i32) -> bool { gpu_wanted(AERO.load(std::sync::atomic::Ordering::Relaxed), gpu_debug(), level) }

fn caps(level: i32) -> (usize, f32) {
    let t = grip::table();
    match level { 0 | 3 => (0, 0.0), 2 => (t.dust_cap_lots, 1.5), _ => (t.dust_cap_normal, 1.0) }   // 3: the fluid dust (no particles)
}

impl Dust {
    pub fn new(seed: u64) -> Dust {
        let (cap, _) = caps(1);
        Dust { seed, rng: (seed as u32) ^ 0x9E37_79B9 | 1, parts: Vec::new(), head: 0, cap, level: 1, acc: [0.0; 4], wind: V3::ZERO, gpu: false, pending: std::cell::Cell::new(Vec::new()), pick: 0.0, puffs: [Puff::default(); 4] }
    }
    /// Whether the particles are on the GPU (then `update` only emits, `particles` is empty).
    pub fn gpu(&self) -> bool { self.gpu }
    pub fn set_gpu(&mut self, on: bool) {
        if on != self.gpu { self.parts.clear(); self.head = 0; self.pending.get_mut().clear(); self.acc = [0.0; 4]; self.pick = 0.0; self.puffs = [Puff::default(); 4]; }
        self.gpu = on;
    }
    /// The wheels' new particles since the last call (GPU mode).
    pub fn take_emits(&self) -> Vec<P> { self.pending.take() }
    pub fn wind(&self) -> V3 { self.wind }
    pub fn len(&self) -> usize { self.parts.len() }
    pub fn cap(&self) -> usize { self.cap }
    /// The ring size on the GPU: the GPU holds and emits `dust.gpu_factor` times more than the CPU path.
    pub fn gpu_cap(&self) -> usize { (self.cap as f32 * grip::table().dust_gpu_factor) as usize }

    /// 0 off, 1 normal, 2 lots, 3 fluid (the experiment: the particles stay off, the grid does the dust).
    pub fn level(&self) -> i32 { self.level }
    pub fn set_level(&mut self, level: i32) {
        let level = level.clamp(0, 3);
        self.level = level;
        self.cap = caps(level).0;
        if self.parts.len() > self.cap { self.parts.clear(); self.head = 0; }
        if self.cap == 0 { self.parts = Vec::new(); self.head = 0; self.acc = [0.0; 4]; }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 17; self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }
    fn push(&mut self, p: P) {
        if self.cap == 0 { return; }
        if self.gpu { let q = self.pending.get_mut(); if q.len() < MAX_EMITS { let mut p = p; p.size0 *= grip::table().dust_gpu_size; q.push(p); } return; }
        if self.parts.len() < self.cap { self.parts.push(p); } else { self.parts[self.head] = p; self.head = (self.head + 1) % self.cap; }
    }

    pub fn update(&mut self, v: &Vehicle, t: &Terrain, time: f32, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 || !time.is_finite() { return; }
        let dt = dt.min(0.1);
        self.wind = wind::wind_now(self.seed, time);
        let table = grip::table();
        let ph = phys();
        let air = Air { wind: self.wind, car_pos: v.pos, car_vel: v.vel, fwd: v.forward(), right: v.right(), half: v3(v.spec.size.x * 0.5, v.spec.size.y * 0.5, v.spec.size.z * 0.5), time };
        // motion and ageing (on the GPU in GPU mode)
        let mut i = 0;
        while i < self.parts.len() && !self.gpu {
            let p = &mut self.parts[i];
            let u = air_velocity(p.pos, &air, t.height(p.pos.x, p.pos.z), &ph);
            if !dust_ref::step(p, u, |x, z| t.height(x, z), &ph, dt) { self.parts.swap_remove(i); if self.head >= self.parts.len() { self.head = 0; } continue; }
            i += 1;
        }
        // emission
        let (cap, mut factor) = caps(self.level);
        factor *= 1.0 - 0.85 * v.wet.clamp(0.0, 1.0);   // wet ground raises little dust
        if cap == 0 { return; }
        if self.gpu { factor *= table.dust_gpu_factor; }
        let vh = horizontal(v.vel);
        let lateral = v.vel.dot(v.right());
        for w in 0..4 {
            let info = v.wheels[w];
            if !info.grounded || !info.speed.is_finite() || !info.slide.is_finite() || !info.lat_slip.is_finite() || !info.contact.is_finite() { self.acc[w] = 0.0; continue; }
            let d = table.dust(info.surface);
            let roll = if info.speed > 2.0 { d.roll * 14.0 * (info.speed / 20.0).min(1.5) } else { 0.0 };
            let slide = d.slide * (40.0 * info.slide.min(1.5) + 18.0 * (info.lat_slip / 6.0).min(1.5));
            let rate = (roll + slide) * factor;
            self.acc[w] = (self.acc[w] + rate * dt).min(8.0 * if self.gpu { table.dust_gpu_factor } else { 1.0 });
            while self.acc[w] >= 1.0 {
                self.acc[w] -= 1.0;
                if self.gpu { self.gpu_emit(v, w, &info, &d, &ph, vh, lateral); continue; }
                let (a, b, c, e, f, g2, h2) = (self.rand(), self.rand(), self.rand(), self.rand(), self.rand(), self.rand(), self.rand());
                let coarse = h2 < (ph.coarse_share + 0.3 * info.slide.min(1.0)).min(0.9);
                let mut pos = v3(info.contact.x + (a - 0.5) * 0.4, info.contact.y + 0.1, info.contact.z + (b - 0.5) * 0.4);
                let mut out_push = V3::ZERO;
                if self.gpu {
                    // the dust leaves the contact patch behind the tyre, under the wheel arch, and the air under the body carries it out at the side
                    pos = pos - v.forward() * 0.25;
                    let lx = (pos - v.pos).dot(v.right());
                    out_push = v.right() * (if lx >= 0.0 { 1.0 } else { -1.0 });
                }
                // the tyre throws the dust back and up, and away from the direction of a slide
                let kick = if coarse { 1.35 } else { 0.8 };
                let spray = v.right() * (-lateral * 0.5 * (info.slide.min(1.0) + 0.2));
                let vel = v3(-vh.x * ph.throw * kick + spray.x + (c - 0.5) * 1.5, (1.0 + 3.0 * info.slide.min(1.5) + e * 1.2) * if coarse { 1.1 } else { 0.9 }, -vh.z * ph.throw * kick + spray.z + (f - 0.5) * 1.5) + out_push;
                let shade = 0.9 + 0.2 * g2;
                let tone = if coarse { 0.82 } else { 1.0 };
                self.push(P { pos, vel, age: 0.0, life: d.life * (0.75 + 0.5 * a) * if coarse { 0.4 } else { 1.0 }, size0: d.size * if coarse { 1.0 } else { 1.3 },
                    color: [d.color[0] * shade * tone, d.color[1] * shade * tone, d.color[2] * shade * tone], coarse, seed: g2, speck: false, smoke: false });
            }
        }
        // the car's own aerodynamics raises dust too: its wake (the recirculation, the trailing vortices) scours the loose surface behind it, in proportion to
        // the square of its speed, whatever the wheels do; the particles start at rest-ish on the road behind the rear bumper and the wake lifts them
        if self.gpu {
            let rear = v.wheels[2];
            let speed = vh.len();
            if rear.grounded && speed > 3.0 && rear.speed.is_finite() {
                let d = table.dust(rear.surface);
                let rate = d.roll * 45.0 * (speed / 20.0).powi(2).min(2.2) * factor;
                self.pick = (self.pick + rate * dt).min(8.0 * table.dust_gpu_factor);
                let (fwd, right) = (v.forward(), v.right());
                let (hw, hl) = (v.spec.size.x * 0.5, v.spec.size.z * 0.5);
                while self.pick >= 1.0 {
                    self.pick -= 1.0;
                    let (a, b, c, e, g2) = (self.rand(), self.rand(), self.rand(), self.rand(), self.rand());
                    let back = hl + 0.3 + b * 4.0;                       // 0.3 to 4.3 m behind the rear bumper
                    let side = (a - 0.5) * 2.0 * (hw + 0.5);
                    let mut pos = v.pos + fwd * (-back) + right * side;
                    pos.y = rear.contact.y + 0.05 + c * 0.15;
                    let vel = vh * 0.12 + v3(0.0, 0.3 + e * 0.8, 0.0);
                    let shade = 0.9 + 0.2 * g2;
                    let look = gpu_look(rear.surface);
                    let base = gpu_colour(d.color, look);
                    self.push(P { pos, vel, age: 0.0, life: look.life * (0.6 + 0.8 * a), size0: d.size, color: [base[0] * shade, base[1] * shade, base[2] * shade], coarse: false, seed: g2, speck: false, smoke: look.smoke });
                }
            }
        }
    }

    /// One particle of the GPU dust from wheel `w`: it belongs to the wheel's current puff (a cluster sharing a throw and a roll about a horizontal axis across it, so
    /// the cloud rolls in lumps like a real one), and now and then a speck of grit goes with it. The CPU particles never come here.
    #[allow(clippy::too_many_arguments)]
    fn gpu_emit(&mut self, v: &Vehicle, w: usize, info: &super::vehicle::WheelInfo, d: &super::grip::DustSpec, ph: &Phys, vh: V3, lateral: f32) {
        let t = grip::table();
        let slide = info.slide.min(1.5);
        if self.puffs[w].left == 0 {
            let (a, b, e, f, g) = (self.rand(), self.rand(), self.rand(), self.rand(), self.rand());
            let (fwd, right) = (v.forward(), v.right());
            let mut centre = v3(info.contact.x, info.contact.y + 0.1 + 0.15 * b, info.contact.z) - fwd * 0.3 + right * ((a - 0.5) * 0.3);
            let lx = (centre - v.pos).dot(right);
            let spray = right * (-lateral * 0.5 * (slide.min(1.0) + 0.2));
            let out = right * (if lx >= 0.0 { 1.0 } else { -1.0 });
            let base = v3((-vh.x * ph.throw * 0.8 + spray.x) * t.dust_gpu_kick + out.x, (1.0 + 3.0 * slide + e * 1.2) * 0.9 * t.dust_gpu_kick * 0.9, (-vh.z * ph.throw * 0.8 + spray.z) * t.dust_gpu_kick + out.z);
            let dir = v3(base.x, 0.0, base.z).norm();
            // the roll: about the horizontal axis across the throw, the top of the puff moving away from the car
            let axis = v3(-dir.z, 0.0, dir.x) * (if g < 0.5 { 1.0 } else { -1.0 });
            centre.y = centre.y.max(info.contact.y + 0.08);
            self.puffs[w] = Puff { centre, vel: base, axis, omega: (2.0 + 4.0 * slide) * (0.7 + 0.6 * f), left: 40 + (slide * 80.0) as u32 };
        }
        self.puffs[w].left -= 1;
        let puff = self.puffs[w];
        let (a, b, c, e, g2, h2) = (self.rand(), self.rand(), self.rand(), self.rand(), self.rand(), self.rand());
        // a point in a ball of 0.3 m round the puff's centre
        let r = v3(a - 0.5, (b - 0.5) * 0.8, c - 0.5) * 0.9;
        let coarse = h2 < (ph.coarse_share + 0.3 * info.slide.min(1.0)).min(0.9);
        let vel = puff.vel + puff.axis.cross(r) * puff.omega + v3(e - 0.5, (g2 - 0.5) * 0.6, self.rand() - 0.5) * 3.0;
        let shade = 0.96 + 0.08 * g2;
        let tone = if coarse { 0.92 } else { 1.0 };
        let pos = puff.centre + r;
        let look = gpu_look(info.surface);
        if look.amount < 1.0 && self.rand() > look.amount { return; }   // turf holds the soil: little dust rises
        let base = gpu_colour(d.color, look);
        if look.smoke {
            // tyre smoke or spray: fine and pale, it hangs and rises a little, and is gone in seconds
            self.push(P { pos, vel: vel * 0.8, age: 0.0, life: look.life * (0.6 + 0.8 * a), size0: d.size * 1.3, color: [base[0] * shade, base[1] * shade, base[2] * shade], coarse: false, seed: g2, speck: false, smoke: true });
            return;
        }
        self.push(P { pos, vel, age: 0.0, life: look.life * (0.7 + 0.6 * a) * if coarse { 0.2 } else { 1.0 }, size0: d.size * if coarse { 1.0 } else { 1.3 },
            color: [base[0] * shade * tone, base[1] * shade * tone, base[2] * shade * tone], coarse, seed: g2, speck: false, smoke: false });
        // grit flung by the tyre: tiny dark flecks that fly in arcs and bounce
        if info.surface == super::surface::Surface::Gravel && self.rand() < t.dust_gpu_specks * (0.4 + slide) {
            let (sa, sb, sc, sd) = (self.rand(), self.rand(), self.rand(), self.rand());
            let sp = puff.vel * (0.7 + 0.6 * sa) + v3(sb - 0.5, 0.0, sc - 0.5) * 5.0 + v3(0.0, 1.0 + 3.0 * sd * (0.5 + slide), 0.0);
            let bright = 0.5 + 0.9 * self.rand();
            self.push(P { pos: puff.centre + r * 0.5, vel: sp, age: 0.0, life: 1.6 + 1.6 * sa, size0: 0.012 + 0.02 * sd,
                color: [(d.color[0] * 0.6 * bright).min(1.0), (d.color[1] * 0.55 * bright).min(1.0), (d.color[2] * 0.5 * bright).min(1.0)], coarse: true, seed: sb, speck: true, smoke: false });
        }
    }

    pub fn particles(&self) -> Vec<Particle> { self.parts.iter().map(dust_ref::render).collect() }
}

#[cfg(test)]
#[path = "dust_tests.rs"]
mod tests;
