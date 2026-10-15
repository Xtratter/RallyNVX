//! The airflow around the car (aero step 2, see docs/superpowers/specs/2026-10-15-airflow-design.md): the CPU reference solver in the car's
//! frame (x lateral, y up, z forward) that the compute shaders mirror, and the voxel body.
use super::vehicle::VehicleSpec;

/// The numbers of `surfaces.json` -> `airflow`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirflowCfg { pub debug: bool, pub cell: f32, pub n: [usize; 3], pub iterations: u32, pub substeps: u32, pub confinement: f32 }

impl AirflowCfg {
    pub fn from_json(text: &str) -> Result<AirflowCfg, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("surfaces file is not valid JSON: {e}"))?;
        let f = v.get("airflow").filter(|o| o.is_object()).ok_or("surfaces: missing object \"airflow\"")?;
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = f.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("airflow: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("airflow: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        let debug = f.get("debug").and_then(|d| d.as_f64()).map_or(false, |d| d > 0.5);
        Ok(AirflowCfg { debug, cell: num("cell", 0.1, 1.0)?, n: [num("nx", 8.0, 128.0)? as usize, num("ny", 8.0, 64.0)? as usize, num("nz", 16.0, 256.0)? as usize],
            iterations: num("iterations", 1.0, 64.0)? as u32, substeps: num("substeps", 1.0, 4.0)? as u32, confinement: num("confinement", 0.0, 2.0)? })
    }
}

/// The wake model (see `Airflow::wake`): its length in metres, the relaxation rate (1/s) right behind a solid and the speed left there (share of the inflow).
pub const WAKE_LEN: f32 = 6.0;
pub const WAKE_RATE: f32 = 10.0;
pub const WAKE_FLOOR: f32 = 0.2;

pub fn cfg() -> AirflowCfg { AirflowCfg::from_json(super::grip::EMBEDDED).expect("the embedded surfaces.json has a bad airflow block") }

fn clampi(a: i32, n: usize) -> usize { a.clamp(0, n as i32 - 1) as usize }

pub struct Airflow {
    pub n: [usize; 3], pub dx: f32,
    pub vel: Vec<[f32; 3]>, pres: Vec<f32>, div: Vec<f32>,
    /// `Some(velocity)` where the cell is solid (the car).
    pub solid: Vec<Option<[f32; 3]>>,
    pub inflow: [f32; 3],
}

impl Airflow {
    pub fn new(n: [usize; 3], dx: f32) -> Airflow {
        let c = n[0] * n[1] * n[2];
        Airflow { n, dx, vel: vec![[0.0; 3]; c], pres: vec![0.0; c], div: vec![0.0; c], solid: vec![None; c], inflow: [0.0; 3] }
    }
    pub fn idx(&self, c: [i32; 3]) -> usize { (clampi(c[2], self.n[2]) * self.n[1] + clampi(c[1], self.n[1])) * self.n[0] + clampi(c[0], self.n[0]) }
    fn cells(&self) -> Vec<[i32; 3]> {
        let n = self.n;
        (0..n[2] as i32).flat_map(move |z| (0..n[1] as i32).flat_map(move |y| (0..n[0] as i32).map(move |x| [x, y, z]))).collect()
    }
    /// The air velocity coming at the car (in the car frame); the air in the box starts at it.
    pub fn set_inflow(&mut self, v: [f32; 3]) {
        self.inflow = v;
        for i in 0..self.vel.len() { if self.solid[i].is_none() { self.vel[i] = v; } }
    }
    /// A solid box of cells `lo..hi` (the car; it does not move in its own frame).
    pub fn set_solid_box(&mut self, lo: [i32; 3], hi: [i32; 3]) {
        for c in self.cells() {
            if (0..3).all(|a| c[a] >= lo[a] && c[a] < hi[a]) { let i = self.idx(c); self.solid[i] = Some([0.0; 3]); self.vel[i] = [0.0; 3]; }
        }
    }
    /// The air beyond the box: the incoming air in front, the road (moving with the inflow, no vertical speed) below.
    fn outside(&self, c: [i32; 3]) -> Option<[f32; 3]> {
        if c[2] >= self.n[2] as i32 { return Some(self.inflow); }
        if c[1] < 0 { return Some([self.inflow[0], 0.0, self.inflow[2]]); }
        None
    }
    fn v_at(&self, c: [i32; 3]) -> [f32; 3] {
        if let Some(v) = self.outside(c) { return v; }
        let i = self.idx(c);
        self.solid[i].unwrap_or(self.vel[i])
    }
    fn sample(&self, f: &[[f32; 3]], p: [f32; 3]) -> [f32; 3] {
        let g = p.map(|v| v / self.dx - 0.5);
        let i = g.map(|v| v.floor());
        let t = [g[0] - i[0], g[1] - i[1], g[2] - i[2]];
        let c0 = [i[0] as i32, i[1] as i32, i[2] as i32];
        let at = |dx: i32, dy: i32, dz: i32| { let c = [c0[0] + dx, c0[1] + dy, c0[2] + dz]; self.outside(c).unwrap_or_else(|| f[self.idx(c)]) };
        let l = |a: [f32; 3], b: [f32; 3], t: f32| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t);
        let x00 = l(at(0, 0, 0), at(1, 0, 0), t[0]); let x10 = l(at(0, 1, 0), at(1, 1, 0), t[0]);
        let x01 = l(at(0, 0, 1), at(1, 0, 1), t[0]); let x11 = l(at(0, 1, 1), at(1, 1, 1), t[0]);
        l(l(x00, x10, t[1]), l(x01, x11, t[1]), t[2])
    }
    fn centre(&self, c: [i32; 3]) -> [f32; 3] { c.map(|v| (v as f32 + 0.5) * self.dx) }
    /// Back-trace of at most a few cells (a large dt or speed must not throw the sample out of the box).
    fn trace(&self, p: [f32; 3], v: [f32; 3], dt: f32) -> [f32; 3] {
        let max = 4.0 * self.dx;
        [0, 1, 2].map(|k| p[k] - (v[k] * dt).clamp(-max, max))
    }
    /// BFECC: forward, backward, then the error-corrected semi-Lagrangian advection of the velocity.
    fn advect(&mut self, dt: f32) {
        let cells = self.cells();
        let old = self.vel.clone();
        let fwd: Vec<[f32; 3]> = cells.iter().map(|&c| { let i = self.idx(c); self.sample(&old, self.trace(self.centre(c), old[i], dt)) }).collect();
        let back: Vec<[f32; 3]> = cells.iter().map(|&c| { let i = self.idx(c); self.sample(&fwd, self.trace(self.centre(c), fwd[i], -dt)) }).collect();
        let corrected: Vec<[f32; 3]> = (0..old.len()).map(|i| [0, 1, 2].map(|k| old[i][k] + 0.5 * (old[i][k] - back[i][k]))).collect();
        for c in cells {
            let i = self.idx(c);
            if let Some(s) = self.solid[i] { self.vel[i] = s; continue; }
            let q = self.trace(self.centre(c), old[i], dt);
            let plain = self.sample(&old, q);
            let mut v = self.sample(&corrected, q);
            // the correction may only move the plain result a little, which keeps it stable
            for k in 0..3 { let m = 0.5 * old[i][k].abs().max(plain[k].abs()) + 1.0; v[k] = v[k].clamp(plain[k] - m, plain[k] + m); }
            self.vel[i] = v;
        }
    }
    /// The wake: the cells are too coarse to resolve a boundary layer, so the ideal flow would close behind the car without a trace. Air in the
    /// shadow of a solid (a solid up to `WAKE_LEN` m upstream in the same column) relaxes towards a slowed-down air, slowest right behind it.
    fn wake(&mut self, dt: f32) {
        let len = (WAKE_LEN / self.dx).max(1.0) as i32;
        let cells = self.cells();
        let shadow: Vec<Option<i32>> = cells.iter().map(|&c| {
            if self.solid[self.idx(c)].is_some() { return None; }
            (1..=len).find(|&d| { let n = [c[0], c[1], c[2] + d]; self.outside(n).is_none() && self.solid[self.idx(n)].is_some() })
        }).collect();
        for (c, sh) in cells.into_iter().zip(shadow) {
            let Some(d) = sh else { continue };
            let t = d as f32 / len as f32;
            let k = (WAKE_RATE * (1.0 - t) * dt).min(1.0);
            let target = self.inflow.map(|v| v * (WAKE_FLOOR + (1.0 - WAKE_FLOOR) * t));
            let i = self.idx(c);
            for a in 0..3 { self.vel[i][a] += (target[a] - self.vel[i][a]) * k; }
        }
    }
    /// Vorticity confinement: pushes the air around the vorticity maxima so the wake keeps its swirl.
    fn confine(&mut self, dt: f32, strength: f32) {
        if strength <= 0.0 { return; }
        let cells = self.cells();
        let h = 2.0 * self.dx;
        let w: Vec<[f32; 3]> = cells.iter().map(|&c| {
            let g = |d: [i32; 3], a: usize| self.v_at([c[0] + d[0], c[1] + d[1], c[2] + d[2]])[a];
            [(g([0, 1, 0], 2) - g([0, -1, 0], 2) - g([0, 0, 1], 1) + g([0, 0, -1], 1)) / h,
             (g([0, 0, 1], 0) - g([0, 0, -1], 0) - g([1, 0, 0], 2) + g([-1, 0, 0], 2)) / h,
             (g([1, 0, 0], 1) - g([-1, 0, 0], 1) - g([0, 1, 0], 0) + g([0, -1, 0], 0)) / h]
        }).collect();
        let mag: Vec<f32> = w.iter().map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()).collect();
        for c in cells {
            let i = self.idx(c);
            if self.solid[i].is_some() { continue; }
            let m = |d: [i32; 3]| mag[self.idx([c[0] + d[0], c[1] + d[1], c[2] + d[2]])];
            let grad = [m([1, 0, 0]) - m([-1, 0, 0]), m([0, 1, 0]) - m([0, -1, 0]), m([0, 0, 1]) - m([0, 0, -1])];
            let len = (grad[0] * grad[0] + grad[1] * grad[1] + grad[2] * grad[2]).sqrt();
            if len < 1e-5 { continue; }
            let nn = grad.map(|g| g / len);
            let f = [nn[1] * w[i][2] - nn[2] * w[i][1], nn[2] * w[i][0] - nn[0] * w[i][2], nn[0] * w[i][1] - nn[1] * w[i][0]];
            for k in 0..3 { self.vel[i][k] += f[k] * strength * self.dx * dt; }
        }
    }
    /// The pressure of a neighbour: 0 beyond the back (open), the cell's own against a solid, the cell's own at the sides and the top.
    fn nb(&self, pres: &[f32], i: usize, c: [i32; 3], d: [i32; 3]) -> f32 {
        let n = [c[0] + d[0], c[1] + d[1], c[2] + d[2]];
        if n[2] < 0 { return 0.0; }
        let j = self.idx(n);
        if self.solid[j].is_some() { pres[i] } else { pres[j] }
    }
    fn project(&mut self, iterations: u32) {
        let cells = self.cells();
        for &c in &cells {
            let i = self.idx(c);
            if self.solid[i].is_some() { self.div[i] = 0.0; continue; }
            let g = |d: [i32; 3], a: usize| self.v_at([c[0] + d[0], c[1] + d[1], c[2] + d[2]])[a];
            self.div[i] = (g([1, 0, 0], 0) - g([-1, 0, 0], 0) + g([0, 1, 0], 1) - g([0, -1, 0], 1) + g([0, 0, 1], 2) - g([0, 0, -1], 2)) / (2.0 * self.dx);
        }
        let front = self.n[2] as i32 - 1;
        for _ in 0..iterations {
            let next: Vec<f32> = cells.iter().map(|&c| {
                let i = self.idx(c);
                if self.solid[i].is_some() || c[2] == front { return 0.0; }
                let sum: f32 = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]].iter().map(|&d| self.nb(&self.pres, i, c, d)).sum();
                (sum - self.div[i] * self.dx * self.dx) / 6.0
            }).collect();
            self.pres = next;
        }
        let next: Vec<[f32; 3]> = cells.iter().map(|&c| {
            let i = self.idx(c);
            if self.solid[i].is_some() || c[2] == front { return self.vel[i]; }
            let p = |d: [i32; 3]| self.nb(&self.pres, i, c, d);
            let g = [p([1, 0, 0]) - p([-1, 0, 0]), p([0, 1, 0]) - p([0, -1, 0]), p([0, 0, 1]) - p([0, 0, -1])];
            [0, 1, 2].map(|a| self.vel[i][a] - g[a] / (2.0 * self.dx))
        }).collect();
        self.vel = next;
    }
    /// One step: advect, confine, hold the incoming air at the front, project.
    pub fn step(&mut self, dt: f32, iterations: u32, confinement: f32) {
        self.advect(dt);
        self.wake(dt);
        self.confine(dt, confinement);
        let front = self.n[2] as i32 - 1;
        for y in 0..self.n[1] as i32 { for x in 0..self.n[0] as i32 { let i = self.idx([x, y, front]); if self.solid[i].is_none() { self.vel[i] = self.inflow; } } }
        self.project(iterations);
    }
    /// The largest divergence of a fluid cell away from the front face (what the projection must remove).
    pub fn max_divergence(&self) -> f32 {
        let mut m = 0.0f32;
        for c in self.cells() {
            let i = self.idx(c);
            if self.solid[i].is_some() || c[2] >= self.n[2] as i32 - 2 { continue; }
            let g = |d: [i32; 3], a: usize| self.v_at([c[0] + d[0], c[1] + d[1], c[2] + d[2]])[a];
            m = m.max(((g([1, 0, 0], 0) - g([-1, 0, 0], 0) + g([0, 1, 0], 1) - g([0, -1, 0], 1) + g([0, 0, 1], 2) - g([0, 0, -1], 2)) / (2.0 * self.dx)).abs());
        }
        m
    }
    pub fn speed_at(&self, c: [i32; 3]) -> f32 { let v = self.vel[self.idx(c)]; (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() }
}

/// The car as boxes `(min, max)` in metres in the box's frame: x centred on the car, y up from the road, z forward with the car's centre at `car_z`.
/// The body, plus what the garage parts add (see game/aero.rs): the splitter under the nose, the wing over the tail, the diffuser behind the underbody.
pub fn body_boxes(spec: &VehicleSpec, ground_clear: f32, car_z: f32) -> Vec<([f32; 3], [f32; 3])> {
    let (w, h, l) = (spec.size.x, spec.size.y, spec.size.z);
    let (front, back) = (car_z + l * 0.5, car_z - l * 0.5);
    let k = spec.kit;
    let mut v = vec![([-w * 0.5, ground_clear, back], [w * 0.5, ground_clear + h, front])];
    if k.splitter > 0 { v.push(([-w * 0.5, ground_clear * 0.6, front], [w * 0.5, ground_clear * 0.6 + 0.05, front + 0.1 + 0.08 * k.splitter as f32])); }
    if k.wing > 0 { let y = ground_clear + h + 0.25; v.push(([-w * 0.5, y, back], [w * 0.5, y + 0.06 + 0.05 * k.wing_angle as f32, back + 0.9])); }
    if k.underbody > 0 { v.push(([-w * 0.45, ground_clear * 0.5, back - 0.15 * k.underbody as f32], [w * 0.45, ground_clear + 0.15, back])); }
    v
}

/// The ground clearance of the body in the box, in metres.
pub const GROUND_CLEAR: f32 = 0.15;

/// The boxes for a grid of cell `dx`: every box at least one cell thick, so the thin plates of the parts are not lost.
pub fn thick_boxes(spec: &VehicleSpec, dx: f32, car_z: f32) -> Vec<([f32; 3], [f32; 3])> {
    body_boxes(spec, GROUND_CLEAR, car_z).into_iter().map(|(mut lo, mut hi)| {
        for a in 0..3 { if hi[a] - lo[a] < dx { let m = (hi[a] + lo[a]) * 0.5; lo[a] = m - dx * 0.5; hi[a] = m + dx * 0.5; } }
        (lo, hi)
    }).collect()
}

/// Marks the cells whose centres are inside the car's boxes solid.
pub fn voxelize(af: &mut Airflow, spec: &VehicleSpec, car_z: f32) {
    let dx = af.dx;
    let half_x = af.n[0] as f32 * dx * 0.5;
    let cell = |v: f32| (v / dx - 0.5).ceil() as i32;
    for (lo, hi) in thick_boxes(spec, dx, car_z) {
        af.set_solid_box([cell(lo[0] + half_x), cell(lo[1]), cell(lo[2])], [cell(hi[0] + half_x), cell(hi[1]), cell(hi[2])]);
    }
}

#[cfg(test)]
#[path = "airflow_tests.rs"]
mod tests;
