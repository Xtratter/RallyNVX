//! Fixed-step game loop shared by the Android and desktop front ends.
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

#[cfg(any(target_os = "android", feature = "desktop"))]
pub use game::Game;

#[cfg(any(target_os = "android", feature = "desktop"))]
mod game {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use super::{step_budget, STEP};
    use crate::game::{input::Input, world::World};
    use crate::raster::Raster;

    /// World + renderer + fixed-step loop. Stats layout: `[0]` fps, `[1]` km/h, `[2]` seed (low 24 bits), `[3]` frame ms.
    pub struct Game {
        world: World, raster: Raster, pub input: Input,
        acc: f32, last: Instant, fps_n: u32, fps_t: Instant, fps: f32, seed: u64, frame_ms: f32,
        stats: Arc<Mutex<[f32; 16]>>,
    }

    impl Game {
        pub fn new(raster: Raster, seed: u64, stats: Arc<Mutex<[f32; 16]>>) -> Game {
            let seed = super::level_seed(seed);
            Game { world: World::new(seed), raster, input: Input::default(), acc: 0.0, last: Instant::now(),
                fps_n: 0, fps_t: Instant::now(), fps: 0.0, seed, frame_ms: 0.0, stats }
        }

        pub fn resize(&mut self, w: u32, h: u32) { self.raster.resize(w, h); }

        pub fn new_level(&mut self, seed: u64) {
            let seed = super::level_seed(seed);
            self.world = World::new(seed); self.seed = seed; self.input = Input::default(); self.acc = 0.0;
        }

        pub fn speed_kmh(&self) -> f32 { self.world.vehicle.vel.len() * 3.6 }
        pub fn fps(&self) -> f32 { self.fps }
        pub fn seed(&self) -> u64 { self.seed }

        pub fn frame(&mut self) {
            let now = Instant::now();
            let dt = (now - self.last).as_secs_f32();
            self.last = now;
            for _ in 0..step_budget(&mut self.acc, dt, 6) {
                self.world.step(&self.input, STEP);
                self.input.reset = false;
            }
            let t0 = Instant::now();
            self.raster.render(&mut self.world);
            self.frame_ms = t0.elapsed().as_secs_f32() * 1000.0;
            self.fps_n += 1;
            let el = self.fps_t.elapsed().as_secs_f32();
            if el >= 0.5 { self.fps = self.fps_n as f32 / el; self.fps_n = 0; self.fps_t = Instant::now(); }
            let mut s = [0.0f32; 16];
            s[0] = self.fps; s[1] = self.speed_kmh(); s[2] = (self.seed & 0xFF_FFFF) as f32; s[3] = self.frame_ms;
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
