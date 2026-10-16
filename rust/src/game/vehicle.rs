//! Arcade off-road vehicle: rigid body, four spring/damper wheels over a heightfield, friction-circle tyres.
use super::grip::{self, LooseMap};
use super::input::Input;
use super::math::{v3, Q, V3};
use super::props::{Prop, PropKind};
use super::surface::Surface;
use super::terrain::Terrain;

/// Which generated body the renderer builds for a car.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarStyle { Suv, Rally }

#[derive(Clone, Copy, Debug)]
pub struct VehicleSpec {
    /// Body style, price in the garage (0 = owned from the start) and the base price of one upgrade level.
    pub style: CarStyle, pub price: u32, pub upgrade_cost: u32,
    /// The wheel set shown (0 tarmac, 1 gravel, 2 snow): set from the garage; the renderer picks the wheel mesh by it and the tyre table of `surfaces.json` (`tyre_sets`) scales the grip by it.
    pub wheel_set: u8,
    pub mass: f32,
    /// Body box (width, height, length) in metres and the renderer's colours.
    pub size: V3, pub look: super::params::Look,
    /// Principal moments of inertia about body x (pitch), y (yaw), z (roll).
    pub inertia: V3,
    /// Suspension attachment points in body space: FL, FR, RL, RR.
    pub wheels: [V3; 4],
    pub rest: f32, pub radius: f32, pub spring: f32, pub damper: f32,
    pub engine_force: f32, pub brake_force: f32, pub max_speed: f32, pub mu: f32,
    /// Handling constants (see docs/vehicles.md): steering angle limit and its speed fade, steering rate, lateral tyre
    /// stiffness, rolling resistance, air drag, angular damping, handbrake rear grip factor, reverse speed factor,
    /// suspension force cap in car weights per wheel.
    pub steer_max: f32, pub steer_speed_ref: f32, pub steer_rate: f32, pub lateral_k: f32, pub rolling: f32,
    pub aero: f32, pub aero_base: super::aero::AeroBase, pub kit: super::aero::AeroKit, pub cam_hood: [f32; 2], pub cam_bumper: [f32; 2], pub cam_cockpit: Option<[f32; 3]>, pub ang_damp: f32, pub handbrake_grip: f32, pub reverse_factor: f32, pub max_susp_factor: f32,
    /// Extra drag in water, 1/s with all four wheels wet (see docs/vehicles.md).
    pub water_drag: f32,
}

impl VehicleSpec {
    /// The off-roader, from `vehicles.json` (see docs/vehicles.md).
    pub fn offroader() -> VehicleSpec { Self::by_id("offroader").expect("vehicles.json must define cars.offroader") }
    pub fn by_id(id: &str) -> Option<VehicleSpec> { super::params::get().car(id) }
    /// Every car of the vehicles file with its id, sorted by id.
    pub fn all() -> Vec<(String, VehicleSpec)> { super::params::get().cars.clone() }
}

/// What one wheel did in the last sub-step: for the dust (and later sound/tracks).
#[derive(Clone, Copy, Debug)]
pub struct WheelInfo { pub grounded: bool, pub surface: Surface, pub contact: V3, pub speed: f32, pub lat_slip: f32, pub slide: f32 }
impl WheelInfo { pub const NONE: WheelInfo = WheelInfo { grounded: false, surface: Surface::Asphalt, contact: V3::ZERO, speed: 0.0, lat_slip: 0.0, slide: 0.0 }; }

#[derive(Clone, Copy, Debug)]
pub struct Vehicle {
    pub spec: VehicleSpec,
    pub pos: V3, pub rot: Q, pub vel: V3, pub omega: V3,
    pub steer: f32,
    /// Spring compression per wheel in metres (0 = airborne); the wheel centre sits `rest - comp` below its attachment.
    pub comp: [f32; 4],
    pub contacts: u8,
    /// Driver input of the last step, for the lights.
    pub braking: bool, pub reversing: bool,
    /// Brake disc heat per wheel, 0..1 (grows with the braking power, cools with time and the air; only visual: the discs glow).
    pub brake_heat: [f32; 4],
    /// The driver holds reverse (negative throttle), as opposed to merely rolling backwards.
    pub reverse_throttle: bool,
    /// The surface under the first wheel in contact (the HUD shows it); tests can force one for every wheel.
    pub surface: Surface, pub force_surface: Option<Surface>,
    pub wheels: [WheelInfo; 4],
}

const SUB: usize = 4;
const G: f32 = 9.81;

impl Vehicle {
    pub fn new(spec: VehicleSpec, pos: V3, yaw: f32) -> Vehicle {
        Vehicle { spec, pos, rot: Q::axis_angle(v3(0., 1., 0.), yaw), vel: V3::ZERO, omega: V3::ZERO, steer: 0.0, comp: [0.0; 4], contacts: 0, braking: false, reversing: false, brake_heat: [0.0; 4], reverse_throttle: false, surface: Surface::Asphalt, force_surface: None, wheels: [WheelInfo::NONE; 4] }
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
        let mut none = LooseMap::empty();
        self.step_with(t, props, inp, dt, &mut none);
    }

    /// One step; `loose` is the level's loose gravel map, which the wheels on a stage wear.
    pub fn step_with(&mut self, t: &Terrain, props: &[Prop], inp: &Input, dt: f32, loose: &mut LooseMap) {
        let inp = inp.sanitized();
        self.braking = inp.brake || inp.handbrake;
        self.reverse_throttle = inp.throttle < -0.05;
        self.reversing = self.reverse_throttle || self.speed() < -0.5;
        let max_a = self.spec.steer_max / (1.0 + self.vel.len() / self.spec.steer_speed_ref);
        let rate = self.spec.steer_rate * dt;
        self.steer += (inp.steer * max_a - self.steer).clamp(-rate, rate);
        let h = dt / SUB as f32;
        for _ in 0..SUB { self.substep(t, &inp, h, loose); }
        self.collide_props(props);
        let g = t.height(self.pos.x, self.pos.z);
        if self.pos.y < g + 0.2 {
            // the body touches the ground (flipped or bottomed out): it scrapes and slows down
            self.pos.y = g + 0.2;
            if self.vel.y < 0.0 { self.vel.y = 0.0; }
            let k = (1.0 - 3.0 * dt).max(0.0);
            self.vel.x *= k; self.vel.z *= k; self.omega = self.omega * k;
        }
    }

    fn substep(&mut self, t: &Terrain, inp: &Input, h: f32, loose: &mut LooseMap) {
        let s = self.spec;
        let (fwd, up, right) = (self.forward(), self.up(), self.right());
        let fspeed = self.vel.dot(fwd);
        let mut force = v3(0., -G * s.mass, 0.);
        let mut torque = V3::ZERO;
        self.contacts = 0;
        self.wheels = [WheelInfo::NONE; 4];
        let wheel_mass = s.mass / 4.0;
        let max_susp = s.max_susp_factor * G * wheel_mass;
        let level = t.water_level();
        let mut wet = 0;
        for w in 0..4 {
            self.comp[w] = 0.0;
            let front = w < 2;
            // the discs cool all the time, faster the faster the car goes
            self.brake_heat[w] *= (-h * (1.0 + self.vel.len() / 15.0) / 6.0).exp();
            let r_att = self.rot.rotate(s.wheels[w]);
            let attach = self.pos + r_att;
            let ground = t.height(attach.x, attach.z);
            let n = t.normal(attach.x, attach.z);
            if level.map_or(false, |l| ground < l - 0.05) { wet += 1; }
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
            // the ground under this wheel: grip, side stiffness and rolling resistance follow the surface; gravel wears
            let contact = self.pos + r_c;
            let (surf, looseness, cell) = match self.force_surface { Some(f) => (f, 1.0, None), None => grip::ground_at(t, loose, contact.x, contact.z) };
            let sg = grip::grip(surf, looseness);
            if self.contacts == 1 { self.surface = surf; }
            if let Some((ri, s_along, side)) = cell { loose.wear(ri, s_along, side, vf.abs().max(vl.abs()), h); }
            let tf = grip::tyre_factor(s.wheel_set, surf);   // the wheel set: the right tyres for the surface grip more
            let (mu_s, lat_k, roll) = (s.mu * sg.mu * tf.0, s.lateral_k * sg.lateral * tf.1, s.rolling * sg.rolling);
            let lat_mu = if !front && inp.handbrake { mu_s * s.handbrake_grip } else { mu_s };
            let mut fl = -vl * wheel_mass * lat_k;
            let lim_speed = if inp.throttle >= 0.0 { s.max_speed } else { s.max_speed * s.reverse_factor };
            let taper = (1.0 - (fspeed * inp.throttle.signum()).max(0.0) / lim_speed).clamp(0.0, 1.0);
            let mut fx = inp.throttle * s.engine_force / 4.0 * taper - vf * roll;
            // brake pedal: scaled by how hard it is pressed (0 = legacy full force); the handbrake always bites the rear wheels fully
            let kb = if inp.brake_amt > 0.0 { inp.brake_amt.clamp(0.05, 1.0) } else { 1.0 };
            let k = if inp.handbrake && !front { 1.0 } else if inp.brake { kb } else { 0.0 };
            if k > 0.0 {
                fx -= (vf / 0.5).clamp(-1.0, 1.0) * s.brake_force / 4.0 * k;
                // the braking power heats the disc (the front brakes do more of the work)
                self.brake_heat[w] = (self.brake_heat[w] + s.brake_force / 4.0 * k * vf.abs() * if front { 1.3 } else { 0.7 } * 4.5e-6 * h).min(1.0);
            }
            let lim = lat_mu * fs;
            let mag = fl.hypot(fx);
            let excess = if lim > 1e-3 { ((mag / lim) - 1.0).clamp(0.0, 1.5) } else { 0.0 };
            self.wheels[w] = WheelInfo { grounded: true, surface: surf, contact, speed: vc.len(), lat_slip: vl.abs(), slide: excess };
            if mag > lim && mag > 1e-6 { let k = lim / mag; fl *= k; fx *= k; }
            let tire = rw * fl + fw * fx;
            force = force + tire; torque = torque + r_c.cross(tire);
        }
        // aerodynamics: drag at the centre, downforce pressing the body down at the two axles (the springs load the tyres, so the grip
        // grows), the side force behind the centre (it turns the nose back into the flow); the clearance comes from the suspension
        let ac = super::aero::cfg();
        let static_comp = s.mass * G / (4.0 * s.spring.max(1.0));
        let ride = s.aero_base.ride_ref;
        let clear = |m: f32| (ride - (m - static_comp)).clamp(0.3 * ride, 2.5 * ride);
        let af = super::aero::forces(&s, self.vel, fwd, right, clear(0.5 * (self.comp[0] + self.comp[1])), clear(0.5 * (self.comp[2] + self.comp[3])), &ac);
        force = force + af.drag + af.side;
        let (zf, zr) = (0.5 * (s.wheels[0].z + s.wheels[1].z), 0.5 * (s.wheels[2].z + s.wheels[3].z));
        let (pf, pr) = (up * (-af.down_front), up * (-af.down_rear));
        force = force + pf + pr;
        torque = torque + self.rot.rotate(v3(0.0, 0.0, zf)).cross(pf) + self.rot.rotate(v3(0.0, 0.0, zr)).cross(pr) + self.rot.rotate(v3(0.0, 0.0, -ac.cp)).cross(af.side);
        if wet > 0 { force = force - self.vel * (s.mass * s.water_drag * wet as f32 / 4.0); }
        self.vel = self.vel + force * (h / s.mass);
        let wb = self.rot.conj().rotate(self.omega);
        let tb = self.rot.conj().rotate(torque);
        let iw = v3(s.inertia.x * wb.x, s.inertia.y * wb.y, s.inertia.z * wb.z);
        let gyro = wb.cross(iw);
        let acc_b = v3((tb.x - gyro.x) / s.inertia.x, (tb.y - gyro.y) / s.inertia.y, (tb.z - gyro.z) / s.inertia.z);
        self.omega = (self.omega + self.rot.rotate(acc_b) * h) * (1.0 - s.ang_damp * h);
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
