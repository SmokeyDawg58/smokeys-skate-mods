//! Draft `InputCvarReferences` child: same processed pointer and live COM cvar.
use super::{InputCvarError, InputCvarReferences};
use crate::{
    skeleton_physics_history::SkeletonHistoryInput,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex};

pub(crate) struct PostPhysicsHistoryInput<'a> {
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
}
#[derive(Debug)]
pub(crate) enum HistoryInputError {
    Cvar(InputCvarError),
    Poisoned,
    MissingUp,
    UnknownUpW,
}
impl std::fmt::Display for HistoryInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(e) => write!(f, "post-physics history COM cvar: {e:?}"),
            Self::Poisoned => f.write_str("post-physics history processed owner poisoned"),
            Self::MissingUp => f.write_str("post-physics history processed Up is unwritten"),
            Self::UnknownUpW => f.write_str(
                "projected COM full-vector Up W is not known by current history carrier",
            ),
        }
    }
}
impl SkeletonHistoryInput for PostPhysicsHistoryInput<'_> {
    type Error = HistoryInputError;
    type Up = Arc<Mutex<SkeletonProcessedContact>>;
    fn acquire_up_vector(&mut self) -> Result<Self::Up, Self::Error> {
        // Native only captures Processed+220 address; payload read is later.
        Ok(Arc::clone(self.processed))
    }
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.references
            .try_current_boolean(66)
            .map(u8::from)
            .map_err(HistoryInputError::Cvar)
    }
    fn read_up_vector(&mut self, captured: &Self::Up) -> Result<[f32; 4], Self::Error> {
        captured
            .lock()
            .map_err(|_| HistoryInputError::Poisoned)?
            .prior_output()
            .reckoning_up()
            .ok_or(HistoryInputError::MissingUp)?
            .value
            .known_lanes()
            .ok_or(HistoryInputError::UnknownUpW)
    }
    fn read_up_vector_typed(
        &mut self,
        captured: &Self::Up,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        captured
            .lock()
            .map_err(|_| HistoryInputError::Poisoned)?
            .prior_output()
            .reckoning_up()
            .ok_or(HistoryInputError::MissingUp)
            .map(|up| up.value)
    }
}
