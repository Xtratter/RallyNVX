//! The autopilot's numbers (`rust/autopilot.json`).
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoCfg {
    /// 0 off, 1 slide assist (steering by the velocity direction, a handbrake flick into slow bends).
    pub slide: f32, pub flick_ratio: f32, pub flick_time: f32,
    pub lat_factor: f32, pub brake_factor: f32, pub look_base: f32, pub look_gain: f32, pub offset_max: f32,
    pub crest_speed: f32, pub field_speed: f32, pub stuck_time: f32,
}

pub const EMBEDDED: &str = include_str!("../../../autopilot.json");

/// The three driving styles: calm, normal (the default numbers) and aggressive (slides allowed).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Styles { pub calm: AutoCfg, pub normal: AutoCfg, pub aggressive: AutoCfg }
impl Styles { pub fn get(&self, style: i32) -> AutoCfg { match style { 0 => self.calm, 2 => self.aggressive, _ => self.normal } } }

impl AutoCfg {
    /// The base numbers of the file; `over` (a style object, may be empty) replaces the keys it names.
    fn parse(v: &serde_json::Value, over: Option<&serde_json::Value>, ctx: &str) -> Result<AutoCfg, String> {
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = over.and_then(|o| o.get(key)).or_else(|| v.get(key)).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("{ctx}: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("{ctx}: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        Ok(AutoCfg {
            slide: num("slide", 0.0, 1.0)?, flick_ratio: num("flick_ratio", 1.0, 3.0)?, flick_time: num("flick_time", 0.1, 1.0)?,
            lat_factor: num("lat_factor", 0.2, 3.0)?, brake_factor: num("brake_factor", 0.2, 1.5)?, look_base: num("look_base", 2.0, 40.0)?, look_gain: num("look_gain", 0.0, 2.0)?,
            offset_max: num("offset_max", 0.0, 0.9)?, crest_speed: num("crest_speed", 5.0, 60.0)?, field_speed: num("field_speed", 3.0, 40.0)?,
            stuck_time: num("stuck_time", 1.0, 30.0)?,
        })
    }
    pub fn from_json(text: &str) -> Result<AutoCfg, String> { Ok(Styles::from_json(text)?.normal) }
}

impl Styles {
    pub fn from_json(text: &str) -> Result<Styles, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("autopilot file is not valid JSON: {e}"))?;
        let styles = v.get("styles").filter(|s| s.is_object());
        let one = |name: &str| -> Result<AutoCfg, String> {
            let over = styles.and_then(|s| s.get(name)).filter(|o| o.is_object());
            AutoCfg::parse(&v, over, &format!("autopilot.{name}"))
        };
        Ok(Styles { calm: one("calm")?, normal: AutoCfg::parse(&v, None, "autopilot")?, aggressive: one("aggressive")? })
    }
}

static CFG: RwLock<Option<Styles>> = RwLock::new(None);

pub fn cfg() -> AutoCfg { styles().normal }
/// The numbers of a driving style: 0 calm, 1 normal, 2 aggressive.
pub fn cfg_style(style: i32) -> AutoCfg { styles().get(style) }
pub fn styles() -> Styles {
    if let Some(c) = *CFG.read().unwrap_or_else(|e| e.into_inner()) { return c; }
    let c = Styles::from_json(EMBEDDED).expect("the embedded autopilot.json is invalid");
    *CFG.write().unwrap_or_else(|e| e.into_inner()) = Some(c);
    c
}
pub fn reset_cfg() { *CFG.write().unwrap_or_else(|e| e.into_inner()) = None; }

/// Replaces the numbers with the file's when it exists and parses; a bad file is logged and ignored.
pub fn load_override(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    match Styles::from_json(&text) {
        Ok(c) => { log::info!("autopilot numbers loaded from {path:?}"); *CFG.write().unwrap_or_else(|e| e.into_inner()) = Some(c); }
        Err(e) => log::error!("autopilot file {path:?} ignored: {e}"),
    }
}

#[cfg(test)]
#[path = "cfg_tests.rs"]
mod tests;
