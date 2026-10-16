//! A lumped volume of gas (moles, thermal energy, the mixture) and the flow between two of them. Doubles throughout: the pressures span 1e4 to 1e7 Pa in volumes of a few cubic centimetres.

/// The universal gas constant, J/(mol K).
pub const R: f64 = 8.314_462;
/// Degrees of freedom of the gas (diatomic): the thermal energy is `f/2 n R T`.
pub const F_DOF: f64 = 5.0;
/// The heat capacity ratio `1 + 2/f`.
pub const GAMMA: f64 = 1.0 + 2.0 / F_DOF;
/// The molar mass of air, kg/mol.
pub const M_AIR: f64 = 0.028_97;
/// The specific gas constant of air, J/(kg K).
const R_AIR: f64 = 287.05;
/// The atmosphere.
pub const P_ATM: f64 = 101_325.0;
pub const T_ATM: f64 = 298.15;
/// Moles of air per mole of fuel (C8H16, stoichiometric: 12 O2 per fuel, 21 % of the air).
pub const AIR_PER_FUEL: f64 = 12.0 / 0.21;

#[derive(Clone, Copy, Debug)]
pub struct Gas {
    /// Moles of gas, of which `fuel` is fuel vapour and `o2` oxygen (the rest is inert).
    pub n: f64, pub fuel: f64, pub o2: f64,
    /// Volume, m^3.
    pub v: f64,
    /// Thermal energy, J.
    pub e: f64,
}

impl Gas {
    /// A volume `v` at pressure `p` and temperature `t`, filled with air.
    pub fn air(v: f64, p: f64, t: f64) -> Gas {
        let n = p * v / (R * t);
        Gas { n, fuel: 0.0, o2: 0.21 * n, v, e: 0.5 * F_DOF * n * R * t }
    }
    pub fn pressure(&self) -> f64 { 2.0 * self.e / (F_DOF * self.v) }
    pub fn temperature(&self) -> f64 { if self.n > 1e-12 { 2.0 * self.e / (F_DOF * self.n * R) } else { T_ATM } }
    /// The volume changes by `dv` without heat: the gas does its work on the walls (an isentrope, stable for any step).
    pub fn change_volume(&mut self, dv: f64) {
        let v2 = (self.v + dv).max(1e-9);
        self.e *= (self.v / v2).powf(GAMMA - 1.0);
        self.v = v2;
    }
    /// The enthalpy per mole at the gas's temperature, J/mol.
    fn molar_enthalpy(&self) -> f64 { (0.5 * F_DOF + 1.0) * R * self.temperature() }
}

/// The mass flow (kg/s) of an isentropic orifice of effective area `a` (m^2, discharge coefficient included) from a gas at `p0`, `t0` into pressure `p1` (`p1 <= p0`).
pub fn orifice_mass_flow(a: f64, p0: f64, t0: f64, p1: f64) -> f64 {
    if p0 <= p1 || a <= 0.0 || p0 <= 0.0 { return 0.0; }
    let r = p1 / p0;
    let crit = (2.0 / (GAMMA + 1.0)).powf(GAMMA / (GAMMA - 1.0));
    let base = a * p0 / (R_AIR * t0).sqrt();
    if r <= crit { base * GAMMA.sqrt() * (2.0 / (GAMMA + 1.0)).powf((GAMMA + 1.0) / (2.0 * (GAMMA - 1.0))) }
    else { base * (2.0 * GAMMA / (GAMMA - 1.0) * (r.powf(2.0 / GAMMA) - r.powf((GAMMA + 1.0) / GAMMA))).max(0.0).sqrt() }
}

/// Moves gas through an orifice of effective area `a` between `x` and `y` for `dt` seconds, from the higher pressure to the lower; returns the moles moved from `x` to `y`
/// (negative when the flow goes the other way). The flow is capped at 90 % of what equalises the pressures, so a long step never overshoots. The gas carries its enthalpy and its mixture.
pub fn flow(x: &mut Gas, y: &mut Gas, a: f64, dt: f64) -> f64 {
    let (px, py) = (x.pressure(), y.pressure());
    if (px - py).abs() < 1.0 { return 0.0; }
    let forward = px > py;
    let (src, dst) = if forward { (&mut *x, &mut *y) } else { (&mut *y, &mut *x) };
    let (p0, t0, p1) = (src.pressure(), src.temperature(), dst.pressure());
    let mut dn = orifice_mass_flow(a, p0, t0, p1) * dt / M_AIR;
    // the moles that would level the pressures if the temperatures stayed: p0 - p1 = R (t0 / v0 + t1 / v1) dn
    let t1 = dst.temperature();
    let eq = (p0 - p1) / (R * (t0 / src.v + t1 / dst.v));
    dn = dn.min(0.9 * eq).min(0.9 * src.n).max(0.0);
    if dn <= 0.0 { return 0.0; }
    let share = dn / src.n;
    let (dfuel, do2) = (src.fuel * share, src.o2 * share);
    let h = src.molar_enthalpy() * dn;
    src.n -= dn; src.fuel -= dfuel; src.o2 -= do2; src.e = (src.e - h).max(1e-9);
    dst.n += dn; dst.fuel += dfuel; dst.o2 += do2; dst.e += h;
    if forward { dn } else { -dn }
}

#[cfg(test)]
#[path = "gas_tests.rs"]
mod tests;
