//! Render mode choice: the default by quality and the optional automatic fallback from Hybrid to Raster.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode { Raster, Hybrid }

/// Hybrid on High (quality index 2), Raster otherwise, until the user picks a mode.
pub fn default_mode(quality_index: i32) -> RenderMode { if quality_index == 2 { RenderMode::Hybrid } else { RenderMode::Raster } }

const FPS_FLOOR: f32 = 50.0;
const SLOW_SECONDS: f32 = 4.0;

/// Counts how long the frame rate stays below the floor in Hybrid; `update` returns true once when the fallback should
/// happen (only if `auto`), then stays quiet until `reset`.
#[derive(Debug, Default)]
pub struct FallbackMonitor { slow: f32, fired: bool }

impl FallbackMonitor {
    pub fn new() -> FallbackMonitor { FallbackMonitor::default() }
    pub fn reset(&mut self) { self.slow = 0.0; self.fired = false; }
    pub fn update(&mut self, fps: f32, dt: f32, hybrid: bool, auto: bool) -> bool {
        if !auto || !hybrid { self.slow = 0.0; return false; }
        if !fps.is_finite() || !dt.is_finite() || dt <= 0.0 || self.fired { return false; }
        if fps >= FPS_FLOOR { self.slow = 0.0; return false; }
        self.slow += dt.min(1.0);
        if self.slow >= SLOW_SECONDS - 1e-4 { self.fired = true; return true; }
        false
    }
}

#[cfg(test)]
#[path = "rendermode_tests.rs"]
mod tests;
