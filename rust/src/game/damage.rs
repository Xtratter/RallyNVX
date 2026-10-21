//! What a crash does to the car: the body (it drags more), the four suspension corners (a bent wheel toes out, a spring goes soft) and the engine (it loses power). A hit is a sudden change
//! of the car's velocity (m/s); under 2.5 m/s nothing is hurt, 11.5 m/s hurts all it can. A hard landing hurts the springs. All of it is 0 (new) to 1 (wrecked), and a repair makes it new.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Damage { pub body: f32, pub susp: [f32; 4], pub engine: f32 }

impl Damage {
    /// A collision that changed the velocity by `dv` m/s, at the front or the rear and on the left or the right of the car.
    pub fn hit(&mut self, dv: f32, front: bool, left: bool) {
        if !dv.is_finite() { return; }
        let s = ((dv - 2.5) / 9.0).clamp(0.0, 1.2);
        if s <= 0.0 { return; }
        self.body += 0.8 * s;
        self.engine += (if front { 0.5 } else { 0.1 }) * s;
        let base = if front { 0 } else { 2 };
        let (main, other) = if left { (base + 1, base) } else { (base, base + 1) };   // the left wheels are 1 and 3, the right ones 0 and 2
        self.susp[main] += 0.9 * s; self.susp[other] += 0.35 * s;
        self.clamp();
    }
    /// A touchdown at `vy` m/s down (positive or negative): over 7 m/s it hurts the four springs.
    pub fn landing(&mut self, vy: f32) {
        if !vy.is_finite() { return; }
        let s = ((vy.abs() - 7.0) / 12.0).clamp(0.0, 1.0);
        if s <= 0.0 { return; }
        for k in self.susp.iter_mut() { *k += 0.5 * s; }
        self.body += 0.1 * s;
        self.clamp();
    }
    fn clamp(&mut self) { self.body = self.body.clamp(0.0, 1.0); self.engine = self.engine.clamp(0.0, 1.0); for k in self.susp.iter_mut() { *k = k.clamp(0.0, 1.0); } }
    pub fn repair(&mut self) { *self = Damage::default(); }
    /// The mean of everything, 0..1.
    pub fn total(&self) -> f32 { (self.body + self.engine + self.susp.iter().sum::<f32>()) / 6.0 }
    /// The air drag times this (a crumpled body, up to a quarter more).
    pub fn drag_factor(&self) -> f32 { 1.0 + 0.25 * self.body }
    /// The share of the engine's power left (0.6 at the worst).
    pub fn power(&self) -> f32 { 1.0 - 0.4 * self.engine }
    /// The spring's stiffness of wheel `w` (FL, FR, RL, RR) times this (0.7 at the worst).
    pub fn spring(&self, w: usize) -> f32 { 1.0 - 0.3 * self.susp[w.min(3)] }
    /// How far a bent wheel points off the car's heading (radians, positive to the right): the wheels toe out (the right wheels 0 and 2 to the right, the left ones 1 and 3 to the left), 3 degrees at the worst.
    pub fn heading(&self, w: usize) -> f32 { let w = w.min(3); (if w % 2 == 0 { 1.0 } else { -1.0 }) * (3.0 * self.susp[w]).to_radians() }
}

#[cfg(test)]
#[path = "damage_tests.rs"]
mod tests;
