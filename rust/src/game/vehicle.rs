//! Arcade off-road vehicle: rigid body, four spring/damper wheels over a heightfield, friction-circle tyres.
use super::input::Input;
use super::math::{v3, Q, V3};
use super::props::{Prop, PropKind};
use super::terrain::Terrain;

#[derive(Clone, Copy, Debug)]
pub struct VehicleSpec {
    pub mass: f32,
    /// Principal moments of inertia about body x (pitch), y (yaw), z (roll).
    pub inertia: V3,
    /// Suspension attachment points in body space: FL, FR, RL, RR.
    pub wheels: [V3; 4],
    pub rest: f32, pub radius: f32, pub spring: f32, pub damper: f32,
    pub engine_force: f32, pub brake_force: f32, pub max_speed: f32, pub mu: f32,
}

impl VehicleSpec {
    pub fn offroader() -> VehicleSpec {
        let (m, w, h, l) = (1500.0f32, 2.0f32, 1.4f32, 4.2f32);
        VehicleSpec {
            mass: m,
            inertia: v3(m / 12.0 * (h * h + l * l), m / 12.0 * (w * w + l * l), m / 12.0 * (w * w + h * h)),
            wheels: [v3(-0.95, 0.0, 1.4), v3(0.95, 0.0, 1.4), v3(-0.95, 0.0, -1.4), v3(0.95, 0.0, -1.4)],
            rest: 0.6, radius: 0.45, spring: 30000.0, damper: 3500.0,
            engine_force: 14000.0, brake_force: 16000.0, max_speed: 35.0, mu: 1.1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vehicle {
    pub spec: VehicleSpec,
    pub pos: V3, pub rot: Q, pub vel: V3, pub omega: V3,
    pub steer: f32,
    /// Spring compression per wheel in metres (0 = airborne); the wheel centre sits `rest - comp` below its attachment.
    pub comp: [f32; 4],
    pub contacts: u8,
}

const SUB: usize = 4;
const G: f32 = 9.81;

impl Vehicle {
    pub fn new(spec: VehicleSpec, pos: V3, yaw: f32) -> Vehicle {
        Vehicle { spec, pos, rot: Q::axis_angle(v3(0., 1., 0.), yaw), vel: V3::ZERO, omega: V3::ZERO, steer: 0.0, comp: [0.0; 4], contacts: 0 }
    }
    pub fn forward(&self) -> V3 { self.rot.rotate(v3(0., 0., 1.)) }
    pub fn up(&self) -> V3 { self.rot.rotate(v3(0., 1., 0.)) }
    /// The car's right-hand side: forward x up = -X for x right / y up / z forward in a right-handed system.
    pub fn right(&self) -> V3 { self.rot.rotate(v3(-1., 0., 0.)) }
    pub fn speed(&self) -> f32 { self.vel.dot(self.forward()) }
    pub fn flipped(&self) -> bool { self.up().y < -0.17 }

    /// Respawns upright at the current spot, keeping the heading.
    pub fn reset_upright(&mut self, t: &Terrain) {
        let f = self.forward();
        let yaw = if f.x.hypot(f.z) > 1e-3 { f.x.atan2(f.z) } else { 0.0 };
        self.rot = Q::axis_angle(v3(0., 1., 0.), yaw);
        self.pos.y = t.height(self.pos.x, self.pos.z) + 1.2;
        self.vel = V3::ZERO; self.omega = V3::ZERO; self.steer = 0.0; self.comp = [0.0; 4];
    }

    pub fn step(&mut self, t: &Terrain, props: &[Prop], inp: &Input, dt: f32) {
        let inp = inp.sanitized();
        let max_a = 0.55 / (1.0 + self.vel.len() / 15.0);
        let rate = 3.0 * dt;
        self.steer += (inp.steer * max_a - self.steer).clamp(-rate, rate);
        let h = dt / SUB as f32;
        for _ in 0..SUB { self.substep(t, &inp, h); }
        self.collide_props(props);
        let g = t.height(self.pos.x, self.pos.z);
        if self.pos.y < g + 0.2 { self.pos.y = g + 0.2; if self.vel.y < 0.0 { self.vel.y = 0.0; } }
    }

    fn substep(&mut self, t: &Terrain, inp: &Input, h: f32) {
        let s = self.spec;
        let (fwd, up, right) = (self.forward(), self.up(), self.right());
        let fspeed = self.vel.dot(fwd);
        let mut force = v3(0., -G * s.mass, 0.);
        let mut torque = V3::ZERO;
        self.contacts = 0;
        let wheel_mass = s.mass / 4.0;
        let max_susp = 5.0 * G * wheel_mass;
        for w in 0..4 {
            self.comp[w] = 0.0;
            let front = w < 2;
            let r_att = self.rot.rotate(s.wheels[w]);
            let attach = self.pos + r_att;
            let ground = t.height(attach.x, attach.z);
            let n = t.normal(attach.x, attach.z);
            if up.dot(n) < 0.2 { continue; }
            let d = (attach.y - ground) / up.y.max(0.2);
            let compress = s.rest + s.radius - d;
            if compress <= 0.0 { continue; }
            self.contacts += 1;
            self.comp[w] = compress.min(s.rest + s.radius);
            let vp = self.vel + self.omega.cross(r_att);
            let fs = (s.spring * compress - s.damper * vp.dot(n)).clamp(0.0, max_susp);
            let susp = n * fs;
            force = force + susp; torque = torque + r_att.cross(susp);
            // tyre
            let r_c = r_att - up * d;
            let vc = self.vel + self.omega.cross(r_c);
            let base = if front { fwd * self.steer.cos() + right * self.steer.sin() } else { fwd };
            let fw = (base - n * base.dot(n)).norm();
            let rw = n.cross(fw);
            let (vl, vf) = (vc.dot(rw), vc.dot(fw));
            let lat_mu = if !front && inp.handbrake { s.mu * 0.4 } else { s.mu };
            let mut fl = -vl * wheel_mass * 12.0;
            let lim_speed = if inp.throttle >= 0.0 { s.max_speed } else { s.max_speed * 0.4 };
            let taper = (1.0 - (fspeed * inp.throttle.signum()).max(0.0) / lim_speed).clamp(0.0, 1.0);
            let mut fx = inp.throttle * s.engine_force / 4.0 * taper - vf * 40.0;
            if inp.brake || (inp.handbrake && !front) { fx -= (vf / 0.5).clamp(-1.0, 1.0) * s.brake_force / 4.0; }
            let lim = lat_mu * fs;
            let mag = fl.hypot(fx);
            if mag > lim && mag > 1e-6 { let k = lim / mag; fl *= k; fx *= k; }
            let tire = rw * fl + fw * fx;
            force = force + tire; torque = torque + r_c.cross(tire);
        }
        force = force - self.vel * (self.vel.len() * 0.35);
        self.vel = self.vel + force * (h / s.mass);
        let wb = self.rot.conj().rotate(self.omega);
        let tb = self.rot.conj().rotate(torque);
        let iw = v3(s.inertia.x * wb.x, s.inertia.y * wb.y, s.inertia.z * wb.z);
        let gyro = wb.cross(iw);
        let acc_b = v3((tb.x - gyro.x) / s.inertia.x, (tb.y - gyro.y) / s.inertia.y, (tb.z - gyro.z) / s.inertia.z);
        self.omega = (self.omega + self.rot.rotate(acc_b) * h) * (1.0 - 0.8 * h);
        self.pos = self.pos + self.vel * h;
        self.rot = self.rot.integrate(self.omega, h);
    }

    fn collide_props(&mut self, props: &[Prop]) {
        for p in props {
            let (c, min) = match p.kind {
                PropKind::Rock => (p.pos + v3(0., p.radius * 0.5, 0.), p.radius + 1.2),
                PropKind::Tree => { if self.pos.y > p.pos.y + p.height { continue; } (v3(p.pos.x, self.pos.y, p.pos.z), p.radius + 1.1) }
            };
            let vc = if p.kind == PropKind::Rock { self.pos + v3(0., 0.4, 0.) } else { self.pos };
            let d = vc - c;
            let dist = d.len();
            if dist < min && dist > 1e-3 {
                let n = d * (1.0 / dist);
                self.pos = self.pos + n * (min - dist);
                let vn = self.vel.dot(n);
                if vn < 0.0 { self.vel = self.vel - n * (vn * 1.2); }
            }
        }
    }
}

#[cfg(test)]
#[path = "vehicle_tests.rs"]
mod tests;
