//! Car aerodynamics as physics: drag, downforce on each axle with a ground effect that depends on the real ride height, and the
//! effect of sideslip; body parts of the garage (splitter, wing, underbody) change the coefficients. See
//! docs/superpowers/specs/2026-10-15-aero-design.md. Forces are `q x area` with `q = 0.5 rho v^2`; `ClA` positive is downforce.
use super::math::{v3, V3};
use super::vehicle::VehicleSpec;

/// The stock car's aero (from `vehicles.json` -> `aero`): downforce areas of the axles (m^2, negative is lift) and the
/// reference clearance of the floor (m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AeroBase { pub cla_front: f32, pub cla_rear: f32, pub ride_ref: f32 }

/// The parts of the garage: levels 0..3 and the wing angle (0 low drag, 1 mid, 2 high downforce).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AeroKit { pub splitter: u8, pub wing: u8, pub underbody: u8, pub wing_angle: u8 }
impl Default for AeroKit { fn default() -> Self { AeroKit { splitter: 0, wing: 0, underbody: 0, wing_angle: 1 } } }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartCfg { pub cla: [f32; 4], pub cda: [f32; 4], pub ground: f32 }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AeroCfg {
    pub rho: f32,
    pub peak: f32, pub gain: f32, pub stall: f32, pub stall_floor: f32, pub decay: f32,
    pub yaw_drag: f32, pub yaw_down: f32, pub yaw_side: f32, pub cp: f32,
    pub splitter: PartCfg, pub wing: PartCfg, pub underbody: PartCfg,
    pub angle_cla: [f32; 3], pub angle_cda: [f32; 3],
}

pub const EMBEDDED: &str = include_str!("../../aero.json");

impl AeroCfg {
    pub fn from_json(text: &str) -> Result<AeroCfg, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("aero file is not valid JSON: {e}"))?;
        let obj = |k: &str| v.get(k).filter(|o| o.is_object()).ok_or_else(|| format!("aero: missing object \"{k}\""));
        let num = |o: &serde_json::Value, ctx: &str, k: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = o.get(k).and_then(|x| x.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("{ctx}: missing or non-numeric \"{k}\""))?;
            if x < lo || x > hi { return Err(format!("{ctx}: \"{k}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        let list = |o: &serde_json::Value, ctx: &str, k: &str, n: usize, lo: f64, hi: f64| -> Result<Vec<f32>, String> {
            let a = o.get(k).and_then(|x| x.as_array()).filter(|a| a.len() == n).ok_or_else(|| format!("{ctx}: \"{k}\" must be a list of {n} numbers"))?;
            a.iter().map(|x| x.as_f64().filter(|f| f.is_finite() && *f >= lo && *f <= hi).map(|f| f as f32).ok_or_else(|| format!("{ctx}: \"{k}\" values must be between {lo} and {hi}"))).collect()
        };
        let part = |name: &str| -> Result<(PartCfg, serde_json::Value), String> {
            let o = obj(name)?.clone();
            let (cla, cda) = (list(&o, name, "cla", 4, 0.0, 5.0)?, list(&o, name, "cda", 4, 0.0, 2.0)?);
            Ok((PartCfg { cla: [cla[0], cla[1], cla[2], cla[3]], cda: [cda[0], cda[1], cda[2], cda[3]], ground: num(&o, name, "ground", 0.0, 3.0)? }, o))
        };
        let (g, y) = (obj("ground")?, obj("yaw")?);
        let (splitter, _) = part("splitter")?; let (wing, wo) = part("wing")?; let (underbody, _) = part("underbody")?;
        let (ac, ad) = (list(&wo, "wing", "angle_cla", 3, 0.1, 3.0)?, list(&wo, "wing", "angle_cda", 3, 0.1, 5.0)?);
        Ok(AeroCfg { rho: num(&v, "aero", "rho", 0.5, 2.0)?,
            peak: num(g, "ground", "peak", 0.3, 0.95)?, gain: num(g, "ground", "gain", 0.0, 2.0)?, stall: num(g, "ground", "stall", 0.05, 0.9)?, stall_floor: num(g, "ground", "stall_floor", 0.0, 1.0)?, decay: num(g, "ground", "decay", 0.0, 10.0)?,
            yaw_drag: num(y, "yaw", "drag", 0.0, 10.0)?, yaw_down: num(y, "yaw", "downforce", 0.0, 5.0)?, yaw_side: num(y, "yaw", "side", 0.0, 10.0)?, cp: num(y, "yaw", "cp", 0.0, 3.0)?,
            splitter, wing, underbody, angle_cla: [ac[0], ac[1], ac[2]], angle_cda: [ad[0], ad[1], ad[2]] })
    }
}

pub fn cfg() -> AeroCfg {
    static CELL: std::sync::OnceLock<AeroCfg> = std::sync::OnceLock::new();
    *CELL.get_or_init(|| AeroCfg::from_json(EMBEDDED).expect("the embedded aero.json is invalid"))
}

/// The ground-effect multiplier of the clearance ratio `x` (clearance / reference): 1 at the reference, rising to `1 + gain`
/// at `peak`, falling to `stall_floor` at `stall` (the flow stalls) and below; decaying above the reference.
pub fn ground_effect(x: f32, c: &AeroCfg) -> f32 {
    if !x.is_finite() { return 1.0; }
    if x >= 1.0 { 1.0 / (1.0 + c.decay * (x - 1.0)) }
    else if x >= c.peak { 1.0 + c.gain * (1.0 - x) / (1.0 - c.peak) }
    else if x >= c.stall { let t = (x - c.stall) / (c.peak - c.stall); c.stall_floor + (1.0 + c.gain - c.stall_floor) * t }
    else { c.stall_floor * (x / c.stall).max(0.0) }
}

/// What the car feels: the forces in world space, the downforce (N, downwards) on each axle and the moment about the vertical.
#[derive(Clone, Copy, Debug)]
pub struct AeroForces { pub drag: V3, pub down_front: f32, pub down_rear: f32, pub side: V3, pub q: f32 }

fn part_cda(spec: &VehicleSpec, c: &AeroCfg) -> f32 {
    let k = spec.kit;
    let wing = c.wing.cda[k.wing.min(3) as usize] * c.angle_cda[k.wing_angle.min(2) as usize];
    c.splitter.cda[k.splitter.min(3) as usize] + wing + c.underbody.cda[k.underbody.min(3) as usize]
}

/// The base drag area of the car (m^2) from its `aero_drag` constant (the old model: `F = aero v^2`).
pub fn base_cda(spec: &VehicleSpec, c: &AeroCfg) -> f32 { 2.0 * spec.aero / c.rho }

/// The forces for a car with velocity `vel`, body axes `fwd` / `right`, and the clearance of the front and rear axle (m).
pub fn forces(spec: &VehicleSpec, vel: V3, fwd: V3, right: V3, clear_front: f32, clear_rear: f32, c: &AeroCfg) -> AeroForces {
    let speed = vel.len();
    if !(speed > 0.05) || !speed.is_finite() { return AeroForces { drag: V3::ZERO, down_front: 0.0, down_rear: 0.0, side: V3::ZERO, q: 0.0 }; }
    let q = 0.5 * c.rho * speed * speed;
    let dir = vel * (1.0 / speed);
    let (sin_b, cos_b) = (dir.dot(right), dir.dot(fwd));
    let s2 = sin_b * sin_b;
    let k = spec.kit;
    let r = spec.aero_base.ride_ref.max(0.02);
    let (xf, xr) = (clear_front / r, clear_rear / r);
    let ge = |part: &PartCfg, x: f32| 1.0 + part.ground * (ground_effect(x, c) - 1.0);
    let xm = 0.5 * (xf + xr);
    let split = c.splitter.cla[k.splitter.min(3) as usize] * ge(&c.splitter, xf);
    let floor = c.underbody.cla[k.underbody.min(3) as usize] * ge(&c.underbody, xm);
    let wing = c.wing.cla[k.wing.min(3) as usize] * c.angle_cla[k.wing_angle.min(2) as usize];
    let yaw_loss = (1.0 - c.yaw_down * s2).max(0.0);
    let down_front = q * (spec.aero_base.cla_front + split + 0.5 * floor) * yaw_loss;
    let down_rear = q * (spec.aero_base.cla_rear + wing + 0.5 * floor) * yaw_loss;
    let cda = (base_cda(spec, c) + part_cda(spec, c)) * (1.0 + c.yaw_drag * s2);
    let drag = dir * (-q * cda);
    // the side force turns the car back into the flow: it pushes against the sideslip
    let side = right * (-q * c.yaw_side * sin_b * cos_b.abs());
    AeroForces { drag, down_front, down_rear, side, q }
}

/// The numbers the garage shows for a car with its kit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AeroStats { pub downforce_kg_150: f32, pub drag_delta_pct: f32, pub top_kmh: f32 }

/// Downforce at 150 km/h at the reference clearance (kg), the drag change against the same car without parts (%) and the
/// estimated top speed (km/h) where the engine (with its taper) meets the drag and the rolling resistance.
pub fn stats(spec: &VehicleSpec) -> AeroStats {
    let c = cfg();
    let v = 150.0 / 3.6;
    let f = forces(spec, v3(0.0, 0.0, v), v3(0.0, 0.0, 1.0), v3(-1.0, 0.0, 0.0), spec.aero_base.ride_ref, spec.aero_base.ride_ref, &c);
    let base = base_cda(spec, &c);
    let drag_delta = 100.0 * part_cda(spec, &c) / base.max(1e-4);
    AeroStats { downforce_kg_150: (f.down_front + f.down_rear) / 9.81, drag_delta_pct: drag_delta, top_kmh: top_speed(spec) * 3.6 }
}

/// Where the engine force (tapering to zero at `max_speed`) equals the drag plus the rolling resistance (m/s).
pub fn top_speed(spec: &VehicleSpec) -> f32 {
    let c = cfg();
    let cda = base_cda(spec, &c) + part_cda(spec, &c);
    let net = |v: f32| spec.engine_force * (1.0 - v / spec.max_speed).max(0.0) - 0.5 * c.rho * cda * v * v - spec.rolling * 4.0 * v;
    let (mut lo, mut hi) = (0.0f32, spec.max_speed);
    for _ in 0..40 { let mid = 0.5 * (lo + hi); if net(mid) > 0.0 { lo = mid; } else { hi = mid; } }
    lo
}

#[cfg(test)]
#[path = "aero_tests.rs"]
mod tests;
