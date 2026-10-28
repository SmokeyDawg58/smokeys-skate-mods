//! Registered child of existing board entry state, `UpdateSpeedModel` a268..a2dc.
//! Retains the actual targetSpeed field; this is not a second speed model.
use super::SkateboardEntryState;

pub(crate) trait SpeedModelPrefixSource {
    type Error;
    type Surface;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error>;
    fn flags_8cc(&mut self) -> Result<u32, Self::Error>;
    fn wheel_count(&mut self) -> Result<i32, Self::Error>;
    fn capture_surface(&mut self) -> Result<Self::Surface, Self::Error>;
    fn forward_speed(&mut self) -> Result<f32, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn maximum_speed_difference(&mut self, surface: &Self::Surface) -> Result<f32, Self::Error>;
}

/// Complete `ShouldSpeedModelApplyForce82b23e38`. Wheel count is unconditional,
/// despite the misleading short-circuit expression in the decompiler.
fn should_apply<S: SpeedModelPrefixSource>(source: &mut S) -> Result<bool, S::Error> {
    let flags = source.flags_8c4()?;
    let inhibited = if flags & 0x6201_0000 != 0 {
        true
    } else {
        source.flags_8cc()? & 0x2000_0000 != 0
    };
    let wheels = source.wheel_count()?;
    Ok(!inhibited && wheels >= 2)
}

/// Actual source locals retained for the continuation beginning at a2e0.
/// `starting_target` is not published in the ordinary branch at this stage.
pub(crate) struct SpeedModelBeforeJumpContribution {
    pub(crate) should_apply_force: bool,
    pub(crate) absolute_forward_speed: f32,
    pub(crate) time_delta: f32,
    pub(crate) maximum_difference: f32,
    pub(crate) starting_target: f32,
}
impl SkateboardEntryState {
    pub(crate) fn try_speed_model_prefix<S: SpeedModelPrefixSource>(
        &mut self,
        source: &mut S,
    ) -> Result<SpeedModelBeforeJumpContribution, S::Error> {
        let should_apply_force = should_apply(source)?;
        // Proposed SAME +530/80000000 field, initialized by actual ctor38b58.
        let reset = self.reset_target_after_collision;
        let surface = source.capture_surface()?;
        let speed = source.forward_speed()?.abs();
        let dt = source.time_delta()?;
        let maximum = source.maximum_speed_difference(&surface)?;
        let lower = speed - maximum;
        let upper = maximum + speed;
        let lower = if -lower >= 0.0 { 0.0 } else { lower };
        let target = if reset {
            // Native stores target a2d4 before clearing this flag a2dc.
            self.target_speed = speed;
            self.reset_target_after_collision = false;
            speed
        } else {
            let old = self.target_speed;
            let low_test = lower - old;
            let bounded = if low_test >= 0.0 { lower } else { old };
            let high_test = upper - bounded;
            if high_test >= 0.0 { bounded } else { upper }
        };
        Ok(SpeedModelBeforeJumpContribution {
            should_apply_force,
            absolute_forward_speed: speed,
            time_delta: dt,
            maximum_difference: maximum,
            starting_target: target,
        })
    }
}

#[cfg(test)]
#[path = "skateboard_speed_model_prefix_tests.rs"]
mod tests;
