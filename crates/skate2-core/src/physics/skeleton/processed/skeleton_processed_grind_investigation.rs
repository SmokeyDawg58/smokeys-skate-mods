//! `ProcessedPhysIn::mGrindInvestigation` (+420): the manager's published
//! investigation, read by the state changer, the grind states and the fills.
use super::SkeletonProcessedContact;
use crate::physical_player_grind_manager::types::Investigation;

impl SkeletonProcessedContact {
    /// The constructed investigation; absent before the Processed constructor.
    pub(crate) fn grind_investigation(&self) -> Option<&Investigation> {
        self.grind_investigation.as_deref()
    }
    /// `GrindInvestigation::operator=` into +420 at the end of the manager
    /// Update; `mIsGrindPossible` is the same byte the physics fields expose.
    pub(crate) fn publish_grind_investigation(&mut self, investigation: &Investigation) {
        self.physics_state.grind_possible = Some(investigation.possible);
        match &mut self.grind_investigation {
            Some(slot) => slot.as_mut().clone_from(investigation),
            None => self.grind_investigation = Some(Box::new(investigation.clone())),
        }
    }
    /// `ProcessedPhysIn` 8cc bit writes of the grind code (jumper exits).
    pub(crate) fn set_flag_8cc(&mut self, mask: u32, value: bool) -> bool {
        let Some(word) = &mut self.physics_state.flags_8cc else {
            return false;
        };
        *word = (*word & !mask) | if value { mask } else { 0 };
        true
    }
}
