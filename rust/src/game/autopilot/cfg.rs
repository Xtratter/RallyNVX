//! The autopilot's numbers (`rust/autopilot.json`).
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoCfg {
    /// 0 off, 1 slide assist (steering by the velocity direction, a handbrake flick into slow bends).
    pub slide: f32, pub flick_ratio: f32, pub flick_time: f32,
    /// Fraction of the car's top speed that may be used, and the margin kept to the road edge (m).
    pub speed_cap: f32, pub edge_margin: f32,
    /// Laps of the city loop when the car starts on it.
    pub laps: u32,
    pub lat_factor: f32, pub brake_factor: f32, pub look_base: f32, pub look_gain: f32, pub offset_max: f32,
    pub crest_speed: f32, pub field_speed: f32, pub stuck_time: f32,
    /// 1 for the Max Attack style: full throttle, the line may touch the edge and the shoulder, slides and the handbrake in the hairpins are all used; 0 otherwise.
    pub attack: f32,
}

pub const EMBEDDED: &str = include_str!("../../../autopilot.json");

/// The three driving styles: calm, normal (the default numbers) and aggressive (slides allowed).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Styles { pub calm: AutoCfg, pub normal: AutoCfg, pub aggressive: AutoCfg, pub attack: AutoCfg }
impl Styles { pub fn get(&self, style: i32) -> AutoCfg { match style { 0 => self.calm, 2 => self.aggressive, 5 => self.attack, _ => self.normal } } }

impl AutoCfg {
    /// The base numbers of the file; `over` (a style object, may be empty) replaces the keys it names.
    fn parse(v: &serde_json::Value, over: Option<&serde_json::Value>, ctx: &str) -> Result<AutoCfg, String> {
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = over.and_then(|o| o.get(key)).or_else(|| v.get(key)).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("{ctx}: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("{ctx}: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        Ok(AutoCfg {
            laps: num("laps", 1.0, 5.0)? as u32, speed_cap: num("speed_cap", 0.3, 1.0)?, edge_margin: num("edge_margin", -1.0, 4.0)?,
            slide: num("slide", 0.0, 1.0)?, flick_ratio: num("flick_ratio", 1.0, 3.0)?, flick_time: num("flick_time", 0.1, 1.0)?,
            lat_factor: num("lat_factor", 0.2, 3.0)?, brake_factor: num("brake_factor", 0.2, 1.5)?, look_base: num("look_base", 2.0, 40.0)?, look_gain: num("look_gain", 0.0, 2.0)?,
            offset_max: num("offset_max", 0.0, 1.2)?, crest_speed: num("crest_speed", 5.0, 60.0)?, field_speed: num("field_speed", 3.0, 40.0)?,
            stuck_time: num("stuck_time", 1.0, 30.0)?,
            attack: over.and_then(|o| o.get("attack")).or_else(|| v.get("attack")).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).unwrap_or(0.0).clamp(0.0, 1.0) as f32,
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
        Ok(Styles { calm: one("calm")?, normal: AutoCfg::parse(&v, None, "autopilot")?, aggressive: one("aggressive")?, attack: one("attack")? })
    }
}

static CFG: RwLock<Option<Styles>> = RwLock::new(None);

pub fn cfg() -> AutoCfg { styles().normal }
/// The numbers of a driving style: 0 calm, 1 normal, 2 aggressive.
pub fn cfg_style(style: i32) -> AutoCfg { match style { 3 => custom(), 4 => learned(), 5 => attack(), 6 => net().map_or_else(cfg, |(_, src)| cfg_style(src)), _ => styles().get(style) } }

/// The neural style (6): the net the autopilot trained on the drives of the controller of style `src` (1 normal, 5 Max Attack), which also gives the plan; the normal style's controller until a net is set.
static NET: RwLock<Option<(std::sync::Arc<super::nn::Mlp>, i32)>> = RwLock::new(None);
pub fn net() -> Option<(std::sync::Arc<super::nn::Mlp>, i32)> { NET.read().unwrap_or_else(|e| e.into_inner()).clone() }
/// Sets the net from its saved parameters; `false` for a wrong size or non-finite numbers.
pub fn set_net(w: &[f32], src: i32) -> bool {
    let Some(m) = super::nn::Mlp::from_vec(w) else { return false };
    *NET.write().unwrap_or_else(|e| e.into_inner()) = Some((std::sync::Arc::new(m), if src == 5 { 5 } else { 1 })); true
}
pub fn reset_net() { *NET.write().unwrap_or_else(|e| e.into_inner()) = None; }

/// The numbers the autopilot taught itself (style 4); the normal style until it has learned.
static LEARNED: RwLock<Option<AutoCfg>> = RwLock::new(None);
pub fn learned() -> AutoCfg { (*LEARNED.read().unwrap_or_else(|e| e.into_inner())).unwrap_or_else(cfg) }
/// Sets the learned numbers (the searched six of `train`, in its order), clamped into the search's bounds; all-NaN or short input leaves them alone.
pub fn set_learned(g: &[f32]) { if let Some(c) = taught(g, super::train::Mode::Safe) { *LEARNED.write().unwrap_or_else(|e| e.into_inner()) = Some(c); } }
pub fn reset_learned() { *LEARNED.write().unwrap_or_else(|e| e.into_inner()) = None; }

/// The Max Attack style (5): the numbers the autopilot taught itself for it, or the file's `attack` style until then.
static ATTACK: RwLock<Option<AutoCfg>> = RwLock::new(None);
pub fn attack() -> AutoCfg { (*ATTACK.read().unwrap_or_else(|e| e.into_inner())).unwrap_or_else(|| styles().attack) }
pub fn set_attack(g: &[f32]) { if let Some(c) = taught(g, super::train::Mode::Attack) { *ATTACK.write().unwrap_or_else(|e| e.into_inner()) = Some(c); } }
pub fn reset_attack() { *ATTACK.write().unwrap_or_else(|e| e.into_inner()) = None; }

/// The numbers of a training (the searched seven, in their own units) clamped into the bounds of the mode; `None` for short or non-finite input.
fn taught(g: &[f32], m: super::train::Mode) -> Option<AutoCfg> {
    if g.len() < super::train::N || g.iter().take(super::train::N).any(|x| !x.is_finite()) { return None; }
    let (l, h) = (super::train::lo(m), super::train::hi(m));
    let a: [f32; super::train::N] = std::array::from_fn(|i| if h[i] > l[i] { (g[i].clamp(l[i], h[i]) - l[i]) / (h[i] - l[i]) } else { 0.0 });
    Some(super::train::to_cfg(match m { super::train::Mode::Safe => cfg(), super::train::Mode::Attack => styles().attack }, &a, m))
}

/// The player's own numbers (style 3), set from the settings; the normal style until they are set.
static CUSTOM: RwLock<Option<AutoCfg>> = RwLock::new(None);
pub fn custom() -> AutoCfg { (*CUSTOM.read().unwrap_or_else(|e| e.into_inner())).unwrap_or_else(cfg) }
/// Sets the custom style from the settings; every number is clamped into its allowed range.
pub fn set_custom(lat: f32, brake: f32, offset: f32, speed_cap: f32, slide: f32, edge: f32) {
    let mut c = cfg();
    let f = |v: f32, lo: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
    c.lat_factor = f(lat, 0.2, 3.0, c.lat_factor); c.brake_factor = f(brake, 0.2, 1.5, c.brake_factor); c.offset_max = f(offset, 0.0, 0.9, c.offset_max);
    c.speed_cap = f(speed_cap, 0.3, 1.0, 1.0); c.slide = f(slide, 0.0, 1.0, 0.0); c.edge_margin = f(edge, 0.0, 4.0, c.edge_margin);
    *CUSTOM.write().unwrap_or_else(|e| e.into_inner()) = Some(c);
}
pub fn reset_custom() { *CUSTOM.write().unwrap_or_else(|e| e.into_inner()) = None; }
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
