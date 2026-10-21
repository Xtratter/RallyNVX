//! The four-wheel driveline: the engine's torque at the wheels goes through the centre differential to the front and the rear axle, and through the axle's differential to its two wheels.
//! The engine sees the wheels through the same chain, each differential weighting its two outputs by its share, so the power in equals the power out.
use super::diff::{input_speed, split, Diff, Kind};

/// What the active centre differential reads from the driver and the car.
#[derive(Clone, Copy, Debug)]
pub struct Ctl { pub throttle: f32, pub brake: bool, pub steer: f32, pub speed: f32 }

/// The driver's tuning of the differentials as multipliers (1 = as built): the lock of each differential and the centre split (1.4 = 70 % to the front, 0.6 = 30 %).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiffTuning { pub front: f32, pub rear: f32, pub centre: f32, pub split: f32 }
impl Default for DiffTuning { fn default() -> DiffTuning { DiffTuning { front: 1.0, rear: 1.0, centre: 1.0, split: 1.0 } } }

#[derive(Clone, Copy, Debug)]
pub struct Driveline { pub front: Diff, pub rear: Diff, pub centre: Diff, pub active: bool }

/// The active centre differential's lock as a multiplier of its built value: it closes on the power and at speed (traction, stability), opens for the turn-in (the nose turns) and under the brake.
/// Every term is continuous in its input.
pub fn active_lock(c: &Ctl) -> f32 {
    let sm = |a: f32, b: f32, x: f32| { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) };
    let thr = if c.throttle.is_finite() { c.throttle.clamp(0.0, 1.0) } else { 0.0 };
    let steer = if c.steer.is_finite() { c.steer.abs().clamp(0.0, 1.0) } else { 0.0 };
    let speed = if c.speed.is_finite() { c.speed.abs() } else { 0.0 };
    let turn_in = sm(0.25, 0.7, steer) * (1.0 - sm(0.2, 0.6, thr));   // steering without the power
    let brake = if c.brake { 1.0 } else { 0.0 };
    (0.5 + 0.5 * thr - 0.35 * turn_in - 0.2 * brake + 0.2 * sm(20.0, 45.0, speed)).clamp(0.0, 1.3)
}

impl Driveline {
    /// The Peugeot 206 WRC's: a mechanical limited-slip differential on each axle and an electronically controlled centre differential, a nominal half and half split. The numbers are
    /// approximations (the sources name the layout, not the values).
    pub fn peugeot() -> Driveline {
        Driveline {
            front: Diff { kind: Kind::Lsd, split: 0.5, preload: 30.0, power: 0.25, coast: 0.15, visc: 0.0, cap: 700.0 },
            rear: Diff { kind: Kind::Lsd, split: 0.5, preload: 40.0, power: 0.35, coast: 0.20, visc: 0.0, cap: 900.0 },
            centre: Diff { kind: Kind::Lsd, split: 0.5, preload: 60.0, power: 0.40, coast: 0.25, visc: 0.0, cap: 1200.0 },
            active: true,
        }
    }

    /// The driveline with the driver's tuning applied (any value is made safe: the locks 0..2 times, the split 30 to 70 % to the front).
    pub fn tuned(self, t: &DiffTuning) -> Driveline {
        let f = |v: f32, lo: f32, hi: f32| if v.is_finite() { v.clamp(lo, hi) } else { 1.0 };
        let lock = |d: Diff, k: f32| Diff { preload: d.preload * k, power: d.power * k, coast: d.coast * k, visc: d.visc * k, cap: d.cap * k.max(0.25), ..d };
        let mut c = lock(self.centre, f(t.centre, 0.0, 2.0));
        c.split = (c.split * f(t.split, 0.6, 1.4)).clamp(0.3, 0.7);
        Driveline { front: lock(self.front, f(t.front, 0.0, 2.0)), rear: lock(self.rear, f(t.rear, 0.0, 2.0)), centre: c, active: self.active }
    }

    /// The speed the engine sees (rad/s): the centre differential's input follows the front and the rear axle, each follows its two wheels.
    pub fn input_speed(&self, w: [f32; 4]) -> f32 {
        let (wf, wr) = (input_speed(&self.front, w[0], w[1]), input_speed(&self.rear, w[2], w[3]));
        input_speed(&self.centre, wf, wr)
    }

    /// The torque of each wheel (FL, FR, RL, RR) from the torque `t` the driveline puts on the four wheels together; they add up to `t`.
    pub fn torques(&self, t: f32, w: [f32; 4], c: &Ctl) -> [f32; 4] {
        let (wf, wr) = (input_speed(&self.front, w[0], w[1]), input_speed(&self.rear, w[2], w[3]));
        let mut centre = self.centre;
        if self.active {
            let k = active_lock(c);
            centre = Diff { preload: centre.preload * k, power: centre.power * k, coast: centre.coast * k, visc: centre.visc * k, cap: centre.cap * k.max(0.25), ..centre };
        }
        let (tf, tr) = split(&centre, t, wf, wr);
        let (a, b) = split(&self.front, tf, w[0], w[1]);
        let (cc, d) = split(&self.rear, tr, w[2], w[3]);
        [a, b, cc, d]
    }
}

#[cfg(test)]
#[path = "driveline_tests.rs"]
mod tests;
