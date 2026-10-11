//! Stateless helpers for the Android menu (no GPU, no game thread).
use std::path::Path;

use super::flight::AircraftKind;
use super::meta::{kind_id, Profile};
use super::missions::roll;

/// `[money, round, profile_seed, gold_count]` followed by 6 missions x
/// `[kind(0 cargo/1 race/2 rings/3 sling/4 recon), seed, difficulty, length_m, time_limit, reward_base, best_medal, best_time, aircraft(0 none/1 heli/2 drone)]`.
pub fn menu_info(path: &Path, fresh_seed: u64, refresh: bool) -> Vec<f32> {
    let mut p = Profile::load_or_new(path, fresh_seed);
    if refresh { p.round = p.round.wrapping_add(1); let _ = p.save(path); }
    let mut out = vec![p.money as f32, p.round as f32, p.profile_seed as f32, p.gold_count() as f32];
    for m in roll(p.profile_seed, p.round, p.gold_count()) {
        let best = p.results.iter().find(|r| r.seed == m.seed && r.kind == m.kind);
        out.extend_from_slice(&[kind_id(m.kind) as f32, m.seed as f32, m.difficulty as f32, m.length_m,
            m.time_limit, m.reward_base, best.map_or(0.0, |r| r.medal as u8 as f32), best.map_or(0.0, |r| r.best_time),
            match m.aircraft { None => 0.0, Some(AircraftKind::Heli) => 1.0, Some(AircraftKind::Drone) => 2.0 }]);
    }
    out
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
