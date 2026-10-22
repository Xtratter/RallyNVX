//! The line the autopilot drives: the road's centre pulled taut inside the free width, with a target speed per point.
use super::cfg::AutoCfg;
use crate::game::math::{v3, V3};
use crate::game::roads::shape::radius;

const G: f32 = 9.81;
/// The share of the measured pull the plan counts on (the rest is for the steering, the gear changes and the bumps).
const ACC_K: f32 = 0.9;
/// The measured cornering grip over the nominal one of the Vulpes on gravel at speed (see `model::measure_the_cornering_of_the_cars`): the car the style numbers were tuned on.
const LAT_REF: f32 = 0.89;

/// What the speed profile needs to know about the car and the ground (`stop_at_end`: the last point is a standstill).
#[derive(Clone, Copy, Debug)]
pub struct Limits { pub mu: f32, pub v_max: f32, pub accel: f32, pub stop_at_end: bool }

/// `half`: the half width of the road at every point (0 off the roads). `kraw` is the curvature at each point alone, `kappa` the worst of the neighbours
/// (so the car brakes before the bend); `mu` the grip of the ground, `lim` what the profile was built with.
#[derive(Clone, Debug)]
pub struct Route { pub p: Vec<V3>, pub s: Vec<f32>, pub kappa: Vec<f32>, pub speed: Vec<f32>, pub half: Vec<f32>, pub kraw: Vec<f32>, pub mu: Vec<f32>, pub lim: Limits, pub c: Vec<V3>,
    /// The usable width beyond the road's edge at every point (see `shoulder`): along the normal `(-tz, tx)` and against it.
    pub ext: Vec<(f32, f32)> }

fn xz(a: V3) -> (f32, f32) { (a.x, a.z) }

impl Route {
    /// `centre` and `width` are the road's points (about 4 m apart) and their widths.
    pub fn build(centre: &[V3], width: &[f32], lim: Limits, cfg: &AutoCfg) -> Route {
        Route::build_with(centre, width, &vec![lim.mu; centre.len()], lim, cfg)
    }

    /// As `build`, with the grip `mu` of the ground at every point (asphalt, city streets, gravel on one route).
    pub fn build_with(centre: &[V3], width: &[f32], mu: &[f32], lim: Limits, cfg: &AutoCfg) -> Route { Route::build_ext(centre, width, mu, lim, cfg, None) }

    /// As `build_with`, with the usable shoulder beyond the road's edge at every point (`None`: none): the line may use it, so a corner can be cut over the edge where the ground allows.
    pub fn build_ext(centre: &[V3], width: &[f32], mu: &[f32], lim: Limits, cfg: &AutoCfg, ext: Option<&[(f32, f32)]>) -> Route {
        let n = centre.len();
        assert!(n >= 3 && width.len() == n && mu.len() == n, "a route needs at least three points");
        let ext: Vec<(f32, f32)> = match ext { Some(e) if e.len() == n => e.to_vec(), _ => vec![(0.0, 0.0); n] };
        let p = taut(centre, width, cfg.offset_max, cfg.edge_margin, &ext);
        let mut s = vec![0.0f32; n];
        for i in 1..n { s[i] = s[i - 1] + ((p[i].x - p[i - 1].x).hypot(p[i].z - p[i - 1].z)).max(1e-3); }
        let kraw: Vec<f32> = (0..n).map(|i| if i == 0 || i == n - 1 { 0.0 } else { let r = radius(xz(p[i - 1]), xz(p[i]), xz(p[i + 1])); if r.is_finite() { 1.0 / r } else { 0.0 } }).collect();
        let kappa: Vec<f32> = (0..n).map(|i| kraw[i.saturating_sub(3)..=(i + 3).min(n - 1)].iter().cloned().fold(0.0, f32::max)).collect();
        let mut r = Route { p, s, kappa, speed: Vec::new(), half: width.iter().map(|w| w * 0.5).collect(), kraw, mu: mu.to_vec(), lim, c: centre.to_vec(), ext };
        r.speed = r.profile(cfg, None);
        r
    }

    /// The speed profile again, with what the car measured of itself (`models`): its real pull and bite at every speed, the grip shared between turning and
    /// braking or accelerating (the friction ellipse) instead of the flat guesses of `build`.
    pub fn replan(&mut self, models: &super::model::Models, cfg: &AutoCfg) { self.speed = self.profile(cfg, Some(models)); }

    fn profile(&self, cfg: &AutoCfg, models: Option<&super::model::Models>) -> Vec<f32> {
        let (n, p, s, mu, lim) = (self.len(), &self.p, &self.s, &self.mu, self.lim);
        // with a model the passes below see the grip shared with the braking, so the bend is only widened by two points (8 m)
        let kappa: Vec<f32> = if models.is_some() { (0..n).map(|i| self.kraw[i.saturating_sub(2)..=(i + 2).min(n - 1)].iter().cloned().fold(0.0, f32::max)).collect() } else { self.kappa.clone() };
        // the lateral acceleration the plan allows at speed `v` on point `i`: the style's number times the nominal grip, with the shape the car measured of itself over the speed (the downforce, the
        // steering lock) relative to `LAT_REF`, the measured share of the nominal grip of the car the style's numbers were tuned on (the Vulpes on gravel at speed)
        let allow = |i: usize, v: f32| -> f32 {
            let base = cfg.lat_factor * mu[i] * G;
            match models { None => base, Some(m) => { let cm = m.at(mu[i]); let r = cm.lateral(v).max(0.75 * cm.lateral_plateau()) / (m.key_at(mu[i]).max(0.05) * G); base * (r / LAT_REF).clamp(0.4, 1.45) } }
        };
        let mut speed: Vec<f32> = (0..n).map(|i| {
            if kappa[i] <= 1e-5 { return lim.v_max; }
            let mut v = (cfg.lat_factor * mu[i] * G / kappa[i]).sqrt().min(lim.v_max);
            if models.is_some() { for _ in 0..4 { v = (allow(i, v) / kappa[i]).sqrt().min(lim.v_max); } }
            v
        }).collect();
        // crests: where the profile is convex the car must stay planted (or at least land tamely); Max Attack takes them flat out
        for i in 2..n - 2 {
            let (ds0, ds1) = (s[i] - s[i - 1], s[i + 1] - s[i]);
            let slope0 = (p[i].y - p[i - 1].y) / ds0; let slope1 = (p[i + 1].y - p[i].y) / ds1;
            let curv = (slope1 - slope0) / (0.5 * (ds0 + ds1));
            if curv < -0.004 && cfg.attack < 0.5 { speed[i] = speed[i].min(cfg.crest_speed).min((0.8 * G / -curv).sqrt()); }
        }
        // the steering lock: a bend tighter than the wheels can turn at that speed (with 85 % of the lock kept in hand)
        if let Some(m) = models { for i in 0..n { speed[i] = speed[i].min(m.steer_speed(kappa[i] / 0.85, lim.v_max)); } }
        if lim.stop_at_end { speed[n - 1] = 0.0; }
        // the share of the grip a bend leaves for the pedals: the friction ellipse, `sqrt(1 - (lateral / limit)^2)`
        let left = |i: usize, v: f32| { let cap = allow(i, v); if cap <= 0.0 { 1.0 } else { let u = (v * v * kappa[i] / cap).clamp(0.0, 1.0); (1.0 - u * u).sqrt() } };
        // the braking distance (backward) and the engine (forward), with the slope
        for i in (0..n - 1).rev() {
            let ds = s[i + 1] - s[i]; let dy = p[i + 1].y - p[i].y;
            let a = match models {
                None => cfg.brake_factor * mu[i].min(mu[i + 1]) * G + G * dy / ds,
                Some(m) => cfg.brake_factor * m.at(mu[i].min(mu[i + 1])).brake(speed[i + 1]) * left(i + 1, speed[i + 1]) + G * dy / ds,
            }.max(0.5);
            speed[i] = speed[i].min((speed[i + 1] * speed[i + 1] + 2.0 * a * ds).sqrt());
        }
        for i in 0..n - 1 {
            let ds = s[i + 1] - s[i]; let dy = p[i + 1].y - p[i].y;
            let a = match models {
                None => lim.accel - G * dy / ds,
                Some(m) => ACC_K * m.at(mu[i]).accel(speed[i]) * left(i, speed[i]) - G * dy / ds,
            }.max(0.3);
            speed[i + 1] = speed[i + 1].min((speed[i] * speed[i] + 2.0 * a * ds).sqrt());
        }
        speed
    }

    /// How far the point (x, z) is from the edge of the usable ground by the centre line at index `i` (negative: beyond it; the shoulder counts as usable); `None` off the roads (the half width is 0).
    pub fn room(&self, i: usize, x: f32, z: f32) -> Option<f32> { self.room_split(i, x, z).map(|(r, _)| r) }

    /// The room as `room` gives it, and how much of it is the shoulder (so `room - shoulder` is the room by the road alone, negative while the car is beyond the road's edge).
    pub fn room_split(&self, i: usize, x: f32, z: f32) -> Option<(f32, f32)> {
        if self.half[i] <= 1.0 { return None; }
        let (a, b) = (self.c[i.saturating_sub(1)], self.c[(i + 1).min(self.len() - 1)]);
        let (tx, tz) = (b.x - a.x, b.z - a.z); let l = tx.hypot(tz).max(1e-3);
        let off = ((x - self.c[i].x) * -tz + (z - self.c[i].z) * tx) / l;
        let side = if off > 0.0 { self.ext[i].0 } else { self.ext[i].1 };
        Some((self.half[i] + side - off.abs(), side))
    }

    /// The sideways distance (m) of (x, z) from the line at index `i`: across its direction, not along it (the points are 4 m apart).
    pub fn cross(&self, i: usize, x: f32, z: f32) -> f32 {
        let (a, b) = (self.p[i.saturating_sub(1)], self.p[(i + 1).min(self.len() - 1)]);
        let (tx, tz) = (b.x - a.x, b.z - a.z); let l = tx.hypot(tz).max(1e-3);
        (((x - self.p[i].x) * -tz + (z - self.p[i].z) * tx) / l).abs()
    }

    pub fn len(&self) -> usize { self.p.len() }
    pub fn length(&self) -> f32 { *self.s.last().unwrap_or(&0.0) }

    /// The nearest point to (x, z) among the indices `lo..=hi`, with its horizontal distance.
    pub fn nearest_in(&self, x: f32, z: f32, lo: usize, hi: usize) -> (usize, f32) {
        let hi = hi.min(self.len() - 1);
        let mut best = (lo.min(hi), f32::INFINITY);
        for i in lo..=hi { let d = (self.p[i].x - x).hypot(self.p[i].z - z); if d < best.1 { best = (i, d); } }
        best
    }
    pub fn nearest(&self, x: f32, z: f32) -> (usize, f32) { self.nearest_in(x, z, 0, self.len() - 1) }

    /// The point `dist` metres further along the route than index `i` (clamped at the end), and its index.
    pub fn ahead(&self, i: usize, dist: f32) -> (usize, V3) {
        let target = self.s[i] + dist;
        let mut j = i;
        while j + 1 < self.len() && self.s[j] < target { j += 1; }
        (j, self.p[j])
    }
}

/// The line of least curvature inside the corridor `+-offset_max * (width / 2 - margin)`: Gauss-Seidel sweeps of the
/// biharmonic smoothing (every point goes to `(4 (p-1 + p+1) - (p-2 + p+2)) / 6`, projected on its own normal and clamped
/// to the corridor), the first and last two points stay on the centre line. Outside-inside-outside comes out by itself.
fn taut(c: &[V3], width: &[f32], offset_max: f32, edge_margin: f32, ext: &[(f32, f32)]) -> Vec<V3> {
    let n = c.len();
    let nor: Vec<V3> = (0..n).map(|i| { let (a, b) = (c[i.saturating_sub(1)], c[(i + 1).min(n - 1)]); let (tx, tz) = (b.x - a.x, b.z - a.z); let l = tx.hypot(tz).max(1e-3); v3(-tz / l, 0.0, tx / l) }).collect();
    // the corridor: along the normal up to `hi`, against it down to `lo` (the shoulder widens it where the ground allows)
    let hi: Vec<f32> = (0..n).map(|i| offset_max * (width[i] * 0.5 + ext[i].0 - edge_margin).max(0.0)).collect();
    let lo: Vec<f32> = (0..n).map(|i| offset_max * (width[i] * 0.5 + ext[i].1 - edge_margin).max(0.0)).collect();
    let mut d = vec![0.0f32; n];
    if n < 7 || (hi.iter().all(|l| *l <= 0.0) && lo.iter().all(|l| *l <= 0.0)) { return c.to_vec(); }
    let pos = |d: &[f32], i: usize| (c[i].x + nor[i].x * d[i], c[i].z + nor[i].z * d[i]);
    for _ in 0..1500 {
        for i in 2..n - 2 {
            let (a, b, e, f) = (pos(&d, i - 1), pos(&d, i + 1), pos(&d, i - 2), pos(&d, i + 2));
            let q = ((4.0 * (a.0 + b.0) - (e.0 + f.0)) / 6.0, (4.0 * (a.1 + b.1) - (e.1 + f.1)) / 6.0);
            d[i] = ((q.0 - c[i].x) * nor[i].x + (q.1 - c[i].z) * nor[i].z).clamp(-lo[i], hi[i]);
        }
    }
    (0..n).map(|i| { let q = c[i] + nor[i] * d[i]; v3(q.x, c[i].y, q.z) }).collect()
}

#[cfg(test)]
#[path = "route_tests.rs"]
mod tests;
