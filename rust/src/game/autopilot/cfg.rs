//! The autopilot's numbers (`rust/autopilot.json`).
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoCfg {
    pub lat_factor: f32, pub brake_factor: f32, pub look_base: f32, pub look_gain: f32, pub offset_max: f32,
    pub crest_speed: f32, pub field_speed: f32, pub stuck_time: f32, pub finish_stop: f32,
}

pub const EMBEDDED: &str = include_str!("../../../autopilot.json");

impl AutoCfg {
    pub fn from_json(text: &str) -> Result<AutoCfg, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("autopilot file is not valid JSON: {e}"))?;
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = v.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("autopilot: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("autopilot: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        Ok(AutoCfg {
            lat_factor: num("lat_factor", 0.2, 1.2)?, brake_factor: num("brake_factor", 0.2, 1.2)?, look_base: num("look_base", 2.0, 40.0)?, look_gain: num("look_gain", 0.0, 2.0)?,
            offset_max: num("offset_max", 0.0, 0.9)?, crest_speed: num("crest_speed", 5.0, 60.0)?, field_speed: num("field_speed", 3.0, 40.0)?,
            stuck_time: num("stuck_time", 1.0, 30.0)?, finish_stop: num("finish_stop", 5.0, 200.0)?,
        })
    }
}

static CFG: RwLock<Option<AutoCfg>> = RwLock::new(None);

pub fn cfg() -> AutoCfg {
    if let Some(c) = *CFG.read().unwrap_or_else(|e| e.into_inner()) { return c; }
    let c = AutoCfg::from_json(EMBEDDED).expect("the embedded autopilot.json is invalid");
    *CFG.write().unwrap_or_else(|e| e.into_inner()) = Some(c);
    c
}
pub fn reset_cfg() { *CFG.write().unwrap_or_else(|e| e.into_inner()) = None; }

/// Replaces the numbers with the file's when it exists and parses; a bad file is logged and ignored.
pub fn load_override(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    match AutoCfg::from_json(&text) {
        Ok(c) => { log::info!("autopilot numbers loaded from {path:?}"); *CFG.write().unwrap_or_else(|e| e.into_inner()) = Some(c); }
        Err(e) => log::error!("autopilot file {path:?} ignored: {e}"),
    }
}

#[cfg(test)]
#[path = "cfg_tests.rs"]
mod tests;
