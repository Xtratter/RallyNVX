//! The gearbox and the clutch of the sound: an automatic that picks the gear from the speed and the throttle, takes a short pause for every shift (the clutch opens, the engine
//! revs free) and slips the clutch at a standstill. It does not drive the car; it tells the engine model which revs the wheels ask for.

/// The ratios of the six gears and the reverse, times the final drive.
const RATIOS: [f32; 6] = [2.92, 2.12, 1.62, 1.31, 1.08, 0.92];
const REVERSE: f32 = 3.2;
const FINAL: f32 = 5.0;
/// The seconds a shift takes with the clutch open.
const SHIFT_TIME: f32 = 0.22;

#[derive(Clone, Copy, Debug)]
pub struct Gearbox { pub gear: i32, shift: f32, pub clutch: f32 }

/// What the box asks of the engine: the revs the wheels give in this gear, and the clutch's grip (0 open .. 1 locked).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drive { pub rpm: f32, pub clutch: f32, pub gear: i32 }

impl Default for Gearbox { fn default() -> Gearbox { Gearbox { gear: 1, shift: 0.0, clutch: 0.0 } } }

fn ratio(gear: i32) -> f32 { if gear < 0 { REVERSE * FINAL } else { RATIOS[(gear.clamp(1, 6) - 1) as usize] * FINAL } }

/// The engine's revs when the wheels (radius `r`, m) roll at `speed` m/s in `gear`.
pub fn rpm_in_gear(speed: f32, r: f32, gear: i32) -> f32 { speed.abs() / r.max(0.05) * ratio(gear) * 60.0 / std::f32::consts::TAU }

impl Gearbox {
    /// `speed` is the signed forward speed (m/s), `throttle` the pedal (-1..1: negative is reverse), `r` the wheel radius, `airborne` the wheels are off the ground.
    pub fn update(&mut self, dt: f32, speed: f32, throttle: f32, r: f32, airborne: bool) -> Drive {
        let dt = dt.clamp(0.0, 0.1);
        let thr = throttle.clamp(0.0, 1.0);
        let reversing = speed < -0.5 || (throttle < -0.1 && speed < 1.0);
        if reversing { self.gear = -1; } else if self.gear < 0 { self.gear = 1; }
        if self.shift > 0.0 { self.shift = (self.shift - dt).max(0.0); }
        if self.gear > 0 && self.shift <= 0.0 {
            let rpm = rpm_in_gear(speed, r, self.gear);
            let up_at = 4600.0 + 2800.0 * thr;                          // a light foot shifts up early, a hard one holds the gear to the top of the revs
            let down_at = 2300.0 + 600.0 * thr;
            if rpm > up_at && self.gear < 6 { self.gear += 1; self.shift = SHIFT_TIME; }
            else if rpm < down_at && self.gear > 1 && rpm_in_gear(speed, r, self.gear - 1) < 6600.0 { self.gear -= 1; self.shift = SHIFT_TIME; }
        }
        let rpm = rpm_in_gear(speed, r, self.gear);
        // the clutch: open while shifting and in the air; slipping at a standstill, in the first metres per second, locked beyond
        let want = if self.shift > 0.0 || airborne { 0.0 }
            else if speed.abs() < 1.0 && thr < 0.05 { 0.0 }
            else { (0.35 + 0.65 * ((speed.abs() - 1.0) / 5.0).clamp(0.0, 1.0)).min(1.0) };
        self.clutch += (want - self.clutch) * (1.0 - (-dt / 0.04).exp());
        Drive { rpm, clutch: self.clutch, gear: self.gear }
    }
}


#[cfg(test)]
#[path = "gearbox_tests.rs"]
mod tests;
