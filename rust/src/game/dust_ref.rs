//! One step of a dust particle, as the GPU runs it (`shaders/dust_gpu.wgsl`): the host-tested reference. `game/dust.rs` (the CPU particles) uses the
//! same functions, so both paths move the dust the same way.
use super::dust::{Particle, Phys};
use super::math::{v3, V3};

/// A dust particle: `life` in seconds, `size0` the size when it is born, `coarse` heavy grit (else fine dust), `seed` a stable random number.
#[derive(Clone, Copy, Debug)]
pub struct GP { pub pos: V3, pub vel: V3, pub age: f32, pub life: f32, pub size0: f32, pub color: [f32; 3], pub coarse: bool, pub seed: f32,
    /// A flying speck of grit or soil (the GPU dust): tiny, dark, ballistic; it bounces and slides on the ground. Only the GPU path makes them.
    pub speck: bool }

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

// ---- curl noise: the divergence-free turbulence of the GPU dust (Bridson, Hourihan, Nordenstam, "Curl-Noise for Procedural Fluid Flow", 2007).
// The potential is three decorrelated value noises; the velocity is its curl, so it has no sinks or sources and particles swirl without clumping.
// The hash and the noise are written with integer ops so the shader can repeat them exactly (`dust_gpu.wgsl`).

fn pcg(v: u32) -> u32 {
    let s = v.wrapping_mul(747796405).wrapping_add(2891336453);
    let w = ((s >> ((s >> 28) + 4)) ^ s).wrapping_mul(277803737);
    (w >> 22) ^ w
}
/// A random number in [-1, 1] for an integer lattice point and a channel.
pub fn hash3(x: i32, y: i32, z: i32, k: u32) -> f32 {
    let h = pcg((x as u32) ^ pcg((y as u32) ^ pcg((z as u32) ^ pcg(k))));
    h as f32 / 4294967295.0 * 2.0 - 1.0
}
fn fade(t: f32) -> f32 { t * t * t * (t * (t * 6.0 - 15.0) + 10.0) }
/// Smooth value noise in [-1, 1] at `p` for channel `k`.
pub fn vnoise3(p: V3, k: u32) -> f32 {
    let (ix, iy, iz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (f, g, h) = (fade(p.x - ix), fade(p.y - iy), fade(p.z - iz));
    let (x, y, z) = (ix as i32, iy as i32, iz as i32);
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c = |dx: i32, dy: i32, dz: i32| hash3(x + dx, y + dy, z + dz, k);
    let x00 = l(c(0, 0, 0), c(1, 0, 0), f); let x10 = l(c(0, 1, 0), c(1, 1, 0), f);
    let x01 = l(c(0, 0, 1), c(1, 0, 1), f); let x11 = l(c(0, 1, 1), c(1, 1, 1), f);
    l(l(x00, x10, g), l(x01, x11, g), h)
}
fn dfade(t: f32) -> f32 { 30.0 * t * t * (t - 1.0) * (t - 1.0) }
/// The value noise and its analytic gradient (with respect to `p`) in one go.
pub fn vnoise3_grad(p: V3, k: u32) -> (f32, V3) {
    let (ix, iy, iz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (tx, ty, tz) = (p.x - ix, p.y - iy, p.z - iz);
    let (u, v, w) = (fade(tx), fade(ty), fade(tz));
    let (du, dv, dw) = (dfade(tx), dfade(ty), dfade(tz));
    let (x, y, z) = (ix as i32, iy as i32, iz as i32);
    let c = |dx: i32, dy: i32, dz: i32| hash3(x + dx, y + dy, z + dz, k);
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let (c000, c100, c010, c110, c001, c101, c011, c111) = (c(0, 0, 0), c(1, 0, 0), c(0, 1, 0), c(1, 1, 0), c(0, 0, 1), c(1, 0, 1), c(0, 1, 1), c(1, 1, 1));
    let (x00, x10, x01, x11) = (l(c000, c100, u), l(c010, c110, u), l(c001, c101, u), l(c011, c111, u));
    let (y0, y1) = (l(x00, x10, v), l(x01, x11, v));
    let val = l(y0, y1, w);
    let dx = du * l(l(c100 - c000, c110 - c010, v), l(c101 - c001, c111 - c011, v), w);
    let dy = dv * l(x10 - x00, x11 - x01, w);
    let dz = dw * (y1 - y0);
    (val, v3(dx, dy, dz))
}
/// The curl of the noise potential at `q` (noise-space coordinates; the result is of the order of 1 whatever the scale): the potential is three value
/// noises, the curl comes from their analytic gradients, so it is exactly divergence-free.
pub fn curl_noise(q: V3) -> V3 {
    let (g0, g1, g2) = (vnoise3_grad(q, 0).1, vnoise3_grad(q, 1).1, vnoise3_grad(q, 2).1);
    v3(g2.y - g1.z, g0.z - g2.x, g1.x - g0.y)
}
/// Three octaves of curl noise as a velocity (m/s): wavelengths 6, 2.5 and 1 m with weights 1, 0.55 and 0.3, the pattern carried along by `drift`
/// (the wind) and slowly evolving in time `t`; `amp` is the speed of the largest octave.
pub fn turbulence_curl(p: V3, t: f32, drift: V3, amp: f32) -> V3 {
    let q = p - drift * t;
    let mut v = V3::ZERO;
    for (i, (lambda, w)) in [(6.0f32, 1.0f32), (2.5, 0.55), (1.0, 0.3)].iter().enumerate() {
        let shift = v3(0.0, 0.0, t * 0.35 * (i as f32 + 1.0)) + v3(17.0 * i as f32, 31.0 * i as f32, 7.0 * i as f32);
        v = v + curl_noise(q * (1.0 / lambda) + shift) * (amp * w * 0.35);
    }
    v
}

/// The car's trailing vortices (the C-pillar and rear-corner vortices of a hatchback or wing tips of a wing): two counter-rotating line vortices
/// along the car behind its rear corners, rotating so that the air between them goes down and outboard of them up. In the car's frame (x across,
/// y up from the road, z forward, origin under the car's centre): `c`, the half width `hw`, the half length `hl` and the height `h` of the body, the car's speed.
/// The strength is the peak swirl speed as a share of the car's speed (`strength`); the core grows and the swirl fades with the distance behind the rear.
pub fn trailing_vortices(c: V3, hw: f32, hl: f32, h: f32, speed: f32, strength: f32) -> V3 {
    let behind = -c.z - hl;                     // metres behind the rear face
    if behind < -1.0 { return V3::ZERO; }
    let ramp = smooth((behind + 1.0) / 1.5);    // the vortices form over the first metres
    let b = behind.max(0.0);
    let fade = (-b / 11.0).exp();
    let rc = 0.7 + 0.2 * b;                   // the core grows as the vortex ages
    let v_peak = strength * speed * ramp * fade;
    let mut out = V3::ZERO;
    for s in [1.0f32, -1.0] {
        let (cx, cy) = (s * hw * 0.8, 0.55 * h + 0.15);
        let (rx, ry) = (c.x - cx, c.y - cy);
        let r2 = rx * rx + ry * ry;
        // a smooth vortex (peak v_peak at r = rc): v = 2 v_peak r rc / (r^2 + rc^2)
        let k = 2.0 * v_peak * rc / (r2 + rc * rc);
        out = out + v3(-s * ry * k, s * rx * k, 0.0);
    }
    out
}

/// The vortex street behind the bluff body (Karman): vortices with vertical axes are shed alternately from the two sides of the car (Strouhal number
/// 0.2 of the car's width) and drift back with the wake, each turning opposite to its neighbour; seen from behind or above they are the rolling eddies in the dust.
/// Same frame as `trailing_vortices`; `t` is the time, `strength` the peak swirl speed as a share of the car's speed. Five vortices at a time, the oldest fading out.
pub fn karman_street(c: V3, t: f32, hw: f32, hl: f32, h: f32, speed: f32, strength: f32) -> V3 {
    if speed < 0.5 || c.y > 2.0 * h + 1.0 { return V3::ZERO; }
    let period = (2.0 * hw / (0.2 * speed)).clamp(0.12, 2.0);   // one vortex per period
    let n = (t / period).floor();
    let s = t / period - n;
    let up = 1.0 - smooth((c.y - h) / (0.8 * h + 0.2));       // the street lives up to the roof, fading above
    let mut out = V3::ZERO;
    for j in 0..5 {
        let age = (j as f32 + s) * period;
        let z = -hl - 0.6 - 0.8 * speed * age;
        let x = if ((j as f32 + n) as i32) % 2 == 0 { hw * 0.9 } else { -hw * 0.9 };
        let sign = if x > 0.0 { 1.0 } else { -1.0 };
        let fade = (1.0 - age / (5.0 * period)).clamp(0.0, 1.0) * smooth(age / (0.4 * period));   // forms, then fades
        let rc = 0.6 + 0.12 * speed * age * 0.25;
        let (rx, rz) = (c.x - x, c.z - z);
        let k = 2.0 * strength * speed * fade * rc / (rx * rx + rz * rz + rc * rc);
        out = out + v3(-sign * rz * k, 0.0, sign * rx * k);
    }
    out * up
}

/// The recirculation right behind the car's rear face: the air there is drawn in towards the centre line, back to the car and up its tail, which is what
/// holds the dust against the rear and lifts it into the cloud. Same frame as `trailing_vortices`; it reaches about 6 m behind and fades quickly.
pub fn wake_inflow(c: V3, hw: f32, hl: f32, h: f32, speed: f32, k: f32) -> V3 {
    let behind = -c.z - hl;
    if behind < -0.5 || behind > 8.0 || speed < 0.5 { return V3::ZERO; }
    let f = (-behind.max(0.0) / 2.5).exp() * smooth((behind + 0.5) / 1.0);
    let wide = (-(c.x / (hw + 1.0)).powi(2)).exp();
    let tall = (-(c.y / (h * 1.1)).powi(2)).exp();
    let m = k * speed * f;
    v3(-c.x / (hw + 0.5) * 0.6 * m * tall, 0.12 * m * wide * tall, 0.35 * m * wide * tall)
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
pub struct Coupling { pub push: f32, pub drag: f32, pub drag_max: f32, pub mass_fine: f32, pub mass_coarse: f32, pub shadow_k: f32 }

impl Coupling {
    pub fn from_json(text: &str) -> Result<Coupling, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("surfaces file is not valid JSON: {e}"))?;
        let f = v.get("dust_air").filter(|o| o.is_object()).ok_or("surfaces: missing object \"dust_air\"")?;
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = f.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("dust_air: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("dust_air: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        Ok(Coupling { push: num("push", 0.0, 50.0)?, drag: num("drag", 0.0, 5.0)?, drag_max: num("drag_max", 0.0, 1.0)?, mass_fine: num("mass_fine", 0.01, 20.0)?, mass_coarse: num("mass_coarse", 0.01, 20.0)?,
            shadow_k: f.get("shadow_k").and_then(|x| x.as_f64()).filter(|x| (0.0..=10.0).contains(x)).map_or(0.8, |x| x as f32) })
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
