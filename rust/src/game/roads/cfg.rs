//! The relief numbers of the roads (`rust/roads.json`): crests, ditches, the gravel verge and the roadside rocks.
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrestCfg {
    pub height: (f32, f32), pub length: (f32, f32), pub spacing: (f32, f32), pub series: (u32, u32), pub series_gap: (f32, f32),
    pub min_radius: f32, pub keep_ends: f32, pub asym: f32, pub max_slope: f32, pub causeway_clear: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DitchCfg { pub depth: f32, pub width: f32, pub offset: f32 }
/// Side roads (junctions with dead ends along the stage).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideCfg {
    pub count: (u32, u32), pub spacing: (f32, f32), pub keep_ends: f32, pub min_radius: f32, pub angle: (f32, f32), pub length: (f32, f32), pub width: f32, pub min_clear: f32,
}
/// The city super-special loop: its size in blocks, the number of detours and the spacing of the check lines (m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CityLoopCfg { pub size: (u32, u32), pub jogs: (u32, u32), pub check_every: f32 }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadsCfg { pub city_loop: CityLoopCfg, pub side: SideCfg, pub crest: CrestCfg, pub ditch_stage: DitchCfg, pub ditch_liaison: DitchCfg, pub verge: f32, pub rock_band: (f32, f32), pub rock_candidates: u32 }

pub const EMBEDDED: &str = include_str!("../../../roads.json");

impl RoadsCfg {
    pub fn from_json(text: &str) -> Result<RoadsCfg, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("roads file is not valid JSON: {e}"))?;
        let obj = |o: &serde_json::Value, name: &str| -> Result<serde_json::Value, String> { o.get(name).filter(|x| x.is_object()).cloned().ok_or_else(|| format!("roads: missing object \"{name}\"")) };
        let num = |o: &serde_json::Value, ctx: &str, key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = o.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("{ctx}: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("{ctx}: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        let pair = |o: &serde_json::Value, ctx: &str, key: &str, lo: f64, hi: f64| -> Result<(f32, f32), String> {
            let a = o.get(key).and_then(|k| k.as_array()).filter(|a| a.len() == 2).ok_or_else(|| format!("{ctx}: \"{key}\" must be [min, max]"))?;
            let (x, y) = (a[0].as_f64().filter(|x| x.is_finite()), a[1].as_f64().filter(|x| x.is_finite()));
            let (Some(x), Some(y)) = (x, y) else { return Err(format!("{ctx}: \"{key}\" must be two numbers")) };
            if x < lo || y > hi || x > y { return Err(format!("{ctx}: \"{key}\" must be [min, max] inside {lo}..{hi} (got [{x}, {y}])")); }
            Ok((x as f32, y as f32))
        };
        let c = obj(&v, "crest")?;
        let series = pair(&c, "crest", "series", 1.0, 12.0)?;
        let crest = CrestCfg {
            height: pair(&c, "crest", "height", 0.0, 6.0)?, length: pair(&c, "crest", "length", 8.0, 200.0)?, spacing: pair(&c, "crest", "spacing", 40.0, 1000.0)?,
            series: (series.0 as u32, series.1 as u32), series_gap: pair(&c, "crest", "series_gap", 100.0, 5000.0)?,
            min_radius: num(&c, "crest", "min_radius", 20.0, 1000.0)?, keep_ends: num(&c, "crest", "keep_ends", 0.0, 1000.0)?, asym: num(&c, "crest", "asym", 0.0, 0.8)?,
            max_slope: num(&c, "crest", "max_slope", 0.01, 0.3)?, causeway_clear: num(&c, "crest", "causeway_clear", 0.0, 10.0)?,
        };
        let d = obj(&v, "ditch")?;
        let ditch = |name: &str| -> Result<DitchCfg, String> {
            let o = obj(&d, name)?; let ctx = format!("ditch.{name}");
            Ok(DitchCfg { depth: num(&o, &ctx, "depth", 0.0, 3.0)?, width: num(&o, &ctx, "width", 0.2, 6.0)?, offset: num(&o, &ctx, "offset", 0.0, 6.0)? })
        };
        let verge = num(&obj(&v, "verge")?, "verge", "gravel_width", 0.0, 2.0)?;
        let r = obj(&v, "rocks")?;
        let sd = obj(&v, "side")?;
        let count = pair(&sd, "side", "count", 0.0, 20.0)?;
        let side = SideCfg {
            count: (count.0 as u32, count.1 as u32), spacing: pair(&sd, "side", "spacing", 100.0, 5000.0)?, keep_ends: num(&sd, "side", "keep_ends", 0.0, 2000.0)?,
            min_radius: num(&sd, "side", "min_radius", 20.0, 1000.0)?, angle: pair(&sd, "side", "angle", 30.0, 150.0)?, length: pair(&sd, "side", "length", 50.0, 2000.0)?,
            width: num(&sd, "side", "width", 3.0, 12.0)?, min_clear: num(&sd, "side", "min_clear", 8.0, 200.0)?,
        };
        let cl = obj(&v, "city_loop")?;
        let (size, jogs) = (pair(&cl, "city_loop", "size", 3.0, 9.0)?, pair(&cl, "city_loop", "jogs", 0.0, 4.0)?);
        let city_loop = CityLoopCfg { size: (size.0 as u32, size.1 as u32), jogs: (jogs.0 as u32, jogs.1 as u32), check_every: num(&cl, "city_loop", "check_every", 100.0, 5000.0)? };
        Ok(RoadsCfg { city_loop, side, crest, ditch_stage: ditch("stage")?, ditch_liaison: ditch("liaison")?, verge, rock_band: pair(&r, "rocks", "band", 2.0, 30.0)?, rock_candidates: num(&r, "rocks", "candidates", 0.0, 200.0)? as u32 })
    }
}

static CFG: RwLock<Option<RoadsCfg>> = RwLock::new(None);

/// The active numbers: the embedded file unless an override was loaded.
pub fn cfg() -> RoadsCfg {
    if let Some(c) = *CFG.read().unwrap_or_else(|e| e.into_inner()) { return c; }
    let c = RoadsCfg::from_json(EMBEDDED).expect("the embedded roads.json is invalid");
    *CFG.write().unwrap_or_else(|e| e.into_inner()) = Some(c);
    c
}
pub fn reset_cfg() { *CFG.write().unwrap_or_else(|e| e.into_inner()) = None; }

/// Replaces the numbers with the file's when it exists and parses; a bad file is logged and ignored.
pub fn load_override(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    match RoadsCfg::from_json(&text) {
        Ok(c) => { log::info!("roads numbers loaded from {path:?}"); *CFG.write().unwrap_or_else(|e| e.into_inner()) = Some(c); }
        Err(e) => log::error!("roads file {path:?} ignored: {e}"),
    }
}

#[cfg(test)]
#[path = "cfg_tests.rs"]
mod tests;
