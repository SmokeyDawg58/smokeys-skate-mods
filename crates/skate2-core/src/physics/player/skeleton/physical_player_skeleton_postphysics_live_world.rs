//! Draft constraint-session child: selected complete Skeleton.UpdatePostPhysics.
//! Caller must first execute the actual solver/contact-feedback prerequisites.
use super::{
    contact_feedback::SimulationContactFeedbackProgress,
    physical_contact_list::SimulationPhysicalContactListError,
    physical_island::SimulationPhysicalBodyError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonCollisionError,
        SimulationSkeletonPostRecordsError, postphysics_live::PostPhysicsPoolError,
    },
    input_cvar_runtime::{
        InputCvarReferences,
        postphysics_history_input::PostPhysicsHistoryInput,
        skeleton_postphysics_live::{
            SkeletonPostPhysicsCvars, SkeletonPostPhysicsServiceError, SkeletonPostRecordServices,
        },
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
    simulation_contact_jacobian::contact_spy::SimulationContactSpyError,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum SkeletonPostPhysicsLiveError<N> {
    Owner(SimulationPhysicalBodyError),
    ForeignPair,
    Missing(&'static str),
    IncompleteFrame,
    Spy(SimulationContactSpyError),
    Contacts(SimulationPhysicalContactListError),
    Poisoned,
    Prepare(PostPhysicsPoolError<N>),
    Input(PostPhysicsPoolError<N>),
    Collision(SimulationSkeletonCollisionError<SkeletonPostPhysicsServiceError>),
    Records(SimulationSkeletonPostRecordsError<SkeletonPostPhysicsServiceError>),
}
impl<N: std::fmt::Debug> std::fmt::Display for SkeletonPostPhysicsLiveError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(e) => e.fmt(f),
            Self::ForeignPair => f.write_str("Skeleton post-physics pair mismatch"),
            Self::Missing(s) => write!(f, "Skeleton post-physics missing {s}"),
            Self::IncompleteFrame => {
                f.write_str("Skeleton post-physics requires completed feedback")
            }
            Self::Spy(e) => e.fmt(f),
            Self::Contacts(e) => e.fmt(f),
            Self::Poisoned => f.write_str("Skeleton post-physics owner poisoned"),
            Self::Prepare(e) | Self::Input(e) => e.fmt(f),
            Self::Collision(e) => e.fmt(f),
            Self::Records(e) => e.fmt(f),
        }
    }
}

impl PhysicalAssemblyWorld {
    /// `HiLOD1d34` samples actual dt+93c before calling Skeleton at1d4c. The
    /// explicit boolean is that native caller's already-evaluated r30 argument;
    /// this does not fabricate deck-in-hand/bad-board predicate results.
    /// This covers Skeleton575d0..578b4, not surrounding HiLOD.Update4 stages.
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered caller over retained owners and frame checks"
    )]
    pub(crate) fn try_update_skeleton_postphysics_live<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        references: &InputCvarReferences,
        request_partial_ragdoll: bool,
        numeric: &mut N,
    ) -> Result<(), SkeletonPostPhysicsLiveError<N::Error>> {
        use SkeletonPostPhysicsLiveError as E;
        let owner = Arc::clone(
            self.physical_body_owner_for_contacts(skeleton)
                .map_err(E::Owner)?,
        );
        if !Arc::ptr_eq(
            self.physical_body_owner_for_contacts(board)
                .map_err(E::Owner)?,
            &owner,
        ) {
            return Err(E::ForeignPair);
        }
        // Same host admission as existing normal history: require the actual
        // sealed solved frame even if the retained contact vector is empty.
        let frame = Arc::clone(
            self.contact_spies
                .as_ref()
                .ok_or(E::Missing("solved contact frame"))?,
        );
        if !matches!(
            self.contact_feedback_progress,
            SimulationContactFeedbackProgress::Complete { .. }
        ) {
            return Err(E::IncompleteFrame);
        }
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(E::Spy)?;
        self.physical_contacts
            .try_validate_completed_frame(&frame)
            .map_err(E::Contacts)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let timestep = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("processed dt+93c"))?;
        let overlay = self.debug_overlay.clone();
        let mut cvars = SkeletonPostPhysicsCvars::new(references, &overlay);
        let mut history_input = PostPhysicsHistoryInput {
            references,
            processed: &processed,
        };
        let before = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_prepare_skeleton_postphysics_live(
                &mut self.backend,
                board,
                skeleton,
                &owner,
                &processed,
                frame,
                timestep,
                request_partial_ragdoll,
                &mut cvars,
                &mut history_input,
                numeric,
            )
            .map_err(E::Prepare)?;
        // Actual native GetCollisions5770c obtains the SAME retained vector.
        // Exclusive world access is this implementation's existing Island lock.
        self.try_get_physical_collisions(skeleton)
            .map_err(E::Contacts)?;
        // Native GetRWSimulation57728 resolves the same manager/body pool that
        // existing collision services use. No substitute or copied simulation
        // object is constructed between the contact pull and late input reads.
        let info = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_read_skeleton_postphysics_collision_input::<N::Error>(
                &self.backend,
                skeleton,
                &before,
            )
            .map_err(E::Input)?;
        let contacts = self.physical_contacts.retained_contacts(skeleton);
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_update_skeleton_normal_collision(
                &self.backend,
                skeleton,
                &before.frame,
                contacts,
                &info,
                &mut cvars,
                self.world_line_queries
                    .as_ref()
                    .map(|queries| &queries.vehicle),
                self.dmos.as_ref(),
                self.pedestrians.as_ref(),
            )
            .map_err(E::Collision)?;
        let mut records = SkeletonPostRecordServices::new(references, &overlay, &owner);
        // The callback executes inside the SAME UpdateRecords kernel, AFTER
        // CalculateErrors, COM and velocity publication at the native bit1000
        // read. A missing field therefore retains that entire write prefix.
        let flags = || {
            processed
                .lock()
                .map_err(|_| SkeletonPostPhysicsServiceError::ProcessedPoisoned)?
                .known_flags_8c8()
                .ok_or(SkeletonPostPhysicsServiceError::MissingProcessedFlags)
        };
        // Existing method updates SAME volume states first, then SAME records.
        // All processed/Reckoning guards above have been dropped; its lazy
        // ground-normal callback acquires actual Reckoning without deadlock.
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_update_skeleton_post_physics_records_with_flags(
                &self.backend,
                skeleton,
                &before.frame,
                before.timestep,
                &mut records,
                flags,
            )
            .map_err(E::Records)
    }
}
