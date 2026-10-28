//! Shared straight/directional anticipation Update828edbc8.
use super::{
    ActionUpdateError, ActorConstruction, ActorIdentity, BoundMotionGraphContext, Callback,
    GeneratedActionController,
};
use crate::generated_action_load::ids::state as state_id;

fn expected_antic_slots(state: u16) -> Option<Vec<u16>> {
    let leaf = match state {
        state_id::OLLIE_ANTICIPATION_83 => 157..159,
        state_id::NOLLIE_ANTICIPATION_88 => 159..161,
        // Authored regular/switch leaves of the eight directional parents.
        // All sixteen descriptors point at the same native update function.
        state_id::REGULAR_164..=state_id::SWITCH_179 => {
            let start = 249 + (state - state_id::REGULAR_164) * 2;
            start..start + 2
        }
        _ => return None,
    };
    Some(
        (0..11)
            .chain(42..50)
            .chain(68..80)
            .chain([119])
            .chain(leaf)
            .collect(),
    )
}
impl GeneratedActionController<ActorIdentity> {
    /// Source executes the 31 grounded slots, Antic transfer, const intent, transfer.
    pub(super) fn update_ground_antic(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), ActionUpdateError> {
        let address = self.states[usize::from(self.current_state)].update_address;
        let Some(expected) = expected_antic_slots(self.current_state) else {
            return Err(ActionUpdateError::PendingStateUpdate(address));
        };
        if address != 0x828e_dbc8 {
            return Err(ActionUpdateError::PendingStateUpdate(address));
        }
        if self.current_behaviours != expected {
            return Err(ActionUpdateError::InvalidActiveSet);
        }
        for position in 0..34 {
            self.dispatch_callback(actor, context, position, Callback::Update)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_shared_anticipation_state_preserves_native_packed_order() {
        use super::super::super::{NULL_INDEX, data};
        let sharing: Vec<u16> = data::STATES
            .iter()
            .enumerate()
            .filter(|(_, state)| state.update_address == 0x828e_dbc8)
            .map(|(index, _)| index as u16)
            .collect();
        assert_eq!(
            sharing,
            [83, 88].into_iter().chain(164..180).collect::<Vec<_>>()
        );
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
            assert_eq!(
                expected_antic_slots(state).unwrap(),
                declared,
                "state {state}"
            );
            assert_eq!(declared.len(), 34);
            for (slot, name) in [
                (31, "CreateMGIntentFromAGIntent"),
                (32, "CreateConstMGIntent"),
                (33, "CreateMGIntentFromAGIntent"),
            ] {
                assert_eq!(data::BEHAVIOURS[usize::from(declared[slot])].name, name);
            }
        }
    }
    #[test]
    fn ollie_and_nollie_keep_distinct_leaf_occurrences_and_parent_order() {
        let ollie = expected_antic_slots(83).unwrap();
        let nollie = expected_antic_slots(88).unwrap();
        assert_eq!(
            &ollie[19..],
            &[
                68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 119, 157, 158
            ]
        );
        assert_eq!(&nollie[31..], &[119, 159, 160]);
        assert_eq!(&ollie[..31], &nollie[..31]);
        assert!(expected_antic_slots(84).is_none());
    }
}
