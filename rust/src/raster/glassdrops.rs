//! Raindrops on the car's glass: how they form, run and dry.
//!
//! The model (sources in docs/vehicles.md):
//! * A drop on glass is pinned by contact-angle hysteresis. Furmidge's criterion: it starts to slide when the force pushing it along the glass exceeds
//!   `gamma * (cos(theta_receding) - cos(theta_advancing)) * D` (surface tension times the hysteresis times the drop's width).
//! * What pushes it: its weight along the slope (`rho g V sin(alpha)`, growing with `r^3`) and the air's drag (`1/2 rho_air u^2 Cd A`, growing with `r^2` and the square of
//!   the speed of the air near the glass). The pinning grows only with `r`, so above a critical radius a drop runs, and the faster the car goes the smaller that radius is.
//! * Small drops stay put and shrink by diffusion-limited evaporation: `d(r^2)/dt` is constant, proportional to `1 - RH` (the vapour the air can still take), boosted by air
//!   moving over them and by a sun-warmed pane. Real lifetimes are minutes to hours; the game runs this 60 times faster.
//! * Rain arrives continuously: a cell of the glass is hit at a random moment of the accumulated rain, then its drop grows (by coalescing) as the cube root of the water it gets.

pub const GAMMA: f32 = 0.0728;            // surface tension of water, N/m
pub const HYSTERESIS: f32 = 0.42;         // cos(40 deg receding) - cos(70 deg advancing), clean glass
pub const RHO_WATER: f32 = 1000.0;
pub const RHO_AIR: f32 = 1.2;
pub const DRAG_COEFF: f32 = 0.6;          // a low dome of water in a wall flow
pub const NEAR_WALL: f32 = 0.5;           // the air a millimetre off the glass moves at this share of the free stream
pub const G: f32 = 9.81;
pub const VISCOSITY: f32 = 1.0e-3;        // Pa s
pub const CONTACT_LINE_DRAG: f32 = 12.0;  // how much harder than Stokes' drag the sliding of a pinned-edge drop is
pub const MAX_SLIDE: f32 = 0.8;           // m/s
/// Seconds of heavy rain that bring the glass from dry to fully wetted.
pub const FILL_TIME: f32 = 3.0;
/// Evaporation of `r^2` (mm^2 per second) for dry air (RH 0) at rest: the real `~8.7e-4 mm^2/s` times 60 (game time) times about 3 for the thin spread of a drop on glass.
pub const EVAP_K: f32 = 0.16;
pub const MAX_SLIDERS: usize = 32;

/// The force (N, along the glass) pushing a drop of radius `r_mm` (a hemispherical dome): its weight along the slope plus the air's drag. `g_slope` is gravity's component along
/// the glass as a vector in the glass's plane (length `sin(alpha)`), `air` the air's velocity relative to the glass projected into the plane (m/s, the free stream).
pub fn drive_force(r_mm: f32, g_slope: [f32; 2], air: [f32; 2]) -> [f32; 2] {
    let r = r_mm.max(0.0) * 1e-3;
    let weight = RHO_WATER * G * (2.0 / 3.0) * std::f32::consts::PI * r * r * r;
    let speed = (air[0] * air[0] + air[1] * air[1]).sqrt();
    let u = NEAR_WALL * speed;
    let drag = 0.5 * RHO_AIR * u * u * DRAG_COEFF * (std::f32::consts::PI * 0.5 * r * r);
    let dir = if speed > 1e-4 { [air[0] / speed, air[1] / speed] } else { [0.0, 0.0] };
    [g_slope[0] * weight + dir[0] * drag, g_slope[1] * weight + dir[1] * drag]
}

/// Furmidge's pinning force (N) of a drop of radius `r_mm`.
pub fn pinning_force(r_mm: f32) -> f32 { GAMMA * HYSTERESIS * 2.0 * r_mm.max(0.0) * 1e-3 }

/// The smallest radius (mm) at which a drop slides under this gravity along the glass and this air, scanning up to 6 mm (6 when none does). It is not a simple threshold: on a
/// windscreen at speed the air pushes up the slope against gravity, so mid-sized drops run up and the big ones, where weight wins, run down.
pub fn critical_radius_mm(g_slope: [f32; 2], air: [f32; 2]) -> f32 {
    let mut r = 0.1f32;
    while r <= 6.0 {
        let f = drive_force(r, g_slope, air);
        if (f[0] * f[0] + f[1] * f[1]).sqrt() > pinning_force(r) { return r; }
        r += 0.02;
    }
    6.0
}

/// How fast (m/s) a drop of `r_mm` runs once it is above critical: the excess force over the contact line's and the liquid's resistance.
pub fn slide_speed(r_mm: f32, force: [f32; 2]) -> f32 {
    let f = (force[0] * force[0] + force[1] * force[1]).sqrt();
    let excess = f - pinning_force(r_mm);
    if excess <= 0.0 { return 0.0; }
    (excess / (6.0 * std::f32::consts::PI * VISCOSITY * (r_mm * 1e-3).max(1e-5) * CONTACT_LINE_DRAG)).min(MAX_SLIDE)
}

/// The relative humidity of the air at the glass `dry` seconds after the rain stopped: saturated at first, drying towards a damp 55 %.
pub fn humidity(dry: f32) -> f32 { 0.55 + 0.42 * (-dry.max(0.0) / 45.0).exp() }

/// The evaporation of `r^2` (mm^2 per second) in air of humidity `rh` moving over the pane at `air_speed` (m/s) with `sun` (0..1) warming it.
pub fn evaporation_rate(rh: f32, air_speed: f32, sun: f32) -> f32 {
    EVAP_K * (1.0 - rh.clamp(0.0, 1.0)) * (1.0 + 0.15 * air_speed.max(0.0).sqrt()) * (1.0 + 0.6 * sun.clamp(0.0, 1.0))
}

/// The radius (mm) of the static drop of a cell: it is hit when the accumulated `water` passes `arrival`, grows as the cube root of the water it has received (`GROW` units
/// to its full size `r0`), loses `evap` mm^2 to evaporation and is never wider than `cap` (it would run). Mirrored in `fs_glass`.
pub fn static_drop_radius(r0: f32, arrival: f32, water: f32, evap: f32, cap: f32) -> f32 {
    const GROW: f32 = 0.35;
    let g = ((water - arrival) / GROW).clamp(0.0, 1.0);
    let r = r0 * g.cbrt();
    (r * r - evap).max(0.0).sqrt().min(cap)
}

/// The plane of the windscreen in the car's object space, where the running drops live.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Windscreen { pub origin: [f32; 3], pub u: [f32; 3], pub v: [f32; 3], pub n: [f32; 3], pub u_range: [f32; 2], pub v_range: [f32; 2] }

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn norm(a: [f32; 3]) -> [f32; 3] { let l = dot(a, a).sqrt().max(1e-9); [a[0] / l, a[1] / l, a[2] / l] }

impl Windscreen {
    /// Finds the windscreen in the glass mesh: the triangles that face forward and up, high on the car. `None` when there are none.
    pub fn from_glass(verts: &[[f32; 3]], idx: &[u32]) -> Option<Windscreen> {
        let mut pts: Vec<[f32; 3]> = Vec::new();
        let mut nsum = [0.0f32; 3];
        for t in idx.chunks_exact(3) {
            let (a, b, c) = (verts[t[0] as usize], verts[t[1] as usize], verts[t[2] as usize]);
            let fnrm = cross(sub(b, a), sub(c, a));
            let area = dot(fnrm, fnrm).sqrt();
            if area < 1e-9 { continue; }
            let mut n = [fnrm[0] / area, fnrm[1] / area, fnrm[2] / area];
            if n[1] < 0.0 { n = [-n[0], -n[1], -n[2]]; }   // the glass's winding may face either way: take the upward side
            let cen = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
            if n[1] > 0.45 && n[2] > 0.3 && cen[1] > 0.85 && cen[2] > 0.2 {
                pts.extend_from_slice(&[a, b, c]);
                for k in 0..3 { nsum[k] += n[k] * area; }
            }
        }
        if pts.len() < 3 { return None; }
        let n = norm(nsum);
        let k = pts.len() as f32;
        let origin = [pts.iter().map(|p| p[0]).sum::<f32>() / k, pts.iter().map(|p| p[1]).sum::<f32>() / k, pts.iter().map(|p| p[2]).sum::<f32>() / k];
        let x = [1.0, 0.0, 0.0];
        let u = norm(sub(x, [n[0] * dot(x, n), n[1] * dot(x, n), n[2] * dot(x, n)]));
        let mut v = cross(n, u);
        if v[1] < 0.0 { v = [-v[0], -v[1], -v[2]]; }   // v runs up the slope
        let (mut ur, mut vr) = ([f32::MAX, f32::MIN], [f32::MAX, f32::MIN]);
        for p in &pts { let d = sub(*p, origin); let (a, b) = (dot(d, u), dot(d, v)); ur = [ur[0].min(a), ur[1].max(a)]; vr = [vr[0].min(b), vr[1].max(b)]; }
        Some(Windscreen { origin, u, v, n, u_range: ur, v_range: vr })
    }
    /// The object-space point at plane coordinates `(a, b)`.
    pub fn at(&self, a: f32, b: f32) -> [f32; 3] { [self.origin[0] + self.u[0] * a + self.v[0] * b, self.origin[1] + self.u[1] * a + self.v[1] * b, self.origin[2] + self.u[2] * a + self.v[2] * b] }
    /// A vector of the object space as components in the plane.
    pub fn project(&self, w: [f32; 3]) -> [f32; 2] { [dot(w, self.u), dot(w, self.v)] }
    /// Whether plane coordinates are inside the glass's bounding box (shrunk by `margin` metres).
    pub fn contains(&self, a: f32, b: f32, margin: f32) -> bool { a > self.u_range[0] + margin && a < self.u_range[1] - margin && b > self.v_range[0] + margin && b < self.v_range[1] - margin }
}

/// A drop that runs down (or up, or across) the windscreen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slider { pub a: f32, pub b: f32, pub r_mm: f32, pub dir: [f32; 2], pub speed: f32 }

/// What the film on the glass needs to know each frame.
#[derive(Clone, Copy, Debug)]
pub struct Weather { pub rain: f32, pub air_speed: f32, pub sun: f32 }

/// The state of the water on the glass: the static field's accumulated water and evaporation, and the running drops.
#[derive(Clone, Debug)]
pub struct Film { pub water: f32, pub evap: f32, pub dry: f32, pub sliders: Vec<Slider>, rng: u32, spawn: f32 }

impl Default for Film { fn default() -> Self { Film { water: 0.0, evap: 0.0, dry: 0.0, sliders: Vec::new(), rng: 0x9E3779B9, spawn: 0.0 } } }

impl Film {
    fn rand(&mut self) -> f32 { self.rng ^= self.rng << 13; self.rng ^= self.rng >> 17; self.rng ^= self.rng << 5; (self.rng >> 8) as f32 / 16777216.0 }

    /// Advances the film by `dt` seconds. `g_slope` and `air` are as in `drive_force`, in the windscreen's plane; `wiped` says whether a point of the windscreen (plane
    /// coordinates) is under a wiper blade right now (a running drop there is swept away). Returns the radius (mm) above which a drop would run.
    pub fn advance(&mut self, dt: f32, w: Weather, screen: Option<&Windscreen>, g_slope: [f32; 2], air: [f32; 2], wiped: &dyn Fn(f32, f32) -> bool) -> f32 {
        let dt = dt.clamp(0.0, 0.25);
        let rain = if w.rain.is_finite() { w.rain.clamp(0.0, 1.0) } else { 0.0 };
        if rain >= 0.05 {
            self.water = (self.water + dt * rain / FILL_TIME).min(3.0);
            self.dry = 0.0;
            self.evap = (self.evap - dt * rain * 1.5).max(0.0);
        } else {
            self.dry += dt;
            self.evap += dt * evaporation_rate(humidity(self.dry), w.air_speed, w.sun);
            if self.evap > 40.0 { self.water = 0.0; self.evap = 0.0; }   // everything has dried: start over
        }
        let rcrit = critical_radius_mm(g_slope, air);
        if let Some(s) = screen {
            // new runners: drops that have grown to the critical size by coalescing, at a rate that follows the rain
            if rain >= 0.05 {
                self.spawn += dt * rain * 1.6 * (0.4 + (w.air_speed / 20.0).min(1.0));
                while self.spawn >= 1.0 {
                    self.spawn -= 1.0;
                    if self.sliders.len() >= MAX_SLIDERS { break; }
                    let (ra, rb, rr) = (self.rand(), self.rand(), self.rand());
                    let a = s.u_range[0] + 0.04 + (s.u_range[1] - s.u_range[0] - 0.08) * ra;
                    let b = s.v_range[0] + 0.04 + (s.v_range[1] - s.v_range[0] - 0.08) * rb;
                    self.sliders.push(Slider { a, b, r_mm: (rcrit * (1.05 + 0.5 * rr)).clamp(1.2, 4.5), dir: [0.0, 1.0], speed: 0.0 });
                }
            }
            let (water, evap_rate) = (self.water.clamp(0.0, 1.0), evaporation_rate(humidity(self.dry), w.air_speed, w.sun));
            let film_h = 0.04 * water;   // mm of water a runner sweeps up per unit length of its path
            let rain_on = rain >= 0.05;
            self.sliders.retain_mut(|d| {
                let f = drive_force(d.r_mm, g_slope, air);
                let sp = slide_speed(d.r_mm, f);
                let fl = (f[0] * f[0] + f[1] * f[1]).sqrt();
                if sp > 0.0 && fl > 1e-12 {
                    d.dir = [f[0] / fl, f[1] / fl];
                    d.speed = sp;
                    d.a += d.dir[0] * sp * dt; d.b += d.dir[1] * sp * dt;
                    // it sweeps up the film in its path and grows: dV = film thickness * width * distance
                    let v = (2.0 / 3.0) * std::f32::consts::PI * d.r_mm.powi(3) + film_h * 2.0 * d.r_mm * sp * 1000.0 * dt;
                    d.r_mm = (v / ((2.0 / 3.0) * std::f32::consts::PI)).cbrt().min(5.0);
                } else {
                    d.speed = 0.0;
                    d.r_mm = (d.r_mm * d.r_mm - evap_rate * dt * if rain_on { 0.0 } else { 1.0 }).max(0.0).sqrt();   // stuck: it dries like any other drop
                }
                s.contains(d.a, d.b, 0.0) && d.r_mm > 0.45 * rcrit.min(2.0) && !wiped(d.a, d.b)
            });
        } else {
            self.sliders.clear();
        }
        rcrit
    }
}

#[cfg(test)]
#[path = "glassdrops_tests.rs"]
mod tests;
