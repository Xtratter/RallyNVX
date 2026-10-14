//! Turning an A* path into a road: smooth curve with a minimum radius, a vertical profile with a grade limit and pinned
//! ends, and a width profile.
use super::path::value_noise;
use super::cfg::CrestCfg;
use super::RoadKind;
use crate::game::terrain::hash01;

/// Point spacing, tightest allowed bend radius and the grade limit of the road profile.
pub const STEP: f32 = 4.0;
pub const MIN_RADIUS: f32 = 13.0;
pub const MAX_GRADE: f32 = 0.14;

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 { (a.0 - b.0).hypot(a.1 - b.1) }
pub fn length(p: &[(f32, f32)]) -> f32 { p.windows(2).map(|w| dist(w[0], w[1])).sum() }

/// `n + 1` points at equal arc-length spacing close to `step` (the end points are kept exactly).
pub fn resample_uniform(p: &[(f32, f32)], step: f32) -> Vec<(f32, f32)> {
    if p.len() < 2 { return p.to_vec(); }
    let total = length(p);
    if total < 1e-3 { return vec![p[0], *p.last().unwrap()]; }
    let n = ((total / step).round() as usize).max(1);
    let spacing = total / n as f32;
    let mut out = Vec::with_capacity(n + 1);
    out.push(p[0]);
    let (mut seg, mut done) = (0usize, 0.0f32);   // current segment and the distance covered before it
    for k in 1..n {
        let target = k as f32 * spacing;
        while seg + 1 < p.len() - 1 && done + dist(p[seg], p[seg + 1]) < target { done += dist(p[seg], p[seg + 1]); seg += 1; }
        let l = dist(p[seg], p[seg + 1]).max(1e-6);
        let t = ((target - done) / l).clamp(0.0, 1.0);
        out.push((p[seg].0 + (p[seg + 1].0 - p[seg].0) * t, p[seg].1 + (p[seg + 1].1 - p[seg].1) * t));
    }
    out.push(*p.last().unwrap());
    out
}

/// Chaikin corner cutting (the end points stay).
pub fn chaikin(p: &[(f32, f32)], rounds: usize) -> Vec<(f32, f32)> {
    let mut q = p.to_vec();
    for _ in 0..rounds {
        if q.len() < 3 { break; }
        let mut out = vec![q[0]];
        for w in q.windows(2) {
            let (a, b) = (w[0], w[1]);
            out.push((0.75 * a.0 + 0.25 * b.0, 0.75 * a.1 + 0.25 * b.1));
            out.push((0.25 * a.0 + 0.75 * b.0, 0.25 * a.1 + 0.75 * b.1));
        }
        out.push(*q.last().unwrap());
        q = out;
    }
    q
}

/// Circumradius of three points (infinite for a straight line).
pub fn radius(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    let area2 = ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs();
    if area2 < 1e-6 { return f32::INFINITY; }
    dist(a, b) * dist(b, c) * dist(c, a) / (2.0 * area2)
}

/// The path as a smooth centre line at `STEP` spacing: corners cut, then every bend tighter than `MIN_RADIUS` is relaxed
/// towards the chord of its neighbours until the radius is met (the end points never move).
pub fn smooth_curve(path: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut p = resample_uniform(&chaikin(path, 2), STEP);
    for _ in 0..4000 {
        let q = p.clone();
        let mut worst = f32::INFINITY;
        for i in 1..p.len().saturating_sub(1) {
            let r = radius(q[i - 1], q[i], q[i + 1]);
            worst = worst.min(r);
            if r < MIN_RADIUS {
                let mid = ((q[i - 1].0 + q[i + 1].0) * 0.5, (q[i - 1].1 + q[i + 1].1) * 0.5);
                p[i] = (q[i].0 + (mid.0 - q[i].0) * 0.5, q[i].1 + (mid.1 - q[i].1) * 0.5);
            }
        }
        if worst >= MIN_RADIUS * 0.97 { break; }
    }
    resample_uniform(&p, STEP)
}

/// The road height along the centre line from the terrain heights `raw` (spacing `spacing`): a moving average over
/// +-40 m clamped to the grade limit inside the envelope of the two end heights (which stay exactly). When the ends are
/// too far apart in height for the limit, the limit is raised just enough.
pub fn profile(raw: &[f32], spacing: f32) -> Vec<f32> {
    let n = raw.len();
    if n < 3 { return raw.to_vec(); }
    let len = spacing * (n as f32 - 1.0);
    let (y0, y1) = (raw[0], raw[n - 1]);
    let g = MAX_GRADE.max((y1 - y0).abs() / len * 1.05);
    let w = 10usize;
    let mut y: Vec<f32> = (0..n).map(|i| { let r = w.min(i).min(n - 1 - i); let (a, b) = (i - r, i + r); raw[a..=b].iter().sum::<f32>() / (b - a + 1) as f32 }).collect();   // a symmetric window that shrinks towards the ends
    for i in 0..n {
        let s = spacing * i as f32;
        let lo = (y0 - g * s).max(y1 - g * (len - s));
        let hi = (y0 + g * s).min(y1 + g * (len - s));
        y[i] = y[i].clamp(lo, hi.max(lo));
    }
    y[0] = y0; y[n - 1] = y1;
    let step = g * spacing;
    for _ in 0..4 {
        for i in 1..n { y[i] = y[i].clamp(y[i - 1] - step, y[i - 1] + step); }
        for i in (0..n - 1).rev() { y[i] = y[i].clamp(y[i + 1] - step, y[i + 1] + step); }
    }
    y[0] = y0; y[n - 1] = y1;
    y
}

/// Road width at every point: the liaison is 8 m; the stage varies between 5 and 9 m with a slow random field and loses up
/// to 1 m in tight bends, smoothed over five points.
pub fn widths(kind: RoadKind, pts: &[(f32, f32)], spacing: f32, seed: u64) -> Vec<f32> {
    let n = pts.len();
    if kind == RoadKind::Liaison { return vec![8.0; n]; }
    let raw: Vec<f32> = (0..n).map(|i| {
        let s = i as f32 * spacing;
        let base = 5.0 + 4.0 * value_noise(seed ^ 0x51DE, s / 350.0, 0.0);
        let tight = if i > 0 && i + 1 < n { let r = radius(pts[i - 1], pts[i], pts[i + 1]); if r < 30.0 { (1.0 - r / 30.0).max(0.0) } else { 0.0 } } else { 0.0 };
        (base - tight).clamp(4.6, 9.0)
    }).collect();
    (0..n).map(|i| { let (a, b) = (i.saturating_sub(2), (i + 2).min(n - 1)); raw[a..=b].iter().sum::<f32>() / (b - a + 1) as f32 }).collect()
}

/// Adds series of crests ("yumps") to the height profile `y` of a road with centre line `xz`: half-cosine bumps whose
/// take-off side is longer than the landing side, so the car launches. A crest is skipped within `keep_ends` of the road
/// ends, where the curve is tighter than `min_radius`, and where the road lies less than `causeway_clear` above `floor`
/// (the water level). The added slope never exceeds `max_slope`.
pub fn add_crests(y: &mut [f32], xz: &[(f32, f32)], spacing: f32, floor: f32, seed: u64, c: &CrestCfg) {
    let n = y.len();
    if n < 3 || xz.len() != n || spacing <= 0.0 { return; }
    let len = spacing * (n as f32 - 1.0);
    let range = |lo: f32, hi: f32, k: i32, salt: u64| lo + (hi - lo) * hash01(seed ^ 0xC7E57 ^ salt, k, 0);
    let base: Vec<f32> = y.to_vec();
    let mut s = c.keep_ends + range(0.0, c.series_gap.0, 0, 1);
    let mut k = 0;
    while s < len - c.keep_ends {
        let count = c.series.0 + (hash01(seed ^ 0x5E71, k, 1) * (c.series.1 - c.series.0 + 1) as f32) as u32;
        for _ in 0..count.min(c.series.1) {
            k += 1;
            let (l, h) = (range(c.length.0, c.length.1, k, 2), range(c.height.0, c.height.1, k, 3));
            let (take, land) = (l * 0.5 * (1.0 + c.asym), l * 0.5 * (1.0 - c.asym));
            if s - take < c.keep_ends || s + land > len - c.keep_ends { break; }
            let ci = (s / spacing).round() as usize;
            let reach = ((take.max(land) / spacing) as usize + 1).min(n / 2);
            let (a, b) = (ci.saturating_sub(reach).max(1), (ci + reach).min(n - 2));
            let tight = (a..=b).step_by(2).any(|i| radius(xz[i - 1], xz[i], xz[i + 1]) < c.min_radius);
            let wet = base[a..=b].iter().any(|v| *v - floor < c.causeway_clear);
            if !tight && !wet {
                let h = h.min(c.max_slope * 2.0 * land / std::f32::consts::PI);
                for i in a..=b {
                    let d = spacing * i as f32 - s;
                    let half = if d < 0.0 { take } else { land };
                    if d.abs() < half { y[i] += h * 0.5 * (1.0 + (std::f32::consts::PI * d / half).cos()); }
                }
            }
            s += range(c.spacing.0, c.spacing.1, k, 4);
        }
        s += range(c.series_gap.0, c.series_gap.1, k, 5);
        k += 1;
    }
}

#[cfg(test)]
#[path = "shape_tests.rs"]
mod tests;
