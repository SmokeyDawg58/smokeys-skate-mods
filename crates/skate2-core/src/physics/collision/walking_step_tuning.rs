//! Source-timed walking query and rail arbitration reads from linked cvars.
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    step_selector_geometry::StepGeometryTuning,
    walking_step_query::StepQueryTuning,
    walking_step_selection::StepSelectionTuning,
};
#[derive(Debug)]
pub(crate) enum WalkingTuningError {
    Cvar(InputCvarError),
    Debug(DebugOverlayError),
}
pub(crate) struct WalkingTuning<'a> {
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
impl StepGeometryTuning for WalkingTuning<'_> {
    type Error = WalkingTuningError;
    fn minimum_bad_y(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_minimum_bad_y()
            .map_err(WalkingTuningError::Cvar)
    }
    fn vertical_normal_y(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_vertical_normal_y()
            .map_err(WalkingTuningError::Cvar)
    }
    fn additional_gap(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_additional_gap()
            .map_err(WalkingTuningError::Cvar)
    }
    fn display_bad_triangles(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_step_display_bad_triangles()
            .map_err(WalkingTuningError::Cvar)
    }
    fn require_debug_owner(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(WalkingTuningError::Debug)
    }
}
impl StepQueryTuning for WalkingTuning<'_> {
    fn trajectory_radius(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_trajectory_radius()
            .map_err(WalkingTuningError::Cvar)
    }
    fn test_height(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_test_height()
            .map_err(WalkingTuningError::Cvar)
    }
    fn test_radius(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_test_radius()
            .map_err(WalkingTuningError::Cvar)
    }
    fn step_radius(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_step_radius()
            .map_err(WalkingTuningError::Cvar)
    }
    fn normal_y_threshold(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_normal_y_threshold()
            .map_err(WalkingTuningError::Cvar)
    }
    fn sprint_trajectory_count(&mut self) -> Result<i32, Self::Error> {
        self.cvars
            .try_step_sprint_trajectory_count()
            .map_err(WalkingTuningError::Cvar)
    }
    fn maximum_time(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_maximum_time()
            .map_err(WalkingTuningError::Cvar)
    }
    fn sin_cos(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        Ok(crate::reckoning_sin_cos::try_sin_cos(
            angle,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
        .unwrap_or_else(|never| match never {}))
    }
}
impl StepSelectionTuning for WalkingTuning<'_> {
    type Error = WalkingTuningError;
    fn added_distance_ratio(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_added_distance_ratio()
            .map_err(WalkingTuningError::Cvar)
    }
    fn height_margin(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_height_margin()
            .map_err(WalkingTuningError::Cvar)
    }
    fn box_half_size(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_box_half_size()
            .map_err(WalkingTuningError::Cvar)
    }
    fn vertical_sine(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_vertical_sine()
            .map_err(WalkingTuningError::Cvar)
    }
    fn minimum_distance(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_minimum_distance()
            .map_err(WalkingTuningError::Cvar)
    }
    fn cosine_spread(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_cosine_spread()
            .map_err(WalkingTuningError::Cvar)
    }
    fn low_step_height(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_low_step_height()
            .map_err(WalkingTuningError::Cvar)
    }
    fn reduce_speed_ratio(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_reduce_speed_ratio()
            .map_err(WalkingTuningError::Cvar)
    }
    fn acceptable_distance(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_acceptable_distance()
            .map_err(WalkingTuningError::Cvar)
    }
    fn shoe_offset(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_shoe_offset()
            .map_err(WalkingTuningError::Cvar)
    }
}
