//! Player profile: money and the garage, saved as JSON (atomic write, corrupt files kept as .bak).
use std::path::Path;

use serde_json::{json, Value};

use super::garage::{default_levels, Garage, Upgrade, MAX_LEVEL, MAX_WHEEL_SET, MAX_WING_ANGLE, NLEVELS};
use super::vehicle::VehicleSpec;

pub const PROFILE_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct Profile { pub version: u32, pub money: u32, pub garage: Garage }

/// Car ids that were renamed: the 2.11.0 profile files used `peugeot206` for what is now `vulpes`.
fn legacy_id(id: &str) -> &str { if id == "peugeot206" { "vulpes" } else { id } }

impl Profile {
    pub fn new() -> Profile { Profile { version: PROFILE_VERSION, money: 0, garage: Garage::new() } }

    pub fn to_json(&self) -> String {
        let cars: serde_json::Map<String, Value> = self.garage.cars.iter()
            .map(|(id, l)| (id.clone(), json!({ "engine": l[0], "tyres": l[1], "suspension": l[2], "brakes": l[3], "splitter": l[4], "wing": l[5], "underbody": l[6], "wing_angle": l[7], "wheel_set": l[8] }))).collect();
        serde_json::to_string_pretty(&json!({ "version": self.version, "money": self.money,
            "garage": { "selected": self.garage.selected, "cars": cars } })).unwrap()
    }

    pub fn from_json(s: &str) -> Result<Profile, String> {
        let v: Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
        let num = |k: &str| v.get(k).and_then(Value::as_u64).ok_or_else(|| format!("bad field {k}"));
        let version = num("version")?;
        if version > PROFILE_VERSION as u64 { return Err(format!("profile version {version} is newer than {PROFILE_VERSION}")); }
        let money = u32::try_from(num("money")?).map_err(|_| "money out of range".to_string())?;
        // the garage is optional: unknown cars are dropped, levels are clamped, free cars are always owned, a bad selection falls back
        let mut garage = Garage::new();
        if let Some(g) = v.get("garage") {
            if let Some(cars) = g.get("cars").and_then(Value::as_object) {
                for (raw_id, l) in cars {
                    let id = legacy_id(raw_id);
                    if VehicleSpec::by_id(id).is_none() { continue; }
                    let lv = |k: &str| l.get(k).and_then(Value::as_u64).unwrap_or(0).min(MAX_LEVEL as u64) as u8;
                    let angle = l.get("wing_angle").and_then(Value::as_u64).map_or(default_levels()[7], |a| a.min(MAX_WING_ANGLE as u64) as u8);
                    let set = l.get("wheel_set").and_then(Value::as_u64).map_or(default_levels()[8], |a| a.min(MAX_WHEEL_SET as u64) as u8);
                    let levels: [u8; NLEVELS] = [lv("engine"), lv("tyres"), lv("suspension"), lv("brakes"), lv("splitter"), lv("wing"), lv("underbody"), angle, set];
                    match garage.cars.iter_mut().find(|(i, _)| i == id) { Some(c) => c.1 = levels, None => garage.cars.push((id.to_string(), levels)) }
                }
            }
            if let Some(sel) = g.get("selected").and_then(Value::as_str) { let sel = legacy_id(sel); if garage.owns(sel) { garage.selected = sel.to_string(); } }
        }
        Ok(Profile { version: PROFILE_VERSION, money, garage })
    }

    pub fn owns(&self, id: &str) -> bool { self.garage.owns(id) }
    pub fn levels(&self, id: &str) -> [u8; NLEVELS] { self.garage.levels(id) }

    /// Buys a car from the garage list for its price; false (nothing changes) when unknown, owned or too expensive.
    pub fn buy_car(&mut self, id: &str) -> bool {
        let Some(spec) = VehicleSpec::by_id(id) else { return false };
        if self.owns(id) || spec.price == 0 || self.money < spec.price { return false; }
        self.money -= spec.price;
        self.garage.cars.push((id.to_string(), default_levels()));
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

    /// Sets the wing angle (0 low drag, 1 mid, 2 high downforce) of an owned car; free.
    pub fn set_wing_angle(&mut self, id: &str, angle: u8) -> bool {
        if angle > MAX_WING_ANGLE { return false; }
        let Some(entry) = self.garage.cars.iter_mut().find(|(i, _)| i == id) else { return false };
        entry.1[7] = angle;
        true
    }

    /// Sets the wheel set of an owned car: 0 tarmac, 1 gravel, 2 snow.
    pub fn set_wheel_set(&mut self, id: &str, set: u8) -> bool {
        if set > MAX_WHEEL_SET { return false; }
        let Some(entry) = self.garage.cars.iter_mut().find(|(i, _)| i == id) else { return false };
        entry.1[8] = set;
        true
    }

    /// The selected car with its upgrades applied (the off-roader if the selection is somehow missing).
    pub fn selected_spec(&self) -> VehicleSpec {
        let id = &self.garage.selected;
        let spec = VehicleSpec::by_id(id).unwrap_or_else(VehicleSpec::offroader);
        super::garage::apply(&spec, self.levels(id))
    }

    /// A car of the vehicles file with this profile's upgrades and wheel set applied (`None` for an unknown id): what the garage shows on the turntable.
    pub fn spec_of(&self, id: &str) -> Option<VehicleSpec> { VehicleSpec::by_id(id).map(|s| super::garage::apply(&s, self.levels(id))) }

    /// Writes to `<path>.tmp` then renames over `path`.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() { std::fs::create_dir_all(d)?; }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(&tmp, path)
    }

    /// Missing file: a fresh profile (saved). Unreadable or newer file: moved to `.bak`, then a fresh profile. Old files with
    /// mission results (versions up to 2.13) load fine: the results are ignored.
    pub fn load_or_new(path: &Path) -> Profile {
        if let Ok(text) = std::fs::read_to_string(path) {
            match Profile::from_json(&text) {
                Ok(p) => return p,
                Err(e) => { log::warn!("profile unusable ({e}); keeping it as .bak"); let _ = std::fs::rename(path, path.with_extension("json.bak")); }
            }
        }
        let p = Profile::new();
        let _ = p.save(path);
        p
    }
}

#[cfg(test)]
#[path = "meta_tests.rs"]
mod tests;
