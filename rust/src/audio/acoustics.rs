//! Geometric acoustics on the game thread: what stands between a source and the listener, how the sound bends round it and what the ground adds.
//! The world is handed over as a few simple shapes (spheres, boxes, the car's cabin) and the terrain's height function; the result is a handful of numbers per source
//! for the audio thread (`Occlusion`): the loss in three bands, the extra travel time and the ground's reflection.
use super::spatial::C;
use crate::game::math::{v3, V3};

/// The bands the losses are worked out in, Hz.
pub const BANDS: [f32; 3] = [250.0, 1000.0, 4000.0];
/// The most any obstacle (or all of them together) takes off a band, dB.
pub const MAX_LOSS_DB: f32 = 40.0;

/// Maekawa's rule for a screen: the loss (dB) of a band behind an edge whose detour is `delta` metres longer than the straight path
/// (the Fresnel number `N = 2 delta / lambda`); about 5 dB at the shadow boundary, growing with the frequency, capped at 24 dB.
pub fn maekawa_db(delta: f32, hz: f32) -> f32 {
    let n = 2.0 * delta / (C / hz);
    if n < -0.1 { 0.0 } else { (10.0 * (3.0 + 20.0 * n.max(-0.1)).log10()).clamp(0.0, 24.0) }
}

/// How much longer the path over an edge is: the line passes `h` metres inside the obstacle's silhouette at `d1` from the listener and `d2` from the source.
pub fn detour(d1: f32, d2: f32, h: f32) -> f32 { (d1 * d1 + h * h).sqrt() + (d2 * d2 + h * h).sqrt() - (d1 + d2) }

/// What a thing does to a sound that meets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// A rock: it is a screen, the sound bends round it (diffraction); nothing goes through.
    Hard,
    /// A tree's crown: the sound goes through, scattered and dulled a little.
    Foliage,
    /// A wall or the car's body: a screen, or a muffled sound through it, whichever is louder.
    Wall,
}

impl Kind {
    /// The loss of the sound passing through (dB in the three bands), `None` when nothing gets through.
    fn through(self) -> Option<[f32; 3]> {
        match self { Kind::Hard => None, Kind::Foliage => Some([0.5, 2.0, 5.0]), Kind::Wall => Some([18.0, 25.0, 32.0]) }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shape { Sphere { c: V3, r: f32 }, Aabb { min: V3, max: V3 } }

#[derive(Clone, Copy, Debug)]
pub struct Obstacle { pub shape: Shape, pub kind: Kind }

/// The car's cabin (a box in the car's frame) seen from the world: the listener inside it hears the outside through the body.
#[derive(Clone, Copy, Debug)]
pub struct Cabin { pub centre: V3, pub x: V3, pub y: V3, pub z: V3, pub min: V3, pub max: V3 }

impl Cabin {
    pub fn local(&self, p: V3) -> V3 { let d = p - self.centre; v3(d.dot(self.x), d.dot(self.y), d.dot(self.z)) }
    pub fn contains(&self, p: V3) -> bool { let q = self.local(p); q.x >= self.min.x && q.x <= self.max.x && q.y >= self.min.y && q.y <= self.max.y && q.z >= self.min.z && q.z <= self.max.z }
}

/// The loss a source suffers on its way to the listener.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Occlusion {
    /// dB lost at `BANDS`.
    pub loss: [f32; 3],
    /// The extra travel time of the bent path, s.
    pub extra_delay: f32,
}
impl Default for Occlusion { fn default() -> Self { Occlusion { loss: [0.0; 3], extra_delay: 0.0 } } }

impl Occlusion {
    /// The gain of the low band and the cutoff (Hz) of a one-pole low-pass that gives the high band the same loss relative to it.
    pub fn gain_and_cutoff(&self) -> (f32, f32) {
        let g = 10f32.powf(-self.loss[0] / 20.0);
        let r = 10f32.powf(-(self.loss[2] - self.loss[0]).max(0.0) / 20.0);   // the 4 kHz band against the 250 Hz one
        let hz = if r > 0.97 { 20000.0 } else { (BANDS[2] / (1.0 / (r * r) - 1.0).sqrt()).clamp(150.0, 20000.0) };
        (g, hz)
    }
}

/// The closest approach of the segment `a..b` to the point `c`: (parameter 0..1 of the closest point, the distance).
fn closest(a: V3, b: V3, c: V3) -> (f32, f32) {
    let ab = b - a; let l2 = ab.dot(ab);
    let t = if l2 < 1e-9 { 0.0 } else { ((c - a).dot(ab) / l2).clamp(0.0, 1.0) };
    (t, (a + ab * t - c).len())
}

/// The interval of the segment (parameters) inside the box, `None` when it misses (slab test).
fn seg_box(a: V3, b: V3, min: V3, max: V3) -> Option<(f32, f32)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (o, dd, lo, hi) in [(a.x, d.x, min.x, max.x), (a.y, d.y, min.y, max.y), (a.z, d.z, min.z, max.z)] {
        if dd.abs() < 1e-9 { if o < lo || o > hi { return None; } }
        else {
            let (mut u0, mut u1) = ((lo - o) / dd, (hi - o) / dd);
            if u0 > u1 { std::mem::swap(&mut u0, &mut u1); }
            t0 = t0.max(u0); t1 = t1.min(u1);
            if t0 > t1 { return None; }
        }
    }
    Some((t0, t1))
}

/// How far the segment cuts into an obstacle: the metres it would have to move to clear it (the line's depth inside the silhouette) and where along the segment.
fn penetration(a: V3, b: V3, o: &Obstacle) -> Option<(f32, f32)> {
    match o.shape {
        Shape::Sphere { c, r } => { let (t, d) = closest(a, b, c); if d < r { Some((r - d, t)) } else { None } }
        Shape::Aabb { min, max } => {
            let (t0, t1) = seg_box(a, b, min, max)?;
            let tm = 0.5 * (t0 + t1); let p = a + (b - a) * tm;
            let h = (max.y - p.y).min(p.x - min.x).min(max.x - p.x).min(p.z - min.z).min(max.z - p.z).max(0.0);
            Some((h, tm))
        }
    }
}

/// The loss over the terrain: the segment is stepped, and where the ground rises above it the line passes that deep inside the ridge.
fn terrain_penetration(a: V3, b: V3, ground: &dyn Fn(f32, f32) -> f32) -> Option<(f32, f32)> {
    const N: usize = 32;
    let mut best: Option<(f32, f32)> = None;
    for i in 1..N {
        let t = i as f32 / N as f32;
        let p = a + (b - a) * t;
        let h = ground(p.x, p.z) + 0.05 - p.y;
        if h > 0.0 && best.map_or(true, |(bh, _)| h > bh) { best = Some((h, t)); }
    }
    best
}

/// The loss from `src` to `listener`: the ground's ridges and the obstacles bend the sound (Maekawa, per band), the soft ones let it through dulled; the car's cabin muffles the
/// outside when the listener sits inside it. `cabin_trans` is the loss through the car's body (dB in three bands).
pub fn occlusion(listener: V3, src: V3, ground: &dyn Fn(f32, f32) -> f32, obs: &[Obstacle], cabin: Option<&Cabin>, cabin_trans: [f32; 3]) -> Occlusion {
    let total = (listener - src).len().max(0.05);
    let mut loss = [0.0f32; 3];
    let mut extra = 0.0f32;
    let mut add = |h: f32, t: f32, through: Option<[f32; 3]>, loss: &mut [f32; 3], extra: &mut f32| {
        let (d1, d2) = (total * t, total * (1.0 - t));
        let delta = detour(d1, d2, h);
        let mut worst = 0.0f32;
        for k in 0..3 {
            let bend = maekawa_db(delta, BANDS[k]);
            let l = through.map_or(bend, |tr| bend.min(tr[k]));
            loss[k] = (loss[k] + l).min(MAX_LOSS_DB);
            worst = worst.max(l);
        }
        if through.is_none() || through.map_or(false, |tr| maekawa_db(delta, BANDS[1]) < tr[1]) { *extra = extra.max(delta / C); }
        let _ = worst;
    };
    if let Some((h, t)) = terrain_penetration(listener, src, ground) { add(h, t, None, &mut loss, &mut extra); }
    for o in obs {
        if let Some((h, t)) = penetration(listener, src, o) {
            if o.kind == Kind::Foliage { for k in 0..3 { loss[k] = (loss[k] + o.kind.through().unwrap()[k]).min(MAX_LOSS_DB); } }
            else { add(h, t, o.kind.through(), &mut loss, &mut extra); }
        }
    }
    if let Some(c) = cabin {
        if c.contains(listener) != c.contains(src) {   // one of them is inside the cabin: the sound goes through the body
            for k in 0..3 { loss[k] = (loss[k] + cabin_trans[k]).min(MAX_LOSS_DB); }
        } else if !c.contains(listener) && !c.contains(src) {   // both outside: the car may stand between them
            let (a, b) = (c.local(listener), c.local(src));
            if let Some((h, t)) = penetration(a, b, &Obstacle { shape: Shape::Aabb { min: c.min, max: c.max }, kind: Kind::Wall }) { add(h, t, Some(cabin_trans), &mut loss, &mut extra); }
        }
    }
    Occlusion { loss, extra_delay: extra.min(0.05) }
}

/// The ground's echo of a source: the image of the source under the ground, how much of the sound the ground returns and how dull it keeps it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reflection {
    /// The image source's position (mirrored in the ground at the specular point).
    pub image: V3,
    /// The share of the pressure the ground returns (0..1).
    pub coeff: f32,
    /// The cutoff (Hz) above which a soft ground (grass, snow) takes the sound.
    pub hz: f32,
}

/// The ground reflection for a listener and a source above `ground`: the specular point lies between them in proportion to their heights. `None` when either is
/// under the ground, or the reflection is negligible. `(coeff, hz)` is the ground's character at the point (see `surface_reflection`).
pub fn ground_reflection(listener: V3, src: V3, ground: &dyn Fn(f32, f32) -> f32, character: &dyn Fn(f32, f32) -> (f32, f32)) -> Option<Reflection> {
    let (gl, gs) = (ground(listener.x, listener.z), ground(src.x, src.z));
    let (hl, hs) = ((listener.y - gl).max(0.0), (src.y - gs).max(0.0));
    if hl + hs < 1e-3 { return None; }
    let k = hl / (hl + hs);
    let sp = listener + (src - listener) * k;
    let g = ground(sp.x, sp.z);
    if listener.y < g || src.y < g { return None; }
    let image = v3(src.x, 2.0 * g - src.y, src.z);
    let (coeff, hz) = character(sp.x, sp.z);
    if coeff < 0.02 { return None; }
    Some(Reflection { image, coeff, hz })
}

/// How a ground returns the sound: (the pressure coefficient, the cutoff in Hz): hard surfaces return nearly all of it, grass and sand dull and absorb it.
pub fn surface_reflection(s: crate::game::surface::Surface) -> (f32, f32) {
    use crate::game::surface::Surface::*;
    match s { Asphalt => (0.93, 16000.0), Rock => (0.9, 16000.0), Water => (0.97, 16000.0), Gravel => (0.7, 8000.0), Dirt => (0.65, 6000.0), Sand => (0.55, 5000.0), Grass => (0.5, 3000.0) }
}

/// What a ray from the listener met: how far, how much of the sound that thing absorbs (0..1) and whether it was the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit { pub dist: f32, pub alpha: f32, pub ground: bool }

/// A ray cast from the listener: its direction and what it met (`None`: the sky, or nothing within the range).
#[derive(Clone, Copy, Debug)]
pub struct RayResult { pub dir: V3, pub hit: Option<RayHit> }

/// `n` directions spread evenly over the sphere (the golden spiral).
pub fn sphere_dirs(n: usize) -> Vec<V3> {
    let ga = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    (0..n).map(|i| { let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32; let r = (1.0 - y * y).max(0.0).sqrt(); let t = ga * i as f32; v3(r * t.cos(), y, r * t.sin()) }).collect()
}

/// How much of the sound a thing takes with every bounce (broadband): a hard floor and walls almost none, grass and foliage a good deal.
pub fn surface_alpha(s: crate::game::surface::Surface) -> f32 {
    use crate::game::surface::Surface::*;
    match s { Asphalt => 0.03, Rock => 0.03, Water => 0.02, Gravel => 0.10, Dirt => 0.12, Sand => 0.12, Grass => 0.20 }
}
fn kind_alpha(k: Kind) -> f32 { match k { Kind::Hard => 0.05, Kind::Foliage => 0.35, Kind::Wall => 0.06 } }

/// The first thing a ray meets within `range` metres: the ground (stepped and refined), a sphere or a box.
pub fn trace(o: V3, d: V3, range: f32, ground: &dyn Fn(f32, f32) -> f32, ground_alpha: &dyn Fn(f32, f32) -> f32, obs: &[Obstacle]) -> Option<RayHit> {
    let mut best: Option<RayHit> = None;
    // the ground: steps of 2 m, then a bisection
    let step = 2.0;
    let mut t = 0.0; let mut prev_above = o.y >= ground(o.x, o.z);
    while t < range {
        let t2 = (t + step).min(range);
        let p = o + d * t2;
        let above = p.y >= ground(p.x, p.z);
        if prev_above && !above {
            let (mut lo, mut hi) = (t, t2);
            for _ in 0..6 { let m = 0.5 * (lo + hi); let q = o + d * m; if q.y >= ground(q.x, q.z) { lo = m; } else { hi = m; } }
            let q = o + d * hi;
            best = Some(RayHit { dist: hi, alpha: ground_alpha(q.x, q.z), ground: true });
            break;
        }
        prev_above = above; t = t2;
    }
    let far = o + d * range;
    for ob in obs {
        let hit = match ob.shape {
            Shape::Sphere { c, r } => {
                let oc = o - c; let b = oc.dot(d); let cc = oc.dot(oc) - r * r;
                let disc = b * b - cc;
                if disc < 0.0 || cc < 0.0 { None } else { let t = -b - disc.sqrt(); if t > 0.0 && t < range { Some(t) } else { None } }
            }
            Shape::Aabb { min, max } => match seg_box(o, far, min, max) { Some((t0, _)) if t0 > 0.0 => Some(t0 * range), _ => None },
        };
        if let Some(t) = hit { if best.map_or(true, |b| t < b.dist) { best = Some(RayHit { dist: t, alpha: kind_alpha(ob.kind), ground: false }); } }
    }
    best
}

/// The tail and the echoes the rays ask for. The tail's decay follows from the bounce rate: a ray travels `d` (the mean free path) between bounces, `rho = (1 - escaped) (1 - alpha)`
/// of the energy survives each one, so the level falls 60 dB in `60 d / (c * -10 log10 rho)` seconds (Eyring, with the sky as an open window); the level of the tail is the share
/// of rays that come back. Echoes: in each of the four sides (front, right, back, left of `fwd`) the nearest wall-like hit near the horizon.
pub fn environment(rays: &[RayResult], fwd: V3) -> super::reverb::Env {
    use super::reverb::{Env, ECHOES};
    let n = rays.len().max(1) as f32;
    let hits: Vec<&RayHit> = rays.iter().filter_map(|r| r.hit.as_ref()).collect();
    let escaped = 1.0 - hits.len() as f32 / n;
    let (dbar, abar) = if hits.is_empty() { (150.0, 0.5) } else { (hits.iter().map(|h| h.dist).sum::<f32>() / hits.len() as f32, hits.iter().map(|h| h.alpha).sum::<f32>() / hits.len() as f32) };
    let rho = ((1.0 - escaped) * (1.0 - abar)).clamp(1e-3, 0.999);
    let rt60 = (60.0 * dbar / (C * (-10.0 * rho.log10()))).clamp(0.08, 3.5);
    let level = (0.5 * (1.0 - escaped) * (1.0 - 0.6 * abar)).clamp(0.0, 1.0);
    let damp_hz = (14000.0 * (1.0 - 1.6 * abar)).clamp(1500.0, 14000.0);
    let f = { let h = v3(fwd.x, 0.0, fwd.z); if h.len() > 1e-3 { h.norm() } else { v3(0.0, 0.0, 1.0) } };
    let right = f.cross(v3(0.0, 1.0, 0.0));
    let mut echoes = [(0.0, 0.0, 0.0); ECHOES];
    // sector k: 0 front, 1 right, 2 back, 3 left
    for k in 0..ECHOES {
        let axis = [f, right, f * -1.0, right * -1.0][k];
        let mut nearest: Option<&RayHit> = None; let mut count = 0; let mut total = 0;
        for r in rays {
            if r.dir.y.abs() > 0.45 { continue; }
            let h = v3(r.dir.x, 0.0, r.dir.z); if h.len() < 1e-3 || h.norm().dot(axis) < 0.707 { continue; }
            total += 1;
            if let Some(hit) = &r.hit { if !hit.ground { count += 1; if nearest.map_or(true, |m| hit.dist < m.dist) { nearest = Some(hit); } } }
        }
        if let (Some(h), true) = (nearest, total > 0) {
            if h.dist > 4.0 && h.dist < 120.0 {
                let share = count as f32 / total as f32;
                let gain = (0.7 * share * (1.0 - h.alpha) * (25.0 / h.dist).min(1.0)).clamp(0.0, 0.8);
                echoes[k] = (2.0 * h.dist / C, gain, [0.0, 1.0, 0.0, -1.0][k]);
            }
        }
    }
    Env { rt60, level, damp_hz, echoes }
}

#[cfg(test)]
#[path = "acoustics_tests.rs"]
mod tests;
