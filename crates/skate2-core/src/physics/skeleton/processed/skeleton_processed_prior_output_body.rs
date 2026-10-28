//! Consecutive prior Skeleton/Filtered/OffBoard copies at Update1 82e02e60..312c.
//! These execute for every category, including the constructor's invalid state.
use super::{PhysicalPlayerPhysOut, SkeletonProcessedContact};
use crate::{
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::construct::prior_input::PriorInputReadError,
    physics::FilteredPhysicsState,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PriorOutputBodyFields {
    /// Processed +8f4, distinct from its raw physics state +8e8.
    pub(crate) filtered_physics_state: Option<FilteredPhysicsState>,
    /// Processed +a0c, distinct from the later `HiLOD` time-offboard accumulator.
    pub(crate) offboard_time_to_land: Option<f32>,
    pub(crate) complete: bool,
}

impl PriorOutputBodyFields {
    pub(super) const fn new() -> Self {
        Self {
            filtered_physics_state: None,
            offboard_time_to_land: None,
            complete: false,
        }
    }
    pub(super) fn reset(&mut self) {
        // Reset82b394b8 stores integer zero; Reset82b39580 stores float zero.
        self.filtered_physics_state = Some(FilteredPhysicsState::Invalid);
        self.offboard_time_to_land = Some(0.0);
        self.complete = false;
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum PriorOutputBodyError {
    ForeignOutput,
    Poisoned,
    UnknownWord(&'static str),
    Read(PriorInputReadError),
}
impl std::fmt::Display for PriorOutputBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("prior-output body occurrence differs"),
            Self::Poisoned => f.write_str("prior-output body owner lock poisoned"),
            Self::UnknownWord(word) => write!(f, "prior-output body word {word} unknown"),
            Self::Read(error) => write!(f, "prior-output body guarded read: {error:?}"),
        }
    }
}

fn flag(
    word: &mut Option<u32>,
    mask: u32,
    value: bool,
    name: &'static str,
) -> Result<(), PriorOutputBodyError> {
    let old = word.ok_or(PriorOutputBodyError::UnknownWord(name))?;
    *word = Some((old & !mask) | if value { mask } else { 0 });
    Ok(())
}

impl SkeletonProcessedContact {
    /// Reads the actual same-pair output owner; does not lock native `PhysOut` for
    /// writing or imply that the next wheel/timer/ground-position work ran.
    pub(crate) fn try_copy_prior_output_body(
        &mut self,
        owner: &PhysicalPlayerAssemblyOwner,
    ) -> Result<(), PriorOutputBodyError> {
        self.prior_output.body_fields.complete = false;
        let output = owner
            .phys_out()
            .lock()
            .map_err(|_| PriorOutputBodyError::Poisoned)?;
        self.try_copy_body_fields(&output)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "source independently guards each component before its ordered store"
    )]
    fn try_copy_body_fields(
        &mut self,
        output: &PhysicalPlayerPhysOut,
    ) -> Result<(), PriorOutputBodyError> {
        use PriorOutputBodyError as E;
        self.prior_output.body_fields.complete = false;
        if !self.prior_output.matches_output_occurrence(output) {
            return Err(E::ForeignOutput);
        }
        flag(
            &mut self.flags_8c8,
            0x800,
            output
                .try_guarded_skeleton_output()
                .map_err(E::Read)?
                .skateboard_dont_push,
            "8c8",
        )?; //2e98
        flag(
            &mut self.physics_state.flags_8d0,
            0x2000,
            output
                .try_guarded_skeleton_output()
                .map_err(E::Read)?
                .is_left_foot_on_deck,
            "8d0",
        )?; //2ed4
        flag(
            &mut self.physics_state.flags_8d0,
            0x1000,
            output
                .try_guarded_skeleton_output()
                .map_err(E::Read)?
                .is_right_foot_on_deck,
            "8d0",
        )?; //2f10
        self.prior_output.body_fields.filtered_physics_state = Some(
            output
                .try_guarded_filtered_output()
                .map_err(E::Read)?
                .filtered_physics_state,
        ); //2f44
        flag(
            &mut self.physics_state.flags_8cc,
            0x0080_0000,
            output
                .try_guarded_offboard_output()
                .map_err(E::Read)?
                .is_grabbing_object,
            "8cc",
        )?; //2f80
        let time_to_land = output
            .try_guarded_offboard_output()
            .map_err(E::Read)?
            .time_to_land;
        // 2fb8=40990008 is BO4/BI25: branch when GT is false, including NaN.
        let has_landed = time_to_land.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater);
        flag(
            &mut self.physics_state.flags_8cc,
            0x0008_0000,
            has_landed,
            "8cc",
        )?; //2fcc
        flag(
            &mut self.physics_state.flags_8d0,
            0x0004_0000,
            output
                .try_guarded_offboard_output()
                .map_err(E::Read)?
                .is_holding_skateboard,
            "8d0",
        )?; //3008
        flag(
            &mut self.physics_state.flags_8cc,
            0x0004_0000,
            output
                .try_guarded_offboard_output()
                .map_err(E::Read)?
                .on_deck,
            "8cc",
        )?; //3044
        flag(
            &mut self.physics_state.flags_8cc,
            0x200,
            output
                .try_guarded_offboard_output()
                .map_err(E::Read)?
                .can_land_on_skateboard,
            "8cc",
        )?; //3080
        self.prior_output.body_fields.offboard_time_to_land = Some(
            output
                .try_guarded_offboard_output()
                .map_err(E::Read)?
                .time_to_land,
        ); //30b4: independent fresh read, not the earlier comparison sample.
        flag(
            &mut self.physics_state.flags_8d0,
            0x10,
            output
                .try_guarded_offboard_output()
                .map_err(E::Read)?
                .land_on_skateboard_flip_board,
            "8d0",
        )?; //30f0
        let invalid = output
            .try_guarded_offboard_output()
            .map_err(E::Read)?
            .is_board_mount_invalid;
        let word = &mut self.physics_state.flags_8d4;
        word.0 = (word.0 & !0x0080_0000) | if invalid { 0x0080_0000 } else { 0 };
        word.1 |= 0x0080_0000; //312c establishes just this bit, preserving unknown neighbors.
        self.prior_output.body_fields.complete = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "skeleton_processed_prior_output_body_tests.rs"]
mod tests;
