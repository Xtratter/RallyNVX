//! What heats up in the drivetrain: the clutch that slips (a launch, a feathered start on a hill) and the coolant of an engine that is pushed. The clutch loses its grip as it burns; the
//! engine loses power as the coolant boils. Both are lumped single temperatures with a heat capacity, a heat input and a cooling that grows with the car's speed.

fn smooth(a: f32, b: f32, x: f32) -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }
fn finite_or(v: f32, d: f32) -> f32 { if v.is_finite() { v } else { d } }

/// The clutch's temperature (degrees C). Heat capacity 350 J/K; cooling 6 W/K and more the hotter it is.
#[derive(Clone, Copy, Debug)]
pub struct Clutch { pub temp: f32 }
impl Clutch {
    pub fn new(air_c: f32) -> Clutch { Clutch { temp: finite_or(air_c, 20.0).clamp(-40.0, 50.0) } }
    /// `slip_w` the power lost in the slip (W), the torque it carries times the speed difference of its two sides.
    pub fn step(&mut self, h: f32, slip_w: f32, air_c: f32) {
        if !h.is_finite() || h <= 0.0 { return; }
        let h = h.min(0.1);
        let p = finite_or(slip_w, 0.0).clamp(0.0, 3.0e5);
        let air = finite_or(air_c, 20.0).clamp(-40.0, 50.0);
        let cool = (6.0 + 0.04 * (self.temp - air).max(0.0)) * (self.temp - air);
        self.temp = (self.temp + h * (p - cool) / 350.0).clamp(-60.0, 1500.0);
    }
    /// How much of its torque the clutch still carries (1 up to 250 degrees C, falling to 0.4 by 450: a burnt clutch).
    pub fn fade(&self) -> f32 { 1.0 - 0.6 * smooth(250.0, 450.0, self.temp) }
}

/// The coolant's temperature (degrees C). Heat capacity 60000 J/K (the water and the engine's iron); the heat is 8 kW at idle plus 0.9 of the shaft power; the thermostat opens the
/// radiator between 84 and 94 degrees, which the air cools better the faster the car goes.
#[derive(Clone, Copy, Debug)]
pub struct Coolant { pub temp: f32 }
impl Coolant {
    pub fn new(start_c: f32) -> Coolant { Coolant { temp: finite_or(start_c, 70.0).clamp(-40.0, 150.0) } }
    pub fn step(&mut self, h: f32, shaft_w: f32, speed: f32, air_c: f32) {
        if !h.is_finite() || h <= 0.0 { return; }
        let h = h.min(0.1);
        let q = 8000.0 + 0.9 * finite_or(shaft_w, 0.0).clamp(0.0, 4.0e5);
        let v = finite_or(speed, 0.0).abs().min(120.0);
        let air = finite_or(air_c, 20.0).clamp(-40.0, 50.0);
        let open = smooth(84.0, 94.0, self.temp);
        let k = 120.0 + open * (780.0 + 90.0 * v);
        self.temp = (self.temp + h * (q - k * (self.temp - air)) / 60000.0).clamp(-60.0, 300.0);
    }
    /// The share of the power the engine still has (1 up to 105 degrees C, 0.5 at 130 and over).
    pub fn power(&self) -> f32 { 1.0 - 0.5 * smooth(105.0, 130.0, self.temp) }
}

#[cfg(test)]
#[path = "heat_tests.rs"]
mod tests;
