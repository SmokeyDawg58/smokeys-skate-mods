//! Exact TrickFromSquare/Tail/Nose Update828ed790, proposed runtime child.
use super::{
    ActionUpdateError, ActorConstruction, ActorIdentity, BoundMotionGraphContext, Callback,
    GeneratedActionController,
};
use crate::generated_action_load::ids::state as state_id;
fn expected_slots(state: u16) -> Option<Vec<u16>> {
    let leaf = match state {
        state_id::TRICK_FROM_SQUARE_52 => 131,
        state_id::TRICK_FROM_TAIL_53 => 132,
        state_id::TRICK_FROM_NOSE_54 => 133,
        _ => return None,
    };
    Some((0..11).chain(42..50).chain(80..87).chain([leaf]).collect())
}
impl GeneratedActionController<ActorIdentity> {
    pub(super) fn update_selected_trick(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), ActionUpdateError> {
        let address = self.states[usize::from(self.current_state)].update_address;
        let Some(expected) = expected_slots(self.current_state) else {
            return Err(ActionUpdateError::PendingStateUpdate(address));
        };
        if address != 0x828e_d790 {
            return Err(ActionUpdateError::PendingStateUpdate(address));
        }
        if self.current_behaviours != expected {
            return Err(ActionUpdateError::InvalidActiveSet);
        }
        for position in 0..27 {
            self.dispatch_callback(actor, context, position, Callback::Update)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trick_dispatch_replaces_ground_slots_with_actual_trick_parent() {
        for (state, leaf) in [(52, 131), (53, 132), (54, 133)] {
            let slots = expected_slots(state).unwrap();
            assert_eq!(&slots[19..], &[80, 81, 82, 83, 84, 85, 86, leaf]);
            assert_eq!(slots.len(), 27);
            assert!(!slots.contains(&68));
        }
        assert!(expected_slots(83).is_none());
    }
}
