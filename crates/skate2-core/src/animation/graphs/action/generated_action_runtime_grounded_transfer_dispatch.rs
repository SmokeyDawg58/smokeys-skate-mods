//! Generated 828ed930 Update: grounded prefix and one packed transfer slot.
use super::{
    ActionUpdateError, ActorConstruction, ActorIdentity, BoundMotionGraphContext, Callback,
    GeneratedActionController,
};
use crate::generated_action_load::ids::state as state_id;

fn expected_slots(state: u16) -> Option<Vec<u16>> {
    // The shared native thunk always updates packed slot 31. The declaration
    // owning that slot changes with the selected state.
    let leaf = match state {
        state_id::ANTIC_46
        | state_id::POP_SHUVIT_ANTICIPATION_84
        | state_id::FS_POP_SHUVIT_ANTICIPATION_85
        | state_id::STATE_360_POP_SHUVIT_ANTICIPATION_86
        | state_id::FS_360_POP_SHUVIT_ANTICIPATION_87
        | state_id::N_POP_SHUVIT_ANTICIPATION_89
        | state_id::N_FS_POP_SHUVIT_ANTICIPATION_90
        | state_id::N_360_POP_SHUVIT_ANTICIPATION_91
        | state_id::N_FS_360_POP_SHUVIT_ANTICIPATION_92 => 119, // Antic or its PopShuvit descendants
        state_id::BRAKE_49 => 128,     // Brake
        state_id::HARD_TURN_93 => 161, // IdleTurning.HardTurn
        _ => return None,
    };
    Some((0..11).chain(42..50).chain(68..80).chain([leaf]).collect())
}

impl GeneratedActionController<ActorIdentity> {
    /// StateUpdate828ed930 calls OnGround828ed1f0, then the slot-31 transfer.
    pub(super) fn update_grounded_transfer(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), ActionUpdateError> {
        let address = self.states[usize::from(self.current_state)].update_address;
        if address != 0x828e_d930 {
            return Err(ActionUpdateError::PendingStateUpdate(address));
        }
        let Some(expected) = expected_slots(self.current_state) else {
            return Err(ActionUpdateError::PendingStateUpdate(address));
        };
        if self.current_behaviours != expected {
            return Err(ActionUpdateError::InvalidActiveSet);
        }
        for position in 0..32 {
            self.dispatch_callback(actor, context, position, Callback::Update)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{NULL_INDEX, data};
    use super::expected_slots;

    #[test]
    fn every_state_using_the_native_thunk_matches_its_declared_ancestry() {
        let sharing: Vec<u16> = data::STATES
            .iter()
            .enumerate()
            .filter(|(_, state)| state.update_address == 0x828e_d930)
            .map(|(index, _)| index as u16)
            .collect();
        assert_eq!(sharing, [46, 49, 84, 85, 86, 87, 89, 90, 91, 92, 93]);

        for state in sharing {
            let mut ancestry = Vec::new();
            let mut index = state;
            while index != NULL_INDEX {
                ancestry.push(index);
                index = data::STATES[usize::from(index)].parent;
            }
            let declared: Vec<u16> = ancestry
                .into_iter()
                .rev()
                .flat_map(|index| {
                    let section = data::STATES[usize::from(index)].behaviours;
                    section.index..section.index + section.count
                })
                .collect();
            assert_eq!(expected_slots(state).unwrap(), declared, "state {state}");
            assert_eq!(declared.len(), 32);
            assert_eq!(
                data::BEHAVIOURS[usize::from(declared[31])].name,
                "CreateMGIntentFromAGIntent"
            );
        }
    }
}
