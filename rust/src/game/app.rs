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

pub const STAT_LEN: usize = 108;

fn surface_code(s: crate::game::surface::Surface) -> u8 {
    use crate::game::surface::Surface::*;
    match s { Grass => 0, Dirt => 1, Sand => 2, Rock => 3, Gravel => 4, Asphalt => 5, Water => 6 }
}

/// Flat state for the UI. `[0]` fps, `[1]` km/h, `[2]` seed (24-bit), `[3]` frame ms, `[16]` money, `[25]` signed forward
/// speed of the car, km/h, `[26]` hybrid render mode active (0/1), `[27]` fell back from hybrid to raster automatically (0/1),
/// `[28]` the surface under the car (0 grass, 1 dirt, 2 sand, 3 rock, 4 gravel, 5 asphalt, 6 water), `[45..77]` the next four pace notes as 4 x 8 numbers (see `Playback::upcoming`), `[42]` the serial of the last pace note, `[43]` metres to the next one, `[44]` the progress along the stage; `[30..41]` the rally run (0 none, 1 countdown, 2 running, 3 finished; time; penalty; countdown digit; stage; next line; lines; last event code, serial and value; assisted; total), `[29]` the autopilot style plus one while it drives (1 calm, 2 normal, 3 aggressive, 4 custom; 0 off);
/// the other slots are zero (kept so the indices stay put).
/// The text of the last pace note (serial, Russian, English) for the UI thread to fetch.
pub static NOTE_TEXT: std::sync::Mutex<(u32, String, String, String)> = std::sync::Mutex::new((0, String::new(), String::new(), String::new()));

pub fn fill_state(out: &mut [f32; STAT_LEN], w: &World, money: u32, fps: f32, frame_ms: f32, seed: u64) {
    *out = [0.0; STAT_LEN];
    out[0] = fps; out[1] = w.vehicle.vel.len() * 3.6; out[2] = (seed & 0xFF_FFFF) as f32; out[3] = frame_ms; out[16] = money as f32;
    out[28] = surface_code(w.vehicle.surface) as f32;
    out[29] = w.autopilot.as_ref().map_or(0.0, |a| a.style() as f32 + 1.0);
    if let Some(r) = w.rally.as_ref() {
        use crate::game::rally::Phase;
        out[30] = match r.run.phase { Phase::Countdown => 1.0, Phase::Running => 2.0, Phase::Finished => 3.0 };
        out[31] = r.run.time; out[32] = r.run.penalty; out[33] = r.run.countdown_digit() as f32; out[34] = r.stage as f32;
        for (i, u) in r.pace.upcoming(3, (5.5 * w.vehicle.speed().max(0.0)).max(110.0)).iter().enumerate() { out[45 + i * 8..53 + i * 8].copy_from_slice(u); }   // the pictogram strip
        out[42] = r.note.0 as f32; out[43] = r.pace.to_next(); out[44] = r.pace.progress();
        { let mut t = NOTE_TEXT.lock().unwrap_or_else(|e| e.into_inner()); if t.0 != r.note.0 { *t = (r.note.0, r.note.1.clone(), r.note.2.clone(), r.note_clips.clone()); } }
        out[35] = r.run.next_line() as f32; out[36] = r.run.line_count() as f32;
        out[37] = r.event.0 as f32; out[38] = r.event.2 as f32; out[39] = r.event.1; out[40] = if r.run.assisted { 1.0 } else { 0.0 }; out[41] = r.run.total();
    }
    // the drivetrain: `[80]` the gear (-1 reverse, 0 neutral), `[81]` the engine's revs, `[82]` 1 in the manual, `[83]` 1 when the engine has stalled, `[84]` how firmly the clutch grips, `[85]` the clutch pedal
    { let d = &w.drivetrain; out[80] = d.gear as f32; out[81] = d.rpm(); out[82] = (d.mode == crate::game::drivetrain::Mode::Manual) as u8 as f32; out[83] = d.last.stalled as u8 as f32; out[84] = d.last.grip; out[85] = d.clutch_pedal; out[86] = d.last.boost_bar; }
    // the tyres: `[88..91]` the temperatures (degrees C) and `[92..95]` the wear (0 new, 1 worn out) of FL, FR, RL, RR
    for k in 0..4 { out[88 + k] = w.vehicle.tyres[k].temp; out[92 + k] = w.vehicle.tyres[k].wear; }
    // the heat and the damage: `[96]` the coolant's and `[97]` the clutch's temperature (degrees C), `[98]` the body's and `[99]` the engine's damage, `[100..103]` the four springs' (0 new, 1 wrecked)
    out[96] = w.drivetrain.coolant.temp; out[97] = w.drivetrain.clutch.temp; out[98] = w.vehicle.damage.body; out[99] = w.vehicle.damage.engine;
    for k in 0..4 { out[100 + k] = w.vehicle.damage.susp[k]; }
    // the fuel and the stability control: `[104]` the litres in the tank, `[105]` 1 while the stability control brakes a wheel
    out[104] = w.drivetrain.fuel.litres; out[105] = w.vehicle.esc_brake.iter().any(|b| *b > 0.0) as u8 as f32;
    out[25] = w.vehicle.speed() * 3.6;   // signed: negative while rolling backwards (the brake pedal reverses below 3 km/h)
}

#[cfg(any(target_os = "android", feature = "desktop"))]
pub use game::Game;

#[cfg(any(target_os = "android", feature = "desktop"))]
mod game {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use super::{fill_state, step_budget, STAT_LEN, STEP};
    use crate::game::{input::Input, meta::Profile, world::World};
    use crate::raster::Raster;

    /// World + renderer + fixed-step loop + optional profile. State layout: see `fill_state`.
    pub struct Game {
        world: World, raster: Raster, pub input: Input,
        acc: f32, last: Instant, fps_n: u32, fps_t: Instant, fps: f32, seed: u64, frame_ms: f32,
        stats: Arc<Mutex<[f32; STAT_LEN]>>,
        profile: Option<Profile>, profile_path: Option<PathBuf>, paused: bool,
        director: crate::audio::Director,
        car: crate::game::vehicle::VehicleSpec, garage: bool,
        dust_level: i32,
        auto_fallback: bool, monitor: crate::game::rendermode::FallbackMonitor, fell_back: bool, manual: bool, clutch: f32, reverse: bool, assists: u32, tuning: [f32; crate::game::assists::TUNING_PARTS], gear_cfg: (i32, i32),
    }

    impl Game {
        pub fn new(raster: Raster, seed: u64, stats: Arc<Mutex<[f32; STAT_LEN]>>) -> Game {
            let seed = super::level_seed(seed);
            Game { world: World::with_roads(seed, crate::game::vehicle::VehicleSpec::default_car()), raster, input: Input::default(), acc: 0.0, last: Instant::now(),
                fps_n: 0, fps_t: Instant::now(), fps: 0.0, seed, frame_ms: 0.0, stats,
                profile: None, profile_path: None, paused: false, director: crate::audio::Director::new(), car: crate::game::vehicle::VehicleSpec::default_car(), garage: false, dust_level: 1, auto_fallback: false, monitor: { let mut m = crate::game::rendermode::FallbackMonitor::new(); m.warm_up(6.0); m }, fell_back: false, manual: false, clutch: 0.0, reverse: false, assists: 0xFF, tuning: [1.0; crate::game::assists::TUNING_PARTS], gear_cfg: (0, 1) }
        }

        pub fn resize(&mut self, w: u32, h: u32) { self.raster.resize(w, h); }

        /// Quality level 0/1/2 (Low/Medium/High) sets the city draw distance: 300 / 400 / 450 m.
        pub fn set_quality(&mut self, level: i32) { self.raster.set_city_radius([300.0, 400.0, 450.0][level.clamp(0, 2) as usize]); }
        pub fn set_city_radius(&mut self, r: f32) { self.raster.set_city_radius(r); }
        pub fn set_hybrid(&mut self, on: bool) { self.raster.set_hybrid(on); self.monitor.reset(); self.monitor.warm_up(6.0); self.fell_back = false; }
        pub fn set_auto_fallback(&mut self, on: bool) { self.auto_fallback = on; self.monitor.reset(); }
        /// The player drags on the screen: orbit the chase camera (radians); `camera_release` when the finger goes up.
        pub fn camera_drag(&mut self, dyaw: f32, dpitch: f32) { self.world.cam.drag(dyaw, dpitch); }
        /// The free camera flies `fwd` metres along its gaze and `up` metres up.
        pub fn camera_fly(&mut self, fwd: f32, up: f32) { self.world.cam.fly(fwd, up); }
        pub fn camera_reset(&mut self) { self.world.cam.reset(); }
        /// The test track (a flat sheet of tarmac) instead of a level.
        pub fn test_track(&mut self) { self.world = World::test_track(self.car); self.world.dust.set_level(self.dust_level); self.acc = 0.0; self.input = Input::default(); }
        /// The driver shifts (+1 up, -1 down), picks the manual or the automatic and works the clutch pedal (0 up .. 1 down).
        pub fn shift(&mut self, dir: i32) { self.world.drivetrain.shift(dir); }
        pub fn set_manual(&mut self, on: bool) { self.manual = on; self.world.manual = on; }
        /// The automatic's selector: false drive, true reverse.
        pub fn set_reverse(&mut self, on: bool) { self.world.drivetrain.reverse_sel = on; self.reverse = on; }
        /// The aids (bits, see `Assists::from_mask`) and one part of the tuning (see `Tuning::set`), from the game's menu.
        pub fn set_assists(&mut self, mask: u32) { self.assists = mask; self.world.assists = crate::game::assists::Assists::from_mask(mask); }
        pub fn set_tuning(&mut self, part: usize, v: f32) { if part < self.tuning.len() && v.is_finite() { self.tuning[part] = v; self.world.tuning.set(part, v); } }
        pub fn service(&mut self) { self.world.service(); }
        pub fn set_gear_cfg(&mut self, boxt: i32, profile: i32) { self.gear_cfg = (boxt, profile); }
        pub fn set_clutch(&mut self, pedal: f32) { self.clutch = pedal; self.world.drivetrain.set_clutch(pedal); }
        pub fn camera_release(&mut self) { self.world.cam.set_dragging(false); }
        /// The camera view (see `CamView`), its distance scale or zoom and its height or tilt offset.
        pub fn camera_view(&mut self, view: i32, dist: f32, height: f32) { self.world.cam.set_view(view, dist, height); }
        /// Tyre dust: 0 off, 1 normal, 2 lots.
        pub fn set_dust(&mut self, level: i32) { self.dust_level = level.clamp(0, 2); self.world.dust.set_level(self.dust_level); }
        /// Rally: stage 0 the special stage, 1 the city loop, -1 leaves the rally mode.
        pub fn rally(&mut self, stage: i32) { if stage < 0 { self.world.rally_stop(); } else { self.world.rally_start(stage as u32); } }
        pub fn set_signs(&mut self, on: bool) { self.world.signs_on = on; }
        pub fn set_autopilot(&mut self, style: i32) { self.world.set_autopilot_style(style); }
        /// Starts (or stops) the autopilot's self-training for the car in the world.
        pub fn train(&mut self, mode: i32) { match mode { 1 => { crate::game::autopilot::job::start(self.world.car_spec, 10, crate::game::autopilot::train::Mode::Safe); } 2 => { crate::game::autopilot::job::start(self.world.car_spec, 10, crate::game::autopilot::train::Mode::Attack); } 3 => { crate::game::autopilot::job::start_net(self.world.car_spec, 1); } 5 => { crate::game::autopilot::job::start_es(self.world.car_spec, 1, 30); } 6 => { crate::game::autopilot::job::start_es(self.world.car_spec, 5, 30); } 4 => { crate::game::autopilot::job::start_net(self.world.car_spec, 5); } _ => crate::game::autopilot::job::stop() } }
        pub fn place_on_road(&mut self, mode: i32) -> bool { self.world.place_on_road(mode) }
        pub fn place_near_city(&mut self) -> bool { self.world.place_near_city() }
        pub fn place_in_city(&mut self) -> bool { self.world.place_in_city() }
        pub fn place_near_water(&mut self) -> bool { self.world.place_near_water() }

        pub fn new_level(&mut self, seed: u64) {
            self.monitor.warm_up(6.0);
            let seed = super::level_seed(seed);
            self.world = World::with_roads(seed, self.car); self.world.dust.set_level(self.dust_level); self.seed = seed; self.input = Input::default(); self.acc = 0.0;
        }

        /// Loads (or creates) the profile at `path`; the selected car (with its upgrades) drives in the free-drive world.
        pub fn with_profile(&mut self, path: PathBuf) {
            let p = Profile::load_or_new(&path);
            // the grip numbers: the embedded surfaces.json unless `surfaces.json` lies next to the profile (tune without a rebuild)
            crate::game::grip::reset_table();
            if let Some(dir) = path.parent() { crate::game::grip::load_override(&dir.join("surfaces.json")); }
            // the road relief numbers likewise (`roads.json`; used by the next level that is generated)
            crate::game::roads::cfg::reset_cfg();
            if let Some(dir) = path.parent() { crate::game::roads::cfg::load_override(&dir.join("roads.json")); }
            crate::game::autopilot::cfg::reset_cfg();
            if let Some(dir) = path.parent() { crate::game::autopilot::cfg::load_override(&dir.join("autopilot.json")); }
            self.car = p.selected_spec();
            self.world.swap_car(self.car);   // the free-drive world made in `new` used the default car
            self.profile = Some(p);
            self.profile_path = Some(path);
        }

        /// The garage scene (a standing car and an orbiting camera) with the profile's selected car.
        pub fn start_garage_selected(&mut self) {
            let cars = crate::game::vehicle::VehicleSpec::all();
            let sel = self.profile.as_ref().and_then(|p| cars.iter().position(|(id, _)| *id == p.garage.selected)).unwrap_or(0);
            self.set_garage_car(sel as i32);
        }
        /// Shows car `index` (file order) in the garage scene.
        pub fn set_garage_car(&mut self, index: i32) {
            let cars = crate::game::vehicle::VehicleSpec::all();
            let Some((id, spec)) = cars.get(index.max(0) as usize).or_else(|| cars.first()) else { return };
            // the turntable shows the car as the profile on disk has it (upgrades and the wheel set; the garage screen changes the file, then asks for the car again)
            if let Some(path) = &self.profile_path { self.profile = Some(Profile::load_or_new(path)); }
            let shown = self.profile.as_ref().and_then(|p| p.spec_of(id)).unwrap_or(*spec);
            self.garage = true; self.paused = false;
            self.world = World::garage(shown);
        }
        pub fn set_paused(&mut self, p: bool) { self.paused = p; self.last = Instant::now(); self.acc = 0.0; }

        pub fn speed_kmh(&self) -> f32 { self.world.vehicle.vel.len() * 3.6 }
        pub fn fps(&self) -> f32 { self.fps }
        pub fn seed(&self) -> u64 { self.seed }
        pub fn stats(&self) -> [f32; STAT_LEN] { *self.stats.lock().unwrap() }

        pub fn frame(&mut self) {
            let now = Instant::now();
            let dt = (now - self.last).as_secs_f32();
            self.last = now;
            if self.garage {
                self.world.step_garage(dt.clamp(0.0, 0.1));
            } else if !self.paused {
                self.world.manual = self.manual; self.world.drivetrain.set_clutch(self.clutch); self.world.drivetrain.reverse_sel = self.reverse;
                self.world.drivetrain.box_type = crate::game::drivetrain::BoxType::from_i32(self.gear_cfg.0); self.world.drivetrain.profile = crate::game::shiftlogic::Profile::from_i32(self.gear_cfg.1);
                self.world.assists = crate::game::assists::Assists::from_mask(self.assists); for (i, v) in self.tuning.iter().enumerate() { self.world.tuning.set(i, *v); }   // a new level makes a new world: the driver's choice goes with the game
                for _ in 0..step_budget(&mut self.acc, dt, 6) {
                    self.world.step(&self.input, STEP);
                    self.input.reset = false;
                }
            }
            let throttle = self.input.throttle;
            let (params, events) = self.director.update(&self.world, throttle, !self.paused);
            let scene = self.director.scene(&self.world);
            crate::audio::feed(&params, &events, &scene);
            let t0 = Instant::now();
            self.raster.render(&mut self.world);
            self.frame_ms = t0.elapsed().as_secs_f32() * 1000.0;
            self.fps_n += 1;
            let el = self.fps_t.elapsed().as_secs_f32();
            if el >= 0.5 {
                self.fps = self.fps_n as f32 / el; self.fps_n = 0; self.fps_t = Instant::now();
                if !self.paused && self.monitor.update(self.fps, el, self.raster.is_hybrid(), self.auto_fallback) { self.raster.set_hybrid(false); self.fell_back = true; }
            }
            let mut s = [0.0f32; STAT_LEN];
            fill_state(&mut s, &self.world, self.profile.as_ref().map_or(0, |p| p.money), self.fps, self.frame_ms, self.seed);
            s[26] = self.raster.is_hybrid() as u8 as f32; s[27] = self.fell_back as u8 as f32;
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
