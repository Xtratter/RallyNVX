//! A four-stroke engine simulated at the level of the gas: each cylinder is a volume whose size follows the crank, the valves open by crank angle and let the charge in and the
//! burnt gas out through orifices, the fuel burns from the spark by a Wiebe curve, and the pressure pushes the crank. What comes out of the exhaust and the intake is the
//! sound's source (`exhaust_pressure`, `intake_pressure`). The ideas follow engine-sim (MIT); the code and the numbers are our own (see docs/superpowers/specs/2026-10-16-engine-model-design.md).
pub mod gas;
pub mod pipe;
pub mod voice;

use gas::{flow, Gas, AIR_PER_FUEL, P_ATM, T_ATM};

const TAU: f64 = std::f64::consts::TAU;
/// The heat of combustion of the fuel per mole (C8H16, about 44 MJ/kg), J/mol, and the share that becomes heat of the gas.
const HEAT_PER_FUEL_MOL: f64 = 4.9e6;
const BURN_EFFICIENCY: f64 = 0.80;

/// What an engine is made of (SI units, angles in degrees of the crank).
#[derive(Clone, Copy, Debug)]
pub struct EngineSpec {
    pub cylinders: usize,
    pub bore: f64, pub stroke: f64, pub rod: f64, pub compression: f64,
    /// The valve diameters (m) and the lift (m) of one valve; two valves each; the discharge coefficient of the valve curtain.
    pub intake_valve: f64, pub exhaust_valve: f64, pub max_lift: f64, pub valve_cd: f64,
    /// The cams: opening and closing angles in degrees of the cycle (0 = the firing top dead centre of the cylinder, 720 per cycle).
    pub intake_open: f64, pub intake_close: f64, pub exhaust_open: f64, pub exhaust_close: f64,
    /// Which cylinder fires in which slot of the cycle (a firing order such as 1-3-4-2 is `[0, 2, 3, 1]`), the slots are `720 / cylinders` degrees apart.
    pub firing_order: [usize; 8],
    pub idle_rpm: f64, pub limiter_rpm: f64,
    /// The crank's inertia with the flywheel (kg m^2) and the friction torque `a + b w` (N m, w in rad/s).
    pub inertia: f64, pub friction_a: f64, pub friction_b: f64,
    /// The gas volumes (m^3): the intake plenum, one exhaust runner, the collector; the throttle's full area, the idle bypass, the exhaust outlet (the muffler) and the runner to collector areas (m^2).
    pub plenum: f64, pub runner: f64, pub collector: f64,
    pub throttle_area: f64, pub idle_area: f64, pub outlet_area: f64, pub runner_area: f64,
    pub starter_torque: f64,
}

impl EngineSpec {
    /// A two-litre turbocharged four of a rally car without its turbo (85 x 88 mm, 8.5:1), idling at 950 rpm and cutting the fuel at 7800.
    pub fn rally_four() -> EngineSpec {
        EngineSpec {
            cylinders: 4, bore: 0.085, stroke: 0.088, rod: 0.143, compression: 8.5,
            intake_valve: 0.033, exhaust_valve: 0.029, max_lift: 0.0105, valve_cd: 0.62,
            intake_open: 350.0, intake_close: 590.0, exhaust_open: 140.0, exhaust_close: 370.0,
            firing_order: [0, 2, 3, 1, 0, 0, 0, 0],
            idle_rpm: 950.0, limiter_rpm: 7800.0,
            inertia: 0.18, friction_a: 9.0, friction_b: 0.030,
            plenum: 0.0028, runner: 0.0003, collector: 0.0012,
            throttle_area: 8.0e-4, idle_area: 3.0e-5, outlet_area: 9.0e-4, runner_area: 6.0e-4,
            starter_torque: 200.0,
        }
    }
    pub fn displacement(&self) -> f64 { self.cylinders as f64 * std::f64::consts::PI * self.bore * self.bore / 4.0 * self.stroke }
}

/// The burn in progress in a cylinder.
#[derive(Clone, Copy, Debug, Default)]
struct Burn { active: bool, start: f64, duration: f64, fuel0: f64, done: f64 }

#[derive(Clone, Copy, Debug)]
struct Cylinder { gas: Gas, runner: Gas, burn: Burn, angle: f64 }

/// What the driver and the car ask of the engine.
#[derive(Clone, Copy, Debug)]
pub struct Controls {
    /// The throttle pedal, 0..1.
    pub throttle: f64,
    /// The starter is cranking.
    pub starter: bool,
    /// The clutch's grip, 0 (open, the engine runs free) .. 1 (closed, the engine follows the driveline's speed `drive_rpm`).
    pub clutch: f64, pub drive_rpm: f64,
    /// The ignition and the fuel (false: the engine is off).
    pub ignition: bool,
}
impl Default for Controls { fn default() -> Controls { Controls { throttle: 0.0, starter: false, clutch: 0.0, drive_rpm: 0.0, ignition: true } } }

pub struct Engine {
    pub spec: EngineSpec,
    cyl: Vec<Cylinder>, plenum: Gas, collector: Gas, atmosphere: Gas, supply: Gas,
    /// The crank's angle over the cycle (rad, 0..4 pi) and its speed (rad/s).
    theta: f64, omega: f64,
    idle_i: f64, fuel_cut: bool,
    pub controls: Controls,
    /// The torque the gas gave the crank in the last step (N m), the torque the clutch took (positive: the engine drives the car).
    pub gas_torque: f64, pub clutch_torque: f64,
    /// The boost (the pressure of the air before the throttle over the atmosphere's, 1 = none).
    pub boost: f64,
    /// The air going through the throttle (mol/s): the rush the intake makes.
    pub air_flow: f64,
}

impl Engine {
    pub fn new(spec: EngineSpec) -> Engine {
        let cyl = (0..spec.cylinders).map(|_| Cylinder { gas: Gas::air(spec.displacement() / spec.cylinders as f64 / (spec.compression - 1.0), P_ATM, T_ATM), runner: Gas::air(spec.runner, P_ATM, T_ATM), burn: Burn::default(), angle: 0.0 }).collect();
        Engine { spec, cyl, plenum: Gas::air(spec.plenum, P_ATM, T_ATM), collector: Gas::air(spec.collector, P_ATM, T_ATM), atmosphere: Gas::air(1000.0, P_ATM, T_ATM), supply: Gas::air(1000.0, P_ATM, T_ATM),
            theta: 0.0, omega: 0.0, idle_i: 0.0, fuel_cut: false, controls: Controls::default(), gas_torque: 0.0, clutch_torque: 0.0, boost: 1.0, air_flow: 0.0 }
    }

    pub fn rpm(&self) -> f64 { self.omega * 60.0 / TAU }
    /// The exhaust collector's pressure over the atmosphere's (Pa): the pulses that leave the pipe.
    pub fn exhaust_pressure(&self) -> f64 { self.collector.pressure() - P_ATM }
    /// The intake plenum's pressure over the atmosphere's (Pa): negative while the engine pulls on a closed throttle.
    pub fn intake_pressure(&self) -> f64 { self.plenum.pressure() - P_ATM }
    pub fn fuel_cut(&self) -> bool { self.fuel_cut }
    /// The sum of the cylinders' pressures (Pa): its rate of change shakes the block.
    pub fn cylinder_pressure_sum(&self) -> f64 { self.cyl.iter().map(|c| c.gas.pressure()).sum() }

    fn clearance_volume(&self) -> f64 { self.spec.displacement() / self.spec.cylinders as f64 / (self.spec.compression - 1.0) }
    fn bore_area(&self) -> f64 { std::f64::consts::PI * self.spec.bore * self.spec.bore / 4.0 }

    /// The lift (0..1 of the maximum) of a cam that opens at `open` and closes at `close` degrees at the cycle angle `deg`.
    fn lift(open: f64, close: f64, deg: f64) -> f64 {
        let span = (close - open).rem_euclid(720.0);
        let d = (deg - open).rem_euclid(720.0);
        if span <= 0.0 || d >= span { 0.0 } else { (std::f64::consts::PI * d / span).sin().powi(2) }
    }

    /// One step of `dt` seconds (about 11 microseconds): the gas, the burn and the crank.
    pub fn step(&mut self, dt: f64) {
        let spec = self.spec;
        let n_cyl = spec.cylinders;
        let slot = 720.0 / n_cyl as f64;
        let rpm = self.rpm();
        let c = self.controls;
        // the fuel is cut above the limiter (with a little hysteresis) and when the ignition is off
        if rpm > spec.limiter_rpm { self.fuel_cut = true; } else if rpm < spec.limiter_rpm - 150.0 { self.fuel_cut = false; }
        let fuelling = c.ignition && !self.fuel_cut;
        // the idle governor: when the pedal is up and the engine slower than the idle, the bypass opens
        let idle_err = if rpm < 300.0 { 0.25 } else { ((spec.idle_rpm - rpm) / spec.idle_rpm).clamp(-1.0, 1.0) };
        self.idle_i = (self.idle_i + idle_err * dt * 2.0).clamp(0.0, 1.0);
        let gov = if c.ignition { (self.idle_i + 0.8 * idle_err).clamp(0.0, 1.0) } else { 0.0 };
        let thr = c.throttle.clamp(0.0, 1.0);
        let a_throttle = 0.3 * spec.idle_area + 2.0 * spec.idle_area * gov + spec.throttle_area * thr.powf(2.0);
        // the air before the throttle: the atmosphere, or the boost
        self.supply.e = 0.5 * gas::F_DOF * (P_ATM * self.boost * self.supply.v) / 1.0 * 1.0;
        self.supply.n = self.supply.e * 2.0 / (gas::F_DOF * gas::R * T_ATM);
        self.supply.o2 = 0.21 * self.supply.n; self.supply.fuel = 0.0;
        self.air_flow = flow(&mut self.supply, &mut self.plenum, a_throttle * 0.9, dt) / dt;
        // the engine's torque
        let mut torque = 0.0;
        let a_bore = self.bore_area(); let clear = self.clearance_volume();
        let d_theta = self.omega * dt;
        let load = ((self.plenum.pressure() - 0.25 * P_ATM) / (0.75 * P_ATM)).clamp(0.0, 1.2);
        for k in 0..n_cyl {
            let slot_idx = spec.firing_order[..n_cyl].iter().position(|&x| x == k).unwrap_or(k) as f64;
            let cyc_prev = (self.theta - (slot * slot_idx).to_radians()).rem_euclid(2.0 * TAU);   // rad over the cycle, 0 = the firing TDC
            let cyc = cyc_prev + d_theta;
            let (deg_prev, deg) = (cyc_prev.to_degrees(), cyc.to_degrees());
            let cy = &mut self.cyl[k];
            // the volume follows the piston
            let ang = cyc.rem_euclid(TAU);
            let x_new = r_x(&spec, ang);
            let x_old = r_x(&spec, cyc_prev.rem_euclid(TAU));
            let _ = x_old;
            let v_new = clear + a_bore * x_new;
            let dv = v_new - cy.gas.v;
            cy.gas.change_volume(dv);
            // the heat into the walls
            let area = std::f64::consts::PI * spec.bore * (x_new + 0.006) + 2.0 * a_bore;
            let tg = cy.gas.temperature();
            cy.gas.e = (cy.gas.e + 250.0 * area * (430.0 - tg) * dt).max(1e-6);
            // the valves
            let il = Self::lift(spec.intake_open, spec.intake_close, deg % 720.0);
            let el = Self::lift(spec.exhaust_open, spec.exhaust_close, deg % 720.0);
            if il > 0.0 {
                let a = valve_area(&spec, spec.intake_valve, il);
                let n0 = cy.gas.n;
                let moved = flow(&mut self.plenum, &mut cy.gas, a, dt);
                if moved > 0.0 && fuelling {   // port injection: the fuel goes in with the air (a rich mixture for power under load)
                    let phi = 0.98 + 0.14 * load;
                    cy.gas.fuel += moved * phi / AIR_PER_FUEL; cy.gas.n += moved * phi / AIR_PER_FUEL;
                    cy.gas.e += moved * phi / AIR_PER_FUEL * 0.5 * gas::F_DOF * gas::R * T_ATM;
                }
                let _ = n0;
            }
            if el > 0.0 { let a = valve_area(&spec, spec.exhaust_valve, el); flow(&mut cy.gas, &mut cy.runner, a, dt); }
            flow(&mut cy.runner, &mut self.collector, spec.runner_area, dt);
            // the spark: before the top dead centre, earlier with the revs, later under load
            let adv = 10.0 + 16.0 * (rpm / 7000.0).min(1.0) + 8.0 * (1.0 - load.min(1.0));
            let spark = (720.0 - adv) % 720.0;
            if fuelling && !cy.burn.active && crossed(deg_prev % 720.0, deg % 720.0, spark) && cy.gas.fuel > 1e-9 && cy.gas.o2 > 12.0 * cy.gas.fuel * 0.35 {
                cy.burn = Burn { active: true, start: spark, duration: 42.0 + 28.0 * (1.0 - load.min(1.0)) + 6.0 * (rpm / 7000.0), fuel0: cy.gas.fuel, done: 0.0 };
            }
            if cy.burn.active {
                let into = (deg % 720.0 - cy.burn.start).rem_euclid(720.0);
                let xb = if into > 300.0 { 1.0 } else { 1.0 - (-5.0 * (into / cy.burn.duration).powi(3)).exp() };
                let inc = (xb - cy.burn.done).max(0.0);
                if inc > 0.0 {
                    let burnt = (inc * cy.burn.fuel0).min(cy.gas.fuel);
                    cy.gas.fuel -= burnt; cy.gas.o2 = (cy.gas.o2 - 12.0 * burnt).max(0.0); cy.gas.n += 3.0 * burnt;
                    cy.gas.e += burnt * HEAT_PER_FUEL_MOL * BURN_EFFICIENCY;
                    cy.burn.done = xb;
                }
                if xb >= 0.999 || into > 300.0 { cy.burn.active = false; }
            }
            // the push on the crank
            let p = cy.gas.pressure();
            torque += (p - P_ATM) * a_bore * piston_dx(&spec, ang);
            cy.angle = cyc;
        }
        self.collector_to_atmosphere(dt);
        self.atmosphere.e = 0.5 * gas::F_DOF * P_ATM * self.atmosphere.v; self.atmosphere.n = P_ATM * self.atmosphere.v / (gas::R * T_ATM); self.atmosphere.o2 = 0.21 * self.atmosphere.n; self.atmosphere.fuel = 0.0;
        // the crank
        let friction = spec.friction_a + spec.friction_b * self.omega.abs();
        let drive = c.drive_rpm * TAU / 60.0;
        self.clutch_torque = (6.0 * (self.omega - drive)).clamp(-450.0, 450.0) * c.clutch.clamp(0.0, 1.0);
        let starter = if c.starter && self.omega < 40.0 { spec.starter_torque * (1.0 - self.omega / 45.0).max(0.0) } else { 0.0 };
        self.gas_torque = torque;
        self.omega += dt / spec.inertia * (torque - friction * if self.omega > 0.0 { 1.0 } else { 0.0 } - self.clutch_torque + starter);
        if self.omega < 0.0 { self.omega = 0.0; }
        self.theta = (self.theta + self.omega * dt).rem_euclid(2.0 * TAU);
    }

    fn collector_to_atmosphere(&mut self, dt: f64) { flow(&mut self.collector, &mut self.atmosphere, self.spec.outlet_area, dt); }
}

/// The effective flow area of a pair of valves of diameter `dia` at the lift fraction `l`.
fn valve_area(spec: &EngineSpec, dia: f64, l: f64) -> f64 {
    let curtain = std::f64::consts::PI * dia * spec.max_lift * l;
    let seat = std::f64::consts::PI * dia * dia / 4.0 * 0.85;
    2.0 * spec.valve_cd * curtain.min(seat)
}
/// The rate of the piston's travel with the crank angle `a` (rad from the top), m/rad.
fn piston_dx(spec: &EngineSpec, a: f64) -> f64 {
    let (r, l) = (spec.stroke / 2.0, spec.rod);
    r * a.sin() + r * r * a.sin() * a.cos() / (l * l - r * r * a.sin() * a.sin()).sqrt()
}
/// The piston's distance from the top dead centre at the crank angle `a` (rad from the top), m.
fn r_x(spec: &EngineSpec, a: f64) -> f64 {
    let (r, l) = (spec.stroke / 2.0, spec.rod);
    r * (1.0 - a.cos()) + l - (l * l - r * r * a.sin() * a.sin()).sqrt()
}

/// Did the angle pass `mark` going from `prev` to `now` (degrees in 0..720, wrapping)?
fn crossed(prev: f64, now: f64, mark: f64) -> bool {
    if now >= prev { mark > prev && mark <= now } else { mark > prev || mark <= now }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
