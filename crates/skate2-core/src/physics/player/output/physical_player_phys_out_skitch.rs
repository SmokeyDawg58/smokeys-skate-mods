//! Skitching.FillPhysOut82d97088 on the existing output allocation.
use super::{PhysOutComponent, PhysicalPlayerPhysOut};
use crate::{
    input_cvar_runtime::{InputCvarReferences, skitch::SkitchFloat},
    physical_player_assembly::remaining_states::{MatrixSlot, Skitching},
    physical_player_grind_manager::view::{GrindError, error, need},
};
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_skitch(
        &mut self,
        s: &Skitching,
        refs: &InputCvarReferences,
    ) -> Result<(), GrindError> {
        let guard = |out: &Self, component| {
            out.check_current_guard().map_err(error)?;
            out.require_complete_component(component).map_err(error)
        };
        if !need(s.force_exit, "Skitch force exit")?
            && need(s.grab_spline_valid, "Skitch valid spline")?
        {
            guard(self, PhysOutComponent::OffBoard)?;
            self.concrete.offboard.as_mut().unwrap().is_grabbing_object = true;
        }
        guard(self, PhysOutComponent::Animation)?;
        self.concrete.animation.as_mut().unwrap().skitch_crouch =
            need(s.anim_crouch, "Skitch crouch")?;
        guard(self, PhysOutComponent::Animation)?;
        self.concrete.animation.as_mut().unwrap().skitch_grip_hand =
            need(s.grip_hand, "Skitch grip")?;
        let hand_speed = need(s.grab_spline_hands_dist_delta, "Skitch hand delta")?
            * f32::from_bits(0x426f_ffff)
            + refs
                .try_skitch_float(SkitchFloat::kSkitchDebugHandSpeed)
                .map_err(error)?;
        guard(self, PhysOutComponent::Animation)?;
        self.concrete.animation.as_mut().unwrap().skitch_hand_speed = hand_speed;
        let physical = match need(s.grab_spline_physical_hand_space, "Skitch physical space")? {
            MatrixSlot::Current => need(s.grab_spline_current_space, "Skitch current frame")?,
            MatrixSlot::Future => need(s.grab_spline_future_space, "Skitch future frame")?,
        };
        let deck_y = need(
            s.base.processed.lock().map_err(error)?.deck_transform(),
            "processed deck",
        )?
        .columns()[3]
            .xyz[1];
        let height = physical[0][1].mul_add(
            need(s.grab_spline_hands_dist, "Skitch hands distance")?,
            physical[3][1],
        ) - deck_y;
        guard(self, PhysOutComponent::Ground)?;
        self.concrete.ground.as_mut().unwrap().skitch_spline_height = height;
        guard(self, PhysOutComponent::Ground)?;
        self.concrete.ground.as_mut().unwrap().skitch_contact_speed =
            need(s.contact_speed, "Skitch contact speed")?;
        guard(self, PhysOutComponent::Ground)?;
        self.concrete.ground.as_mut().unwrap().skitch_parametric_pos =
            need(s.parametric_position, "Skitch parametric position")?;
        guard(self, PhysOutComponent::Ground)?;
        self.concrete
            .ground
            .as_mut()
            .unwrap()
            .skitch_delay_until_reskitchable = need(s.delay_until_reskitchable, "reskitch delay")?;
        guard(self, PhysOutComponent::State)?;
        let descriptor = need(s.grab_spline_desc, "Skitch descriptor")?;
        let out = self.concrete.state.as_mut().unwrap();
        out.grab_spline_desc.object_type = descriptor.object_type;
        out.grab_spline_desc.object_id = descriptor.object_id;
        Ok(())
    }
}
