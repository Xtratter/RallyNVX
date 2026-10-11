//! Procedural sound: a pure synthesiser (engine, rotor, rolling, wind, one-shot events) driven by `AudioParams` and
//! `Event`s. No files, no dependencies; the platform glue only pulls PCM from `Synth::render`.
pub mod dsp;
pub mod synth;

pub use synth::{engine_freq, Synth};

/// Output sample rate, mono PCM16.
pub const RATE: u32 = 44_100;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Machine { #[default] None, Car, Heli, Drone }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Surface { #[default] Ground, Water }

/// What the machine is doing right now (all values 0..1 unless noted). `active == false` fades everything out.
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioParams { pub machine: Machine, pub rpm: f32, pub load: f32, pub roll: f32, pub surface: Surface, pub slip: f32, pub wind: f32, pub active: bool }

/// One-shot sounds. Strengths are 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event { Touchdown(f32), Crash, Splash(f32), Checkpoint, Board, Success, Fail }

#[cfg(test)]
#[path = "synth_tests.rs"]
mod synth_tests;
