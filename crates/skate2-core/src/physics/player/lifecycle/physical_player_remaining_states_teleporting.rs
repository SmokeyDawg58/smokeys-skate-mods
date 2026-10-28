//! `PhysState_Teleporting::Update` at 0x82d86940. The callback result is
//! explicit so the Actor respawn query can run on the retained caller owner.
use super::Teleporting;
use crate::skeleton_processed_contact::SkeletonProcessedContact;
use crate::skeleton_root_transforms::SkeletonRootMatrix;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TeleportingUpdate {
    RequestActorRespawn,
    ResetAcquired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TeleportingUpdateError {
    UnknownExternalResetFlag,
    MissingExternalResetColumn(usize),
    MissingExternalResetOnBoard,
}

impl Teleporting {
    /// Native Update reads Processed+0x8c4 bit 2. Without it the Skeleton
    /// callback requests an Actor-chosen respawn; the current state is not
    /// marked ready. With it all four reset columns and on-board are copied.
    pub(crate) fn try_update_from_processed(
        &mut self,
        processed: &SkeletonProcessedContact,
    ) -> Result<TeleportingUpdate, TeleportingUpdateError> {
        let (flags, known) = processed.known_flags_8c4();
        if known & 4 == 0 {
            return Err(TeleportingUpdateError::UnknownExternalResetFlag);
        }
        if flags & 4 == 0 {
            return Ok(TeleportingUpdate::RequestActorRespawn);
        }
        let input = processed.animation_input();
        let columns = input.external_reset_transform;
        let matrix = SkeletonRootMatrix::from_columns([
            columns[0].ok_or(TeleportingUpdateError::MissingExternalResetColumn(0))?,
            columns[1].ok_or(TeleportingUpdateError::MissingExternalResetColumn(1))?,
            columns[2].ok_or(TeleportingUpdateError::MissingExternalResetColumn(2))?,
            columns[3].ok_or(TeleportingUpdateError::MissingExternalResetColumn(3))?,
        ]);
        let on_board = input
            .external_reset_on_board
            .ok_or(TeleportingUpdateError::MissingExternalResetOnBoard)?;
        self.data_acquired = Some(1);
        self.reset_transform = Some(matrix);
        self.teleport_to_on_board = Some(i32::from(on_board));
        self.teleport_ready = Some(1);
        Ok(TeleportingUpdate::ResetAcquired)
    }
}
