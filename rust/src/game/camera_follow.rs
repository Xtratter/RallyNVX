//! Third-person chase camera with smoothing; never goes under the terrain.
use super::math::{v3, V3};
use super::terrain::Terrain;
use super::vehicle::Vehicle;

pub struct FollowCam { pub pos: V3, pub target: V3, heading: V3 }

impl FollowCam {
    pub fn new(v: &Vehicle) -> FollowCam {
        let f = v.forward(); let heading = v3(f.x, 0.0, f.z).norm();
        FollowCam { pos: v.pos - heading * 8.0 + v3(0.0, 3.5, 0.0), target: v.pos + v3(0.0, 1.2, 0.0), heading }
    }

    pub fn update(&mut self, v: &Vehicle, t: &Terrain, dt: f32) {
        let f = v.forward(); let want = v3(f.x, 0.0, f.z).norm();
        let k_h = 1.0 - (-4.0 * dt).exp();
        let h = self.heading + (want - self.heading) * k_h;
        if h.len() > 1e-3 { self.heading = h.norm(); }
        let desired = v.pos - self.heading * 8.0 + v3(0.0, 3.5, 0.0);
        let k = 1.0 - (-6.0 * dt).exp();
        self.pos = self.pos + (desired - self.pos) * k;
        let floor = t.height(self.pos.x, self.pos.z) + 0.9;
        if self.pos.y < floor { self.pos.y = floor; }
        self.target = v.pos + v3(0.0, 1.2, 0.0) + self.heading * 3.0;
    }
}

#[cfg(test)]
#[path = "camera_follow_tests.rs"]
mod tests;
