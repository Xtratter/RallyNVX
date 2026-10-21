//! The car's drivetrain: the engine's speed, the clutch, the gearbox and the force at the wheels. The engine is a torque map measured from the sound's engine model
//! (`engine_torque.json`: net torque over revs and throttle at the spooled boost, made by `generate_torque_table`), a turbo lag and an idle governor; it turns with its own inertia against
//! the clutch, which carries torque to the wheels through the gear. Two modes: an automatic (the gear and the clutch are decided here) and a manual (the driver shifts the gears and works the clutch).
use std::sync::OnceLock;

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
/// Seconds a shift takes in the automatic and in the sequential manual box.
const SHIFT_AUTO: f32 = 0.18;
const SHIFT_MANUAL: f32 = 0.12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Auto, Manual }

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
#[derive(Clone, Copy, Debug)]
pub struct DriveIn {
    /// The throttle pedal: -1..1 in the automatic (negative asks for the reverse), 0..1 in the manual.
    pub throttle: f32,
    /// The car's speed along its heading (m/s, negative backwards) and the wheels' radius (m).
    pub speed: f32, pub wheel_r: f32,
    pub airborne: bool,
    /// The brake pedal is down (the automatic holds a lower gear for the engine brake).
    pub brake: bool,
}

/// What it gives.
#[derive(Clone, Copy, Debug)]
pub struct DriveOut {
    /// The force at the ground along the car's heading (N, positive forward), summed over the wheels.
    pub force: f32,
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
pub struct DriveOpts { pub engine: f32, pub turbo: f32, pub gearing: f32, pub flat_shift: bool, pub blip: bool, pub limiter: bool }
impl Default for DriveOpts { fn default() -> DriveOpts { DriveOpts { engine: 1.0, turbo: 1.0, gearing: 1.0, flat_shift: true, blip: true, limiter: true } } }

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
    load_s: f32, since_shift: f32,
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
            shift: 0.0, shift_up: true, limiter: false, boost: 0.0, idle_i: 0.1, load_s: 0.0, since_shift: 9.0,
            last: DriveOut { force: 0.0, rpm, gear: 1, grip: 0.0, torque: 0.0, throttle: 0.0, boost_bar: 0.0, cut: false, stalled: false } }
    }

    pub fn rpm(&self) -> f32 { self.omega * 60.0 / std::f32::consts::TAU }
    pub fn running(&self) -> bool { self.running }

    /// A new start (a reset, a new level): first gear, the engine at the idle.
    pub fn reset(&mut self) { let mode = self.mode; let sel = self.reverse_sel; let opts = self.opts; let ar = self.auto_reverse; *self = Drivetrain::new(); self.reverse_sel = sel; self.opts = opts; self.auto_reverse = ar; self.mode = mode; if mode == Mode::Manual { self.gear = 0; } }

    pub fn set_mode(&mut self, m: Mode) { if m != self.mode { self.mode = m; if m == Mode::Auto && self.gear == 0 { self.gear = 1; } } }

    /// The driver shifts: +1 up, -1 down (in the manual; the sequence is R, N, 1 .. 6).
    pub fn shift(&mut self, dir: i32) {
        if self.mode != Mode::Manual || self.shift > 0.0 || dir == 0 { return; }
        let g = (self.gear + dir.signum()).clamp(-1, 6);
        if g != self.gear { self.gear = g; self.shift = SHIFT_MANUAL; self.shift_up = dir > 0; }
    }

    pub fn set_clutch(&mut self, pedal: f32) { self.clutch_pedal = if pedal.is_finite() { pedal.clamp(0.0, 1.0) } else { 0.0 }; }

    /// The automatic's choice of the gear.
    fn auto_gear(&mut self, i: &DriveIn, thr: f32, rpm: f32, dt: f32) {
        // R is chosen by hand (the autopilot alone asks for the reverse with a negative throttle); a car that merely rolls back in D takes the first gear as soon as the gas is pressed
        let holding = self.gear == -1 && i.speed < -0.5 && thr < 0.05;
        if self.reverse_sel || (self.auto_reverse && ((i.throttle < -0.1 && i.speed < 1.0) || holding)) { if self.gear != -1 { self.gear = -1; } return; }
        if self.gear <= 0 { self.gear = 1; self.since_shift = 0.0; return; }
        // the schedule follows the pedal the box remembers: a press counts at once, a lift fades over about a second, so a lift in a corner keeps the gear and a flickering pedal does not hunt
        self.load_s = if thr > self.load_s { thr } else { self.load_s + (thr - self.load_s) * (dt / 1.2).min(1.0) };
        self.since_shift += dt;
        if self.shift > 0.0 { return; }
        let (ls, gearing) = (self.load_s, self.opts.gearing);
        let at = |g: i32| rpm_in_gear(i.speed, i.wheel_r, g) * gearing;   // the revs the wheels would give the engine in gear g
        let braking = i.brake && i.speed > 4.0 && thr < 0.1;
        // the gas floored at low revs asks for a lower gear (the kick-down), which may come sooner than a normal shift
        let kick = thr > 0.85 && self.gear > 1 && rpm < 3300.0 && at(self.gear - 1) < 6600.0;
        if self.since_shift < if kick { 0.35 } else { 0.9 } { return; }
        let floor = 1900.0 + 1700.0 * ls;
        let up_at = 3800.0 + 3500.0 * ls;
        let down_at = if braking { floor.max(4300.0) } else { floor };   // braking: down through the gears for the engine brake and the power at the corner's exit
        let lifted = thr < 0.05 && i.speed > 4.0;   // coasting: the box keeps the gear (the engine brake), it does not climb on a lift
        if !braking && !lifted && self.gear < 6 && rpm > up_at && at(self.gear + 1) > floor + 500.0 {
            // up only if the next gear still has the revs to pull (else it would go back down at once)
            self.gear += 1; self.shift = SHIFT_AUTO; self.shift_up = true; self.since_shift = 0.0;
        } else if self.gear > 1 && (kick || rpm < down_at) && at(self.gear - 1) < if braking { 5800.0 } else { 6600.0 } {
            self.gear -= 1; self.shift = SHIFT_AUTO; self.shift_up = false; self.since_shift = 0.0;
        }
    }

    pub fn step(&mut self, dt: f32, i: &DriveIn) -> DriveOut {
        let dt = dt.clamp(0.0, 0.05);
        if self.shift > 0.0 { self.shift = (self.shift - dt).max(0.0); }
        let auto = self.mode == Mode::Auto;
        let rpm0 = self.rpm();
        let gas = i.throttle.max(0.0);
        if auto { self.auto_gear(i, gas, rpm0, dt); }
        // the pedal the engine feels: in the reverse of the automatic the negative throttle is the gas
        let pedal = if auto { if self.gear < 0 { if self.reverse_sel { gas.max((-i.throttle).max(0.0)) } else { (-i.throttle).max(0.0) } } else { gas } } else { i.throttle.clamp(0.0, 1.0) };
        // the throttle the engine gets: the pedal, the idle governor holding the idle, and the blip of the automatic's down-shift
        let mut thr = pedal;
        if self.running {
            let err = (IDLE_RPM - rpm0) / IDLE_RPM;
            self.idle_i = (self.idle_i + err * dt * 0.6).clamp(0.0, 0.4);
            thr = thr.max((self.idle_i + 0.5 * err).clamp(0.0, 0.4));
        }
        if auto && self.shift > 0.0 && !self.shift_up && self.opts.blip { thr = thr.max(0.55); }
        let reverse_dir = if self.gear < 0 { -1.0 } else { 1.0 };
        let g = ratio(self.gear) * self.opts.gearing;
        let cut_shift = self.opts.flat_shift && self.shift > 0.0 && self.shift_up && pedal > 0.6;
        // the clutch's capacity (N m): the automatic's rises with the revs (it bites like a centrifugal clutch and slips at the launch), the manual's follows the pedal; none in neutral or in a shift
        let cap = if g == 0.0 || self.shift > 0.0 || i.airborne { 0.0 }
            // the automatic: the launch curve bites with the revs; on the move the lock-up closes (no slip, the engine and the wheels turn together)
            else if auto { (10.0 + (CLUTCH_MAX - 10.0) * smooth(1300.0, 3000.0, rpm0)).max(CLUTCH_MAX * smooth(2.0, 5.0, i.speed.abs())).min(CLUTCH_MAX) }
            else { CLUTCH_MAX * smooth(0.85, 0.30, self.clutch_pedal) };
        // the starter when the engine is off: it cranks after a pause (the clutch must be open or the box in neutral in the manual)
        if !self.running {
            self.stalled_for += dt;
            let free = auto || self.gear == 0 || self.clutch_pedal > 0.6;
            if self.starting <= 0.0 && self.stalled_for > 0.8 && free { self.starting = 1.0; }
        }
        let w_in = (i.speed / i.wheel_r.max(0.05) * g * reverse_dir).max(-1e4);
        let opts = self.opts;
        let (n, h) = (8, dt / 8.0);
        let w_idle = IDLE_RPM * std::f32::consts::TAU / 60.0;
        let (mut f_sum, mut tq_sum, mut tc_last) = (0.0f32, 0.0f32, 0.0f32);
        // the limiter: fuel cut above the limit until the revs fall a little
        if !opts.limiter { self.limiter = false; } else if rpm0 > LIMITER_RPM { self.limiter = true; } else if rpm0 < LIMITER_RPM - 150.0 { self.limiter = false; }
        let cut = self.limiter || cut_shift || !self.running;
        for _ in 0..n {
            let rpm = self.omega * 60.0 / std::f32::consts::TAU;
            // the boost: it builds with a lag when the throttle is open at the revs where the turbine has the exhaust to spin it, and falls fast
            let want = if thr > 0.5 && rpm > 2200.0 { 1.0 } else { 0.0 };
            self.boost += (want - self.boost) * (h / if want > self.boost { 0.45 } else { 0.25 }).min(1.0);
            let mut t_e = if cut { engine_torque(rpm, 0.0) } else { engine_torque(rpm, thr) };
            if !cut && t_e > 0.0 { t_e *= (0.62 + 0.38 * self.boost * opts.turbo) * opts.engine; }
            else if !cut { t_e *= 1.0 + (opts.engine - 1.0) * 0.3; }   // the engine braking changes a little with the engine
            // no limiter: the power gives out by itself above 7500 rpm and the friction wins at about 9000
            if !opts.limiter && !cut && rpm > 7500.0 { t_e = t_e.min(engine_torque(7500.0, thr) * (1.0 - (rpm - 7500.0) / 1500.0) * opts.engine).max(-60.0); }
            if !self.running { t_e = -engine_torque(rpm.max(1000.0), 0.0).abs() * (rpm / 1000.0).clamp(0.0, 1.0); }
            if self.starting > 0.0 && rpm < 650.0 { t_e += 140.0; }   // the starter turns it until it fires (about 450 rpm)
            let t_c = (GRIP_K * (self.omega - w_in)).clamp(-cap, cap);
            // a shift of the automatic matches the engine's revs to the new gear (the clutch-to-clutch change): the engine falls or rises to meet the wheels, so the clutch closes without a jerk
            let t_s = if auto && self.shift > 0.0 && self.running && self.gear != 0 { (GRIP_K * 1.5 * (w_in.max(w_idle) - self.omega)).clamp(-220.0, 220.0) } else { 0.0 };
            self.omega = (self.omega + h / INERTIA * (t_e - t_c + t_s)).max(0.0);
            tq_sum += t_e; tc_last = t_c;
            f_sum += t_c * g * reverse_dir * ETA / i.wheel_r.max(0.05);
        }
        let rpm = self.rpm();
        if self.running && rpm < 280.0 && !(self.starting > 0.0) { self.running = false; self.stalled_for = 0.0; }
        if self.starting > 0.0 && rpm > 450.0 { self.starting = 0.0; self.running = true; self.stalled_for = 0.0; self.idle_i = 0.12; }
        let _ = tc_last;
        self.last = DriveOut { force: f_sum / n as f32, rpm, gear: self.gear, grip: (cap / CLUTCH_MAX).clamp(0.0, 1.0), torque: tq_sum / n as f32, throttle: thr, boost_bar: self.boost * 0.55 * opts.turbo, cut, stalled: !self.running };
        self.last
    }
}

#[cfg(test)]
#[path = "drivetrain_tests.rs"]
mod tests;
