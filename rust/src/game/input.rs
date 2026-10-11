//! Player commands for one physics step.
#[derive(Clone, Copy, Default, Debug)]
pub struct Input { pub throttle: f32, pub steer: f32, pub brake: bool, pub handbrake: bool, pub reset: bool }

impl Input {
    /// Non-finite axes become 0, finite ones are clamped to -1..1 (JNI / gamepad safety).
    pub fn sanitized(self) -> Input {
        let f = |v: f32| if v.is_finite() { v.clamp(-1.0, 1.0) } else { 0.0 };
        Input { throttle: f(self.throttle), steer: f(self.steer), ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn input_sanitizes_nan() {
        let i = Input { throttle: f32::NAN, steer: 5.0, ..Default::default() }.sanitized();
        assert_eq!(i.throttle, 0.0); assert_eq!(i.steer, 1.0);
        assert_eq!(Input { steer: f32::INFINITY, ..Default::default() }.sanitized().steer, 0.0);
    }
}
