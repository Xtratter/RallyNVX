//! Vehicle parameters in one data file (`rust/vehicles.json`, embedded): cars, aircraft, flight rules, the sling rope.
use super::flight::{AircraftKind, AircraftSpec};
use super::sling::SlingSpec;
use super::vehicle::VehicleSpec;

/// The embedded default file.
pub const EMBEDDED: &str = include_str!("../../vehicles.json");

#[derive(Clone, Debug)]
pub struct Params { pub cars: Vec<(String, VehicleSpec)>, pub heli: AircraftSpec, pub drone: AircraftSpec, pub sling: SlingSpec }

use serde_json::Value;
use std::sync::OnceLock;

use super::math::v3;

fn field(v: &Value, key: &str, ctx: &str) -> Result<f64, String> {
    v.get(key).and_then(Value::as_f64).filter(|x| x.is_finite()).ok_or_else(|| format!("{ctx}: missing or non-numeric \"{key}\""))
}
/// A number that must be greater than `min`.
fn above(v: &Value, key: &str, ctx: &str, min: f64) -> Result<f32, String> {
    let x = field(v, key, ctx)?;
    if x > min { Ok(x as f32) } else { Err(format!("{ctx}: \"{key}\" must be greater than {min} (got {x})")) }
}
fn between(v: &Value, key: &str, ctx: &str, lo: f64, hi: f64) -> Result<f32, String> {
    let x = field(v, key, ctx)?;
    if x >= lo && x <= hi { Ok(x as f32) } else { Err(format!("{ctx}: \"{key}\" must be between {lo} and {hi} (got {x})")) }
}
fn section<'a>(v: &'a Value, key: &str, ctx: &str) -> Result<&'a Value, String> {
    v.get(key).filter(|x| x.is_object()).ok_or_else(|| format!("{ctx}: missing object \"{key}\""))
}

/// Colours and widths used by the renderer (all colours 0..1). Optional in the file: defaults apply when absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look { pub body: [f32; 3], pub accent: [f32; 3], pub glass: [f32; 3], pub wheel_width: f32 }

fn color(v: &Value, key: &str, ctx: &str, default: [f32; 3]) -> Result<[f32; 3], String> {
    let Some(x) = v.get(key) else { return Ok(default) };
    let err = || format!("{ctx}.look: \"{key}\" must be [r, g, b] with every value in 0..1");
    let a = x.as_array().filter(|a| a.len() == 3).ok_or_else(err)?;
    let mut out = [0.0f32; 3];
    for (i, c) in a.iter().enumerate() { let f = c.as_f64().filter(|f| (0.0..=1.0).contains(f)).ok_or_else(err)?; out[i] = f as f32; }
    Ok(out)
}

fn parse_look(v: &Value, ctx: &str, default: Look) -> Result<Look, String> {
    let Some(l) = v.get("look") else { return Ok(default) };
    if !l.is_object() { return Err(format!("{ctx}: \"look\" must be an object")); }
    let ctxl = ctx.to_string();
    let wheel_width = match l.get("wheel_width") { None => default.wheel_width, Some(_) => between(l, "wheel_width", &format!("{ctxl}.look"), 0.05, 1.0)? };
    Ok(Look { body: color(l, "body_color", ctx, default.body)?, accent: color(l, "accent_color", ctx, default.accent)?, glass: color(l, "glass_tint", ctx, default.glass)?, wheel_width })
}

fn parse_car(id: &str, v: &Value) -> Result<VehicleSpec, String> {
    let ctx = format!("cars.{id}");
    let ctx = ctx.as_str();
    let mass = above(v, "mass", ctx, 0.0)?;
    let size = section(v, "size", ctx)?;
    let (w, h, l) = (above(size, "width", ctx, 0.0)?, above(size, "height", ctx, 0.0)?, above(size, "length", ctx, 0.0)?);
    let wheels_v = v.get("wheels").and_then(Value::as_array).ok_or_else(|| format!("{ctx}: missing array \"wheels\""))?;
    if wheels_v.len() != 4 { return Err(format!("{ctx}: \"wheels\" needs 4 wheel positions (front left, front right, rear left, rear right)")); }
    let mut wheels = [v3(0.0, 0.0, 0.0); 4];
    for (i, wv) in wheels_v.iter().enumerate() {
        let xs = wv.as_array().filter(|a| a.len() == 3).and_then(|a| a.iter().map(|x| x.as_f64().filter(|f| f.is_finite())).collect::<Option<Vec<f64>>>())
            .ok_or_else(|| format!("{ctx}: wheel {i} must be [x, y, z]"))?;
        wheels[i] = v3(xs[0] as f32, xs[1] as f32, xs[2] as f32);
    }
    let sus = section(v, "suspension", ctx)?;
    let hnd = section(v, "handling", ctx)?;
    let look = parse_look(v, ctx, Look { body: [0.75, 0.12, 0.08], accent: [0.15, 0.15, 0.17], glass: [0.10, 0.16, 0.22], wheel_width: 0.32 })?;
    Ok(VehicleSpec {
        mass, size: v3(w, h, l), look,
        inertia: v3(mass / 12.0 * (h * h + l * l), mass / 12.0 * (w * w + l * l), mass / 12.0 * (w * w + h * h)),
        wheels,
        rest: above(sus, "rest", ctx, 0.0)?, radius: above(sus, "radius", ctx, 0.0)?,
        spring: above(sus, "spring", ctx, 0.0)?, damper: between(sus, "damper", ctx, 0.0, 1e9)?,
        engine_force: between(v, "engine_force", ctx, 0.0, 1e9)?, brake_force: between(v, "brake_force", ctx, 0.0, 1e9)?,
        max_speed: above(v, "max_speed", ctx, 0.0)?, mu: above(v, "grip", ctx, 0.0)?,
        steer_max: between(hnd, "steer_max", ctx, 0.01, 1.4)?, steer_speed_ref: above(hnd, "steer_speed_ref", ctx, 0.0)?,
        steer_rate: above(hnd, "steer_rate", ctx, 0.0)?, lateral_k: between(hnd, "lateral_stiffness", ctx, 0.0, 1e6)?,
        rolling: between(hnd, "rolling_resistance", ctx, 0.0, 1e6)?, aero: between(hnd, "aero_drag", ctx, 0.0, 1e6)?,
        ang_damp: between(hnd, "angular_damping", ctx, 0.0, 1e3)?, handbrake_grip: between(hnd, "handbrake_grip", ctx, 0.0, 1.0)?,
        reverse_factor: between(hnd, "reverse_speed_factor", ctx, 0.01, 1.0)?,
        max_susp_factor: above(sus, "max_force_factor", ctx, 0.0)?,
    })
}

fn parse_aircraft(id: &str, v: &Value, rules: &Value) -> Result<AircraftSpec, String> {
    let ctx = format!("aircraft.{id}");
    let ctx = ctx.as_str();
    let fl = "flight";
    let default = if id == "heli" { Look { body: [0.95, 0.45, 0.10], accent: [0.55, 0.55, 0.58], glass: [0.10, 0.16, 0.22], wheel_width: 0.1 } } else { Look { body: [0.15, 0.15, 0.18], accent: [0.90, 0.90, 0.95], glass: [0.10, 0.16, 0.22], wheel_width: 0.1 } };
    let look = parse_look(v, ctx, default)?;
    Ok(AircraftSpec {
        look, max_tilt: between(v, "max_tilt_deg", ctx, 1.0, 80.0)?.to_radians(), tilt_lag: above(v, "tilt_lag", ctx, 0.0)?,
        max_speed: above(v, "max_speed", ctx, 0.0)?, drag: between(v, "drag", ctx, 0.0, 100.0)?,
        vmax: above(v, "vertical_speed", ctx, 0.0)?, vacc: above(v, "vertical_accel", ctx, 0.0)?,
        yaw_rate: above(v, "yaw_rate", ctx, 0.0)?, yaw_lag: above(v, "yaw_lag", ctx, 0.0)?, radius: above(v, "radius", ctx, 0.0)?,
        ceiling: above(rules, "ceiling", fl, 0.0)?, soft_speed: above(rules, "soft_speed", fl, 0.0)?,
        soft_tilt: between(rules, "soft_tilt_deg", fl, 1.0, 89.0)?.to_radians(), ground_damp: between(rules, "ground_damp", fl, 0.0, 1e3)?,
    })
}

impl Params {
    pub fn from_json(s: &str) -> Result<Params, String> {
        let v: Value = serde_json::from_str(s).map_err(|e| format!("vehicles file is not valid JSON: {e}"))?;
        let cars_v = section(&v, "cars", "vehicles")?;
        let mut cars = Vec::new();
        for (id, c) in cars_v.as_object().unwrap() { cars.push((id.clone(), parse_car(id, c)?)); }
        if cars.is_empty() { return Err("vehicles: \"cars\" is empty".into()); }
        let air = section(&v, "aircraft", "vehicles")?;
        let rules = section(&v, "flight", "vehicles")?;
        let heli = parse_aircraft("heli", section(air, "heli", "aircraft")?, rules)?;
        let drone = parse_aircraft("drone", section(air, "drone", "aircraft")?, rules)?;
        let sl = section(&v, "sling", "vehicles")?;
        let sling = SlingSpec { rope: above(sl, "rope", "sling", 0.0)?, k: above(sl, "k", "sling", 0.0)?, c: between(sl, "c", "sling", 0.0, 1e6)?, drag: between(sl, "drag", "sling", 0.0, 100.0)? };
        Ok(Params { cars, heli, drone, sling })
    }
    pub fn car(&self, id: &str) -> Option<VehicleSpec> { self.cars.iter().find(|(i, _)| i == id).map(|(_, c)| *c) }
    pub fn aircraft(&self, kind: AircraftKind) -> AircraftSpec { match kind { AircraftKind::Heli => self.heli, AircraftKind::Drone => self.drone } }
}

/// The active parameters: the embedded file, or the file named by `RAYTRACE_VEHICLES` when it parses.
pub fn get() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(|| {
        if let Some(path) = std::env::var_os("RAYTRACE_VEHICLES") {
            match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| Params::from_json(&t)) {
                Ok(p) => { log::info!("vehicle parameters loaded from {path:?}"); return p; }
                Err(e) => log::error!("RAYTRACE_VEHICLES {path:?} ignored: {e}"),
            }
        }
        Params::from_json(EMBEDDED).expect("embedded vehicles.json is invalid")
    })
}

#[cfg(test)]
#[path = "params_tests.rs"]
mod tests;
