//! Where a sound is heard from: the path from a source to the listener (travel time, distance, the head) and the emitter that renders it in stereo.
//! The geometry is pure (`path`), the audio-thread part (`Emitter`) smooths what the game thread sends so that nothing clicks.
use super::dsp::{coef, FracDelay, OnePole};
use super::RATE;
use crate::game::math::{v3, V3};

/// The speed of sound in air, m/s.
pub const C: f32 = 343.0;
/// Half of the distance between the ears, m.
const HEAD: f32 = 0.09;
/// The longest travel time kept (a free camera can fly far away), s.
pub const MAX_DELAY: f32 = 1.4;

/// What reaches the listener's ears from a point source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Path {
    /// Travel time to the head's centre, s.
    pub delay: f32,
    /// Loudness against the distance (1 at the reference distance, the 1/r law beyond it, capped).
    pub gain: f32,
    /// Where it is heard, -1 fully left .. 1 fully right (the sine of the angle to the right of the listener's gaze).
    pub pan: f32,
    /// How far the sound is dulled by the air on the way (the cutoff, Hz).
    pub air_hz: f32,
    /// The extra delay of the far ear (the interaural time difference), s; positive when the right ear hears it first.
    pub itd: f32,
}

impl Default for Path { fn default() -> Path { Path { delay: 0.0, gain: 0.0, pan: 0.0, air_hz: 16000.0, itd: 0.0 } } }

/// The reference distance of the 1/r law: nearer than this the sound stops getting louder (a car is not a point).
const REF_DIST: f32 = 2.0;
/// The loudness of the sources is set for this distance (the chase camera), so it is the unit gain there.
const CHASE_DIST: f32 = 8.0;
const MAX_GAIN: f32 = 4.0;

/// The path from `src` to a listener at `listener` looking along `fwd` (horizontal or not; y up, `right = fwd x up`).
pub fn path(listener: V3, fwd: V3, src: V3) -> Path {
    let d = src - listener;
    let dist = d.len().max(0.05);
    let dir = d * (1.0 / dist);
    let f = { let h = v3(fwd.x, 0.0, fwd.z); if h.len() > 1e-3 { h.norm() } else { v3(0.0, 0.0, 1.0) } };
    let right = f.cross(v3(0.0, 1.0, 0.0));
    let s = dir.dot(right).clamp(-1.0, 1.0);
    // Woodworth's model: the far ear hears it later by (a / c)(theta + sin theta)
    let theta = s.asin();
    let itd = HEAD / C * (theta + s);
    // the air takes the highs: about 0.005 dB/m at 1 kHz, 0.1 dB/m at 8 kHz; one pole whose cutoff falls with the distance
    let air_hz = (18000.0 * (-dist / 220.0).exp()).clamp(900.0, 18000.0);
    Path { delay: (dist / C).min(MAX_DELAY), gain: (CHASE_DIST / dist.max(REF_DIST)).min(MAX_GAIN), pan: s, air_hz, itd }
}

/// One path of the sound to the ears as the audio thread plays it: the direct sound, or an echo off the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tap { pub delay: f32, pub gain: f32, pub pan: f32, pub itd: f32, pub hz: f32 }

impl Path {
    pub fn tap(&self) -> Tap { Tap { delay: self.delay, gain: self.gain, pan: self.pan, itd: self.itd, hz: self.air_hz } }
}

/// A tap's smoothed state (samples / 0..1 / filter coefficients) and its two ear filters.
#[derive(Clone, Copy, Default)]
struct TapState { delay: f32, gain: f32, pan: f32, itd: f32, hz: f32, t_delay: f32, t_gain: f32, t_pan: f32, t_itd: f32, t_hz: f32, lp_l: OnePole, lp_r: OnePole, live: bool }

const TAPS: usize = 2;

/// One source on the audio thread: a delay line (its length is the travel time, so the Doppler shift is free), a direct tap and an echo tap, each with the air, the head and
/// whatever stands in the way (the cutoff `hz`).
pub struct Emitter { line: FracDelay, taps: [TapState; TAPS], a: f32, a_delay: f32, fresh: bool }

impl Default for Emitter { fn default() -> Self { Emitter::new() } }

impl Emitter {
    pub fn new() -> Emitter {
        let rate = RATE as f32;
        Emitter { line: FracDelay::new(17), taps: [TapState::default(); TAPS], a: 1.0 - (-1.0 / (0.03 * rate)).exp(), a_delay: 1.0 - (-1.0 / (0.12 * rate)).exp(), fresh: true }
    }

    /// New targets (call per block): the direct sound and, maybe, the ground's echo. A jump of the position (a reset, a teleport, the first call) is taken at once:
    /// gliding would sound like a swoop.
    pub fn set_taps(&mut self, direct: &Tap, echo: Option<&Tap>) {
        let rate = RATE as f32;
        for (k, t) in [Some(direct), echo].into_iter().enumerate() {
            let s = &mut self.taps[k];
            match t {
                Some(t) => {
                    s.t_delay = t.delay * rate; s.t_gain = t.gain; s.t_pan = t.pan; s.t_itd = t.itd * rate; s.t_hz = t.hz;
                    if !s.live || self.fresh || (s.t_delay - s.delay).abs() > 0.06 * rate {
                        s.delay = s.t_delay; s.gain = s.t_gain; s.pan = s.t_pan; s.itd = s.t_itd; s.hz = s.t_hz; s.live = true;
                    }
                }
                None => { s.t_gain = 0.0; }
            }
        }
        self.fresh = false;
    }

    /// The direct sound only (what a source without an echo has).
    pub fn set_path(&mut self, p: &Path) { self.set_taps(&p.tap(), None); }

    /// Stops the sound at once and forgets the path (a new level).
    pub fn reset(&mut self) { self.line.clear(); self.fresh = true; for s in self.taps.iter_mut() { s.gain = 0.0; s.t_gain = 0.0; s.live = false; } }

    /// One input sample in, the (left, right) ear signals out.
    pub fn process(&mut self, x: f32) -> (f32, f32) {
        let (a, ad) = (self.a, self.a_delay);
        self.line.push(x);
        let rate = RATE as f32;
        let (mut l, mut r) = (0.0f32, 0.0f32);
        for s in self.taps.iter_mut() {
            if !s.live { continue; }
            s.delay += (s.t_delay - s.delay) * ad;     // slow: a moving car bends the pitch smoothly (no zipper noise)
            s.gain += (s.t_gain - s.gain) * a; s.pan += (s.t_pan - s.pan) * a; s.itd += (s.t_itd - s.itd) * a; s.hz += (s.t_hz - s.hz) * a;
            if s.gain < 1e-5 && s.t_gain == 0.0 { s.live = false; continue; }
            // the far ear hears it `itd` later: the right ear leads when `itd > 0`
            let (dl, dr) = (s.delay + s.itd.max(0.0), s.delay + (-s.itd).max(0.0));
            let (xl, xr) = (self.line.read(dl), self.line.read(dr));
            // equal-power panning (1 at the centre); the head's shadow: the ear turned away loses its highs (the right one is the far one when the sound is on the left)
            let p = (s.pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * std::f32::consts::PI;
            let (gl, gr) = (p.cos() * std::f32::consts::SQRT_2, p.sin() * std::f32::consts::SQRT_2);
            let shadow = |side: f32| 13000.0 - 10500.0 * side.clamp(0.0, 1.0);   // the near ear keeps all, the far one down to 2.5 kHz
            let (hl, hr) = (s.hz.min(shadow(s.pan)), s.hz.min(shadow(-s.pan)));
            l += s.lp_l.run(xl, coef(hl, rate)) * gl * s.gain;
            r += s.lp_r.run(xr, coef(hr, rate)) * gr * s.gain;
        }
        (l, r)
    }
}

#[cfg(test)]
#[path = "spatial_tests.rs"]
mod tests;
