//! Stateless helpers for the Android menu (no GPU, no game thread).
use std::path::Path;

use super::meta::{kind_id, Profile};
use super::garage::Upgrade;
use super::missions::roll;
use super::vehicle::VehicleSpec;

/// `[money, round, profile_seed, gold_count]` followed by 6 missions x
/// `[kind(0 cargo/1 race), seed, difficulty, length_m, time_limit, reward_base, best_medal, best_time, urban(0/1)]`.
pub fn menu_info(path: &Path, fresh_seed: u64, refresh: bool) -> Vec<f32> {
    let mut p = Profile::load_or_new(path, fresh_seed);
    if refresh { p.round = p.round.wrapping_add(1); let _ = p.save(path); }
    let mut out = vec![p.money as f32, p.round as f32, p.profile_seed as f32, p.gold_count() as f32];
    for m in roll(p.profile_seed, p.round, p.gold_count()) {
        let best = p.results.iter().find(|r| r.seed == m.seed && r.kind == m.kind);
        out.extend_from_slice(&[kind_id(m.kind) as f32, m.seed as f32, m.difficulty as f32, m.length_m,
            m.time_limit, m.reward_base, best.map_or(0.0, |r| r.medal as u8 as f32), best.map_or(0.0, |r| r.best_time),
            if m.urban { 1.0 } else { 0.0 }]);
    }
    out
}

/// `[money, selected_index, car_count]` then per car (file order, sorted by id) `[owned(0/1), price, upgrade_cost, engine, tyres, suspension, brakes]`.
pub fn garage_info(path: &Path, fresh_seed: u64) -> Vec<f32> {
    let p = Profile::load_or_new(path, fresh_seed);
    let cars = VehicleSpec::all();
    let sel = cars.iter().position(|(id, _)| *id == p.garage.selected).unwrap_or(0);
    let mut out = vec![p.money as f32, sel as f32, cars.len() as f32];
    for (id, s) in &cars {
        let l = p.levels(id);
        out.extend_from_slice(&[p.owns(id) as u8 as f32, s.price as f32, s.upgrade_cost as f32, l[0] as f32, l[1] as f32, l[2] as f32, l[3] as f32]);
    }
    out
}

/// Loads the profile, applies `f` and saves when it succeeded; false for a bad car index or a refused action.
fn garage_act(path: &Path, index: usize, f: impl FnOnce(&mut Profile, &str) -> bool) -> bool {
    let cars = VehicleSpec::all();
    let Some((id, _)) = cars.get(index) else { return false };
    let mut p = Profile::load_or_new(path, 1);
    if !f(&mut p, id) { return false; }
    p.save(path).is_ok()
}
pub fn garage_buy_car(path: &Path, index: usize) -> bool { garage_act(path, index, |p, id| p.buy_car(id)) }
pub fn garage_select(path: &Path, index: usize) -> bool { garage_act(path, index, |p, id| p.select(id)) }
pub fn garage_buy_upgrade(path: &Path, index: usize, upgrade: usize) -> bool {
    let Some(up) = Upgrade::from_index(upgrade) else { return false };
    garage_act(path, index, |p, id| p.buy_upgrade(id, up))
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
