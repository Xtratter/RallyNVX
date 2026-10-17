//! Procedural sound: a pure synthesiser (engine, rolling, wind, one-shot events) driven by `AudioParams` and
//! `Event`s. No files, no dependencies; the platform glue only pulls PCM from `Synth::render`.
pub mod acoustics;
pub mod director;
pub mod dsp;
pub mod engine;
pub mod reverb;
pub mod spatial;
pub mod synth;

pub use director::Director;
pub use synth::Synth;

/// Output sample rate, stereo PCM16 (interleaved).
pub const RATE: u32 = 44_100;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Machine { #[default] None, Car }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Surface { #[default] Ground, Water }

/// What the machine is doing right now (all values 0..1 unless noted): `drive_rpm` the revs the wheels ask of the engine in rpm, `clutch` the grip of the clutch, `load` the throttle pedal. `active == false` fades everything out.
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioParams { pub machine: Machine, pub als: bool, pub cut: bool, pub stalled: bool, pub no_bov: bool, pub drive_rpm: f32, pub clutch: f32, pub load: f32, pub roll: f32, pub surface: Surface, pub slip: f32, pub wind: f32, pub active: bool }

/// The sound sources of the car (indices into `Scene::src`).
pub const SRC_EXHAUST: usize = 0;
pub const SRC_BAY: usize = 1;
/// The four tyres: `SRC_WHEEL + wheel`.
pub const SRC_WHEEL: usize = 2;
/// The car's body: where the one-shots (a landing, a crash, a splash) happen.
pub const SRC_BODY: usize = 6;
pub const N_SRC: usize = 7;

/// What stands between a source and the listener, and the ground's echo of it (worked out on the game thread, see `acoustics`).
#[derive(Clone, Copy, Debug)]
pub struct SrcExtra {
    /// The loss of the low band as a gain, the cutoff (Hz) that gives the highs their loss, and the extra travel time of the bent path (s).
    pub gain: f32, pub hz: f32, pub extra_delay: f32,
    /// The ground's echo: the image source, the share of the sound the ground returns and its cutoff (Hz).
    pub echo: Option<(V3, f32, f32)>,
}
impl Default for SrcExtra { fn default() -> Self { SrcExtra { gain: 1.0, hz: 20000.0, extra_delay: 0.0, echo: None } } }

/// Where the listener (the camera) and the sources are, and what each tyre is doing. The default is a listener at the origin looking along +z with every source 8 m ahead.
#[derive(Clone, Copy, Debug)]
pub struct Scene {
    pub listener: V3, pub fwd: V3, pub src: [V3; N_SRC],
    /// Per tyre: 1 when it touches the ground, and its share of the squeal (0..1).
    pub wheel_on: [f32; 4], pub wheel_slip: [f32; 4],
    pub extra: [SrcExtra; N_SRC],
    /// The listener sits in the car's cabin: the wind is muffled (a gain and a cutoff in Hz).
    pub wind_gain: f32, pub wind_hz: f32,
    /// The environment's tail and echoes, worked out from rays cast round the listener.
    pub env: reverb::Env,
}
impl Default for Scene {
    fn default() -> Scene { Scene { listener: V3::ZERO, fwd: v3(0.0, 0.0, 1.0), src: [v3(0.0, 0.0, 8.0); N_SRC], wheel_on: [1.0; 4], wheel_slip: [1.0; 4], extra: [SrcExtra::default(); N_SRC], wind_gain: 1.0, wind_hz: 20000.0, env: reverb::Env::default() } }
}

/// One-shot sounds. Strengths are 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event { Touchdown(f32), Crash, Splash(f32) }

use crate::game::math::{v3, V3};
use std::sync::Mutex;

/// The synth lives on the audio thread (it holds the lock while it renders a block); everything the game thread, the UI and the activity say to it goes through `PENDING`, a lock that is
/// only ever held for a moment, so that a slow audio thread can never stall the game (a held lock starved the game thread on a slow phone).
static SYNTH: Mutex<Option<Synth>> = Mutex::new(None);
static PENDING: Mutex<Pending> = Mutex::new(Pending::new());

struct Pending { params: Option<AudioParams>, scene: Option<Scene>, events: Vec<Event>, muted: Option<bool>, volume: Option<f32>, silence: bool }
impl Pending {
    const fn new() -> Pending { Pending { params: None, scene: None, events: Vec::new(), muted: None, volume: None, silence: false } }
}

fn pend(f: impl FnOnce(&mut Pending)) {
    // a poisoned lock must never take the game or the audio thread down: recover the data
    let mut g = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

/// The most one-shots kept while the audio thread is not listening.
const MAX_PENDING_EVENTS: usize = 32;

/// Game thread: new parameters and events for the audio thread (a moment's lock, never waits for a render).
pub fn feed(params: &AudioParams, events: &[Event], scene: &Scene) {
    pend(|p| {
        p.params = Some(*params); p.scene = Some(*scene);
        for e in events { if p.events.len() < MAX_PENDING_EVENTS { p.events.push(*e); } }
    })
}
pub fn set_muted(m: bool) { pend(|p| p.muted = Some(m)) }
pub fn set_volume(v: f32) { pend(|p| p.volume = Some(v)) }
/// Stops all sound immediately (the game screen is gone).
pub fn silence() { pend(|p| { *p = Pending::new(); p.silence = true; }) }
/// Audio thread: fills `out` with interleaved stereo PCM16 samples.
pub fn render_into(out: &mut [i16]) {
    let pending = { let mut g = PENDING.lock().unwrap_or_else(|e| e.into_inner()); std::mem::replace(&mut *g, Pending::new()) };
    let mut g = SYNTH.lock().unwrap_or_else(|e| e.into_inner());
    let s = g.get_or_insert_with(Synth::new);
    if pending.silence { s.silence(); }
    if let Some(m) = pending.muted { s.set_muted(m); }
    if let Some(v) = pending.volume { s.set_volume(v); }
    if let Some(p) = &pending.params { s.set_params(p); }
    if let Some(sc) = &pending.scene { s.set_scene(sc); }
    for e in &pending.events { s.push(*e); }
    s.render(out);
    BURNED.fetch_add((s.take_burned_delta() * 1e9) as u64, std::sync::atomic::Ordering::Relaxed);
    let (m, h, f) = s.take_exhaust_delta();
    EXH_MOLES.fetch_add((m * 1e9) as u64, std::sync::atomic::Ordering::Relaxed); EXH_HEAT.fetch_add((h * 1e9) as u64, std::sync::atomic::Ordering::Relaxed); EXH_FUEL.fetch_add((f * 1e9) as u64, std::sync::atomic::Ordering::Relaxed);
}

/// The fuel (mol) that burnt in the engine model's exhaust since the last call: the renderer turns it into the fire at the tail pipe.
pub fn take_burned() -> f32 { BURNED.swap(0, std::sync::atomic::Ordering::Relaxed) as f32 * 1e-9 }
static BURNED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static EXH_MOLES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static EXH_HEAT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static EXH_FUEL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The gas that has left the engine model's exhaust since the last call: (moles, the mean temperature in K, moles of unburnt fuel among them): the renderer makes the plume at the tail pipe of it.
pub fn take_exhaust() -> (f32, f32, f32) {
    use std::sync::atomic::Ordering::Relaxed;
    let (m, h, f) = (EXH_MOLES.swap(0, Relaxed) as f64 * 1e-9, EXH_HEAT.swap(0, Relaxed) as f64 * 1e-9, EXH_FUEL.swap(0, Relaxed) as f64 * 1e-9);
    (m as f32, if m > 1e-12 { (h / m) as f32 } else { 300.0 }, f as f32)
}

#[cfg(test)]
#[path = "synth_tests.rs"]
mod synth_tests;
