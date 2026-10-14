//! The line the autopilot drives: the road's centre pulled taut inside the free width, with a target speed per point.
use super::cfg::AutoCfg;
use crate::game::math::{v3, V3};
use crate::game::roads::shape::radius;

const G: f32 = 9.81;
/// Margin kept to the road edge (half a car width plus a little).
const EDGE_MARGIN: f32 = 1.4;

/// What the speed profile needs to know about the car and the ground (`stop_at_end`: the last point is a standstill).
#[derive(Clone, Copy, Debug)]
pub struct Limits { pub mu: f32, pub v_max: f32, pub accel: f32, pub stop_at_end: bool }

/// `half`: the half width of the road at every point (0 off the roads).
#[derive(Clone, Debug)]
pub struct Route { pub p: Vec<V3>, pub s: Vec<f32>, pub kappa: Vec<f32>, pub speed: Vec<f32>, pub half: Vec<f32> }

fn xz(a: V3) -> (f32, f32) { (a.x, a.z) }

impl Route {
    /// `centre` and `width` are the road's points (about 4 m apart) and their widths.
    pub fn build(centre: &[V3], width: &[f32], lim: Limits, cfg: &AutoCfg) -> Route {
        Route::build_with(centre, width, &vec![lim.mu; centre.len()], lim, cfg)
    }

    /// As `build`, with the grip `mu` of the ground at every point (asphalt, city streets, gravel on one route).
    pub fn build_with(centre: &[V3], width: &[f32], mu: &[f32], lim: Limits, cfg: &AutoCfg) -> Route {
        let n = centre.len();
        assert!(n >= 3 && width.len() == n && mu.len() == n, "a route needs at least three points");
        let p = taut(centre, width, cfg.offset_max);
        let mut s = vec![0.0f32; n];
        for i in 1..n { s[i] = s[i - 1] + ((p[i].x - p[i - 1].x).hypot(p[i].z - p[i - 1].z)).max(1e-3); }
        // curvature, widened to the worst of +-3 points so the car brakes before the bend
        let raw: Vec<f32> = (0..n).map(|i| if i == 0 || i == n - 1 { 0.0 } else { let r = radius(xz(p[i - 1]), xz(p[i]), xz(p[i + 1])); if r.is_finite() { 1.0 / r } else { 0.0 } }).collect();
        let kappa: Vec<f32> = (0..n).map(|i| raw[i.saturating_sub(3)..=(i + 3).min(n - 1)].iter().cloned().fold(0.0, f32::max)).collect();
        let mut speed: Vec<f32> = (0..n).map(|i| if kappa[i] > 1e-5 { (cfg.lat_factor * mu[i] * G / kappa[i]).sqrt().min(lim.v_max) } else { lim.v_max }).collect();
        // crests: where the profile is convex the car must stay planted (or at least land tamely)
        for i in 2..n - 2 {
            let (ds0, ds1) = (s[i] - s[i - 1], s[i + 1] - s[i]);
            let slope0 = (p[i].y - p[i - 1].y) / ds0; let slope1 = (p[i + 1].y - p[i].y) / ds1;
            let curv = (slope1 - slope0) / (0.5 * (ds0 + ds1));
            if curv < -0.004 { speed[i] = speed[i].min(cfg.crest_speed).min((0.8 * G / -curv).sqrt()); }
        }
        if lim.stop_at_end { speed[n - 1] = 0.0; }
        // the braking distance (backward) and the engine (forward), with the slope
        for i in (0..n - 1).rev() {
            let ds = s[i + 1] - s[i]; let dy = p[i + 1].y - p[i].y;
            let a = (cfg.brake_factor * mu[i].min(mu[i + 1]) * G + G * dy / ds).max(0.5);
            speed[i] = speed[i].min((speed[i + 1] * speed[i + 1] + 2.0 * a * ds).sqrt());
        }
        for i in 0..n - 1 {
            let ds = s[i + 1] - s[i]; let dy = p[i + 1].y - p[i].y;
            let a = (lim.accel - G * dy / ds).max(0.3);
            speed[i + 1] = speed[i + 1].min((speed[i] * speed[i] + 2.0 * a * ds).sqrt());
        }
        Route { p, s, kappa, speed, half: width.iter().map(|w| w * 0.5).collect() }
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
fn taut(c: &[V3], width: &[f32], offset_max: f32) -> Vec<V3> {
    let n = c.len();
    let nor: Vec<V3> = (0..n).map(|i| { let (a, b) = (c[i.saturating_sub(1)], c[(i + 1).min(n - 1)]); let (tx, tz) = (b.x - a.x, b.z - a.z); let l = tx.hypot(tz).max(1e-3); v3(-tz / l, 0.0, tx / l) }).collect();
    let lim: Vec<f32> = (0..n).map(|i| offset_max * (width[i] * 0.5 - EDGE_MARGIN).max(0.0)).collect();
    let mut d = vec![0.0f32; n];
    if n < 7 || lim.iter().all(|l| *l <= 0.0) { return c.to_vec(); }
    let pos = |d: &[f32], i: usize| (c[i].x + nor[i].x * d[i], c[i].z + nor[i].z * d[i]);
    for _ in 0..1500 {
        for i in 2..n - 2 {
            let (a, b, e, f) = (pos(&d, i - 1), pos(&d, i + 1), pos(&d, i - 2), pos(&d, i + 2));
            let q = ((4.0 * (a.0 + b.0) - (e.0 + f.0)) / 6.0, (4.0 * (a.1 + b.1) - (e.1 + f.1)) / 6.0);
            d[i] = ((q.0 - c[i].x) * nor[i].x + (q.1 - c[i].z) * nor[i].z).clamp(-lim[i], lim[i]);
        }
    }
    (0..n).map(|i| { let q = c[i] + nor[i] * d[i]; v3(q.x, c[i].y, q.z) }).collect()
}

#[cfg(test)]
#[path = "route_tests.rs"]
mod tests;
