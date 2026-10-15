//! Garage rules: seven upgrade categories (levels 0..3): four change the car's physics numbers, three are aero body parts
//! (splitter, wing, underbody); and the wing angle setting. Prices.
use super::vehicle::VehicleSpec;

pub const MAX_LEVEL: u8 = 3;
/// Levels of a car: the seven upgrades, then the wing angle (0 low drag, 1 mid, 2 high downforce) and, last, the wheel set (0 tarmac, 1 gravel, 2 snow).
pub const NLEVELS: usize = 9;
pub const MAX_WING_ANGLE: u8 = 2;
pub const MAX_WHEEL_SET: u8 = 2;
/// A new car: no upgrades, the wing angle in the middle, the tarmac wheels.
pub fn default_levels() -> [u8; NLEVELS] { [0, 0, 0, 0, 0, 0, 0, 1, 0] }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upgrade { Engine, Tyres, Suspension, Brakes, Splitter, Wing, Underbody }

impl Upgrade {
    pub const ALL: [Upgrade; 7] = [Upgrade::Engine, Upgrade::Tyres, Upgrade::Suspension, Upgrade::Brakes, Upgrade::Splitter, Upgrade::Wing, Upgrade::Underbody];
    pub fn index(self) -> usize { match self { Upgrade::Engine => 0, Upgrade::Tyres => 1, Upgrade::Suspension => 2, Upgrade::Brakes => 3, Upgrade::Splitter => 4, Upgrade::Wing => 5, Upgrade::Underbody => 6 } }
    pub fn from_index(i: usize) -> Option<Upgrade> { Upgrade::ALL.get(i).copied() }
}

const ENGINE_FORCE: [f32; 4] = [1.0, 1.08, 1.16, 1.25];
const MAX_SPEED: [f32; 4] = [1.0, 1.03, 1.06, 1.09];
const GRIP: [f32; 4] = [1.0, 1.05, 1.10, 1.15];
const SUSPENSION: [f32; 4] = [1.0, 1.1, 1.2, 1.3];
const BRAKES: [f32; 4] = [1.0, 1.2, 1.4, 1.6];

/// The car with its upgrades applied; levels above the cap count as the cap. Geometry never changes.
pub fn apply(spec: &VehicleSpec, levels: [u8; NLEVELS]) -> VehicleSpec {
    let l = |i: usize| levels[i].min(MAX_LEVEL) as usize;
    let mut s = *spec;
    s.engine_force *= ENGINE_FORCE[l(0)];
    s.max_speed *= MAX_SPEED[l(0)];
    s.mu *= GRIP[l(1)];
    s.spring *= SUSPENSION[l(2)];
    s.damper *= SUSPENSION[l(2)];
    s.brake_force *= BRAKES[l(3)];
    s.kit = super::aero::AeroKit { splitter: l(4) as u8, wing: l(5) as u8, underbody: l(6) as u8, wing_angle: levels[7].min(MAX_WING_ANGLE) };
    s.wheel_set = levels[8].min(MAX_WHEEL_SET);   // the look only for now: the renderer picks the wheel mesh by it
    s
}

/// The price of the next level for a category currently at `current` (1x, 2x, 4x the car's base), `None` at the cap.
pub fn upgrade_price(spec: &VehicleSpec, current: u8) -> Option<u32> {
    if current >= MAX_LEVEL { return None; }
    Some(spec.upgrade_cost.saturating_mul([1, 2, 4][current as usize]))
}

/// What the player owns: the selected car id and the upgrade levels of every owned car.
#[derive(Clone, Debug, PartialEq)]
pub struct Garage { pub selected: String, pub cars: Vec<(String, [u8; NLEVELS])> }

impl Garage {
    /// The cars that cost nothing (the off-roader), the first of them selected.
    pub fn new() -> Garage {
        let free: Vec<(String, [u8; NLEVELS])> = VehicleSpec::all().into_iter().filter(|(_, s)| s.price == 0).map(|(id, _)| (id, default_levels())).collect();
        let selected = free.first().map_or_else(|| "offroader".to_string(), |c| c.0.clone());
        Garage { selected, cars: free }
    }
    pub fn owns(&self, id: &str) -> bool { self.cars.iter().any(|(i, _)| i == id) }
    pub fn levels(&self, id: &str) -> [u8; NLEVELS] { self.cars.iter().find(|(i, _)| i == id).map_or_else(default_levels, |c| c.1) }
}

impl Default for Garage { fn default() -> Self { Garage::new() } }

#[cfg(test)]
#[path = "garage_tests.rs"]
mod tests;
