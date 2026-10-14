//! Wheel tracks: a ring buffer of dark ground marks left by wheels in contact with the ground.
use super::materials::rock_dominant;
use super::math::{v3, Q, V3};
use super::terrain::Terrain;
use super::vehicle::Vehicle;

pub const MAX_TRACKS: usize = 256;
pub const SPACING: f32 = 0.35;

/// Orientation of a track mark: local y on the ground normal, local z along the travel direction projected onto the slope.
pub fn track_rot(dir: V3, normal: V3) -> Q {
    let n = normal.norm();
    let f = (dir - n * dir.dot(n)).norm();
    let r = n.cross(f);
    Q::from_basis(r, n, f)
}

#[derive(Clone, Copy, Debug)]
pub struct Track { pub pos: V3, pub dir: V3, pub normal: V3 }

pub struct Tracks { segs: Vec<Track>, head: usize, last: [Option<V3>; 4] }

impl Tracks {
    pub fn new() -> Tracks { Tracks { segs: Vec::new(), head: 0, last: [None; 4] } }
    pub fn len(&self) -> usize { self.segs.len() }
    pub fn iter(&self) -> impl Iterator<Item = &Track> { self.segs.iter() }
    pub fn update(&mut self, v: &Vehicle, t: &Terrain, _dt: f32) {
        let up = v.up();
        for w in 0..4 {
            if v.comp[w] <= 0.0 || v.vel.x.hypot(v.vel.z) < 2.0 { self.last[w] = None; continue; }
            let s = &v.spec;
            let p = v.pos + v.rot.rotate(s.wheels[w]) - up * (s.rest + s.radius - v.comp[w]);
            let h = t.height(p.x, p.z); let n = t.normal(p.x, p.z);
            if rock_dominant(h, n, p.x, p.z) || t.in_city(p.x, p.z, 0.0) || t.is_water(p.x, p.z, 0.0) || t.near_road(p.x, p.z, 0.0) { self.last[w] = None; continue; }
            let q = v3(p.x, h + 0.03, p.z);
            match self.last[w] {
                Some(prev) if (q - prev).len() < SPACING => {}
                prev => {
                    let dir = prev.map(|pv| v3(q.x - pv.x, 0.0, q.z - pv.z).norm()).filter(|d| d.len() > 0.5).unwrap_or_else(|| { let f = v.forward(); v3(f.x, 0.0, f.z).norm() });
                    let seg = Track { pos: q, dir, normal: n };
                    if self.segs.len() < MAX_TRACKS { self.segs.push(seg); } else { self.segs[self.head] = seg; self.head = (self.head + 1) % MAX_TRACKS; }
                    self.last[w] = Some(q);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tracks_tests.rs"]
mod tests;
