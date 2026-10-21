//! The self-training as a background job (see `train`): one at a time, polled by the UI. The thread runs at a low priority so the phone stays usable.
use super::train;
use crate::game::vehicle::VehicleSpec;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::Mutex;

static RUNNING: AtomicBool = AtomicBool::new(false);
static STOP: AtomicBool = AtomicBool::new(false);
static PROGRESS: AtomicU32 = AtomicU32::new(0);
static START_COST: AtomicU32 = AtomicU32::new(0);
static BEST_COST: AtomicU32 = AtomicU32::new(0);
static NET_RESULT: Mutex<Option<(i32, Vec<f32>)>> = Mutex::new(None);
static RESULT: Mutex<Option<(train::Mode, [f32; train::N])>> = Mutex::new(None);

#[cfg(target_os = "android")]
fn lower_priority() { extern "C" { fn nice(inc: i32) -> i32; } unsafe { nice(10); } }
#[cfg(not(target_os = "android"))]
fn lower_priority() {}

/// Starts the training for `car`; `false` when one is running already.
pub fn start(car: VehicleSpec, generations: u32, mode: train::Mode) -> bool {
    if RUNNING.swap(true, Relaxed) { return false; }
    STOP.store(false, Relaxed); PROGRESS.store(0, Relaxed); BEST_COST.store(0, Relaxed); START_COST.store(0, Relaxed);
    *RESULT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let spawned = std::thread::Builder::new().name("autopilot-train".into()).spawn(move || {
        lower_priority();
        let start = match mode { train::Mode::Safe => super::cfg::cfg(), train::Mode::Attack => super::cfg::attack() };
        let (best, cost) = train::train(car, None, start, mode, &[2, 21], generations, |p, b| { if p == 0.0 { START_COST.store(b.to_bits(), Relaxed); } PROGRESS.store(p.to_bits(), Relaxed); BEST_COST.store(b.to_bits(), Relaxed); }, || STOP.load(Relaxed));
        if !STOP.load(Relaxed) && cost.is_finite() { *RESULT.lock().unwrap_or_else(|e| e.into_inner()) = Some((mode, train::from_cfg_values(&best))); }
        RUNNING.store(false, Relaxed);
    });
    if spawned.is_err() { RUNNING.store(false, Relaxed); return false; }
    true
}
/// Starts the net's training for `car` from the controller of style `src` (1 normal, 5 Max Attack): four rounds of imitation with DAgger over four levels.
pub fn start_net(car: VehicleSpec, src: i32) -> bool {
    if RUNNING.swap(true, Relaxed) { return false; }
    STOP.store(false, Relaxed); PROGRESS.store(0, Relaxed); BEST_COST.store(0, Relaxed); START_COST.store(0, Relaxed);
    *NET_RESULT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let spawned = std::thread::Builder::new().name("autopilot-net".into()).spawn(move || {
        lower_priority();
        let cfg = super::cfg::cfg_style(if src == 5 { 5 } else { 1 });
        let net = super::learn::dagger(car, cfg, &[2, 3, 5, 8], 4, |p| PROGRESS.store(p.to_bits(), Relaxed), || STOP.load(Relaxed));
        if !STOP.load(Relaxed) { *NET_RESULT.lock().unwrap_or_else(|e| e.into_inner()) = Some((if src == 5 { 5 } else { 1 }, net.to_vec())); }
        RUNNING.store(false, Relaxed);
    });
    if spawned.is_err() { RUNNING.store(false, Relaxed); return false; }
    true
}
/// The finished net's source style and parameters, once.
pub fn take_net() -> Option<(i32, Vec<f32>)> { NET_RESULT.lock().unwrap_or_else(|e| e.into_inner()).take() }
pub fn stop() { STOP.store(true, Relaxed); }
/// `[running (0 or 1), progress 0..1, the best cost so far, the cost of the numbers it started with]`.
pub fn state() -> [f32; 4] { [if RUNNING.load(Relaxed) { 1.0 } else { 0.0 }, f32::from_bits(PROGRESS.load(Relaxed)), f32::from_bits(BEST_COST.load(Relaxed)), f32::from_bits(START_COST.load(Relaxed))] }
/// The finished training's numbers, once (the mode and the searched seven numbers, in their own units).
pub fn take_result() -> Option<(train::Mode, [f32; train::N])> { RESULT.lock().unwrap_or_else(|e| e.into_inner()).take() }

