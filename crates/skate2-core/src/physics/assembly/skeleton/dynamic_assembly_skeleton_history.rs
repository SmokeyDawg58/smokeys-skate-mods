//! Actual initial history (`Skeleton::Init` 0x82b62a70..0x82b62ac8) and the
//! ongoing `UpdatePhysicsRecords` history prefix through 0x82b4cc70.
//! Both update the SAME retained owner; no replacement Part poses or handles.
use super::{SkeletonNormalAssembly, drive_owner::SkeletonDriveOwnerStage};
pub(crate) use crate::skeleton_physics_history::SkeletonHistoryInput;
use crate::{
    dynamic_assembly_session::{
        part_transform::DynamicAssemblyPartTransformError, skater_pool::SimulationSkaterPoolError,
    },
    simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skeleton_physics_history::{
        SkeletonPhysicsHistory, SkeletonPhysicsHistoryError, SkeletonPhysicsHistoryNumeric,
        SkeletonPhysicsMassError,
    },
};
use std::{fmt, sync::Arc};

#[cfg(test)]
#[path = "dynamic_assembly_skeleton_history_tests.rs"]
pub(crate) mod tests;

/// Checked host lifecycle. Failure retains both the stage and actual write
/// prefix; rerunning initialization must not silently replace that history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonHistoryInitialization {
    NotStarted,
    Updating { completed_updates: usize },
    Complete,
}

/// Host one-attempt bookkeeping over the actual completed/exported frame,
/// not a native numeric frame ID. A failed update retains Updating and its
/// record prefix; the same frame cannot replay those writes. Progress describes
/// a retained attempt, NOT freshness after a later frame or body mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonHistoryFrameProgress {
    NotStarted,
    Updating,
    Complete,
}

pub(crate) struct SkeletonHistoryFrameState {
    frame: Option<Arc<SimulationContactSpyFrame>>,
    progress: SkeletonHistoryFrameProgress,
}
impl SkeletonHistoryFrameState {
    pub(crate) fn new() -> Self {
        Self {
            frame: None,
            progress: SkeletonHistoryFrameProgress::NotStarted,
        }
    }
    pub(crate) fn frame(&self) -> Option<&Arc<SimulationContactSpyFrame>> {
        self.frame.as_ref()
    }
    pub(crate) fn progress(&self) -> SkeletonHistoryFrameProgress {
        self.progress
    }
    fn try_begin(&mut self, frame: Arc<SimulationContactSpyFrame>) -> bool {
        if self
            .frame
            .as_ref()
            .is_some_and(|previous| Arc::ptr_eq(previous, &frame))
        {
            return false;
        }
        self.frame = Some(frame);
        self.progress = SkeletonHistoryFrameProgress::Updating;
        true
    }
    fn complete(&mut self) {
        assert_eq!(
            self.progress,
            SkeletonHistoryFrameProgress::Updating,
            "only a begun actual history update may complete"
        );
        self.progress = SkeletonHistoryFrameProgress::Complete;
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonHistoryInitializationError<E> {
    Detached,
    ForeignBackend,
    DrivesNotReady,
    PoseNotSet,
    AlreadyAttempted,
    Mass(SkeletonPhysicsMassError),
    Input(E),
    LiveRoot(DynamicAssemblyPartTransformError),
    History(SkeletonPhysicsHistoryError<DynamicAssemblyPartTransformError, E>),
}
impl<E: fmt::Debug> fmt::Display for SkeletonHistoryInitializationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Detached => f.write_str("physics history requires an attached skeleton"),
            Self::ForeignBackend => f.write_str("physics history backend mismatch"),
            Self::DrivesNotReady => {
                f.write_str("physics history requires complete skeleton drive admission")
            }
            Self::PoseNotSet => {
                f.write_str("physics history requires successful full skeleton Set")
            }
            Self::AlreadyAttempted => {
                f.write_str("initial physics history has already been attempted")
            }
            Self::Mass(e) => write!(f, "skeleton mass initialization: {e:?}"),
            Self::Input(e) => write!(f, "skeleton current input acquisition: {e:?}"),
            Self::LiveRoot(e) => write!(f, "skeleton initial root getter: {e}"),
            Self::History(e) => write!(f, "skeleton physics history: {e:?}"),
        }
    }
}
impl<E: fmt::Debug> std::error::Error for SkeletonHistoryInitializationError<E> {}

#[derive(Debug)]
pub(crate) enum SkeletonHistoryManagerError<E> {
    Pool(SimulationSkaterPoolError),
    Initialization(SkeletonHistoryInitializationError<E>),
}
impl<E: fmt::Debug> fmt::Display for SkeletonHistoryManagerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "skeleton history member: {e}"),
            Self::Initialization(e) => write!(f, "{e}"),
        }
    }
}
impl<E: fmt::Debug> std::error::Error for SkeletonHistoryManagerError<E> {}

#[derive(Debug)]
pub(crate) enum SkeletonHistoryUpdateError<E> {
    Detached,
    ForeignBackend,
    BoardDetached,
    ForeignBoardBackend,
    InitializationIncomplete,
    LiveBoard(DynamicAssemblyPartTransformError),
    Input(E),
    History(SkeletonPhysicsHistoryError<DynamicAssemblyPartTransformError, E>),
}
impl<E: fmt::Debug> fmt::Display for SkeletonHistoryUpdateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Detached => f.write_str("ongoing history requires an attached skeleton"),
            Self::ForeignBackend => f.write_str("ongoing history skeleton backend mismatch"),
            Self::BoardDetached => {
                f.write_str("ongoing history requires the attached paired board")
            }
            Self::ForeignBoardBackend => f.write_str("ongoing history board backend mismatch"),
            Self::InitializationIncomplete => {
                f.write_str("ongoing history requires completed constructor history")
            }
            Self::LiveBoard(error) => write!(f, "ongoing history board Part6: {error}"),
            Self::Input(error) => write!(f, "ongoing history current input: {error:?}"),
            Self::History(error) => write!(f, "ongoing history: {error:?}"),
        }
    }
}
impl<E: fmt::Debug> std::error::Error for SkeletonHistoryUpdateError<E> {}

impl SkeletonNormalAssembly {
    pub(crate) fn physics_history_frame_state(&self) -> &SkeletonHistoryFrameState {
        &self.history_frame
    }

    /// Called only after the manager has validated current solved-frame and
    /// original paired ownership plus attached/initialized prerequisites.
    pub(crate) fn try_begin_physics_history_frame(
        &mut self,
        frame: Arc<SimulationContactSpyFrame>,
    ) -> bool {
        self.history_frame.try_begin(frame)
    }

    pub(crate) fn complete_physics_history_frame(&mut self) {
        self.history_frame.complete();
    }

    pub(crate) fn physics_history(&self) -> &SkeletonPhysicsHistory {
        &self.physics_history
    }

    /// History PREFIX of `Skeleton::UpdatePhysicsRecords` 0x82b4cc00..cc70,
    /// called by `UpdatePostPhysics` at 0x82b576d8 before contact-list readout.
    /// Unlike the constructor, record0 receives the live paired BOARD Part6
    /// matrix. The existing kernel then separately samples skeleton Part0 for
    /// its proxy position, followed by skeleton Parts1..23.
    ///
    /// The manager must supply the original shared-owner pair and enforce its
    /// current completed-solve/one-attempt lifecycle. These same-backend and
    /// completed-initialization checks are additional safe-host prerequisites,
    /// not native assertion order. A later failure retains all kernel writes;
    /// it never recreates masses/history or silently replays initialization.
    ///
    /// The suffix after cc70 uses the unimplemented world-to-animation owner
    /// and writes Skeleton+3d40/+3d60. Neither it nor the preceding bad-data/
    /// skateboard-error work is claimed here. Existing live Part canonical-W,
    /// host numeric and current-input service boundaries remain unchanged.
    pub(crate) fn try_update_physics_history_from_board<I: SkeletonHistoryInput>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SkateboardAssemblySession,
        input: &mut I,
        numeric: &mut impl SkeletonPhysicsHistoryNumeric,
    ) -> Result<
        crate::skeleton_root_transforms::SkeletonRootVector,
        SkeletonHistoryUpdateError<I::Error>,
    > {
        use SkeletonHistoryUpdateError as Error;
        match self.session.attached_authority {
            None => return Err(Error::Detached),
            Some(authority) if authority != backend.authority() => {
                return Err(Error::ForeignBackend);
            }
            Some(_) => {}
        }
        match board.attached_authority {
            None => return Err(Error::BoardDetached),
            Some(authority) if authority != backend.authority() => {
                return Err(Error::ForeignBoardBackend);
            }
            Some(_) => {}
        }
        if self.history_initialization != SkeletonHistoryInitialization::Complete {
            return Err(Error::InitializationIncomplete);
        }
        // cc4c: the actual Body getter completes BEFORE acquiring the current
        // processed-input pointer at cc5c. Do not substitute skeleton Part0.
        let deck = board
            .try_live_part_transform_typed(backend, 6)
            .map_err(Error::LiveBoard)?;
        let up = input.acquire_up_vector().map_err(Error::Input)?;
        self.physics_history
            .try_update_typed(
                deck,
                &up,
                &mut |part| self.session.try_live_part_transform_typed(backend, part),
                input,
                numeric,
            )
            .map_err(Error::History)?;
        // Return the SAME cc4c Part6 capture with its actual lane knowledge.
        Ok(deck.columns()[3])
    }
    pub(crate) const fn physics_history_initialization(&self) -> SkeletonHistoryInitialization {
        self.history_initialization
    }

    /// Source mass initialization then TWO initial samples. Each fresh actual
    /// root getter precedes acquisition of the current processed-input owner;
    /// its up-vector lanes remain lazy until the COM mode requests them.
    pub(crate) fn try_initialize_physics_history<I: SkeletonHistoryInput>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        input: &mut I,
        numeric: &mut impl SkeletonPhysicsHistoryNumeric,
    ) -> Result<(), SkeletonHistoryInitializationError<I::Error>> {
        use SkeletonHistoryInitializationError as Error;
        match self.session.attached_authority {
            None => return Err(Error::Detached),
            Some(authority) if authority != backend.authority() => {
                return Err(Error::ForeignBackend);
            }
            Some(_) => {}
        }
        if self.history_initialization != SkeletonHistoryInitialization::NotStarted {
            return Err(Error::AlreadyAttempted);
        }
        if !self
            .drives
            .as_ref()
            .is_some_and(|runtime| runtime.stage() == SkeletonDriveOwnerStage::Complete)
        {
            return Err(Error::DrivesNotReady);
        }
        if !self.retained_pose_set {
            return Err(Error::PoseNotSet);
        }
        self.history_initialization = SkeletonHistoryInitialization::Updating {
            completed_updates: 0,
        };
        self.physics_history
            .try_initialize_masses()
            .map_err(Error::Mass)?;
        for sample in 0..2 {
            let root = self
                .session
                .try_live_part_transform_typed(backend, 0)
                .map_err(Error::LiveRoot)?;
            let up = input.acquire_up_vector().map_err(Error::Input)?;
            self.physics_history
                .try_update_typed(
                    root,
                    &up,
                    &mut |part| self.session.try_live_part_transform_typed(backend, part),
                    input,
                    numeric,
                )
                .map_err(Error::History)?;
            self.history_initialization = SkeletonHistoryInitialization::Updating {
                completed_updates: sample + 1,
            };
        }
        self.history_initialization = SkeletonHistoryInitialization::Complete;
        Ok(())
    }
}
