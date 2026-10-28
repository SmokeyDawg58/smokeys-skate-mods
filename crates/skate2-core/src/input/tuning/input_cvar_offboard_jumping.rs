//! Source-default OffboardJumpingTest and standing-jump live references597..607.
use super::{InputCvarError, InputCvarReferences};
use crate::physical_player_remaining_constructor_owners::jumping_runtime::JumpingTuning;
impl JumpingTuning for InputCvarReferences {
    type Error = InputCvarError;
    fn max_jump_distance(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(597)
    }
    fn min_jump_distance(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(598)
    }
    fn test_radius(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(599)
    }
    fn max_jump_height(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(600)
    }
    fn absolute_max_jump_distance(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(601)
    }
    fn max_jump_down(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(602)
    }
    fn minimum_distance_from_wall(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(603)
    }
    fn minimum_distance_from_top_of_wall(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(604)
    }
    fn minimum_ground_normal_y(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(605)
    }
    fn debug_draw(&mut self) -> Result<bool, Self::Error> {
        self.try_current_boolean(606)
    }
    fn force_bad_trajectory(&mut self) -> Result<bool, Self::Error> {
        self.try_current_boolean(607)
    }
}
impl InputCvarReferences {
    /// OffBoard_Jump.Update 82d98c5c/82d98c88: kOffboardStandingJumpMaxLandOnBoardAssist
    /// while standing (air time below 0.06), else kOffboardJumpMaxLandOnBoardAssist.
    pub(crate) fn try_offboard_jump_land_on_board_assist(
        &self,
        standing: bool,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(if standing { 609 } else { 608 })
    }
}
