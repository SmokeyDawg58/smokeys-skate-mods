//! The history prefix of `Skeleton::UpdatePhysicsRecords` (0x82b4cc00..cc70),
//! reached from `UpdatePostPhysics` at 0x82b576d8. Uses the SAME paired physical
//! owner and solved manager frame, not constructor history or a copied pose.
//! World-to-animation suffix and full Actor/processed-input scheduling remain
//! outside this operation. Current input is an explicit caller collaborator.
use super::{
    SimulationAssemblyConstraintSession, contact_feedback::SimulationContactFeedbackProgress,
    physical_contact_list::SimulationPhysicalContactListError,
    physical_island::SimulationPhysicalBodyError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        SimulationSkeletonHistoryUpdateError,
    },
    simulation_contact_jacobian::contact_spy::SimulationContactSpyError,
    skateboard_hook::SkateboardHookDefinition,
    skeleton_physics_history::{HostSkeletonPhysicsHistoryNumeric, SkeletonHistoryInput},
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum PhysicalPlayerHistoryError<E> {
    MissingFrame,
    IncompleteFrame,
    Owner(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Spy(SimulationContactSpyError),
    ContactList(SimulationPhysicalContactListError),
    Update(SimulationSkeletonHistoryUpdateError<E>),
}
impl<E: fmt::Debug> fmt::Display for PhysicalPlayerHistoryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFrame => f.write_str("skeleton history requires a solved manager frame"),
            Self::IncompleteFrame => {
                f.write_str("skeleton history requires completed post-solve reset/freezing")
            }
            Self::Owner(error) => write!(f, "skeleton history owner: {error}"),
            Self::Pool(error) => write!(f, "skeleton history pool: {error}"),
            Self::Spy(error) => write!(f, "skeleton history frame: {error}"),
            Self::ContactList(error) => write!(f, "skeleton history contact-list phase: {error}"),
            Self::Update(error) => write!(f, "{error}"),
        }
    }
}
impl<E: fmt::Debug> std::error::Error for PhysicalPlayerHistoryError<E> {}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerNormalPostPhysicsError<H, C, S> {
    History(PhysicalPlayerHistoryError<H>),
    Contacts(SimulationPhysicalContactListError),
    Collision(crate::dynamic_assembly_session::skater_pool::SimulationSkeletonCollisionError<C>),
    Records(crate::dynamic_assembly_session::skater_pool::SimulationSkeletonPostRecordsError<S>),
}
impl<H: fmt::Debug, C: fmt::Debug, S: fmt::Debug> fmt::Display
    for PhysicalPlayerNormalPostPhysicsError<H, C, S>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::History(error) => write!(f, "{error}"),
            Self::Contacts(error) => write!(f, "skeleton contact pull: {error}"),
            Self::Collision(error) => write!(f, "{error}"),
            Self::Records(error) => write!(f, "{error}"),
        }
    }
}
impl<H: fmt::Debug, C: fmt::Debug, S: fmt::Debug> std::error::Error
    for PhysicalPlayerNormalPostPhysicsError<H, C, S>
{
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Controlled current mapped-pose projection into the SAME live drive
    /// payloads for the next Build. Missing `ProcessData` adjustments/IK and hook
    /// movement are explicitly excluded; this is not full `GeneralUpdate`.
    pub(crate) fn try_update_skeleton_drives_from_controlled_animation_pose<
        C: crate::skeleton_drives::update::SkeletonDrivesUpdateCvars,
    >(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        pose: &[crate::animation::AnimationMatrix],
        partial_mask: u8,
        cvars: &mut C,
    ) -> Result<(), PhysicalPlayerDriveFeedbackError<C::Error>> {
        use PhysicalPlayerDriveFeedbackError as Error;
        self.physical_body_owner_for_contacts(skeleton)
            .map_err(Error::Owner)?;
        let frame = self.contact_spies.as_ref().ok_or(Error::MissingFrame)?;
        if !matches!(
            self.contact_feedback_progress,
            SimulationContactFeedbackProgress::Complete { .. }
        ) {
            return Err(Error::IncompleteFrame);
        }
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(Error::Spy)?;
        self.physical_contacts
            .try_validate_completed_frame(frame)
            .map_err(Error::ContactList)?;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_update_skeleton_drives_from_controlled_pose(
                &self.backend,
                &self.hooks,
                skeleton,
                frame,
                pose,
                partial_mask,
                cvars,
            )
            .map_err(Error::Update)
    }

    /// Connected normal-mode history -> collisions -> volume states -> records.
    /// The exclusive world borrow keeps history current through the consumer;
    /// caller mutation cannot intervene between those actual owner operations.
    /// The earlier history animation suffix and full Actor scheduling remain
    /// missing. A later error keeps already-completed history/collision/state.
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "distinct source collaborators and passed flags"
    )]
    pub(crate) fn try_update_skeleton_normal_post_physics<
        I: SkeletonHistoryInput,
        C: crate::skeleton_collision_data::update::SkeletonCollisionCvars,
        S: crate::skeleton_post_physics_records::SkeletonPostPhysicsRecordServices,
    >(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        history_input: &mut I,
        collision_input: &crate::skeleton_collision_data::update::SkeletonCollisionUpdateInput,
        cvars: &mut C,
        flags_8c8: u32,
        records_services: &mut S,
    ) -> Result<(), PhysicalPlayerNormalPostPhysicsError<I::Error, C::Error, S::Error>> {
        use PhysicalPlayerNormalPostPhysicsError as Error;
        self.try_update_skeleton_physics_history(skeleton, board, history_input)
            .map_err(Error::History)?;
        self.try_get_physical_collisions(skeleton)
            .map_err(Error::Contacts)?;
        let frame = self
            .contact_spies
            .as_ref()
            .expect("successful history validated current frame");
        let contacts = self.physical_contacts.retained_contacts(skeleton);
        self.skaters
            .as_mut()
            .expect("successful history validated pool")
            .try_update_skeleton_normal_collision(
                &self.backend,
                skeleton,
                frame,
                contacts,
                collision_input,
                cvars,
                self.world_line_queries
                    .as_ref()
                    .map(|queries| &queries.vehicle),
                self.dmos.as_ref(),
                self.pedestrians.as_ref(),
            )
            .map_err(Error::Collision)?;
        self.skaters
            .as_mut()
            .expect("successful collision validated pool")
            .try_update_skeleton_post_physics_records(
                &self.backend,
                skeleton,
                frame,
                collision_input.timestep,
                flags_8c8,
                records_services,
            )
            .map_err(Error::Records)
    }

    /// Host sequencing: validate the actual completed frame even when it has
    /// zero spies; reject replay on the same Skeleton owner before source reads.
    /// History-only writes do not invalidate live collision publications/spies.
    /// A typed failure retains its write prefix and marks the attempt incomplete.
    pub(crate) fn try_update_skeleton_physics_history<I: SkeletonHistoryInput>(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        input: &mut I,
    ) -> Result<(), PhysicalPlayerHistoryError<I::Error>> {
        use PhysicalPlayerHistoryError as Error;
        self.physical_body_owner_for_contacts(skeleton)
            .map_err(Error::Owner)?;
        self.physical_body_owner_for_contacts(board)
            .map_err(Error::Owner)?;
        let frame = self.contact_spies.as_ref().ok_or(Error::MissingFrame)?;
        if !matches!(
            self.contact_feedback_progress,
            SimulationContactFeedbackProgress::Complete { .. }
        ) {
            return Err(Error::IncompleteFrame);
        }
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(Error::Spy)?;
        self.physical_contacts
            .try_validate_completed_frame(frame)
            .map_err(Error::ContactList)?;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_update_paired_skeleton_history(
                &self.backend,
                skeleton,
                board,
                Arc::clone(frame),
                input,
                &mut HostSkeletonPhysicsHistoryNumeric,
            )
            .map_err(Error::Update)
    }
}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerDriveFeedbackError<C> {
    MissingFrame,
    IncompleteFrame,
    Owner(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Spy(SimulationContactSpyError),
    ContactList(SimulationPhysicalContactListError),
    Update(crate::dynamic_assembly_session::skeleton_normal::drive_owner::update_owner::SkeletonDriveFeedbackError<C>),
}
impl<C: fmt::Debug> fmt::Display for PhysicalPlayerDriveFeedbackError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFrame => f.write_str("drive feedback requires completed frame"),
            Self::IncompleteFrame => f.write_str("drive feedback frame incomplete"),
            Self::Owner(e) => write!(f, "drive feedback owner: {e}"),
            Self::Pool(e) => write!(f, "drive feedback pool: {e}"),
            Self::Spy(e) => write!(f, "drive feedback frame: {e}"),
            Self::ContactList(e) => write!(f, "drive feedback contacts: {e}"),
            Self::Update(e) => write!(f, "{e}"),
        }
    }
}
impl<C: fmt::Debug> std::error::Error for PhysicalPlayerDriveFeedbackError<C> {}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::dynamic_assembly_session::skeleton_normal::history_owner::SkeletonHistoryFrameProgress;
    use crate::simulation_assembly_backend::constraint_session::physical_contact_list::SimulationPhysicalContactLists;

    struct NoInputReads;
    impl SkeletonHistoryInput for NoInputReads {
        type Error = std::convert::Infallible;
        type Up = ();
        fn acquire_up_vector(&mut self) -> Result<(), Self::Error> {
            panic!("guard must precede input")
        }
        fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
            panic!("guard must precede COM mode")
        }
        fn read_up_vector(&mut self, (): &()) -> Result<[f32; 4], Self::Error> {
            panic!("guard must precede up load")
        }
    }

    /// Controlled phase invalidation on the real paired corpus frame. Retain
    /// and restore its actual lists; never manufacture a completed spy export.
    pub(crate) fn check_mounted_phase_guards(
        world: &mut SimulationAssemblyConstraintSession<
            AssemblyPartMaterial,
            SkateboardHookDefinition,
        >,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
    ) {
        let progress = world.contact_feedback_progress;
        world.contact_feedback_progress = SimulationContactFeedbackProgress::NotStarted;
        assert!(matches!(
            world.try_update_skeleton_physics_history(skeleton, board, &mut NoInputReads),
            Err(PhysicalPlayerHistoryError::IncompleteFrame)
        ));
        world.contact_feedback_progress = progress;
        let actual_lists = std::mem::replace(
            &mut world.physical_contacts,
            SimulationPhysicalContactLists::new(),
        );
        assert!(matches!(
            world.try_update_skeleton_physics_history(skeleton, board, &mut NoInputReads),
            Err(PhysicalPlayerHistoryError::ContactList(
                SimulationPhysicalContactListError::CreationIncomplete
            ))
        ));
        world.physical_contacts = actual_lists;
        for (selected_skeleton, selected_board) in [(board, board), (skeleton, skeleton)] {
            assert!(matches!(
                world.try_update_skeleton_physics_history(
                    selected_skeleton,
                    selected_board,
                    &mut NoInputReads
                ),
                Err(PhysicalPlayerHistoryError::Update(
                    SimulationSkeletonHistoryUpdateError::Pool(
                        SimulationSkaterPoolError::WrongMemberKind
                    )
                ))
            ));
        }
        let state = world
            .skeleton(skeleton)
            .unwrap()
            .physics_history_frame_state();
        assert_eq!(state.progress(), SkeletonHistoryFrameProgress::NotStarted);
        assert!(state.frame().is_none());
    }
}
