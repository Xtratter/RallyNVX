//! The driver aids that can be switched off, and the tuning of the car's parts (percent multipliers), both set in the game's menu.
use super::vehicle::VehicleSpec;

/// The systems: all on by default (the way the car has driven so far).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Assists {
    /// Anti-lag: the fire in the exhaust keeps the turbo spinning on a lift.
    pub als: bool,
    /// ABS: the brakes keep every wheel at the peak of its grip; off, a wheel that is braked past the peak locks and slides.
    pub abs: bool,
    /// Traction control: the same for the drive force; off, a wheel driven past the peak spins.
    pub tc: bool,
    /// The automatic's throttle blip on a downshift.
    pub blip: bool,
    /// The flat shift: the spark is cut for a moment on a hard upshift.
    pub flat_shift: bool,
    /// The rev limiter (fuel cut at 7800 rpm); off, the revs go on until the engine's own power gives out.
    pub limiter: bool,
    /// The blow-off valve; off, the pressure goes back through the compressor (flutter).
    pub bov: bool,
}

impl Default for Assists { fn default() -> Assists { Assists::from_mask(0x7F) } }

impl Assists {
    /// The bits, in the order of the fields: als, abs, tc, blip, flat_shift, limiter, bov.
    pub fn from_mask(m: u32) -> Assists { let b = |k: u32| m & (1 << k) != 0; Assists { als: b(0), abs: b(1), tc: b(2), blip: b(3), flat_shift: b(4), limiter: b(5), bov: b(6) } }
    pub fn mask(&self) -> u32 { [self.als, self.abs, self.tc, self.blip, self.flat_shift, self.limiter, self.bov].iter().enumerate().map(|(k, &v)| (v as u32) << k).sum() }
}

/// The power of every part of the car as a multiplier (1 = as the garage built it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    /// The engine's torque, the turbo's boost, the brakes' force, the tyres' grip, the springs and dampers, the downforce, the gearing (the final drive: more is shorter, quicker, slower at the top).
    pub engine: f32, pub turbo: f32, pub brakes: f32, pub grip: f32, pub suspension: f32, pub downforce: f32, pub gearing: f32,
    /// The lock of the front, the rear and the centre differential, and the centre split (1.4 is 70 % to the front, 0.6 is 30 %), as multipliers of what the car was built with.
    pub diff_front: f32, pub diff_rear: f32, pub diff_centre: f32, pub split: f32,
}
pub const TUNING_PARTS: usize = 11;
impl Default for Tuning { fn default() -> Tuning { Tuning { engine: 1.0, turbo: 1.0, brakes: 1.0, grip: 1.0, suspension: 1.0, downforce: 1.0, gearing: 1.0, diff_front: 1.0, diff_rear: 1.0, diff_centre: 1.0, split: 1.0 } } }

impl Tuning {
    /// The allowed range of each part (multipliers).
    pub fn range(part: usize) -> (f32, f32) { match part { 0 => (0.3, 2.0), 1 => (0.0, 2.0), 2 => (0.3, 2.5), 3 => (0.6, 1.5), 4 => (0.5, 1.8), 5 => (0.0, 3.0), 7..=9 => (0.0, 2.0), 10 => (0.6, 1.4), _ => (0.7, 1.3) } }
    /// Sets part `i` (0 engine, 1 turbo, 2 brakes, 3 grip, 4 suspension, 5 downforce, 6 gearing, 7 front diff, 8 rear diff, 9 centre diff, 10 centre split) clamped to its range; garbage is ignored.
    pub fn set(&mut self, i: usize, v: f32) {
        if !v.is_finite() || i >= TUNING_PARTS { return; }
        let (lo, hi) = Tuning::range(i); let v = v.clamp(lo, hi);
        match i { 0 => self.engine = v, 1 => self.turbo = v, 2 => self.brakes = v, 3 => self.grip = v, 4 => self.suspension = v, 5 => self.downforce = v, 6 => self.gearing = v, 7 => self.diff_front = v, 8 => self.diff_rear = v, 9 => self.diff_centre = v, _ => self.split = v }
    }
    pub fn get(&self, i: usize) -> f32 { match i { 0 => self.engine, 1 => self.turbo, 2 => self.brakes, 3 => self.grip, 4 => self.suspension, 5 => self.downforce, 6 => self.gearing, 7 => self.diff_front, 8 => self.diff_rear, 9 => self.diff_centre, _ => self.split } }
    /// The differentials' part of the tuning.
    pub fn diffs(&self) -> super::driveline::DiffTuning { super::driveline::DiffTuning { front: self.diff_front, rear: self.diff_rear, centre: self.diff_centre, split: self.split } }
    /// The car with its brakes, tyres, springs and downforce tuned (the engine, the turbo and the gearing act in the drivetrain).
    pub fn apply(&self, spec: &VehicleSpec) -> VehicleSpec {
        let mut s = *spec;
        s.brake_force *= self.brakes;
        s.mu *= self.grip; s.lateral_k *= self.grip;
        s.spring *= self.suspension; s.damper *= self.suspension;
        s.aero_base.cla_front *= self.downforce; s.aero_base.cla_rear *= self.downforce;
        s
    }
}

#[cfg(test)]
#[path = "assists_tests.rs"]
mod tests;
