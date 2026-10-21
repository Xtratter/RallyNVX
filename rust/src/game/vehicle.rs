//! Arcade off-road vehicle: rigid body, four spring/damper wheels over a heightfield, friction-circle tyres.
use super::grip::{self, LooseMap};
use super::input::Input;
use super::math::{v3, Q, V3};
use super::props::{Prop, PropKind};
use super::surface::Surface;
use super::terrain::Terrain;
use super::wheel;
use super::tyre::{self, Tyre};
use super::chassis::{self, Chassis};
use super::damage::Damage;

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
    /// The car the game starts with when no profile picks one: the first car of the garage (the Peugeot); the stand-in only if the file has no other.
    pub fn default_car() -> VehicleSpec { Self::all().into_iter().next().map(|(_, s)| s).unwrap_or_else(Self::offroader) }
    pub fn by_id(id: &str) -> Option<VehicleSpec> { super::params::get().car(id) }
    /// The car kept only as the engine's stand-in (tests, the world before a profile picks a car): it is not in the garage or the lists.
    pub const INTERNAL_ID: &'static str = "offroader";
    /// Every car of the vehicles file the player can have, with its id, sorted by id (the stand-in `INTERNAL_ID` is left out).
    pub fn all() -> Vec<(String, VehicleSpec)> { super::params::get().cars.iter().filter(|(id, _)| id != Self::INTERNAL_ID).cloned().collect() }
}

/// What one wheel did in the last sub-step: for the dust (and later sound/tracks).
#[derive(Clone, Copy, Debug)]
pub struct WheelInfo { pub grounded: bool, pub surface: Surface, pub contact: V3, pub speed: f32, pub lat_slip: f32, pub slide: f32, pub slip: f32 }
impl WheelInfo { pub const NONE: WheelInfo = WheelInfo { grounded: false, surface: Surface::Asphalt, contact: V3::ZERO, speed: 0.0, lat_slip: 0.0, slide: 0.0, slip: 0.0 }; }

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
    /// The handbrake is on: the rear wheels are locked (they do not turn and hold the car against a slope); the brake lights do not come on for it.
    pub parked: bool,
    /// Brake disc heat per wheel, 0..1 (grows with the braking power, cools with time and the air; only visual: the discs glow).
    pub brake_heat: [f32; 4],
    /// How wet the ground is, 0..1 (set by the world from the weather): the grip of every surface drops with it (`surfaces.json` -> `wet`).
    pub wet: f32,
    /// The driver holds reverse (negative throttle), as opposed to merely rolling backwards.
    pub reverse_throttle: bool,
    /// The torque the driveline puts on each wheel (N m at the wheel, positive pushes the car forward); `None` falls back to the throttle times the spec's engine force.
    pub drive_torque: Option<[f32; 4]>,
    /// Each wheel's spin (rad/s, positive rolls forward) and its slip ratio in shares of the slip at the peak of the grip (1 = at the peak, negative while braking or reversing).
    pub w: [f32; 4], pub slip: [f32; 4],
    /// The wheels have been set rolling with the ground (a car placed with a speed starts with turning wheels), and the velocity at the end of the last step:
    /// a jump of more than 2 m/s that no force could make in one step (a car placed with a speed, a teleport) sets the wheels rolling with the ground again.
    spin_ready: bool, last_vel: V3,
    /// The torque the tyres put against the wheels' spin in the last substep (N m, summed over the wheels).
    pub tyre_sum: f32,
    /// The aids: with ABS (traction control) off a wheel braked (driven) past the peak of its grip locks (spins) and slides on the reduced grip of a slide.
    pub abs_on: bool, pub tc_on: bool,
    /// The share of the brake force on the front axle (the rest on the rear); the car menu moves it.
    pub brake_bias: f32,
    /// Each tyre's temperature and wear, the cold pressure as a multiplier of the set's default and the air's temperature (degrees C): they decide the grip (see `tyre`).
    pub tyres: [Tyre; 4], pub tyre_bar: f32, pub air_c: f32,
    /// The suspension setup: the anti-roll bars, the camber and the toe (see `chassis`).
    pub chassis: Chassis,
    /// What the crashes and the hard landings have done to the body, the springs and the engine (see `damage`).
    pub damage: Damage,
    /// The brake torque the stability control adds to each wheel (N m).
    pub esc_brake: [f32; 4],
    /// The surface under the first wheel in contact (the HUD shows it); tests can force one for every wheel.
    pub surface: Surface, pub force_surface: Option<Surface>,
    pub wheels: [WheelInfo; 4],
}

const SUB: usize = 4;
const G: f32 = 9.81;

impl Vehicle {
    pub fn new(spec: VehicleSpec, pos: V3, yaw: f32) -> Vehicle {
        Vehicle { spec, pos, rot: Q::axis_angle(v3(0., 1., 0.), yaw), vel: V3::ZERO, omega: V3::ZERO, steer: 0.0, comp: [0.0; 4], contacts: 0, parked: false, braking: false, reversing: false, brake_heat: [0.0; 4], wet: 0.0, reverse_throttle: false, drive_torque: None, w: [0.0; 4], slip: [0.0; 4], spin_ready: false, last_vel: V3::ZERO, tyre_sum: 0.0, abs_on: true, tc_on: true, brake_bias: wheel::BRAKE_BIAS, tyres: [Tyre::warm(spec.wheel_set); 4], tyre_bar: 1.0, air_c: 15.0, chassis: Chassis::default(), damage: Damage::default(), esc_brake: [0.0; 4], surface: Surface::Asphalt, force_surface: None, wheels: [WheelInfo::NONE; 4] }
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
        self.vel = V3::ZERO; self.omega = V3::ZERO; self.steer = 0.0; self.comp = [0.0; 4]; self.w = [0.0; 4]; self.slip = [0.0; 4]; self.spin_ready = false; self.tyre_sum = 0.0;
    }

    pub fn step(&mut self, t: &Terrain, props: &[Prop], inp: &Input, dt: f32) {
        let mut none = LooseMap::empty();
        self.step_with(t, props, inp, dt, &mut none);
    }

    /// One step; `loose` is the level's loose gravel map, which the wheels on a stage wear.
    pub fn step_with(&mut self, t: &Terrain, props: &[Prop], inp: &Input, dt: f32, loose: &mut LooseMap) { self.step_driven(t, props, inp, dt, loose, None); }

    /// Like `step_with`, with the driveline solved in the substeps: each one calls `drive(h, the wheels' spins, the torque the tyres put against them)` and puts what it returns on the wheels.
    pub fn step_driven(&mut self, t: &Terrain, props: &[Prop], inp: &Input, dt: f32, loose: &mut LooseMap, mut drive: Option<&mut dyn FnMut(f32, [f32; 4], f32) -> [f32; 4]>) {
        let inp = inp.sanitized();
        let jumped = (self.vel - self.last_vel).len() > 2.0;   // a speed set from outside (a reset, a teleport) is no crash
        if jumped { self.spin_ready = false; }
        let (v0, c0) = (self.vel, self.contacts);
        self.braking = inp.brake;   // the brake lights are for the pedal: a handbrake that is merely held does not light them
        self.parked = inp.handbrake;
        self.reverse_throttle = inp.throttle < -0.05;
        self.reversing = self.reverse_throttle || self.speed() < -0.5;
        let max_a = self.spec.steer_max / (1.0 + self.vel.len() / self.spec.steer_speed_ref);
        let rate = self.spec.steer_rate * dt;
        self.steer += (inp.steer * max_a - self.steer).clamp(-rate, rate);
        let h = dt / SUB as f32;
        for _ in 0..SUB {
            if let Some(d) = drive.as_mut() { self.drive_torque = Some((*d)(h, self.w, self.tyre_sum)); }
            self.substep(t, &inp, h, loose);
        }
        self.collide_props(props);
        let g = t.height(self.pos.x, self.pos.z);
        if self.pos.y < g + 0.2 {
            // the body touches the ground (flipped or bottomed out): it scrapes and slows down
            self.pos.y = g + 0.2;
            if self.vel.y < 0.0 { self.vel.y = 0.0; }
            let k = (1.0 - 3.0 * dt).max(0.0);
            self.vel.x *= k; self.vel.z *= k; self.omega = self.omega * k;
        }
        // a crash is a sudden change of the horizontal velocity within one step, a hard landing a touchdown at speed
        if !jumped {
            self.register_hit(self.vel - v0);
            if c0 == 0 && self.contacts > 0 { self.damage.landing(v0.y.min(0.0)); }
        }
        self.last_vel = self.vel;
    }

    /// A hit that changed the velocity by `dv` (the car's own, from the world's collisions too): the damage by its size and by where on the car it came from.
    pub fn register_hit(&mut self, dv: V3) {
        let h = v3(dv.x, 0.0, dv.z);
        if !(h.len() > 0.0) || !h.len().is_finite() { return; }
        // the velocity changes against the side the obstacle is on: braked in the heading = hit in front, pushed to the right = hit on the left
        self.damage.hit(h.len(), h.dot(self.forward()) < 0.0, h.dot(self.right()) > 0.0);
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
        let mut tyre_sum = 0.0;
        let max_susp = s.max_susp_factor * G * wheel_mass;
        let level = t.water_level();
        let mut wet = 0;
        for w in 0..4 {
            self.comp[w] = 0.0;
            let front = w < 2;
            // the torques at this wheel: the driveline's (with no drivetrain, the throttle's share of the engine force) and the brake's (the handbrake is a plain rear brake)
            let lim_speed = if inp.throttle >= 0.0 { s.max_speed } else { s.max_speed * s.reverse_factor };
            let taper = (1.0 - (fspeed * inp.throttle.signum()).max(0.0) / lim_speed).clamp(0.0, 1.0);
            let drive_t = match self.drive_torque { Some(t) => t[w], None => inp.throttle * s.engine_force / 4.0 * taper * s.radius };
            let kb = if inp.brake_amt > 0.0 { inp.brake_amt.clamp(0.05, 1.0) } else { 1.0 };
            let pedal = if inp.brake { kb } else { 0.0 };
            let part = if front { self.brake_bias } else { 1.0 - self.brake_bias };
            let mut brake_t = s.brake_force * s.radius * 0.5 * part * pedal + self.esc_brake[w];
            if inp.handbrake && !front { brake_t = brake_t.max(s.brake_force * s.radius * 0.5); }
            // the discs cool all the time, faster the faster the car goes
            self.brake_heat[w] *= (-h * (1.0 + self.vel.len() / 15.0) / 6.0).exp();
            let r_att = self.rot.rotate(s.wheels[w]);
            let attach = self.pos + r_att;
            let ground = t.height(attach.x, attach.z);
            let n = t.normal(attach.x, attach.z);
            if level.map_or(false, |l| ground < l - 0.05) { wet += 1; }
            if up.dot(n) < 0.2 { self.spin_free(w, drive_t, brake_t, h); continue; }
            let d = (attach.y - ground) / up.y.max(0.2);
            let compress = s.rest + s.radius - d;
            if compress <= 0.0 { self.spin_free(w, drive_t, brake_t, h); continue; }
            self.contacts += 1;
            self.comp[w] = compress.min(s.rest + s.radius);
            let vp = self.vel + self.omega.cross(r_att);
            // the anti-roll bar of the axle: up on the more compressed wheel, down on the other (the other wheel's compression is the last substep's)
            let (pair, bar_k, track) = if front { (1 - w, self.chassis.arb_front, (s.wheels[0].x - s.wheels[1].x).abs()) } else { (5 - w, self.chassis.arb_rear, (s.wheels[2].x - s.wheels[3].x).abs()) };
            let bar = chassis::bar_force(bar_k, track, compress, self.comp[pair]);
            let fs = (s.spring * self.damage.spring(w) * compress - s.damper * vp.dot(n) + bar).clamp(0.0, max_susp);
            let susp = n * fs;
            force = force + susp; torque = torque + r_att.cross(susp);
            // tyre
            let r_c = r_att - up * d;
            let vc = self.vel + self.omega.cross(r_c);
            // the wheel's heading: the steering (the front) and the toe of its axle
            let left_wheel = s.wheels[w].x > 0.0;
            let toe = chassis::wheel_toe(if front { self.chassis.toe_front } else { self.chassis.toe_rear }, left_wheel);
            let ang = (if front { self.steer } else { 0.0 }) + toe + self.damage.heading(w);
            let base = fwd * ang.cos() + right * ang.sin();
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
            let wf = grip::wet_factor(surf, self.wet);       // the wet ground: less grip, everywhere
            let tyre_grip = tyre::grip(s.wheel_set, self.tyres[w].temp, self.tyres[w].wear, tyre::cold_default(s.wheel_set) * self.tyre_bar);
            let (mu_s, lat_k, roll) = (s.mu * sg.mu * tf.0 * wf.0 * tyre_grip, s.lateral_k * sg.lateral * tf.1 * wf.1, s.rolling * sg.rolling);
            // a wheel placed with a speed starts turning with the ground
            if !self.spin_ready { self.w[w] = vf / s.radius; }
            let (kp, keep) = wheel::slip_params(surf);
            // the grip a tyre gets from its load falls behind the load (so the bar that moves load onto an axle costs it grip), and the camber (static plus the body's roll) shifts it
            let roll_left = right.y.clamp(-1.0, 1.0).asin().to_degrees();
            let camber = chassis::camber_grip(if front { self.chassis.camber_front } else { self.chassis.camber_rear }, roll_left, if left_wheel { 1.0 } else { -1.0 });
            let fmax = mu_s * fs * chassis::load_factor(fs, wheel_mass * G) * camber;
            // the friction circle: while the tyre grips (up to about twice the peak of its slip) the side force comes first, the way the car and the autopilot were tuned, and the drive or the brake gets
            // what is left; a tyre that slips far past the peak (locked or burning out) slides along its slip, so its force is the longitudinal one and the side keeps only what that leaves
            let k_now = (self.w[w] * s.radius - vf) / vf.abs().max(1.0);
            let t_sl = { let x = ((k_now.abs() / kp - 2.0) / 3.0).clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) };
            let fl_dem = -vl * wheel_mass * lat_k;
            let fl0 = fl_dem.clamp(-0.97 * fmax, 0.97 * fmax);
            let fmax_long = { let first = (fmax * fmax - fl0 * fl0).max(0.0).sqrt(); first + (fmax - first) * t_sl };
            // a car with no drivetrain of its own (the throttle times the engine force) gets an ideal traction control: it never asks the tyre for more than it can carry
            let drive_w = if self.drive_torque.is_none() && self.tc_on { drive_t.clamp(-s.radius * fmax_long, s.radius * fmax_long) } else { drive_t };
            let o = wheel::step(&wheel::In { w: self.w[w], vf, radius: s.radius, fmax: fmax_long, kp, slide: keep, drive: drive_w, brake: brake_t, abs: self.abs_on && !(inp.handbrake && !front), h, inertia: wheel::INERTIA });   // the handbrake is mechanical: no ABS on it
            self.w[w] = o.w; self.slip[w] = o.slip / kp;
            tyre_sum += s.radius * o.fx;
            // the rolling resistance (a drag that grows with the speed) adds to the tyre's own force but shares the friction circle with the side force: when the tyre holds the side at its limit,
            // it is squeezed out, as it always was; kept apart it drags the saturated front tyres and the car slides out of a long turn. The total cannot pass what the tyre has: its peak while
            // it grips, its sliding share once it slides, so a locked wheel brakes less than one held at the peak by the ABS
            let roll_f = -vf * roll;
            let mut fx = o.fx + roll_f * (fmax / fl_dem.hypot(roll_f).max(1e-6)).min(1.0);
            let cap_t = fmax * (1.0 - t_sl * (1.0 - keep));
            fx = fx.clamp(-cap_t, cap_t);
            let fl_cap = { let rest = (fmax * fmax - o.fx * o.fx).max(0.0).sqrt(); 0.97 * fmax + (rest - 0.97 * fmax) * t_sl };
            let mut fl = fl_dem.clamp(-fl_cap, fl_cap);
            if o.brake_used > 0.0 {
                // the braking power heats the disc (the front brakes do more of the work)
                self.brake_heat[w] = (self.brake_heat[w] + o.brake_used / s.radius * vf.abs() * if front { 1.3 } else { 0.7 } * 4.5e-6 * h).min(1.0);
            }
            let excess = if fmax > 1e-3 { (fl_dem.hypot(o.fx.abs().max(drive_t.abs() / s.radius)) / fmax - 1.0).clamp(0.0, 1.5).max((o.slip.abs() / kp - 1.0).clamp(0.0, 1.5)) } else { 0.0 };
            self.wheels[w] = WheelInfo { grounded: true, surface: surf, contact, speed: vc.len(), lat_slip: vl.abs(), slide: excess, slip: o.slip };
            // the friction work heats the tread and wears it (the force times the slip speed, along and across)
            let work = (o.fx * (self.w[w] * s.radius - vf)).abs() + (fl_dem.clamp(-fmax, fmax) * vl).abs();
            self.tyres[w].step(h, work, self.vel.len(), fs, self.air_c, tyre::abrasiveness(surf, tf.0));
            let mag = fl.hypot(fx);
            if mag > fmax && mag > 1e-6 { let k = fmax / mag; fl *= k; fx *= k; }
            let tire = rw * fl + fw * fx;
            force = force + tire; torque = torque + r_c.cross(tire);
        }
        self.spin_ready = true;
        self.tyre_sum = tyre_sum;
        // aerodynamics: drag at the centre, downforce pressing the body down at the two axles (the springs load the tyres, so the grip
        // grows), the side force behind the centre (it turns the nose back into the flow); the clearance comes from the suspension
        let ac = super::aero::cfg();
        let static_comp = s.mass * G / (4.0 * s.spring.max(1.0));
        let ride = s.aero_base.ride_ref;
        let clear = |m: f32| (ride - (m - static_comp)).clamp(0.3 * ride, 2.5 * ride);
        let af = super::aero::forces(&s, self.vel, fwd, right, clear(0.5 * (self.comp[0] + self.comp[1])), clear(0.5 * (self.comp[2] + self.comp[3])), &ac);
        force = force + af.drag * self.damage.drag_factor() + af.side;
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

    /// A wheel off the ground: it only turns under its drive torque and its brake.
    fn spin_free(&mut self, w: usize, drive: f32, brake: f32, h: f32) {
        self.tyres[w].step(h, 0.0, self.vel.len(), 0.0, self.air_c, 0.0);
        // a car with no drivetrain of its own has no clutch to open in the air: its throttle must not spin a wheel that touches nothing
        let drive = if self.drive_torque.is_none() { 0.0 } else { drive };
        let o = wheel::step(&wheel::In { w: self.w[w], vf: 0.0, radius: self.spec.radius, fmax: 0.0, kp: 0.1, slide: 0.8, drive, brake, abs: false, h, inertia: wheel::INERTIA });
        self.w[w] = o.w; self.slip[w] = 0.0;
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
