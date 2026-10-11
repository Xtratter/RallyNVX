//! Fixed-step game loop shared by the Android and desktop front ends.
use super::world::World;

pub const STEP: f32 = 1.0 / 60.0;

/// Level seeds are 24-bit so the HUD (an f32) shows the exact value and `--game <seed>` replays it.
pub fn level_seed(raw: u64) -> u64 { raw & 0xFF_FFFF }

/// Accumulates elapsed time (capped at 0.1 s) and returns how many fixed steps to run now.
pub fn step_budget(acc: &mut f32, dt: f32, max_steps: usize) -> usize {
    if !dt.is_finite() || dt <= 0.0 { return 0; }
    *acc += dt.min(0.1);
    let mut n = 0;
    while *acc >= STEP && n < max_steps { *acc -= STEP; n += 1; }
    if *acc >= STEP { *acc = 0.0; }
    n
}

pub const STAT_LEN: usize = 64;

/// Flat state for the UI. `[0]` fps, `[1]` km/h, `[2]` seed (24-bit), `[3]` frame ms, `[4]` phase (0 free drive, 1 ready,
/// 2 running, 3 done), `[5]` time, `[6]` time limit, `[7]` integrity, `[8]` distance to the target, `[9]` bearing to the
/// target (rad, relative to the camera heading, positive = right), `[10]` next checkpoint, `[11]` checkpoint total,
/// `[12]` result (0 none, 1 passed, 2 failed), `[13]` medal 0..3, `[14]` payout, `[15]` failure (1 time, 2 cargo,
/// 3 abandoned, 4 crashed), `[16]` money, `[17]` mission kind (0 cargo, 1 race, 2 rings, 3 sling, 4 recon), `[18]` run counter
/// (bumped on every mission start), `[19]` altitude above ground, `[20]` aircraft (0 none, 1 heli, 2 drone), `[21]` rope taut,
/// `[22]` target height above the aircraft.
pub fn fill_state(out: &mut [f32; STAT_LEN], w: &World, money: u32, payout: u32, fps: f32, frame_ms: f32, seed: u64, run: u32) {
    use crate::game::flight::AircraftKind;
    use crate::game::missions::MissionKind;
    use crate::game::run::{bearing, Failure, Outcome, Phase};
    *out = [0.0; STAT_LEN];
    out[0] = fps; out[1] = w.vehicle.vel.len() * 3.6; out[2] = (seed & 0xFF_FFFF) as f32; out[3] = frame_ms; out[16] = money as f32; out[18] = run as f32;
    if let Some(a) = &w.aircraft {
        out[1] = a.vel.len() * 3.6;
        out[19] = a.pos.y - w.terrain.height(a.pos.x, a.pos.z);
        out[20] = if a.kind == AircraftKind::Heli { 1.0 } else { 2.0 };
        if let Some(s) = &w.sling { out[21] = if s.attached && s.rope_taut(a.pos - crate::game::math::v3(0.0, 0.8, 0.0)) { 1.0 } else { 0.0 }; }
    }
    let Some(r) = w.run_info() else { return };
    out[4] = match r.phase { Phase::Ready => 1.0, Phase::Running => 2.0, Phase::Done => 3.0 };
    out[5] = r.time; out[6] = r.mission.time_limit; out[7] = r.integrity;
    out[10] = r.next as f32; out[11] = r.total as f32;
    out[17] = match r.mission.kind { MissionKind::Cargo => 0.0, MissionKind::Race => 1.0, MissionKind::Rings => 2.0, MissionKind::Sling => 3.0, MissionKind::Recon => 4.0 };
    if let Some(t) = r.target {
        let me = w.player_pos();
        out[8] = (t - me).len();
        let h = (w.cam.target - w.cam.pos).norm();
        out[9] = bearing(h.x, h.z, me, t);
        out[22] = t.y - me.y;
    }
    match r.outcome {
        Some(Outcome::Passed { medal, .. }) => { out[12] = 1.0; out[13] = medal as u8 as f32; out[14] = payout as f32; }
        Some(Outcome::Failed(f)) => { out[12] = 2.0; out[15] = match f { Failure::TimeUp => 1.0, Failure::CargoDestroyed => 2.0, Failure::Abandoned => 3.0, Failure::Crashed => 4.0 }; }
        None => {}
    }
}

#[cfg(any(target_os = "android", feature = "desktop"))]
pub use game::Game;

#[cfg(any(target_os = "android", feature = "desktop"))]
mod game {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use super::{fill_state, step_budget, STAT_LEN, STEP};
    use crate::game::{flight::FlightInput, input::Input, meta::Profile, missions::{roll, Mission}, run::{Outcome, Phase}, world::World};
    use crate::raster::Raster;

    /// World + renderer + fixed-step loop + optional profile and mission list. State layout: see `fill_state`.
    pub struct Game {
        world: World, raster: Raster, pub input: Input, pub flight: FlightInput,
        acc: f32, last: Instant, fps_n: u32, fps_t: Instant, fps: f32, seed: u64, frame_ms: f32,
        stats: Arc<Mutex<[f32; STAT_LEN]>>,
        profile: Option<Profile>, profile_path: Option<PathBuf>, missions: Vec<Mission>,
        current: Option<usize>, paused: bool, recorded: bool, payout: u32, run_id: u32,
    }

    impl Game {
        pub fn new(raster: Raster, seed: u64, stats: Arc<Mutex<[f32; STAT_LEN]>>) -> Game {
            let seed = super::level_seed(seed);
            Game { world: World::new(seed), raster, input: Input::default(), flight: FlightInput::default(), acc: 0.0, last: Instant::now(),
                fps_n: 0, fps_t: Instant::now(), fps: 0.0, seed, frame_ms: 0.0, stats,
                profile: None, profile_path: None, missions: Vec::new(), current: None, paused: false, recorded: false, payout: 0, run_id: 0 }
        }

        pub fn resize(&mut self, w: u32, h: u32) { self.raster.resize(w, h); }

        /// Quality level 0/1/2 (Low/Medium/High) sets the city draw distance: 300 / 400 / 450 m.
        pub fn set_quality(&mut self, level: i32) { self.raster.set_city_radius([300.0, 400.0, 450.0][level.clamp(0, 2) as usize]); }
        pub fn set_city_radius(&mut self, r: f32) { self.raster.set_city_radius(r); }
        pub fn place_near_city(&mut self) -> bool { self.world.place_near_city() }
        pub fn place_in_city(&mut self) -> bool { self.world.place_in_city() }

        pub fn new_level(&mut self, seed: u64) {
            let seed = super::level_seed(seed);
            self.world = World::new(seed); self.seed = seed; self.input = Input::default(); self.acc = 0.0;
            self.current = None; self.recorded = false; self.payout = 0;
        }

        /// Loads (or creates) the profile at `path` and rolls the current mission list.
        pub fn with_profile(&mut self, path: PathBuf, fresh_seed: u64) {
            let p = Profile::load_or_new(&path, fresh_seed);
            self.missions = roll(p.profile_seed, p.round, p.gold_count());
            self.profile = Some(p);
            self.profile_path = Some(path);
        }

        pub fn start_mission(&mut self, index: usize) {
            let Some(m) = self.missions.get(index).cloned() else { return };
            self.seed = m.seed;
            self.world = World::for_mission(m);
            self.input = Input::default(); self.flight = FlightInput::default();
            self.acc = 0.0; self.recorded = false; self.payout = 0; self.current = Some(index); self.paused = false; self.run_id = self.run_id.wrapping_add(1);
        }
        pub fn restart_mission(&mut self) { if let Some(i) = self.current { self.start_mission(i); } }
        pub fn abort_mission(&mut self) { self.world.abandon_mission(); }
        pub fn set_paused(&mut self, p: bool) { self.paused = p; self.last = Instant::now(); self.acc = 0.0; }

        pub fn speed_kmh(&self) -> f32 { self.world.vehicle.vel.len() * 3.6 }
        pub fn fps(&self) -> f32 { self.fps }
        pub fn seed(&self) -> u64 { self.seed }
        pub fn stats(&self) -> [f32; STAT_LEN] { *self.stats.lock().unwrap() }

        /// A passed mission is recorded once (money, best medal, saved); failures are never recorded.
        fn record_if_done(&mut self) {
            if self.recorded { return; }
            let Some(r) = self.world.run_info() else { return };
            if let (Some(Outcome::Passed { medal, time, .. }), Some(p)) = (r.outcome, self.profile.as_mut()) {
                self.payout = p.record(r.mission, medal, time);
                if let Some(path) = &self.profile_path { let _ = p.save(path); }
                self.recorded = true;
            } else if r.phase == Phase::Done { self.recorded = true; }
        }

        pub fn frame(&mut self) {
            let now = Instant::now();
            let dt = (now - self.last).as_secs_f32();
            self.last = now;
            if !self.paused {
                for _ in 0..step_budget(&mut self.acc, dt, 6) {
                    self.world.step_with(&self.input, &self.flight, STEP);
                    self.input.reset = false;
                }
                self.record_if_done();
            }
            let t0 = Instant::now();
            self.raster.render(&mut self.world);
            self.frame_ms = t0.elapsed().as_secs_f32() * 1000.0;
            self.fps_n += 1;
            let el = self.fps_t.elapsed().as_secs_f32();
            if el >= 0.5 { self.fps = self.fps_n as f32 / el; self.fps_n = 0; self.fps_t = Instant::now(); }
            let mut s = [0.0f32; STAT_LEN];
            fill_state(&mut s, &self.world, self.profile.as_ref().map_or(0, |p| p.money), self.payout, self.fps, self.frame_ms, self.seed, self.run_id);
            *self.stats.lock().unwrap() = s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn hitch_is_capped_and_remainder_kept() {
        let mut acc = 0.0;
        let n = step_budget(&mut acc, 5.0, 10);
        assert!((5..=6).contains(&n), "{n}"); // 5 s hitch capped to 0.1 s
        let mut acc = 0.0;
        assert_eq!(step_budget(&mut acc, STEP * 0.5, 10), 0);
        assert_eq!(step_budget(&mut acc, STEP * 0.6, 10), 1);
        assert!(acc < STEP);
    }
    #[test] fn negative_or_nan_dt_runs_nothing() {
        let mut acc = 0.0;
        assert_eq!(step_budget(&mut acc, -1.0, 10), 0);
        assert_eq!(step_budget(&mut acc, f32::NAN, 10), 0);
        assert!(acc.is_finite());
    }
}

#[cfg(test)]
mod seed_tests {
    use super::*;
    #[test] fn level_seed_fits_the_hud_and_is_idempotent() {
        for raw in [0u64, 42, 0xFF_FFFF, 0x100_0000, u64::MAX, 1_234_567_890_123] {
            let s = level_seed(raw);
            assert!(s <= 0xFF_FFFF, "{s}");
            assert_eq!(level_seed(s), s);
            assert_eq!(s as f32 as u64, s, "exactly representable as the f32 shown in the HUD");
        }
    }
}

#[cfg(test)]
#[path = "app_state_tests.rs"]
mod state_tests;
