//! Consecutive unconditional `Update1_ProcessInput82e02a2c..2e5c` copies.
use super::{
    PhysOutAccessError, PhysOutGroundVector, PhysOutValue, SkeletonProcessedContact,
    SkeletonRootVector,
};
use crate::{
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::construct::prior_input::PriorInputReadError,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PriorOutputTailFields {
    pub(crate) dynamic_up: Option<PhysOutValue<SkeletonRootVector>>,
    pub(crate) anti_flip_torque: Option<SkeletonRootVector>,
    pub(crate) trajectory_collision_time: Option<f32>,
    pub(crate) complete: bool,
}

impl PriorOutputTailFields {
    pub(super) const fn new() -> Self {
        Self {
            dynamic_up: None,
            anti_flip_torque: None,
            trajectory_collision_time: None,
            complete: false,
        }
    }
    pub(super) fn reset(&mut self) {
        self.dynamic_up = Some(PhysOutValue {
            value: SkeletonRootVector::known([0.0; 4]),
            origin: crate::physical_player_phys_out::PhysOutOrigin::Reset,
        }); //3925c
        self.trajectory_collision_time = Some(0.0); //3952c
        // Reset39378/80 both write +330. Neither initializes anti-flip torque+340.
        self.complete = false;
    }
    pub(super) fn initialize_sim_residuals(&mut self) {
        self.anti_flip_torque = Some(SkeletonRootVector::known([0.0; 4]));
    }
}
#[derive(Debug, PartialEq)]
pub(crate) enum PriorOutputTailError {
    ForeignOutput,
    Poisoned,
    MissingHandPlant,
    UnknownWord(&'static str),
    Read(PriorInputReadError),
    Output(PhysOutAccessError),
}
impl std::fmt::Display for PriorOutputTailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("prior-output tail occurrence differs"),
            Self::Poisoned => f.write_str("prior-output tail owner lock poisoned"),
            Self::MissingHandPlant => {
                f.write_str("prior-output tail requires the retained HandPlantManager")
            }
            Self::UnknownWord(word) => write!(f, "prior-output tail word {word} unknown"),
            Self::Read(error) => write!(f, "prior-output tail guarded read: {error:?}"),
            Self::Output(error) => write!(f, "prior-output tail output read: {error:?}"),
        }
    }
}
fn flag(
    word: &mut Option<u32>,
    mask: u32,
    value: bool,
    name: &'static str,
) -> Result<(), PriorOutputTailError> {
    let old = word.ok_or(PriorOutputTailError::UnknownWord(name))?;
    *word = Some((old & !mask) | if value { mask } else { 0 });
    Ok(())
}
impl SkeletonProcessedContact {
    /// Samples each output getter separately, then the live hand-plant owner at2c38.
    /// There is no category gate: constructor-invalid category still copies these.
    pub(crate) fn try_copy_prior_output_tail(
        &mut self,
        owner: &PhysicalPlayerAssemblyOwner,
    ) -> Result<(), PriorOutputTailError> {
        use PriorOutputTailError as E;
        self.prior_output.tail_fields.complete = false;
        let output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
        self.try_copy_tail_fields(&output, || {
            let handplant = owner.handplant().lock().map_err(|_| E::Poisoned)?;
            Ok(handplant
                .as_ref()
                .ok_or(E::MissingHandPlant)?
                .animation
                .flags)
        })
    }
    #[expect(
        clippy::too_many_lines,
        reason = "native independently guarded copies retain exact partial-failure order"
    )]
    fn try_copy_tail_fields(
        &mut self,
        output: &crate::physical_player_phys_out::PhysicalPlayerPhysOut,
        handplant_flags: impl FnOnce() -> Result<u32, PriorOutputTailError>,
    ) -> Result<(), PriorOutputTailError> {
        use PriorOutputTailError as E;
        self.prior_output.tail_fields.complete = false;
        if !self.prior_output.matches_output_occurrence(output) {
            return Err(E::ForeignOutput);
        }
        self.prior_output.tail_fields.dynamic_up = Some(
            output
                .ground_vector(PhysOutGroundVector::DynamicUp)
                .map_err(E::Output)?,
        ); //2a60
        self.prior_output.tail_fields.anti_flip_torque = Some(
            output
                .try_guarded_ground_output()
                .map_err(E::Read)?
                .anti_flip_torque,
        ); //2a98
        flag(
            &mut self.physics_state.flags_8cc,
            0x2000_0000,
            output
                .try_guarded_ground_output()
                .map_err(E::Read)?
                .anti_flip_nudge_present,
            "8cc",
        )?; //2ad4
        flag(
            &mut self.physics_state.flags_8cc,
            0x0200_0000,
            output
                .try_guarded_ground_output()
                .map_err(E::Read)?
                .is_pinning,
            "8cc",
        )?; //2b10
        self.prior_output.tail_fields.trajectory_collision_time = Some(
            output
                .try_guarded_air_output()
                .map_err(E::Read)?
                .traj_collision_time,
        ); //2b44
        flag(
            &mut self.physics_state.flags_8cc,
            0x1000_0000,
            output
                .try_guarded_air_output()
                .map_err(E::Read)?
                .is_flipping,
            "8cc",
        )?; //2b80
        flag(
            &mut self.physics_state.flags_8cc,
            2,
            output
                .try_guarded_air_output()
                .map_err(E::Read)?
                .found_hand_plant_pos,
            "8cc",
        )?; //2bbc
        flag(
            &mut self.physics_state.flags_8d0,
            0x0400_0000,
            output
                .try_guarded_air_output()
                .map_err(E::Read)?
                .is_landing_foot_plant,
            "8d0",
        )?; //2bf8
        flag(
            &mut self.physics_state.flags_8d0,
            0x0200_0000,
            output
                .try_guarded_air_output()
                .map_err(E::Read)?
                .is_foot_planting,
            "8d0",
        )?; //2c34
        let flags = handplant_flags()?;
        flag(
            &mut self.physics_state.flags_8d0,
            0x8000_0000,
            flags & 0x1000_0000 != 0,
            "8d0",
        )?; //2c4c
        self.jump_events.process_animation_flag(
            0x200,
            output
                .try_guarded_grinds_output()
                .map_err(E::Read)?
                .is_grind_on_front,
        ); //2c88
        self.jump_events.process_animation_flag(
            0x100,
            output
                .try_guarded_grinds_output()
                .map_err(E::Read)?
                .did_jump,
        ); //2cc4
        self.jump_events.process_animation_flag(0x8000, output.try_guarded_collision_contact(crate::physical_player_phys_out::construct::prior_input::PriorContactFlag::Skeleton).map_err(E::Read)?); //2d00
        self.jump_events.process_animation_flag(
            0x1_0000,
            output
                .try_guarded_collision_contact(
                    crate::physical_player_phys_out::construct::prior_input::PriorContactFlag::Deck,
                )
                .map_err(E::Read)?,
        ); //2d3c
        self.jump_events.process_animation_flag(0x2_0000, output.try_guarded_collision_contact(crate::physical_player_phys_out::construct::prior_input::PriorContactFlag::Truck).map_err(E::Read)?); //2d78
        self.physics_state.num_wheels_in_contact =
            Some(output.try_guarded_wheels_in_contact().map_err(E::Read)?); //2dac
        self.jump_events.process_animation_flag(0x4000, output.try_guarded_collision_contact(crate::physical_player_phys_out::construct::prior_input::PriorContactFlag::Anything).map_err(E::Read)?); //2de8
        flag(
            &mut self.flags_8c8,
            0x2000,
            output
                .try_guarded_physics_output()
                .map_err(E::Read)?
                .is_impulse_for_wipeout,
            "8c8",
        )?; //2e24
        self.prior_output.wipeout_impulse = Some(
            output
                .try_guarded_physics_output()
                .map_err(E::Read)?
                .wipeout_impulse,
        ); //2e5c
        self.prior_output.tail_fields.complete = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_player_phys_out::{PhysicalPlayerPhysOut, construct::PhysOutAllocation};
    fn pair() -> (PhysicalPlayerPhysOut, SkeletonProcessedContact) {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        assert!(processed.prior_output.bind_output_occurrence(&output));
        (output, processed)
    }
    #[test]
    fn invalid_constructor_category_still_copies_air_ground_and_live_handplant() {
        let (output, mut processed) = pair();
        assert_eq!(
            output
                .try_guarded_state_output()
                .unwrap()
                .physics_state_category,
            0
        );
        processed.physics_state.flags_8cc = Some(u32::MAX);
        processed.physics_state.flags_8d0 = Some(u32::MAX);
        processed
            .try_copy_tail_fields(&output, || Ok(0x1000_0000))
            .unwrap();
        assert_eq!(processed.physics_state.flags_8cc, Some(!0x3200_0002));
        assert_eq!(processed.physics_state.flags_8d0, Some(!0x0600_0000));
        assert_eq!(
            processed.prior_output.tail_fields.dynamic_up.unwrap().value,
            SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0])
        );
        assert_eq!(processed.physics_state.num_wheels_in_contact, Some(0));
        assert!(processed.prior_output.tail_fields.complete);
    }
    #[test]
    fn missing_handplant_preserves_prior_air_stores_and_skips_later_contact_copies() {
        let (output, mut processed) = pair();
        processed.physics_state.flags_8d0 = Some(u32::MAX);
        processed.physics_state.num_wheels_in_contact = Some(77);
        processed.prior_output.wipeout_impulse = Some(SkeletonRootVector::known([9.0; 4]));
        assert_eq!(
            processed.try_copy_tail_fields(&output, || Err(PriorOutputTailError::MissingHandPlant)),
            Err(PriorOutputTailError::MissingHandPlant)
        );
        assert_eq!(processed.physics_state.flags_8d0, Some(!0x0600_0000));
        assert_eq!(processed.physics_state.num_wheels_in_contact, Some(77));
        assert_eq!(
            processed.prior_output.wipeout_impulse,
            Some(SkeletonRootVector::known([9.0; 4]))
        );
        assert!(!processed.prior_output.tail_fields.complete);
    }
    #[test]
    fn locked_first_getter_does_not_read_handplant_and_reset_preserves_unwritten_torque() {
        let (mut output, mut processed) = pair();
        output.lock_after_update1().unwrap();
        assert_eq!(
            processed.try_copy_tail_fields(&output, || panic!(
                "handplant sampled before preceding output getter"
            )),
            Err(PriorOutputTailError::Output(
                PhysOutAccessError::LockedForWrite
            ))
        );
        let mut fields = PriorOutputTailFields::new();
        fields.reset();
        assert_eq!(fields.anti_flip_torque, None);
        fields.anti_flip_torque = Some(SkeletonRootVector::known([3.0; 4]));
        fields.reset();
        assert_eq!(
            fields.anti_flip_torque,
            Some(SkeletonRootVector::known([3.0; 4]))
        );
    }
}
