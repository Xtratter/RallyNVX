//! One step of a dust particle, as the GPU runs it (`shaders/dust_gpu.wgsl`): the host-tested reference. `game/dust.rs` (the CPU particles) uses the
//! same functions, so both paths move the dust the same way.
use super::dust::{Particle, Phys};
use super::math::{v3, V3};

/// A dust particle: `life` in seconds, `size0` the size when it is born, `coarse` heavy grit (else fine dust), `seed` a stable random number.
#[derive(Clone, Copy, Debug)]
pub struct GP { pub pos: V3, pub vel: V3, pub age: f32, pub life: f32, pub size0: f32, pub color: [f32; 3], pub coarse: bool, pub seed: f32 }

/// Moves `p` for `dt` seconds in the air velocity `air` (Stokes drag: the exact relaxation towards it with the response time of its class, the settling
/// speed of its class on top) and meets the ground (`ground(x, z)`): grit is deposited, fine dust slides and slows. False when the particle is gone
/// (too old or deposited). A non-finite `air` counts as still air.
pub fn step(p: &mut GP, air: V3, ground: impl Fn(f32, f32) -> f32, ph: &Phys, dt: f32) -> bool {
    p.age += dt;
    if p.age >= p.life { return false; }
    let u = if air.is_finite() { air } else { V3::ZERO };
    let (tau, settle) = if p.coarse { (ph.tau_coarse, ph.settle_coarse) } else { (ph.tau_fine, ph.settle_fine) };
    let target = v3(u.x, u.y - settle, u.z);
    let k = 1.0 - (-dt / tau).exp();   // stable at any step
    p.vel = p.vel + (target - p.vel) * k;
    p.pos = p.pos + p.vel * dt;
    let g = ground(p.pos.x, p.pos.z) + 0.04;
    if p.pos.y < g {
        if p.coarse { return false; }
        p.pos.y = g;
        if p.vel.y < 0.0 { p.vel.y = 0.0; }
        let s = 1.0 - (2.5 * dt).min(1.0);
        p.vel.x *= s; p.vel.z *= s;
    }
    true
}

/// What the renderer needs of a particle: it grows as it spreads, fades in over the first 8 % of its life and out at the end.
pub fn render(p: &GP) -> Particle {
    let x = (p.age / p.life).clamp(0.0, 1.0);
    let fade_in = (x / 0.08).min(1.0);
    // turbulent diffusion spreads a puff like the square root of its age
    let size = p.size0 * (1.0 + if p.coarse { 0.8 } else { 1.7 } * p.age.sqrt() + 1.2 * x);
    Particle { pos: p.pos, size, color: p.color, alpha: (if p.coarse { 0.6 } else { 0.55 }) * fade_in * (1.0 - x).powf(1.4), seed: p.seed, age: x, kind: if p.coarse { 1.0 } else { 0.0 } }
}

/// How far (m) from the faces of the airflow box the grid's air blends into the outside air.
pub const EDGE: f32 = 1.5;

fn smooth(x: f32) -> f32 { let t = x.clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

/// The car's forward and right on the ground plane for a yaw (the car's forward is (sin yaw, 0, cos yaw), as `Vehicle::forward`).
fn axes(yaw: f32) -> (V3, V3) { let f = v3(yaw.sin(), 0.0, yaw.cos()); (f, v3(-f.z, 0.0, f.x)) }

/// A world position in the airflow box's own coordinates, metres from its corner: x across the car (the box's middle on the car's centre line),
/// y up from the road (`ground_y`, the terrain under the car), z along the car (the box's middle at the car's centre). `size` is the box in metres.
pub fn to_box(p: V3, car_pos: V3, yaw: f32, ground_y: f32, size: [f32; 3]) -> [f32; 3] {
    let (f, r) = axes(yaw);
    let q = p - car_pos;
    [q.dot(r) + size[0] * 0.5, p.y - ground_y, q.dot(f) + size[2] * 0.5]
}

/// A vector of the box's frame (x across, y up, z along the car) as a world vector.
pub fn from_box_dir(d: V3, yaw: f32) -> V3 { let (f, r) = axes(yaw); r * d.x + v3(0.0, d.y, 0.0) + f * d.z }

/// 1 inside the box, 0 outside, a smooth blend over `EDGE` metres from its faces (the road is no face: the box has no floor to leave through).
pub fn box_weight(c: [f32; 3], size: [f32; 3]) -> f32 {
    if c[1] < 0.0 { return 0.0; }
    let d = c[0].min(size[0] - c[0]).min(size[1] - c[1]).min(c[2]).min(size[2] - c[2]);
    smooth(d / EDGE)
}

/// The world air velocity at `p`: the box's air (the grid velocity is relative to the car, in the car's frame) inside the box, `outside` beyond it.
pub fn air_in_box(p: V3, car_pos: V3, car_vel: V3, yaw: f32, ground_y: f32, grid_v: V3, outside: V3, size: [f32; 3]) -> V3 {
    let w = box_weight(to_box(p, car_pos, yaw, ground_y, size), size);
    let inside = car_vel + from_box_dir(grid_v, yaw);
    outside + (inside - outside) * w
}

/// The small-scale turbulence the grid cannot carry: grows with the car's speed.
pub fn turbulence(p: V3, t: f32, speed: f32, ph: &Phys) -> V3 {
    let amp = ph.turbulence * (0.35 + 0.05 * speed);
    let n = v3((p.y * 0.9 + t * 1.7 + p.z * 0.35).sin() + 0.5 * (p.z * 1.9 - t * 2.3).sin(),
        0.6 * (p.x * 0.8 + t * 1.1 + p.z * 0.5).sin(),
        (p.y * 1.1 - t * 1.5 + p.x * 0.4).sin() + 0.5 * (p.x * 2.1 + t * 2.9).sin());
    n * amp
}

/// The ground under the dust comes from a window of terrain heights: `WINDOW` cells of 1 m around the car, stored toroidally (a cell lives in the
/// slot `cell mod WINDOW`), so only the cells that come into the window are measured when the car moves.
pub const WINDOW: i32 = 64;

pub fn slot(cell: [i32; 2]) -> usize { (cell[1].rem_euclid(WINDOW) * WINDOW + cell[0].rem_euclid(WINDOW)) as usize }

/// The first cell of the window when the car is in `car_cell`.
pub fn window_base(car_cell: [i32; 2]) -> [i32; 2] { [car_cell[0] - WINDOW / 2, car_cell[1] - WINDOW / 2] }

pub fn in_window(cell: [i32; 2], base: [i32; 2]) -> bool { (0..2).all(|a| cell[a] >= base[a] && cell[a] < base[a] + WINDOW) }

/// The terrain height at (x, z) by bilinear interpolation of the cell-centre heights (`height(cell)`) of the window; `fallback` where a corner is outside.
pub fn ground_height(x: f32, z: f32, base: [i32; 2], height: impl Fn(usize) -> f32, fallback: f32) -> f32 {
    let (gx, gz) = (x - 0.5, z - 0.5);
    let (ix, iz) = (gx.floor(), gz.floor());
    let (fx, fz) = (gx - ix, gz - iz);
    let c = [ix as i32, iz as i32];
    let h = |dx: i32, dz: i32| { let cell = [c[0] + dx, c[1] + dz]; if in_window(cell, base) { height(slot(cell)) } else { fallback } };
    let (a, b) = (h(0, 0) + (h(1, 0) - h(0, 0)) * fx, h(0, 1) + (h(1, 1) - h(0, 1)) * fx);
    a + (b - a) * fz
}

/// The coupling of the dust and the air (`surfaces.json` -> `dust_air`): the particles are splatted into the airflow's grid as a density and a momentum
/// field; the density pushes the particles apart (a stand-in for the particle-particle pressure) and drags the air along (a thick cloud rolls as one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coupling { pub push: f32, pub drag: f32, pub drag_max: f32, pub mass_fine: f32, pub mass_coarse: f32 }

impl Coupling {
    pub fn from_json(text: &str) -> Result<Coupling, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("surfaces file is not valid JSON: {e}"))?;
        let f = v.get("dust_air").filter(|o| o.is_object()).ok_or("surfaces: missing object \"dust_air\"")?;
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = f.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("dust_air: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("dust_air: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        Ok(Coupling { push: num("push", 0.0, 50.0)?, drag: num("drag", 0.0, 5.0)?, drag_max: num("drag_max", 0.0, 1.0)?, mass_fine: num("mass_fine", 0.01, 20.0)?, mass_coarse: num("mass_coarse", 0.01, 20.0)? })
    }
}
pub fn coupling() -> Coupling {
    static CELL: std::sync::OnceLock<Coupling> = std::sync::OnceLock::new();
    *CELL.get_or_init(|| Coupling::from_json(super::grip::EMBEDDED).expect("the embedded surfaces.json has a bad dust_air block"))
}

/// Fixed point of the atomic counters of the GPU field.
pub const FP: f32 = 1024.0;

/// The dust field on the airflow's grid: per cell the mass, and the momentum (mass times velocity relative to the car, in the car's frame), in fixed point
/// (the GPU adds them with integer atomics). Cell (x, y, z) is at `(z * ny + y) * nx + x`, as in the airflow.
pub struct Field { pub n: [usize; 3], pub cell: f32, pub data: Vec<i32> }

impl Field {
    pub fn new(n: [usize; 3], cell: f32) -> Field { Field { n, cell, data: vec![0; n[0] * n[1] * n[2] * 4] } }
    pub fn index(&self, c: [i32; 3]) -> Option<usize> {
        if (0..3).any(|a| c[a] < 0 || c[a] >= self.n[a] as i32) { return None; }
        Some(((c[2] as usize * self.n[1] + c[1] as usize) * self.n[0] + c[0] as usize) * 4)
    }
    /// Adds a particle at `b` (box coordinates, metres) with mass `w` and velocity `v` (relative to the car, in its frame); outside the box it is ignored.
    pub fn splat(&mut self, b: [f32; 3], v: V3, w: f32) {
        let c = b.map(|x| (x / self.cell).floor() as i32);
        let Some(i) = self.index(c) else { return };
        self.data[i] += (w * FP) as i32;
        self.data[i + 1] += (w * v.x * FP) as i32; self.data[i + 2] += (w * v.y * FP) as i32; self.data[i + 3] += (w * v.z * FP) as i32;
    }
    pub fn mass(&self, c: [i32; 3]) -> f32 { self.index(c).map_or(0.0, |i| self.data[i] as f32 / FP) }
    /// The mean velocity of the dust in a cell (zero when empty).
    pub fn mean_velocity(&self, c: [i32; 3]) -> V3 {
        match self.index(c) { Some(i) if self.data[i] > 0 => { let m = self.data[i] as f32; v3(self.data[i + 1] as f32 / m, self.data[i + 2] as f32 / m, self.data[i + 3] as f32 / m) } _ => V3::ZERO }
    }
    /// The mass at a box position, trilinear between the cell centres.
    pub fn mass_at(&self, b: [f32; 3]) -> f32 {
        let g = b.map(|x| x / self.cell - 0.5);
        let i = g.map(|x| x.floor());
        let f = [g[0] - i[0], g[1] - i[1], g[2] - i[2]];
        let c0 = [i[0] as i32, i[1] as i32, i[2] as i32];
        let m = |dx: i32, dy: i32, dz: i32| self.mass([c0[0] + dx, c0[1] + dy, c0[2] + dz]);
        let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let x00 = l(m(0, 0, 0), m(1, 0, 0), f[0]); let x10 = l(m(0, 1, 0), m(1, 1, 0), f[0]);
        let x01 = l(m(0, 0, 1), m(1, 0, 1), f[0]); let x11 = l(m(0, 1, 1), m(1, 1, 1), f[0]);
        l(l(x00, x10, f[1]), l(x01, x11, f[1]), f[2])
    }
    /// The acceleration (box frame) that pushes a particle at `b` away from denser dust: minus `push` times the mass gradient (per cell).
    pub fn push(&self, b: [f32; 3], push: f32) -> V3 {
        let d = |a: usize| { let (mut p, mut m) = (b, b); p[a] += self.cell; m[a] -= self.cell; (self.mass_at(p) - self.mass_at(m)) * 0.5 };
        v3(-d(0), -d(1), -d(2)) * push
    }
}

/// How strongly the dust drags the air in a cell holding `mass`: `drag` per unit of mass, at most `drag_max`.
pub fn air_drag(mass: f32, c: &Coupling) -> f32 { (mass * c.drag).clamp(0.0, c.drag_max) }

#[cfg(test)]
#[path = "dust_ref_tests.rs"]
mod tests;
