//! The fuel: a tank that the engine empties by what it burns (the idle's share and 0.36 kg for every kWh of shaft work), a fuel cut on an overrun, and the mass of what is in it (a full
//! tank of 70 l weighs 52 kg, so the car is lighter as the stage goes).

/// The tank (litres), the fuel's density (kg/l) and the tank's level at which the car's listed mass is true.
pub const TANK_L: f32 = 70.0;
pub const KG_PER_L: f32 = 0.745;
const REF_L: f32 = 35.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fuel { pub litres: f32 }

impl Fuel {
    pub fn with(litres: f32) -> Fuel { Fuel { litres: if litres.is_finite() { litres.clamp(0.0, TANK_L) } else { 0.0 } } }
    pub fn empty(&self) -> bool { self.litres <= 0.0 }
    /// The mass of the fuel over what the car's listed mass has (kg).
    pub fn mass_delta(&self) -> f32 { (self.litres - REF_L) * KG_PER_L }
    /// The flow (litres per second) at the shaft power `shaft_w` and the revs; zero on an overrun (the throttle shut, the engine driven by the wheels) over 1500 rpm.
    pub fn flow(shaft_w: f32, rpm: f32, overrun: bool) -> f32 {
        let rpm = if rpm.is_finite() { rpm.clamp(0.0, 12000.0) } else { 0.0 };
        let p = if shaft_w.is_finite() { shaft_w } else { 0.0 };
        if overrun && p <= 0.0 && rpm > 1500.0 { return 0.0; }
        let idle = 0.4e-3 * (rpm / 950.0).clamp(0.0, 6.0);       // kg/s: 0.4 g/s at the idle
        let load = 1.0e-7 * p.clamp(0.0, 4.0e5);                  // kg/s: 360 g per kWh
        (idle + load) / KG_PER_L
    }
    pub fn burn(&mut self, dt: f32, shaft_w: f32, rpm: f32, overrun: bool) {
        if !dt.is_finite() || dt <= 0.0 { return; }
        self.litres = (self.litres - Self::flow(shaft_w, rpm, overrun) * dt.min(0.1)).max(0.0);
    }
}

#[cfg(test)]
#[path = "fuel_tests.rs"]
mod tests;
