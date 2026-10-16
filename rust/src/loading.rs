//! The start-up progress: the game thread reports how far the loading is, the UI polls it and shows a bar.
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Stage ids the UI turns into words.
pub const GPU: u32 = 1;
pub const TEXTURES: u32 = 2;
pub const SHADERS: u32 = 3;
pub const WORLD: u32 = 4;
pub const CAR: u32 = 5;
pub const FIRST_FRAME: u32 = 6;
/// The game thread died (a panic): the UI says so instead of waiting for ever.
pub const FAILED: u32 = 99;

static STATE: AtomicU32 = AtomicU32::new(0);
static T0: Mutex<Option<Instant>> = Mutex::new(None);

/// A new loading starts: back to 0 %.
pub fn begin() { *T0.lock().unwrap() = Some(Instant::now()); STATE.store(0, Ordering::Relaxed); }

/// The loading reached `pct` (0..=100) in `stage`; progress never goes back.
pub fn stage(stage: u32, pct: u32) {
    let pct = pct.min(100);
    if pct < current().0 && pct != 0 { return; }
    STATE.store(stage << 8 | pct, Ordering::Relaxed);
    let ms = T0.lock().unwrap().map(|t| t.elapsed().as_millis()).unwrap_or(0);
    log::info!("loading: stage {stage} {pct} % at {ms} ms");
}

/// (percent, stage id).
pub fn current() -> (u32, u32) { let s = STATE.load(Ordering::Relaxed); (s & 0xFF, s >> 8) }

/// The value the UI polls: `stage << 8 | percent`.
pub fn packed() -> u32 { STATE.load(Ordering::Relaxed) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_only_grows() {
        begin();
        stage(GPU, 10); stage(TEXTURES, 30); stage(GPU, 20);
        assert_eq!(current(), (30, TEXTURES));
        stage(FIRST_FRAME, 100);
        assert_eq!(current(), (100, FIRST_FRAME));
        assert_eq!(packed(), FIRST_FRAME << 8 | 100);
        stage(FAILED, 0);
        assert_eq!(current(), (0, FAILED), "a failure is reported even after 100 %");
        begin();
        assert_eq!(current(), (0, 0));
    }
}
