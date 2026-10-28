//! Selected ordinary continuation37e64 -> queue reset37e78..80 after the
//! actual Body suffix. Bad-data/accelerated/rewind branch selection is not
//! implemented by this entrypoint; the live Actor caller remains required.
use super::{
    board_up_input::{PhysicalPlayerBoardUpInput, PhysicalPlayerBoardUpInputError},
    physical_board_forces::PhysicalPlayerBoardForceError,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
    skateboard_update_up_attributes::{
        SkateboardUpdateUpAttributeError, SkateboardUpdateUpVaultAttributes,
    },
    skateboard_update_up_vector::SkateboardUpdateUpError,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardUpPostError<N> {
    Owner(PhysicalPlayerBoardForceError),
    Refused(&'static str),
    Input(PhysicalPlayerBoardUpInputError),
    Update(
        SkateboardUpdateUpError<
            PhysicalPlayerBoardUpInputError,
            SkateboardUpdateUpAttributeError,
            N,
        >,
    ),
}
impl<N: fmt::Debug> fmt::Display for PhysicalPlayerBoardUpPostError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "up post owner: {e}"),
            Self::Refused(e) => write!(f, "up post: {e}"),
            Self::Input(e) => write!(f, "up post input: {e:?}"),
            Self::Update(e) => write!(f, "up post update: {e}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    /// Current-frame admission is host scheduling authority, not a fabricated
    /// native branch decision. Failed updates retain earlier state and queue.
    pub(crate) fn try_finish_board_ordinary_up_post<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        processed: &SkeletonProcessedContact,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerBoardUpPostError<N::Error>> {
        use PhysicalPlayerBoardUpPostError as E;
        let (contact, up, ground, physics) = self
            .try_with_board_force_runtime(board, |runtime| {
                (
                    Arc::clone(&runtime.contact_surface),
                    Arc::clone(&runtime.up_state),
                    Arc::clone(&runtime.ground_normal_attributes),
                    Arc::clone(&runtime.up_physics),
                )
            })
            .map_err(E::Owner)?;
        let frame = self
            .contact_spies
            .clone()
            .ok_or(E::Refused("no completed frame"))?;
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(|_| E::Refused("stale solved frame"))?;
        {
            let state = contact
                .lock()
                .map_err(|_| E::Refused("contact state poisoned"))?;
            if !state
                .body_post_completed
                .as_ref()
                .is_some_and(|old| Arc::ptr_eq(old, &frame))
            {
                return Err(E::Refused("Body post suffix not current"));
            }
            if state
                .up_post_attempted
                .as_ref()
                .is_some_and(|old| Arc::ptr_eq(old, &frame))
            {
                return Err(E::Refused("ordinary up post already attempted"));
            }
        }
        {
            let mut input =
                PhysicalPlayerBoardUpInput::try_new(self, board, skeleton, processed, &contact)
                    .map_err(E::Input)?;
            let mut state = up.lock().map_err(|_| E::Refused("up state poisoned"))?;
            let mut globals = ground
                .lock()
                .map_err(|_| E::Refused("global attributes poisoned"))?;
            let mut attributes = SkateboardUpdateUpVaultAttributes::new(&mut globals, &physics);
            contact
                .lock()
                .map_err(|_| E::Refused("contact state poisoned"))?
                .up_post_attempted = Some(Arc::clone(&frame));
            state
                .try_update(&mut input, &mut attributes, numeric)
                .map_err(E::Update)?;
        }
        self.try_with_board_force_runtime(board, |runtime| runtime.queue.clear_post_physics())
            .map_err(E::Owner)?;
        contact
            .lock()
            .map_err(|_| E::Refused("contact state poisoned"))?
            .up_post_completed = Some(frame);
        Ok(())
    }
}
