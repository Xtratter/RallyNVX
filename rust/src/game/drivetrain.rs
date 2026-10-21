//! The car's drivetrain: the engine's speed, the clutch, the gearbox and the force at the wheels. The engine is a torque map measured from the sound's engine model
//! (`engine_torque.json`: net torque over revs and throttle at the spooled boost, made by `generate_torque_table`), a turbo lag and an idle governor; it turns with its own inertia against
//! the clutch, which carries torque to the wheels through the gear. Two modes: an automatic (the gear and the clutch are decided here) and a manual (the driver shifts the gears and works the clutch).
use std::sync::OnceLock;
use super::heat::{Clutch, Coolant};
use super::shiftlogic::{choose, table as shift_table, Choice, Memory, Profile, Situation};

/// The gear ratios (times the final drive below) for 1..6 and the reverse.
pub const RATIOS: [f32; 6] = [2.92, 2.12, 1.62, 1.31, 1.08, 0.92];
pub const REVERSE: f32 = 3.2;
pub const FINAL: f32 = 5.0;
/// The efficiency of the driveline.
const ETA: f32 = 0.90;
/// The engine's inertia with the flywheel and the clutch's plate (kg m^2) and the stiffness of the clutch's grip (N m per rad/s).
const INERTIA: f32 = 0.20;
const GRIP_K: f32 = 40.0;
/// The most torque the clutch carries (N m).
const CLUTCH_MAX: f32 = 520.0;
pub const IDLE_RPM: f32 = 950.0;
pub const LIMITER_RPM: f32 = 7800.0;
/// The revs launch control holds at a standstill with the brake down and the gas floored.
const LAUNCH_RPM: f32 = 4800.0;
/// Seconds a shift takes in the automatic and in the sequential manual box.
/// Seconds a shift takes in the sequential box (up with the spark cut, down with a blip) and in the H-pattern box (a synchro).
const SEQ_UP: f32 = 0.08;
const SEQ_DOWN: f32 = 0.10;
const H_SHIFT: f32 = 0.20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Auto, Manual }

/// The manual box: the sequential of the real car (shifts without the clutch pedal: the spark is cut on the way up, the throttle blipped on the way down; the pedal is only an override)
/// or the H-pattern with a clutch that must be worked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoxType { #[default] Sequential, HPattern }
impl BoxType { pub fn from_i32(v: i32) -> BoxType { if v == 1 { BoxType::HPattern } else { BoxType::Sequential } } }

struct Table { rpm: Vec<f32>, thr: Vec<f32>, t: Vec<Vec<f32>> }

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(include_str!("../../engine_torque.json")).expect("engine_torque.json");
        let f = |k: &str| v[k].as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect::<Vec<_>>();
        let t = v["torque"].as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect()).collect();
        Table { rpm: f("rpm"), thr: f("throttle"), t }
    })
}

/// The engine's net torque (N m) at `rpm` and the throttle `thr` (0..1), bilinear over the map; below the map's first row the torque falls to zero at a standstill.
pub fn engine_torque(rpm: f32, thr: f32) -> f32 {
    let t = table();
    let rpm_c = rpm.clamp(t.rpm[0], *t.rpm.last().unwrap());
    let thr = thr.clamp(0.0, 1.0);
    let at = |a: &[f32], x: f32| { let i = a.iter().rposition(|&v| v <= x).unwrap_or(0).min(a.len() - 2); (i, ((x - a[i]) / (a[i + 1] - a[i])).clamp(0.0, 1.0)) };
    let ((i, fi), (j, fj)) = (at(&t.thr, thr), at(&t.rpm, rpm_c));
    let row = |r: usize| t.t[r][j] * (1.0 - fj) + t.t[r][j + 1] * fj;
    let v = row(i) * (1.0 - fi) + row(i + 1) * fi;
    // below 1000 rpm the map is cut off: the friction and the pumping fade with the speed
    if rpm < t.rpm[0] { v * (rpm / t.rpm[0]).clamp(0.0, 1.0) } else { v }
}

/// What the drivetrain is asked each step.
#[derive(Clone, Copy, Debug, Default)]
pub struct DriveIn {
    /// The throttle pedal: -1..1 in the automatic (negative asks for the reverse), 0..1 in the manual.
    pub throttle: f32,
    /// The car's speed along its heading (m/s, negative backwards) and the wheels' radius (m).
    pub speed: f32, pub wheel_r: f32,
    pub airborne: bool,
    /// The brake pedal is down (the automatic holds a lower gear for the engine brake).
    pub brake: bool,
    /// The lateral acceleration (g) and the grade (the sine of the pitch, positive uphill): the automatic holds its gear in a corner and downshifts uphill.
    pub lat_g: f32, pub slope: f32,
    /// The largest slip over the peak of a wheel spinning in the driving direction (0 if none): the traction control cuts the engine on it.
    pub tc_slip: f32,
}

/// What it gives.
#[derive(Clone, Copy, Debug)]
pub struct DriveOut {
    /// The force at the ground along the car's heading (N, positive forward), summed over the wheels.
    pub force: f32,
    /// The torque the driveline puts on the four wheels together (N m, signed).
    pub wheel_torque: f32,
    pub rpm: f32, pub gear: i32,
    /// How firmly the clutch is closed (its capacity over the largest, 0 open .. 1 closed), the engine's net torque and the throttle the engine got (with the idle governor and a blip).
    pub grip: f32, pub torque: f32, pub throttle: f32,
    /// The boost pressure over the atmosphere, bar.
    pub boost_bar: f32,
    /// The spark is cut (a flat shift or the limiter), the engine is off (stalled).
    pub cut: bool, pub stalled: bool,
}

/// What the driver and the tuning change in the drivetrain: the engine's torque, the turbo's boost and the final drive as multipliers, and the aids.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DriveOpts { pub engine: f32, pub turbo: f32, pub gearing: f32, pub flat_shift: bool, pub blip: bool, pub limiter: bool, pub tc: bool, pub launch: bool }
impl Default for DriveOpts { fn default() -> DriveOpts { DriveOpts { engine: 1.0, turbo: 1.0, gearing: 1.0, flat_shift: true, blip: true, limiter: true, tc: true, launch: true } } }

/// What one frame fixes for the engine's substeps: the gear as a ratio and a direction, the clutch's capacity, the throttle the engine gets, the spark cut, the traction control's scale,
/// whether the engine is being matched to the new gear, and the sums the substeps leave for `finish`.
#[derive(Clone, Copy, Debug, Default)]
struct Frame { dt: f32, speed: f32, slip_w: f32, auto: bool, radius: f32, g: f32, rd: f32, cap: f32, thr: f32, cut: bool, tc_k: f32, sync: bool, n: u32, t_c: f32, t_e: f32 }

#[derive(Clone, Debug)]
pub struct Drivetrain {
    pub opts: DriveOpts,
    pub mode: Mode,
    /// -1 reverse, 0 neutral, 1..6.
    pub gear: i32,
    /// The engine's speed, rad/s.
    omega: f32,
    /// The clutch pedal of the manual, 0 (up, the clutch is closed) .. 1 (down, open).
    pub clutch_pedal: f32,
    /// The automatic's selector: false drive (D), true reverse (R): in R the gas pedal drives backwards.
    pub reverse_sel: bool,
    /// The automatic may pick the reverse by itself (the negative throttle at a standstill): only for the autopilot; the driver chooses R by hand.
    pub auto_reverse: bool,
    running: bool, starting: f32, stalled_for: f32,
    shift: f32, shift_up: bool, limiter: bool, boost: f32, idle_i: f32,
    /// What the automatic remembers of the gas pedal (a press at once, a lift fades) and the seconds since its last shift.
    mem: Memory, pub profile: Profile, pub box_type: BoxType,
    /// The clutch's and the coolant's temperatures, the air's temperature (set by the world) and the damage's share of the engine's power (1 = whole).
    pub clutch: Clutch, pub coolant: Coolant, pub air_c: f32, pub power_k: f32,
    fr: Frame,
    pub last: DriveOut,
}

impl Default for Drivetrain { fn default() -> Drivetrain { Drivetrain::new() } }

fn smooth(a: f32, b: f32, x: f32) -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

pub fn ratio(gear: i32) -> f32 { if gear < 0 { REVERSE * FINAL } else if gear == 0 { 0.0 } else { RATIOS[(gear.min(6) - 1) as usize] * FINAL } }
/// The engine's revs when the wheels (radius `r`) roll at `speed` m/s in `gear`.
pub fn rpm_in_gear(speed: f32, r: f32, gear: i32) -> f32 { speed.abs() / r.max(0.05) * ratio(gear) * 60.0 / std::f32::consts::TAU }

impl Drivetrain {
    pub fn new() -> Drivetrain {
        let rpm = IDLE_RPM;
        Drivetrain { opts: DriveOpts::default(), mode: Mode::Auto, gear: 1, omega: rpm * std::f32::consts::TAU / 60.0, clutch_pedal: 0.0, reverse_sel: false, auto_reverse: false, running: true, starting: 0.0, stalled_for: 0.0,
            shift: 0.0, shift_up: true, limiter: false, boost: 0.0, idle_i: 0.1, mem: Memory::default(), profile: Profile::Sport, box_type: BoxType::Sequential, clutch: Clutch::new(15.0), coolant: Coolant::new(75.0), air_c: 15.0, power_k: 1.0, fr: Frame::default(),
            last: DriveOut { force: 0.0, wheel_torque: 0.0, rpm, gear: 1, grip: 0.0, torque: 0.0, throttle: 0.0, boost_bar: 0.0, cut: false, stalled: false } }
    }

    pub fn rpm(&self) -> f32 { self.omega * 60.0 / std::f32::consts::TAU }
    pub fn running(&self) -> bool { self.running }

    /// A new start (a reset, a new level): first gear, the engine at the idle.
    pub fn reset(&mut self) { let mode = self.mode; let sel = self.reverse_sel; let opts = self.opts; let ar = self.auto_reverse; let profile = self.profile; let box_type = self.box_type; let (clutch, coolant, air_c, power_k) = (self.clutch, self.coolant, self.air_c, self.power_k); *self = Drivetrain::new(); self.reverse_sel = sel; self.opts = opts; self.auto_reverse = ar; self.profile = profile; self.box_type = box_type; self.clutch = clutch; self.coolant = coolant; self.air_c = air_c; self.power_k = power_k; self.mode = mode; if mode == Mode::Manual { self.gear = 0; } }

    pub fn set_mode(&mut self, m: Mode) { if m != self.mode { self.mode = m; if m == Mode::Auto && self.gear == 0 { self.gear = 1; } } }

    /// The driver shifts: +1 up, -1 down (in the manual; the sequence is R, N, 1 .. 6).
    pub fn shift(&mut self, dir: i32) {
        if self.mode != Mode::Manual || self.shift > 0.0 || dir == 0 { return; }
        let g = (self.gear + dir.signum()).clamp(-1, 6);
        if g != self.gear { self.gear = g; self.shift = match (self.box_type, dir > 0) { (BoxType::Sequential, true) => SEQ_UP, (BoxType::Sequential, false) => SEQ_DOWN, (BoxType::HPattern, _) => H_SHIFT }; self.shift_up = dir > 0; }
    }

    pub fn set_clutch(&mut self, pedal: f32) { self.clutch_pedal = if pedal.is_finite() { pedal.clamp(0.0, 1.0) } else { 0.0 }; }

    /// The automatic's choice of the gear (the rules are in `shiftlogic`).
    fn auto_gear(&mut self, i: &DriveIn, thr: f32, rpm: f32, dt: f32) {
        // R is chosen by hand (the autopilot alone asks for the reverse with a negative throttle); a car that merely rolls back in D takes the first gear as soon as the gas is pressed
        let holding = self.gear == -1 && i.speed < -0.5 && thr < 0.05;
        if self.reverse_sel || (self.auto_reverse && ((i.throttle < -0.1 && i.speed < 1.0) || holding)) { if self.gear != -1 { self.gear = -1; } return; }
        if self.gear <= 0 { self.gear = 1; self.mem.shifted(); return; }
        let rate = self.mem.update(dt, thr, i.brake, i.lat_g);
        if self.shift > 0.0 { return; }
        let s = Situation { gear: self.gear, rpm, gas: thr, rate, brake: i.brake, speed: i.speed, wheel_r: i.wheel_r, gearing: self.opts.gearing, lat_g: i.lat_g, slope: i.slope };
        match choose(self.profile, &self.mem, &s) {
            Choice::Up(g) => self.begin_shift(g, true),
            Choice::Down(g) => self.begin_shift(g, false),
            Choice::Hold => {}
        }
    }

    fn begin_shift(&mut self, g: i32, up: bool) { self.gear = g; self.shift = shift_table(self.profile).shift_time; self.shift_up = up; self.mem.shifted(); }

    /// The start of a frame: the gear, the clutch, the throttle and the starter are decided here; the engine's substeps follow (`sub_kin` with the car's speed given, `sub_coupled` against the wheels), and `finish` closes the frame.
    pub fn begin(&mut self, dt: f32, i: &DriveIn) {
        let dt = dt.clamp(0.0, 0.05);
        if self.shift > 0.0 { self.shift = (self.shift - dt).max(0.0); }
        let auto = self.mode == Mode::Auto;
        let seq = !auto && self.box_type == BoxType::Sequential;
        let rpm0 = self.rpm();
        let gas = i.throttle.max(0.0);
        if auto { self.auto_gear(i, gas, rpm0, dt); }
        // the pedal the engine feels: in the reverse of the automatic the negative throttle is the gas
        let pedal = if auto { if self.gear < 0 { if self.reverse_sel { gas.max((-i.throttle).max(0.0)) } else { (-i.throttle).max(0.0) } } else { gas } } else { i.throttle.clamp(0.0, 1.0) };
        // the throttle the engine gets: the pedal, the idle governor holding the idle, and the blip of a down-shift
        let mut thr = pedal;
        if self.running {
            let err = (IDLE_RPM - rpm0) / IDLE_RPM;
            self.idle_i = (self.idle_i + err * dt * 0.6).clamp(0.0, 0.4);
            thr = thr.max((self.idle_i + 0.5 * err).clamp(0.0, 0.4));
        }
        if (auto || seq) && self.shift > 0.0 && !self.shift_up && self.opts.blip { thr = thr.max(0.55); }
        // launch control (the automatic and the sequential): at a standstill in first with the brake down and the gas floored the clutch is open and the engine is held at the launch revs
        // (the boost builds); the brake let go, the clutch bites at once and the traction control looks after the wheels
        let hold = self.opts.launch && (auto || seq) && self.gear == 1 && i.brake && gas > 0.9 && i.speed.abs() < 1.5 && self.running;
        if hold { thr = thr.min((0.55 + (LAUNCH_RPM - rpm0) / 1500.0).clamp(0.0, 1.0)); }
        let reverse_dir = if self.gear < 0 { -1.0 } else { 1.0 };
        let g = ratio(self.gear) * self.opts.gearing;
        let cut_shift = self.opts.flat_shift && self.shift > 0.0 && self.shift_up && pedal > if seq { 0.2 } else { 0.6 };
        // the clutch's capacity (N m): the automatic's and the sequential's electronic clutch rise with the revs and close for good on the move, the H-pattern's follows the pedal; none in neutral or in a shift
        // the brake pedal opens the lock-up (the electronic clutch of the sequential too): only the launch curve is left, which lets go as the revs fall, so the engine does not hold the wheels
        // back from locking and the wheels do not drag it down to a stall
        let launch = 10.0 + (CLUTCH_MAX - 10.0) * smooth(1300.0, 3000.0, rpm0);
        let lock = if i.brake { 0.0 } else { CLUTCH_MAX * smooth(2.0, 5.0, i.speed.abs()) };
        let auto_cap = launch.max(lock).min(CLUTCH_MAX);
        let pedal_cap = CLUTCH_MAX * smooth(0.85, 0.30, self.clutch_pedal);
        let fade = if auto { 1.0 } else { self.clutch.fade() };   // a burnt clutch (the sequential's and the H-pattern's) carries less
        let cap = fade * if g == 0.0 || self.shift > 0.0 || i.airborne || hold { 0.0 }
            else if auto { auto_cap }
            else if seq { if self.clutch_pedal > 0.05 { pedal_cap.min(auto_cap) } else { auto_cap } }
            else { pedal_cap };
        // the starter when the engine is off: it cranks after a pause (the clutch must be open or the box in neutral in the H-pattern)
        if !self.running {
            self.stalled_for += dt;
            let free = auto || seq || self.gear == 0 || self.clutch_pedal > 0.6;
            if self.starting <= 0.0 && self.stalled_for > 0.8 && free { self.starting = 1.0; }
        }
        let opts = self.opts;
        // the limiter: fuel cut above the limit until the revs fall a little
        if !opts.limiter { self.limiter = false; } else if rpm0 > LIMITER_RPM { self.limiter = true; } else if rpm0 < LIMITER_RPM - 150.0 { self.limiter = false; }
        let cut = self.limiter || cut_shift || !self.running;
        // traction control: a wheel spinning past 1.3 times the slip of the peak cuts the engine's torque, to 15 % by 2.3 times
        let tc_k = if opts.tc && i.tc_slip > 1.3 { 1.0 - 0.85 * ((i.tc_slip - 1.3) / 1.0).clamp(0.0, 1.0) } else { 1.0 };
        let sync = (auto || seq) && self.shift > 0.0 && self.running && self.gear != 0;
        self.fr = Frame { dt, speed: i.speed, slip_w: 0.0, auto, radius: i.wheel_r.max(0.05), g, rd: reverse_dir, cap, thr, cut, tc_k, sync, n: 0, t_c: 0.0, t_e: 0.0 };
    }

    /// The engine's torque for one substep of `h` s (the boost moves on): the map at the revs and the throttle, the boost, the tuning, the starter, the traction control's cut.
    fn engine_now(&mut self, h: f32) -> f32 {
        let (f, opts) = (self.fr, self.opts);
        let rpm = self.omega * 60.0 / std::f32::consts::TAU;
        // the boost builds with a lag when the throttle is open at the revs where the turbine has the exhaust to spin it, and falls fast
        let want = if f.thr > 0.5 && rpm > 2200.0 { 1.0 } else { 0.0 };
        self.boost += (want - self.boost) * (h / if want > self.boost { 0.45 } else { 0.25 }).min(1.0);
        let mut t_e = if f.cut { engine_torque(rpm, 0.0) } else { engine_torque(rpm, f.thr) };
        if !f.cut && t_e > 0.0 { t_e *= (0.62 + 0.38 * self.boost * opts.turbo) * opts.engine; }
        else if !f.cut { t_e *= 1.0 + (opts.engine - 1.0) * 0.3; }   // the engine braking changes a little with the engine
        // no limiter: the power gives out by itself above 7500 rpm and the friction wins at about 9000
        if !opts.limiter && !f.cut && rpm > 7500.0 { t_e = t_e.min(engine_torque(7500.0, f.thr) * (1.0 - (rpm - 7500.0) / 1500.0) * opts.engine).max(-60.0); }
        if !self.running { t_e = -engine_torque(rpm.max(1000.0), 0.0).abs() * (rpm / 1000.0).clamp(0.0, 1.0); }
        if self.starting > 0.0 && rpm < 650.0 { t_e += 140.0; }   // the starter turns it until it fires (about 450 rpm)
        if t_e > 0.0 { t_e *= f.tc_k * self.coolant.power() * self.power_k; }
        t_e
    }

    /// A shift of the automatic or the sequential meets the new gear with matching revs: the engine falls or rises to the wheels' (the clutch-to-clutch change), so the clutch closes without a jerk.
    fn sync_torque(&self, w_in: f32) -> f32 {
        if self.fr.sync { (GRIP_K * 1.5 * (w_in.max(IDLE_RPM * std::f32::consts::TAU / 60.0) - self.omega)).clamp(-220.0, 220.0) } else { 0.0 }
    }

    /// One substep with the wheels' speed given (`w_in`, engine side, rad/s): the wheels are taken as infinitely heavy.
    fn sub_kin(&mut self, h: f32, w_in: f32) {
        let f = self.fr;
        let t_e = self.engine_now(h);
        let t_c = (GRIP_K * (self.omega - w_in)).clamp(-f.cap, f.cap);
        self.fr.slip_w += (t_c * (self.omega - w_in)).abs();
        let t_s = self.sync_torque(w_in);
        self.omega = (self.omega + h / INERTIA * (t_e - t_c + t_s)).max(0.0);
        self.fr.n += 1; self.fr.t_c += t_c; self.fr.t_e += t_e;
    }

    /// One substep against the wheels' own inertia: `w_mean` the mean spin of the wheels (rad/s), `i_wheels` their inertia together (kg m2), `tyre` the torque the tyres put against their spin (N m, summed).
    /// The clutch is solved implicitly with the wheels' inertia (the relative speed of engine and driveline has an exact update), so it stays stable however the wheels spin.
    /// Returns the torque to put on the four wheels together (N m, signed).
    pub fn sub_coupled(&mut self, h: f32, w_mean: f32, i_wheels: f32, tyre: f32) -> f32 {
        let f = self.fr;
        let gk = f.g * f.rd;                                   // the wheels' spin times this is the driveline's speed on the engine's side
        let w_in = (w_mean * gk).max(-1e4);
        let t_e = self.engine_now(h);
        let t_s = self.sync_torque(w_in);
        let t_c = if f.cap <= 0.0 || gk == 0.0 { 0.0 } else {
            let iw = i_wheels.max(0.1);
            let a = t_e / INERTIA + gk * tyre / iw;            // what accelerates the engine's lead over the driveline, apart from the clutch
            let b = 1.0 / INERTIA + ETA * gk * gk / iw;        // and how strongly the clutch torque pulls the two together
            let d = (self.omega - w_in + h * a) / (1.0 + h * GRIP_K * b);
            (GRIP_K * d).clamp(-f.cap, f.cap)
        };
        self.fr.slip_w += (t_c * (self.omega - w_in)).abs();
        self.omega = (self.omega + h / INERTIA * (t_e - t_c + t_s)).max(0.0);
        self.fr.n += 1; self.fr.t_c += t_c; self.fr.t_e += t_e;
        t_c * gk * ETA
    }

    /// The end of a frame: the stall and the starter, and the numbers the others read.
    pub fn finish(&mut self) -> DriveOut {
        let f = self.fr;
        let n = f.n.max(1) as f32;
        let (t_c, t_e) = (f.t_c / n, f.t_e / n);
        // the heat: the clutch's slip (not the automatic's converter) and the coolant's share of the shaft's power
        if !f.auto { self.clutch.step(f.dt, f.slip_w / n, self.air_c); } else { self.clutch.step(f.dt, 0.0, self.air_c); }
        self.coolant.step(f.dt, (t_e * self.omega).max(0.0), f.speed, self.air_c);
        let rpm = self.rpm();
        if self.running && rpm < 280.0 && !(self.starting > 0.0) { self.running = false; self.stalled_for = 0.0; }
        if self.starting > 0.0 && rpm > 450.0 { self.starting = 0.0; self.running = true; self.stalled_for = 0.0; self.idle_i = 0.12; }
        let wheel_torque = t_c * f.g * f.rd * ETA;
        self.last = DriveOut { force: wheel_torque / f.radius, wheel_torque, rpm, gear: self.gear, grip: (f.cap / CLUTCH_MAX).clamp(0.0, 1.0), torque: t_e, throttle: f.thr, boost_bar: self.boost * 0.55 * self.opts.turbo, cut: f.cut, stalled: !self.running };
        self.last
    }

    /// A whole frame with the car's speed given as the wheels' (the gearbox tests and any car without wheels use this).
    pub fn step(&mut self, dt: f32, i: &DriveIn) -> DriveOut {
        let dt = dt.clamp(0.0, 0.05);
        self.begin(dt, i);
        let w_in = (i.speed / i.wheel_r.max(0.05) * self.fr.g * self.fr.rd).max(-1e4);
        for _ in 0..8 { self.sub_kin(dt / 8.0, w_in); }
        self.finish()
    }
}

#[cfg(test)]
#[path = "drivetrain_tests.rs"]
mod tests;
