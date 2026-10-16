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
pub struct AudioParams { pub machine: Machine, pub drive_rpm: f32, pub clutch: f32, pub load: f32, pub roll: f32, pub surface: Surface, pub slip: f32, pub wind: f32, pub active: bool }

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

static SYNTH: Mutex<Option<Synth>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Synth) -> R) -> R {
    // a poisoned lock must never take the game or the audio thread down: recover the data
    let mut g = SYNTH.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Synth::new))
}

/// Game thread: new parameters and events for the audio thread (short lock).
pub fn feed(params: &AudioParams, events: &[Event], scene: &Scene) { with(|s| { s.set_params(params); s.set_scene(scene); for e in events { s.push(*e); } }) }
pub fn set_muted(m: bool) { with(|s| s.set_muted(m)) }
pub fn set_volume(v: f32) { with(|s| s.set_volume(v)) }
/// Stops all sound immediately (the game screen is gone).
pub fn silence() { with(|s| s.silence()) }
/// Audio thread: fills `out` with interleaved stereo PCM16 samples.
pub fn render_into(out: &mut [i16]) { with(|s| s.render(out)) }

#[cfg(test)]
#[path = "synth_tests.rs"]
mod synth_tests;
