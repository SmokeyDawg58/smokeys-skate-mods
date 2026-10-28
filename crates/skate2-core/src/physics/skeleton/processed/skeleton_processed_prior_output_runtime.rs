//! Consecutive `Update1_ProcessInput82e02430..2a28` on the actual two owners.
use super::{
    PhysOutAccessError, PhysOutGroundVector, PhysOutMotionScalar, PhysOutMotionVector,
    PhysOutSystemVector, PhysicalPlayerPhysOut, SkeletonProcessedContact, SkeletonRootVector,
};
use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
use crate::{
    actor_construction::anim_out_phys_in::{AnimOutConstructionStage, AnimOutPhysInOwner},
    physical_player_phys_out::construct::prior_input::PriorInputReadError,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PriorInputFields {
    pub(crate) surface_angle: Option<f32>,
    pub(crate) empirical_acceleration: Option<SkeletonRootVector>,
    pub(crate) empirical_velocity: Option<SkeletonRootVector>,
    pub(crate) center_of_mass_delta: Option<WalkVector>,
    pub(crate) center_of_mass_animation: Option<WalkVector>,
    pub(crate) foot_targets: [Option<WalkVector>; 2],
    pub(crate) foot_ik: [Option<f32>; 2],
    pub(crate) next_step_position: Option<WalkVector>,
    pub(crate) next_step_normal: Option<WalkVector>,
    pub(crate) skater_look_direction: Option<SkeletonRootVector>,
    pub(crate) complete: bool,
}
impl PriorInputFields {
    pub(super) const fn new() -> Self {
        Self {
            surface_angle: None,
            empirical_acceleration: None,
            empirical_velocity: None,
            center_of_mass_delta: None,
            center_of_mass_animation: None,
            foot_targets: [None; 2],
            foot_ik: [None; 2],
            next_step_position: None,
            next_step_normal: None,
            skater_look_direction: None,
            complete: false,
        }
    }
    pub(super) fn reset(&mut self) {
        self.surface_angle = Some(0.0); //Reset394ec
        self.empirical_acceleration = Some(SkeletonRootVector::known([0.0; 4])); //3936c
        self.empirical_velocity = Some(SkeletonRootVector::known([0.0; 4])); //39370
        self.center_of_mass_delta = Some(WalkVector::known([0.0; 4])); //39320
        self.center_of_mass_animation = Some(WalkVector::known([0.0; 4])); //39328
        self.foot_targets = [Some(WalkVector::known([0.0; 4])); 2]; //39330/38
        self.foot_ik = [Some(0.0); 2]; //39474/7c
        self.next_step_position = Some(WalkVector::known([0.0; 4])); //39394
        self.next_step_normal = Some(WalkVector::known([0.0; 4])); //39398
        self.skater_look_direction = Some(SkeletonRootVector::known([0.0; 4])); //39274
        self.complete = false;
    }
}
#[derive(Debug, PartialEq)]
pub(crate) enum PriorInputRuntimeError {
    InputNotConstructed,
    ForeignOutput,
    Read(PriorInputReadError),
    Output(PhysOutAccessError),
    UnknownWord(&'static str),
}
impl std::fmt::Display for PriorInputRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputNotConstructed => f.write_str("prior-input animation owner not constructed"),
            Self::ForeignOutput => f.write_str("prior-input output occurrence differs"),
            Self::Read(e) => write!(f, "prior-input guarded read: {e:?}"),
            Self::Output(e) => write!(f, "prior-input output read: {e:?}"),
            Self::UnknownWord(word) => write!(f, "prior-input processed word {word} unknown"),
        }
    }
}
fn flag(
    word: &mut Option<u32>,
    mask: u32,
    value: bool,
    name: &'static str,
) -> Result<(), PriorInputRuntimeError> {
    let old = word.ok_or(PriorInputRuntimeError::UnknownWord(name))?;
    *word = Some((old & !mask) | if value { mask } else { 0 });
    Ok(())
}
impl SkeletonProcessedContact {
    /// Begins at source2430 after the caller's actual prefix. Ends at2a28,
    /// before the next Ground copy; does not run Reset or lock `PhysOut` for write.
    pub(crate) fn try_copy_prior_input_runtime(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        input: &AnimOutPhysInOwner,
    ) -> Result<(), PriorInputRuntimeError> {
        use PriorInputRuntimeError as E;
        self.toolkit_prefix_complete = false;
        self.toolkit.invalidate();
        self.prior_output.invalidate();
        self.prior_output.input_fields.complete = false;
        if input.stage != AnimOutConstructionStage::Returned {
            return Err(E::InputNotConstructed);
        }
        if !self.prior_output.bind_output_occurrence(output) {
            return Err(E::ForeignOutput);
        }
        self.copy_prior_state(output)?;
        self.prior_output.velocity = Some(
            output
                .motion_vector(PhysOutMotionVector::DeckVelocity)
                .map_err(E::Output)?,
        ); //263c
        self.prior_output.surface_velocity = Some(
            output
                .motion_vector(PhysOutMotionVector::SurfaceVelocity)
                .map_err(E::Output)?,
        ); //2678
        self.prior_output.input_fields.surface_angle = Some(
            output
                .motion_scalar(PhysOutMotionScalar::SurfaceAngle)
                .map_err(E::Output)?
                .value,
        ); //26ac
        self.prior_output.z_speed = Some(
            output
                .motion_scalar(PhysOutMotionScalar::ZSpeed)
                .map_err(E::Output)?,
        ); //26e0
        self.prior_output.surface_speed = Some(
            output
                .motion_scalar(PhysOutMotionScalar::SurfaceSpeed)
                .map_err(E::Output)?,
        ); //2714
        self.prior_output.deck_speed = Some(
            output
                .motion_scalar(PhysOutMotionScalar::DeckSpeed)
                .map_err(E::Output)?,
        ); //2748
        self.prior_output.angular_velocity = Some(
            output
                .motion_vector(PhysOutMotionVector::AngularVelocity)
                .map_err(E::Output)?,
        ); //2784
        self.prior_output.input_fields.empirical_acceleration = Some(
            output
                .try_guarded_motion_remainder()
                .map_err(E::Read)?
                .emperical_deck_acceleration,
        ); //27c0
        self.prior_output.input_fields.empirical_velocity = Some(
            output
                .try_guarded_motion_remainder()
                .map_err(E::Read)?
                .emperical_deck_velocity,
        ); //280c
        self.copy_prior_animation(input)?;
        self.prior_output.centre_of_mass = Some(
            output
                .system_vector(PhysOutSystemVector::CentreOfMass)
                .map_err(E::Output)?,
        ); //2944
        self.prior_output.centre_of_mass_velocity = Some(
            output
                .system_vector(PhysOutSystemVector::CentreOfMassVelocity)
                .map_err(E::Output)?,
        ); //297c
        self.prior_output.reckoning_up = Some(
            output
                .system_vector(PhysOutSystemVector::Up)
                .map_err(E::Output)?,
        ); //29b4
        self.prior_output.input_fields.skater_look_direction = Some(
            output
                .try_guarded_skateboard_reckoning()
                .map_err(E::Read)?
                .skater_look_direction,
        ); //29f0
        self.prior_output.ground_normal = Some(
            output
                .ground_vector(PhysOutGroundVector::LastValidGroundUp)
                .map_err(E::Output)?,
        ); //2a28
        self.prior_output.selected_copies_complete = true;
        self.prior_output.input_fields.complete = true;
        Ok(())
    }
    fn copy_prior_state(
        &mut self,
        output: &PhysicalPlayerPhysOut,
    ) -> Result<(), PriorInputRuntimeError> {
        use PriorInputRuntimeError as E;
        let state = output
            .try_guarded_state_output()
            .map_err(E::Read)?
            .physics_state;
        self.physics_state.physics = Some(state); //2460
        self.physics_state.current = Some(state); //2468
        self.physics_state.category = Some(
            output
                .try_guarded_state_output()
                .map_err(E::Read)?
                .physics_state_category,
        ); //249c
        flag(
            &mut self.flags_8c8,
            0x8000_0000,
            output
                .try_guarded_state_output()
                .map_err(E::Read)?
                .is_teleport_finished,
            "8c8",
        )?; //24d8
        flag(
            &mut self.flags_8c8,
            0x0040_0000,
            output
                .try_guarded_state_output()
                .map_err(E::Read)?
                .is_reverting,
            "8c8",
        )?; //2514
        flag(
            &mut self.flags_8c8,
            0x0008_0000,
            output
                .try_guarded_state_output()
                .map_err(E::Read)?
                .physics_requested_teleport,
            "8c8",
        )?; //2550
        flag(
            &mut self.physics_state.flags_8cc,
            1,
            output
                .try_guarded_state_output()
                .map_err(E::Read)?
                .is_hand_planting,
            "8cc",
        )?; //258c
        self.physics_state.grind_type = Some(
            output
                .try_guarded_grinds_output()
                .map_err(E::Read)?
                .grind_type,
        ); //25c0
        self.physics_state.grind_substate = Some(
            output
                .try_guarded_grinds_output()
                .map_err(E::Read)?
                .grind_substate,
        ); //25f4
        self.physics_state.last_category = self.physics_state.category; //2600 reread
        Ok(())
    }
    fn copy_prior_animation(
        &mut self,
        input: &AnimOutPhysInOwner,
    ) -> Result<(), PriorInputRuntimeError> {
        let fields = &mut self.prior_output.input_fields;
        fields.center_of_mass_delta = Some(input.center_of_mass_delta); //283c
        fields.center_of_mass_animation = Some(input.center_of_mass_animation); //284c
        // lbz at big-endian word2aa0 reads the high byte; source bits31..29
        // become processed bits7..5. No whole word or low-byte substitution.
        for mask in [0x80, 0x40, 0x20] {
            flag(
                &mut self.physics_state.flags_8cc,
                mask,
                (input.locomotion_flags >> 24) & mask != 0,
                "8cc",
            )?;
        } //2864/287c/2894
        fields.foot_ik[0] = Some(input.foot_ik[0]); //28a4
        fields.foot_targets[0] = Some(input.foot_targets[0]); //28b4
        fields.foot_ik[1] = Some(input.foot_ik[1]); //28c4
        fields.foot_targets[1] = Some(input.foot_targets[1]); //28d4
        flag(
            &mut self.physics_state.flags_8d0,
            8,
            input.flags.next_step_valid,
            "8d0",
        )?; //28ec
        fields.next_step_position = Some(input.next_step_position); //28fc
        fields.next_step_normal = Some(input.next_step_normal); //290c
        Ok(())
    }
}

#[cfg(test)]
#[path = "skeleton_processed_prior_output_runtime_tests.rs"]
mod tests;
