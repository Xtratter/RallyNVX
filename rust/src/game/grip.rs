//! How the ground grips: multipliers per surface and the loose gravel layer of the special stage.
use super::roads::{RoadKind, RoadNet};
use super::surface::{surface_at, Surface};
use super::terrain::Terrain;

/// Multipliers on the car's own `mu`, `lateral_k` and `rolling` resistance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceGrip { pub mu: f32, pub lateral: f32, pub rolling: f32 }

/// How a surface's dust looks and how much of it the wheels throw (see `game/dust.rs`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustSpec { pub color: [f32; 3], pub roll: f32, pub slide: f32, pub size: f32, pub life: f32 }

/// The numbers of `rust/surfaces.json`: one grip per surface (gravel's `mu` is the packed value), the loose gravel's
/// own `mu` and how fast the wheels pack it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GripTable { surfaces: [SurfaceGrip; 7], dust: [DustSpec; 7], pub gravel_loose_mu: f32, pub wear_per_metre: f32, pub dust_cap_normal: usize, pub dust_cap_lots: usize, pub wind_speed: f32, pub wind_gust: f32 }

/// The embedded default file.
pub const EMBEDDED: &str = include_str!("../../surfaces.json");

fn index(s: Surface) -> usize { match s { Surface::Grass => 0, Surface::Dirt => 1, Surface::Sand => 2, Surface::Rock => 3, Surface::Gravel => 4, Surface::Asphalt => 5, Surface::Water => 6 } }

impl GripTable {
    pub fn from_json(text: &str) -> Result<GripTable, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("surfaces file is not valid JSON: {e}"))?;
        let num = |obj: &serde_json::Value, name: &str, key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = obj.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("{name}: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("{name}: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        let section = |name: &str| v.get(name).filter(|o| o.is_object()).ok_or_else(|| format!("surfaces: missing object \"{name}\""));
        let plain = |name: &str| -> Result<SurfaceGrip, String> {
            let o = section(name)?;
            Ok(SurfaceGrip { mu: num(o, name, "mu", 0.1, 2.0)?, lateral: num(o, name, "lateral", 0.1, 2.0)?, rolling: num(o, name, "rolling", 0.1, 10.0)? })
        };
        let gravel = section("gravel")?;
        let (loose, packed) = (num(gravel, "gravel", "mu_loose", 0.1, 2.0)?, num(gravel, "gravel", "mu_packed", 0.1, 2.0)?);
        let gravel_grip = SurfaceGrip { mu: packed, lateral: num(gravel, "gravel", "lateral", 0.1, 2.0)?, rolling: num(gravel, "gravel", "rolling", 0.1, 10.0)? };
        let wear = num(section("loose_layer")?, "loose_layer", "wear_per_metre", 0.0, 5.0)?;
        let dust_of = |name: &str| -> Result<DustSpec, String> {
            let o = section(name)?.get("dust").filter(|d| d.is_object()).ok_or_else(|| format!("{name}: missing object \"dust\""))?;
            let ctx = format!("{name}.dust");
            let col = o.get("color").and_then(|c| c.as_array()).filter(|c| c.len() == 3).ok_or_else(|| format!("{ctx}: \"color\" must be three numbers"))?;
            let mut color = [0.0f32; 3];
            for (i, c) in col.iter().enumerate() { let x = c.as_f64().filter(|x| (0.0..=1.0).contains(x)).ok_or_else(|| format!("{ctx}: \"color\" values must be between 0 and 1"))?; color[i] = x as f32; }
            Ok(DustSpec { color, roll: num(o, &ctx, "roll", 0.0, 1.0)?, slide: num(o, &ctx, "slide", 0.0, 2.0)?, size: num(o, &ctx, "size", 0.2, 3.0)?, life: num(o, &ctx, "life", 0.3, 6.0)? })
        };
        let dust_top = section("dust")?;
        let cap = |key: &str| -> Result<usize, String> { Ok(num(dust_top, "dust", key, 0.0, 4096.0)? as usize) };
        let wind_top = section("wind")?;
        Ok(GripTable {
            surfaces: [plain("grass")?, plain("dirt")?, plain("sand")?, plain("rock")?, gravel_grip, plain("asphalt")?, plain("water")?],
            dust: [dust_of("grass")?, dust_of("dirt")?, dust_of("sand")?, dust_of("rock")?, dust_of("gravel")?, dust_of("asphalt")?, dust_of("water")?],
            gravel_loose_mu: loose, wear_per_metre: wear,
            dust_cap_normal: cap("cap_normal")?, dust_cap_lots: cap("cap_lots")?, wind_speed: num(wind_top, "wind", "speed", 0.0, 20.0)?, wind_gust: num(wind_top, "wind", "gust", 0.0, 1.0)?,
        })
    }

    pub fn dust(&self, s: Surface) -> DustSpec { self.dust[index(s)] }

    /// The grip of a surface; `loose` (0 packed .. 1 loose, only used on gravel) interpolates the gravel grip.
    pub fn grip(&self, s: Surface, loose: f32) -> SurfaceGrip {
        let mut g = self.surfaces[index(s)];
        if s == Surface::Gravel {
            let l = if loose.is_finite() { loose.clamp(0.0, 1.0) } else { 1.0 };
            g.mu += (self.gravel_loose_mu - g.mu) * l;
        }
        g
    }
}

/// The active table: the embedded file unless an override was loaded.
static TABLE: std::sync::RwLock<Option<GripTable>> = std::sync::RwLock::new(None);

pub fn table() -> GripTable {
    if let Some(t) = *TABLE.read().unwrap_or_else(|e| e.into_inner()) { return t; }
    let t = GripTable::from_json(EMBEDDED).expect("the embedded surfaces.json is invalid");
    *TABLE.write().unwrap_or_else(|e| e.into_inner()) = Some(t);
    t
}

/// Back to the embedded numbers (a new game start does this before looking for an override file).
pub fn reset_table() { *TABLE.write().unwrap_or_else(|e| e.into_inner()) = None; }

/// Replaces the table with the file's numbers when it exists and parses; a bad file is logged and ignored.
pub fn load_override(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    match GripTable::from_json(&text) {
        Ok(t) => { log::info!("surface table loaded from {path:?}"); *TABLE.write().unwrap_or_else(|e| e.into_inner()) = Some(t); }
        Err(e) => log::error!("surfaces file {path:?} ignored: {e}"),
    }
}

/// The grip of a surface from the active table.
pub fn grip(s: Surface, loose: f32) -> SurfaceGrip { table().grip(s, loose) }

pub const CELL_S: f32 = 1.0;
pub const CELL_X: f32 = 0.5;
pub const HALF_W: f32 = 6.0;
const NX: usize = (2.0 * HALF_W / CELL_X) as usize;

/// Looseness (1 = untouched loose gravel .. 0 = packed) of the cells along every stage road.
#[derive(Clone, Debug, Default)]
pub struct LooseMap { roads: Vec<Vec<f32>> }

impl LooseMap {
    pub fn empty() -> LooseMap { LooseMap { roads: Vec::new() } }
    pub fn new(net: &RoadNet) -> LooseMap {
        LooseMap { roads: net.roads.iter().map(|r| if r.kind.is_gravel() { vec![1.0; ((r.length() / CELL_S) as usize + 2) * NX] } else { Vec::new() }).collect() }
    }
    fn index(&self, road: usize, s: f32, side: f32) -> Option<usize> {
        let cells = self.roads.get(road)?;
        if cells.is_empty() || !s.is_finite() || !side.is_finite() || s < 0.0 || side.abs() >= HALF_W { return None; }
        let (i, j) = ((s / CELL_S) as usize, ((side + HALF_W) / CELL_X) as usize);
        let k = i * NX + j.min(NX - 1);
        if k < cells.len() { Some(k) } else { None }
    }
    pub fn loose_at(&self, road: usize, s: f32, side: f32) -> f32 { self.index(road, s, side).map_or(1.0, |k| self.roads[road][k]) }
    /// A wheel at (s, side) moving at `speed` m/s for `dt` s packs the cells under it (about 0.5 per metre travelled).
    pub fn wear(&mut self, road: usize, s: f32, side: f32, speed: f32, dt: f32) {
        if !speed.is_finite() || !dt.is_finite() { return; }
        let amount = (table().wear_per_metre * speed.abs() * dt).clamp(0.0, 1.0);
        for off in [-0.25f32, 0.25] {   // the tyre's width: two half-metre cells
            if let Some(k) = self.index(road, s, side + off) { let c = &mut self.roads[road][k]; *c = (*c - amount * 0.5).clamp(0.0, 1.0); }
        }
    }
}

/// The surface at a point, its looseness and, on a gravel road, the (road, s, side) cell: what a wheel contact needs.
pub fn ground_at(t: &Terrain, map: &LooseMap, x: f32, z: f32) -> (Surface, f32, Option<(usize, f32, f32)>) {
    let surface = surface_at(t, x, z);
    if surface == Surface::Gravel {
        if let Some(h) = t.road_hit(x, z) { return (surface, map.loose_at(h.road, h.s, h.side), Some((h.road, h.s, h.side))); }
    }
    (surface, 1.0, None)
}

#[cfg(test)]
#[path = "grip_tests.rs"]
mod tests;
