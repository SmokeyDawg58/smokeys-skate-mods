//! PhysState_OffBoard_Pushing.FillPhysOut82d87eb8, then FeetIK.Fill82dc8ba8.
use super::{PhysOutComponent, PhysicalPlayerPhysOut};
use crate::{
    physical_player_assembly::remaining_states::OffBoardPushing,
    physical_player_grind_manager::{
        math::neg,
        view::{GrindError, error, need},
    },
    reckoning_numeric::ReckoningNumeric,
};

impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_pushing_state<N: ReckoningNumeric>(
        &mut self,
        state: &OffBoardPushing,
        num: &mut N,
        mut foot_down: impl FnMut(usize) -> Result<bool, GrindError>,
    ) -> Result<(), GrindError>
    where
        N::Error: std::fmt::Debug,
    {
        let grabbing = need(state.grab_object, "Pushing grab flag")?;
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(error)?;
        self.concrete.offboard.as_mut().unwrap().is_grabbing_object = grabbing;
        // Native stores movement Z, movement -X, force Z, force -X, in order.
        // Preserve each completed store if a later source field/numeric fails.
        for entry in 0..4 {
            let hands = need(
                state.blended_spline_hands_transform,
                "Pushing blended hands",
            )?;
            let vector = if entry < 2 {
                need(
                    state.approximate_skeleton_speed,
                    "Pushing approximate speed",
                )?
            } else {
                need(state.smoothed_force, "Pushing smoothed force")?
            };
            let axis = if entry % 2 == 0 {
                hands[2]
            } else {
                neg(hands[0])
            };
            let value = num.dot3(vector, axis).map_err(error)?;
            self.check_current_guard().map_err(error)?;
            self.require_complete_component(PhysOutComponent::OffBoard)
                .map_err(error)?;
            let out = self.concrete.offboard.as_mut().unwrap();
            match entry {
                0 => out.anim_obj_mov_z = value,
                1 => out.anim_obj_mov_x = value,
                2 => out.anim_obj_force_z = value,
                _ => out.anim_obj_force_x = value,
            }
        }
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(error)?;
        self.concrete
            .offboard
            .as_mut()
            .unwrap()
            .anim_obj_angular_speed = 0.0;
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(error)?;
        self.concrete.offboard.as_mut().unwrap().anim_crouch =
            need(state.anim_crouch, "Pushing crouch")?;
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(error)?;
        // Source reads the retained spline itself, not mGrabSplineDesc.
        let spline = need(state.grab_spline.as_ref(), "Pushing retained spline")?;
        let out = self.concrete.state.as_mut().unwrap();
        out.grab_spline_desc.object_type = spline.descriptor_type;
        out.grab_spline_desc.object_id = spline.descriptor_object;
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(error)?;
        self.concrete.offboard.as_mut().unwrap().object_mass =
            need(state.object_mass, "Pushing mass")?;
        self.check_current_guard().map_err(error)?;
        self.require_complete_component(PhysOutComponent::Ground)
            .map_err(error)?;
        self.concrete
            .ground
            .as_mut()
            .unwrap()
            .pushing_delay_until_repushable =
            need(state.delay_until_repushable, "Pushing repush delay")?;
        for foot in 0..2 {
            self.check_current_guard().map_err(error)?;
            self.require_complete_component(PhysOutComponent::OffBoard)
                .map_err(error)?;
            let down = foot_down(foot)?;
            let out = self.concrete.offboard.as_mut().unwrap();
            if foot == 0 {
                out.left_foot_down = down;
            } else {
                out.right_foot_down = down;
            }
        }
        Ok(())
    }
}
