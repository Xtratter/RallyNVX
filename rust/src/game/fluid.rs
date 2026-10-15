//! The reference solver of the fluid dust (the experiment, see docs/superpowers/specs/2026-10-14-dust-fluid-design.md): the same
//! algorithm the compute shaders run, on a small wrap-around grid, so its behaviour is tested on the host. Cell (i, j, k) is
//! the cell of world x in [i, i + 1) * dx (and the same for y = j, z = k); the grid is a torus of `n` cells per axis.

/// The numbers of `surfaces.json` -> `fluid`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidCfg { pub debug: bool, pub cell: f32, pub n: [usize; 3], pub iterations: u32, pub settle: f32, pub decay: f32, pub inject: f32, pub absorb: f32, pub drag: f32, pub steps: u32 }

impl FluidCfg {
    pub fn from_json(text: &str) -> Result<FluidCfg, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("surfaces file is not valid JSON: {e}"))?;
        let f = v.get("fluid").filter(|o| o.is_object()).ok_or("surfaces: missing object \"fluid\"")?;
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = f.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("fluid: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("fluid: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        let debug = f.get("debug").and_then(|d| d.as_f64()).map_or(false, |d| d > 0.5);   // optional: draws any dust in green
        Ok(FluidCfg { debug, cell: num("cell", 0.25, 3.0)?, n: [num("nx", 8.0, 256.0)? as usize, num("ny", 4.0, 64.0)? as usize, num("nz", 8.0, 256.0)? as usize],
            iterations: num("iterations", 1.0, 64.0)? as u32, settle: num("settle", 0.0, 3.0)?, decay: num("decay", 0.0, 2.0)?, inject: num("inject", 0.0, 10.0)?,
            absorb: num("absorb", 0.01, 10.0)?, drag: num("drag", 0.0, 5.0)?, steps: num("steps", 8.0, 128.0)? as u32 })
    }
}

/// The embedded numbers.
pub fn cfg() -> FluidCfg { FluidCfg::from_json(super::grip::EMBEDDED).expect("the embedded surfaces.json has a bad fluid block") }

fn posmod(a: i32, n: i32) -> i32 { ((a % n) + n) % n }

/// The window of cells around the car: `start` is the first cell of each axis, `n` the size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window { pub start: [i32; 3], pub n: [usize; 3] }

impl Window {
    /// The window centred on a world position (cells of `dx` metres; the vertical window starts `below` metres under it).
    pub fn around(pos: [f32; 3], dx: f32, n: [usize; 3], below: f32) -> Window {
        let c = |p: f32| (p / dx).floor() as i32;
        Window { start: [c(pos[0]) - (n[0] / 2) as i32, c(pos[1] - below), c(pos[2]) - (n[2] / 2) as i32], n }
    }
    /// The cell of the window that texel `t` stands for.
    pub fn cell_of(&self, t: [i32; 3]) -> [i32; 3] { [0, 1, 2].map(|a| self.start[a] + posmod(t[a] - self.start[a], self.n[a] as i32)) }
    /// True when texel `t` held a different cell in `prev` than it does now (its content belongs to another place and is dropped).
    pub fn stale(&self, prev: &Window, t: [i32; 3]) -> bool { self.cell_of(t) != prev.cell_of(t) }
}

pub struct Fluid {
    pub n: [usize; 3],
    pub dx: f32,
    pub vel: Vec<[f32; 3]>,
    pub dens: Vec<f32>,
    pub pres: Vec<f32>,
    div: Vec<f32>,
    /// `Some(velocity)` where the cell is solid (the ground, the car).
    pub solid: Vec<Option<[f32; 3]>>,
    pub wind: [f32; 3],
}

impl Fluid {
    pub fn new(n: [usize; 3], dx: f32) -> Fluid {
        let c = n[0] * n[1] * n[2];
        Fluid { n, dx, vel: vec![[0.0; 3]; c], dens: vec![0.0; c], pres: vec![0.0; c], div: vec![0.0; c], solid: vec![None; c], wind: [0.0; 3] }
    }
    pub fn idx(&self, c: [i32; 3]) -> usize {
        let (x, y, z) = (posmod(c[0], self.n[0] as i32) as usize, posmod(c[1], self.n[1] as i32) as usize, posmod(c[2], self.n[2] as i32) as usize);
        (z * self.n[1] + y) * self.n[0] + x
    }
    fn cells(&self) -> impl Iterator<Item = [i32; 3]> {
        let n = self.n;
        (0..n[2] as i32).flat_map(move |z| (0..n[1] as i32).flat_map(move |y| (0..n[0] as i32).map(move |x| [x, y, z])))
    }
    pub fn centre(&self, c: [i32; 3]) -> [f32; 3] { c.map(|v| (v as f32 + 0.5) * self.dx) }

    /// Trilinear sample of a per-cell field at a world position (cell centres are the sample points).
    fn sample<T: Copy>(&self, f: &[T], p: [f32; 3], lerp: impl Fn(T, T, f32) -> T) -> T {
        let g = p.map(|v| v / self.dx - 0.5);
        let i = g.map(|v| v.floor());
        let t = [g[0] - i[0], g[1] - i[1], g[2] - i[2]];
        let c0 = [i[0] as i32, i[1] as i32, i[2] as i32];
        let at = |dx: i32, dy: i32, dz: i32| f[self.idx([c0[0] + dx, c0[1] + dy, c0[2] + dz])];
        let x00 = lerp(at(0, 0, 0), at(1, 0, 0), t[0]); let x10 = lerp(at(0, 1, 0), at(1, 1, 0), t[0]);
        let x01 = lerp(at(0, 0, 1), at(1, 0, 1), t[0]); let x11 = lerp(at(0, 1, 1), at(1, 1, 1), t[0]);
        lerp(lerp(x00, x10, t[1]), lerp(x01, x11, t[1]), t[2])
    }
    pub fn sample_vel(&self, p: [f32; 3]) -> [f32; 3] { self.sample(&self.vel, p, |a, b, t| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t)) }
    pub fn sample_dens(&self, p: [f32; 3]) -> f32 { self.sample(&self.dens, p, |a, b, t| a + (b - a) * t) }

    /// Marks the solid cells: `f` gets a cell centre and returns the velocity of the solid there, `None` for air. Solid cells hold no dust.
    pub fn set_solids(&mut self, f: impl Fn([f32; 3]) -> Option<[f32; 3]>, window: &Window) {
        for c in self.cells() {
            let w = window.cell_of(c);
            let i = self.idx(c);
            self.solid[i] = f(self.centre(w));
            if self.solid[i].is_some() { self.dens[i] = 0.0; }
        }
    }

    /// Drops what the cells that changed hands when the window moved from `prev` held: air at the wind's velocity, no dust, no pressure.
    pub fn recycle(&mut self, prev: &Window, now: &Window) {
        for c in self.cells() {
            if now.stale(prev, c) { let i = self.idx(c); self.vel[i] = self.wind; self.dens[i] = 0.0; self.pres[i] = 0.0; }
        }
    }

    /// Semi-Lagrangian advection of the velocity with the world positions of the window; solids keep their own velocity.
    pub fn advect_velocity(&mut self, window: &Window, dt: f32, drag: f32) {
        let old = self.vel.clone();
        let src = Fluid { vel: old.clone(), ..self.shallow() };
        for c in self.cells() {
            let i = self.idx(c);
            if let Some(v) = self.solid[i] { self.vel[i] = v; continue; }
            let p = self.centre(window.cell_of(c));
            let back = [0, 1, 2].map(|k| p[k] - old[i][k] * dt);
            let mut v = src.sample_vel(back);
            let k = (drag * dt).min(1.0);
            for a in 0..3 { v[a] += (self.wind[a] - v[a]) * k; }
            self.vel[i] = v;
        }
    }
    fn shallow(&self) -> Fluid { Fluid { n: self.n, dx: self.dx, vel: Vec::new(), dens: Vec::new(), pres: Vec::new(), div: Vec::new(), solid: Vec::new(), wind: self.wind } }

    fn vel_or_solid(&self, c: [i32; 3], from: usize) -> [f32; 3] { let i = self.idx(c); let _ = from; self.solid[i].unwrap_or(self.vel[i]) }

    pub fn divergence(&mut self) {
        for c in self.cells() {
            let i = self.idx(c);
            if self.solid[i].is_some() { self.div[i] = 0.0; continue; }
            let g = |d: [i32; 3], a: usize| self.vel_or_solid([c[0] + d[0], c[1] + d[1], c[2] + d[2]], i)[a];
            self.div[i] = (g([1, 0, 0], 0) - g([-1, 0, 0], 0) + g([0, 1, 0], 1) - g([0, -1, 0], 1) + g([0, 0, 1], 2) - g([0, 0, -1], 2)) / (2.0 * self.dx);
        }
    }
    /// One Jacobi iteration of the pressure; a solid neighbour counts as having the cell's own pressure (no flow through it).
    pub fn jacobi(&mut self) {
        let old = self.pres.clone();
        for c in self.cells() {
            let i = self.idx(c);
            if self.solid[i].is_some() { self.pres[i] = 0.0; continue; }
            let p = |d: [i32; 3]| { let j = self.idx([c[0] + d[0], c[1] + d[1], c[2] + d[2]]); if self.solid[j].is_some() { old[i] } else { old[j] } };
            let sum = p([1, 0, 0]) + p([-1, 0, 0]) + p([0, 1, 0]) + p([0, -1, 0]) + p([0, 0, 1]) + p([0, 0, -1]);
            self.pres[i] = (sum - self.div[i] * self.dx * self.dx) / 6.0;
        }
    }
    pub fn subtract_gradient(&mut self) {
        let old = self.vel.clone();
        for c in self.cells() {
            let i = self.idx(c);
            if self.solid[i].is_some() { continue; }
            let p = |d: [i32; 3]| { let j = self.idx([c[0] + d[0], c[1] + d[1], c[2] + d[2]]); if self.solid[j].is_some() { self.pres[i] } else { self.pres[j] } };
            let g = [(p([1, 0, 0]) - p([-1, 0, 0])), (p([0, 1, 0]) - p([0, -1, 0])), (p([0, 0, 1]) - p([0, 0, -1]))];
            for a in 0..3 { self.vel[i][a] = old[i][a] - g[a] / (2.0 * self.dx); }
        }
    }
    /// The whole incompressible step of the velocity: advect, project.
    pub fn step_velocity(&mut self, window: &Window, dt: f32, drag: f32, iterations: u32) {
        self.advect_velocity(window, dt, drag);
        self.divergence();
        for _ in 0..iterations { self.jacobi(); }
        self.subtract_gradient();
    }
    /// The largest divergence of a fluid cell (what the projection must remove).
    pub fn max_divergence(&mut self) -> f32 { self.divergence(); self.cells().map(|c| self.div[self.idx(c)].abs()).fold(0.0, f32::max) }

    /// Advection of the dust with the velocity, settling by `settle` m/s and decay by `decay` per second; solids hold nothing.
    pub fn advect_dust(&mut self, window: &Window, dt: f32, settle: f32, decay: f32) {
        let old = self.dens.clone();
        let src = Fluid { dens: old.clone(), ..self.shallow() };
        for c in self.cells() {
            let i = self.idx(c);
            if self.solid[i].is_some() { self.dens[i] = 0.0; continue; }
            let p = self.centre(window.cell_of(c));
            let v = self.vel[i];
            let back = [p[0] - v[0] * dt, p[1] - (v[1] - settle) * dt, p[2] - v[2] * dt];
            self.dens[i] = src.sample_dens(back) * (1.0 - decay * dt).max(0.0);
        }
    }
    /// Adds `amount` of dust (volume units: density x cell) in the cell holding the world position.
    pub fn emit(&mut self, window: &Window, pos: [f32; 3], amount: f32) {
        let cell = pos.map(|v| (v / self.dx).floor() as i32);
        for c in self.cells() {
            if window.cell_of(c) == cell { let i = self.idx(c); if self.solid[i].is_none() { self.dens[i] += amount; } }
        }
    }
    pub fn mass(&self) -> f32 { self.dens.iter().sum() }
    pub fn centre_of_mass(&self, window: &Window) -> [f32; 3] {
        let (mut s, mut m) = ([0.0f32; 3], 0.0f32);
        for c in self.cells() { let d = self.dens[self.idx(c)]; if d > 0.0 { let p = self.centre(window.cell_of(c)); for a in 0..3 { s[a] += p[a] * d; } m += d; } }
        if m > 0.0 { [s[0] / m, s[1] / m, s[2] / m] } else { [0.0; 3] }
    }
}

/// What the GPU needs of the car and its wheels each step (see `fluid.wgsl`, struct F).
#[derive(Clone, Copy, Debug)]
pub struct Packed {
    pub car: [f32; 4], pub car_half: [f32; 4], pub car_vel: [f32; 4],
    pub wpos: [[f32; 4]; 4], pub wvel: [[f32; 4]; 4], pub wcol: [[f32; 4]; 4],
}

/// The car's box (its yaw, size, velocity) and the four wheel emitters: the dust rate from the surface under each wheel
/// (rolling and sliding, as the particle dust), the air the wheel throws (back and up, more when it slips) and the dust colour.
pub fn pack(v: &super::vehicle::Vehicle, cfg: &FluidCfg, table: &super::grip::GripTable) -> Packed {
    let fwd = v.forward();
    let yaw = fwd.x.atan2(fwd.z);
    let s = v.spec.size;
    let mut p = Packed { car: [v.pos.x, v.pos.y + 0.1, v.pos.z, yaw], car_half: [s.x * 0.5 + 0.05, s.y * 0.5, s.z * 0.5 + 0.05, 0.0], car_vel: [v.vel.x, v.vel.y, v.vel.z, 0.0],
        wpos: [[0.0; 4]; 4], wvel: [[0.0, 0.0, 0.0, 1.0]; 4], wcol: [[0.0; 4]; 4] };
    for w in 0..4 {
        let info = v.wheels[w];
        if !info.grounded || !info.speed.is_finite() || !info.slide.is_finite() || !info.lat_slip.is_finite() || !info.contact.is_finite() { continue; }
        let d = table.dust(info.surface);
        let roll = if info.speed > 2.0 { d.roll * 14.0 * (info.speed / 20.0).min(1.5) } else { 0.0 };
        let slide = d.slide * (40.0 * info.slide.min(1.5) + 18.0 * (info.lat_slip / 6.0).min(1.5));
        let rate = (roll + slide) * cfg.inject * 0.35;
        p.wpos[w] = [info.contact.x, info.contact.y + 0.15, info.contact.z, rate];
        let up = 1.5 + 2.0 * info.slide.min(1.5);
        p.wvel[w] = [-v.vel.x * 0.55, up, -v.vel.z * 0.55, 1.0];
        p.wcol[w] = [d.color[0], d.color[1], d.color[2], 0.0];
    }
    p
}

#[cfg(test)]
#[path = "fluid_tests.rs"]
mod tests;
