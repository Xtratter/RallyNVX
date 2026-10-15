//! Third-person chase camera with smoothing; never goes under the terrain. The player can orbit it around the car by
//! dragging; it swings back behind the car once the car drives forward, and swings in front of the car while reversing
//! (to show what is behind).
use super::math::{v3, V3};
use super::terrain::Terrain;
use super::vehicle::Vehicle;

/// Seconds of reversing before the camera swings to the front, and the swing time.
const REVERSE_DELAY: f32 = 0.35;
const SWING_TIME: f32 = 0.6;
/// The user's offsets fade out at this forward speed (m/s) when the finger is up; time constant in seconds.
const RETURN_SPEED: f32 = 2.0;
const RETURN_TIME: f32 = 0.5;
/// Elevation limits of the camera above the look-at point (radians) and the matching user pitch offsets.
const ELEV_MIN: f32 = 0.02;
const ELEV_MAX: f32 = 1.25;

/// The camera views the player cycles through: behind the car (the default), on the bonnet, low on the front bumper, high behind the car, a free orbit round it, and the cockpit
/// (the driver's eyes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CamView { Chase = 0, Hood = 1, Bumper = 2, Roof = 3, Orbit = 4, Cockpit = 5 }
pub const VIEWS: usize = 6;

impl CamView {
    /// Any number becomes a view (out of range: the nearest one).
    pub fn from_index(i: i32) -> CamView { match i.clamp(0, VIEWS as i32 - 1) { 0 => CamView::Chase, 1 => CamView::Hood, 2 => CamView::Bumper, 3 => CamView::Roof, 4 => CamView::Orbit, _ => CamView::Cockpit } }
    /// The bonnet, bumper and cockpit views ride on the car; the others follow it from outside.
    pub fn rigid(self) -> bool { matches!(self, CamView::Hood | CamView::Bumper | CamView::Cockpit) }
}

/// Distance scale (the chase views) or zoom (the bonnet and bumper views) limits, and the height offset limits (metres, or the gaze tilt of the rigid views).
pub const DIST_MIN: f32 = 0.5;
pub const DIST_MAX: f32 = 2.5;

pub struct FollowCam {
    pub pos: V3, pub target: V3, heading: V3,
    /// The view, its distance scale (zoom in the rigid views) and height offset (gaze tilt in the rigid views), and the vertical field of view in radians.
    pub view: CamView, pub dist: f32, pub height: f32, pub fov: f32,
    gaze: V3,
    /// The player's orbit: yaw around the car (radians, positive = the camera moves towards the car's left) and a pitch
    /// offset of the camera's elevation angle.
    pub user_yaw: f32, pub user_pitch: f32,
    dragging: bool, rev_t: f32, flip: f32,
}

fn rotate_y(v: V3, a: f32) -> V3 { let (s, c) = a.sin_cos(); v3(v.x * c + v.z * s, v.y, -v.x * s + v.z * c) }
fn smooth01(x: f32) -> f32 { let t = x.clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

impl FollowCam {
    pub fn new(v: &Vehicle) -> FollowCam { FollowCam::new_for(v.pos, v.forward(), 8.0, 3.5) }

    pub fn new_for(pos: V3, forward: V3, back: f32, up: f32) -> FollowCam {
        let heading = v3(forward.x, 0.0, forward.z).norm();
        FollowCam { pos: pos - heading * back + v3(0.0, up, 0.0), target: pos + v3(0.0, 1.2, 0.0), heading, view: CamView::Chase, dist: 1.0, height: 0.0, fov: 1.0, gaze: heading,
            user_yaw: 0.0, user_pitch: 0.0, dragging: false, rev_t: 0.0, flip: 0.0 }
    }

    /// Picks the view with its distance scale and height offset (clamped; garbage becomes the defaults).
    pub fn set_view(&mut self, view: i32, dist: f32, height: f32) {
        self.view = CamView::from_index(view);
        self.dist = if dist.is_finite() { dist.clamp(DIST_MIN, DIST_MAX) } else { 1.0 };
        let h = if self.view.rigid() { 1.0 } else { 6.0 };
        let lo = if self.view.rigid() { -1.0 } else { -3.0 };
        self.height = if height.is_finite() { height.clamp(lo, h) } else { 0.0 };
        if self.view.rigid() { self.fov = (1.0 / self.dist).clamp(0.5, 1.45); } else { self.fov = 1.0; }
        if self.view != CamView::Orbit { self.user_yaw = 0.0; self.user_pitch = 0.0; }
    }

    /// The finger moves on the screen: orbit by `dyaw` and tilt the elevation by `dpitch` (radians).
    pub fn drag(&mut self, dyaw: f32, dpitch: f32) {
        if !dyaw.is_finite() || !dpitch.is_finite() { return; }
        self.dragging = true;
        self.user_yaw += dyaw;
        let lo = ELEV_MIN - Self::base_elev(8.0, 3.5); let hi = ELEV_MAX - Self::base_elev(8.0, 3.5);
        self.user_pitch = (self.user_pitch + dpitch).clamp(lo, hi);
    }
    /// The finger went down (`true`, nothing changes yet) or up (`false`: the camera may return once the car moves).
    pub fn set_dragging(&mut self, on: bool) { self.dragging = on; }

    fn base_elev(back: f32, up: f32) -> f32 { (up - 1.2).atan2(back + 3.0) }

    pub fn update(&mut self, v: &Vehicle, t: &Terrain, dt: f32) {
        if self.view.rigid() { self.ride(v, dt); return; }
        let sp = v.speed();
        // reverse view: after a short delay of reversing the camera swings to the front of the car
        // (the driver holds reverse and the car is nearly stopped or rolling back, or the car really rolls backwards)
        let orbit = self.view == CamView::Orbit;   // the free orbit never swings behind or to the front by itself
        if (v.reverse_throttle && sp < 1.0) || sp < -1.5 { self.rev_t += dt; } else { self.rev_t = (self.rev_t - 3.0 * dt).max(0.0); }
        if sp > 1.5 { self.rev_t = 0.0; }
        let want = if self.rev_t > REVERSE_DELAY && !orbit { 1.0 } else { 0.0 };
        self.flip += (want - self.flip).clamp(-dt / SWING_TIME, dt / SWING_TIME);
        // the player's orbit fades out when the car drives forward and the finger is up
        self.user_yaw = (self.user_yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        if !self.dragging && sp > RETURN_SPEED && !orbit {
            let k = (-dt / RETURN_TIME).exp();
            self.user_yaw *= k; self.user_pitch *= k;
        }
        let yaw = self.user_yaw + std::f32::consts::PI * smooth01(self.flip);
        let (back, up) = match self.view { CamView::Roof => (5.0 * self.dist, 11.0 * self.dist + self.height), _ => (8.0 * self.dist, 3.5 * self.dist + self.height) };
        self.fov = 1.0;
        self.follow_orbit(v.pos, v.forward(), back, up, yaw, self.user_pitch, t, dt);
    }

    /// The bonnet and bumper views ride on the car (the point follows it exactly); the gaze follows its heading with a little damping so bumps do not shake the picture.
    fn ride(&mut self, v: &Vehicle, dt: f32) {
        let s = &v.spec;
        let ground = s.wheels[0].y - s.rest - s.radius;   // the road, in the car's frame
        let spot = if self.view == CamView::Bumper { s.cam_bumper } else { s.cam_hood };   // fractions of the body: from the road up, from the centre forward
        // the cockpit: the driver's eyes in metres (x to the left, y above the road, z ahead of the centre); a car without them rides on the bonnet
        let eye = match (self.view, s.cam_cockpit) { (CamView::Cockpit, Some(e)) => v3(e[0], ground + e[1], e[2]), _ => v3(0.0, ground + spot[0] * s.size.y, spot[1] * s.size.z) };
        self.pos = v.pos + v.rot.rotate(eye);
        let k = 1.0 - (-14.0 * dt).exp();
        let g = self.gaze + (v.forward() - self.gaze) * k;
        if g.len() > 1e-3 { self.gaze = g.norm(); }
        self.target = self.pos + self.gaze * 40.0 + v3(0.0, self.height * 3.0, 0.0);
        self.fov = (1.0 / self.dist).clamp(0.5, 1.45);
    }

    /// Chase any target: `back` metres behind its smoothed heading and `up` metres above it, never under the terrain.
    pub fn follow(&mut self, pos: V3, forward: V3, back: f32, up: f32, t: &Terrain, dt: f32) {
        self.follow_orbit(pos, forward, back, up, 0.0, 0.0, t, dt);
    }

    /// The camera sits on a circle around the look-at point `pos + 1.2 up` (shifted 3 m along the viewing direction):
    /// `yaw` turns it round the vertical axis (pi = in front of the target), `elev_off` raises or lowers it.
    #[allow(clippy::too_many_arguments)]
    fn follow_orbit(&mut self, pos: V3, forward: V3, back: f32, up: f32, yaw: f32, elev_off: f32, t: &Terrain, dt: f32) {
        let want = v3(forward.x, 0.0, forward.z).norm();
        let k_h = 1.0 - (-4.0 * dt).exp();
        let h = self.heading + (want - self.heading) * k_h;
        if h.len() > 1e-3 { self.heading = h.norm(); }
        let dir = rotate_y(self.heading * -1.0, yaw);                       // from the look-at point to the camera, horizontal
        let look = pos + v3(0.0, 1.2, 0.0) - dir * 3.0;
        let (dh, dv) = (back + 3.0, up - 1.2);
        let dist = dh.hypot(dv);
        let elev = (dv.atan2(dh) + elev_off).clamp(ELEV_MIN, ELEV_MAX);
        let desired = look + dir * (dist * elev.cos()) + v3(0.0, dist * elev.sin(), 0.0);
        // smoothing: soft sideways and up (the picture stays calm), stiff along the heading so the camera does not drop back when the car accelerates
        let (k, k_along) = (1.0 - (-6.0 * dt).exp(), 1.0 - (-20.0 * dt).exp());
        let delta = desired - self.pos;
        let along = self.heading * delta.dot(self.heading);
        self.pos = self.pos + along * k_along + (delta - along) * k;
        let floor = t.height(self.pos.x, self.pos.z) + 0.9;
        if self.pos.y < floor { self.pos.y = floor; }
        self.target = look;
    }
}

#[cfg(test)]
#[path = "camera_follow_tests.rs"]
mod tests;
