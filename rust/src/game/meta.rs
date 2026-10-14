//! Player profile: money, mission results, saved as JSON (atomic write, corrupt files kept as .bak).
use std::path::Path;

use serde_json::{json, Value};

use super::garage::{Garage, Upgrade, MAX_LEVEL};
use super::medals::{payout, Medal};
use super::vehicle::VehicleSpec;
use super::missions::{Mission, MissionKind};

pub const PROFILE_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct MissionResult { pub seed: u64, pub kind: MissionKind, pub medal: Medal, pub best_time: f32 }

#[derive(Clone, Debug)]
pub struct Profile { pub version: u32, pub money: u32, pub profile_seed: u64, pub round: u32, pub results: Vec<MissionResult>, pub garage: Garage }

pub(super) fn kind_id(k: MissionKind) -> u64 { match k { MissionKind::Cargo => 0, MissionKind::Race => 1, MissionKind::Rings => 2, MissionKind::Sling => 3, MissionKind::Recon => 4, MissionKind::Rescue => 5 } }

/// Car ids that were renamed: the 2.11.0 profile files used `peugeot206` for what is now `vulpes`.
fn legacy_id(id: &str) -> &str { if id == "peugeot206" { "vulpes" } else { id } }

impl Profile {
    pub fn new(seed: u64) -> Profile { Profile { version: PROFILE_VERSION, money: 0, profile_seed: seed, round: 0, results: Vec::new(), garage: Garage::new() } }

    pub fn to_json(&self) -> String {
        let results: Vec<Value> = self.results.iter().map(|r| json!({ "seed": r.seed, "kind": kind_id(r.kind), "medal": r.medal as u8, "best_time": r.best_time })).collect();
        let cars: serde_json::Map<String, Value> = self.garage.cars.iter()
            .map(|(id, l)| (id.clone(), json!({ "engine": l[0], "tyres": l[1], "suspension": l[2], "brakes": l[3] }))).collect();
        serde_json::to_string_pretty(&json!({ "version": self.version, "money": self.money, "profile_seed": self.profile_seed, "round": self.round, "results": results,
            "garage": { "selected": self.garage.selected, "cars": cars } })).unwrap()
    }

    pub fn from_json(s: &str) -> Result<Profile, String> {
        let v: Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
        let num = |k: &str| v.get(k).and_then(Value::as_u64).ok_or_else(|| format!("bad field {k}"));
        let version = num("version")?;
        if version > PROFILE_VERSION as u64 { return Err(format!("profile version {version} is newer than {PROFILE_VERSION}")); }
        let money = u32::try_from(num("money")?).map_err(|_| "money out of range".to_string())?;
        let round = u32::try_from(num("round")?).map_err(|_| "round out of range".to_string())?;
        let profile_seed = num("profile_seed")?;
        let mut results = Vec::new();
        for r in v.get("results").and_then(Value::as_array).ok_or("bad results")? {
            let seed = r.get("seed").and_then(Value::as_u64).ok_or("bad result seed")?;
            let kind = match r.get("kind").and_then(Value::as_u64) { Some(0) => MissionKind::Cargo, Some(1) => MissionKind::Race, Some(2) => MissionKind::Rings, Some(3) => MissionKind::Sling, Some(4) => MissionKind::Recon, Some(5) => MissionKind::Rescue, _ => return Err("bad result kind".into()) };
            let medal = Medal::from_u8(r.get("medal").and_then(Value::as_u64).unwrap_or(0).min(3) as u8);
            let best_time = r.get("best_time").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            results.push(MissionResult { seed, kind, medal, best_time });
        }
        // the garage is optional: unknown cars are dropped, levels are clamped, free cars are always owned, a bad selection falls back
        let mut garage = Garage::new();
        if let Some(g) = v.get("garage") {
            if let Some(cars) = g.get("cars").and_then(Value::as_object) {
                for (raw_id, l) in cars {
                    let id = legacy_id(raw_id);
                    if VehicleSpec::by_id(id).is_none() { continue; }
                    let lv = |k: &str| l.get(k).and_then(Value::as_u64).unwrap_or(0).min(MAX_LEVEL as u64) as u8;
                    let levels = [lv("engine"), lv("tyres"), lv("suspension"), lv("brakes")];
                    match garage.cars.iter_mut().find(|(i, _)| i == id) { Some(c) => c.1 = levels, None => garage.cars.push((id.to_string(), levels)) }
                }
            }
            if let Some(sel) = g.get("selected").and_then(Value::as_str) { let sel = legacy_id(sel); if garage.owns(sel) { garage.selected = sel.to_string(); } }
        }
        Ok(Profile { version: PROFILE_VERSION, money, profile_seed, round, results, garage })
    }

    pub fn owns(&self, id: &str) -> bool { self.garage.owns(id) }
    pub fn levels(&self, id: &str) -> [u8; 4] { self.garage.levels(id) }

    /// Buys a car from the garage list for its price; false (nothing changes) when unknown, owned or too expensive.
    pub fn buy_car(&mut self, id: &str) -> bool {
        let Some(spec) = VehicleSpec::by_id(id) else { return false };
        if self.owns(id) || spec.price == 0 || self.money < spec.price { return false; }
        self.money -= spec.price;
        self.garage.cars.push((id.to_string(), [0; 4]));
        true
    }

    pub fn select(&mut self, id: &str) -> bool {
        if !self.owns(id) { return false; }
        self.garage.selected = id.to_string();
        true
    }

    /// Buys the next level of an upgrade; atomic: money and level change together or not at all.
    pub fn buy_upgrade(&mut self, id: &str, up: Upgrade) -> bool {
        let Some(spec) = VehicleSpec::by_id(id) else { return false };
        let Some(entry) = self.garage.cars.iter_mut().find(|(i, _)| i == id) else { return false };
        let Some(price) = super::garage::upgrade_price(&spec, entry.1[up.index()]) else { return false };
        if self.money < price { return false; }
        self.money -= price;
        entry.1[up.index()] += 1;
        true
    }

    /// The selected car with its upgrades applied (the off-roader if the selection is somehow missing).
    pub fn selected_spec(&self) -> VehicleSpec {
        let id = &self.garage.selected;
        let spec = VehicleSpec::by_id(id).unwrap_or_else(VehicleSpec::offroader);
        super::garage::apply(&spec, self.levels(id))
    }

    pub fn best_medal(&self, seed: u64, kind: MissionKind) -> Medal {
        self.results.iter().find(|r| r.seed == seed && r.kind == kind).map_or(Medal::None, |r| r.medal)
    }
    pub fn gold_count(&self) -> u32 { self.results.iter().filter(|r| r.medal == Medal::Gold).count() as u32 }

    /// Records a finished mission; returns the money paid (also added to `money`, saturating).
    pub fn record(&mut self, m: &Mission, medal: Medal, time: f32) -> u32 {
        let before = self.best_medal(m.seed, m.kind);
        let pay = payout(m.reward_base, medal, before);
        self.money = self.money.saturating_add(pay);
        match self.results.iter_mut().find(|r| r.seed == m.seed && r.kind == m.kind) {
            Some(r) => { if medal > r.medal { r.medal = medal; } if r.best_time == 0.0 || time < r.best_time { r.best_time = time; } }
            None => self.results.push(MissionResult { seed: m.seed, kind: m.kind, medal, best_time: time }),
        }
        pay
    }

    /// Writes to `<path>.tmp` then renames over `path`.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() { std::fs::create_dir_all(d)?; }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(&tmp, path)
    }

    /// Missing file: a fresh profile (saved). Unreadable or newer file: moved to `.bak`, then a fresh profile.
    pub fn load_or_new(path: &Path, fresh_seed: u64) -> Profile {
        if let Ok(text) = std::fs::read_to_string(path) {
            match Profile::from_json(&text) {
                Ok(p) => return p,
                Err(e) => { log::warn!("profile unusable ({e}); keeping it as .bak"); let _ = std::fs::rename(path, path.with_extension("json.bak")); }
            }
        }
        let p = Profile::new(super::app::level_seed(fresh_seed));
        let _ = p.save(path);
        p
    }
}

#[cfg(test)]
#[path = "meta_tests.rs"]
mod tests;
