//! Controlled graph-boundary stimuli on the original mounted Actor. This tests
//! the loaded graph and live Vault callbacks; it does not claim that the still
//! incomplete full Update5/conditioner schedule produced these stimuli.
use super::*;
use crate::{actor_graph_construction::ActorGraphIntents, physics::FilteredPhysicsState};

#[expect(
    clippy::too_many_lines,
    reason = "one loaded Actor neutral/anticipation/held/released integration scenario"
)]
pub(crate) fn check(actor: &ActorConstruction) {
    use crate::physical_player_phys_out::construct::conditioned_output::ConditionedStore;
    let physical = actor.cached_physical_allocation().unwrap();
    let prior = {
        let mut output = physical.owner().phys_out().lock().unwrap();
        let prior = output.try_filtered_output().unwrap().filtered_physics_state;
        output
            .try_store_conditioned(ConditionedStore::FilteredState(
                FilteredPhysicsState::Ground,
            ))
            .unwrap();
        prior
    };
    let controller = {
        let mut state = actor.prefix().unwrap();
        let action = state.graphs.as_mut().unwrap().action_mut().unwrap();
        action.intents.as_mut().unwrap().clear();
        action
            .controller
            .as_ref()
            .unwrap()
            .generated_action()
            .unwrap()
    };
    let dt = 1.0 / 60.0;
    actor.try_update_action(dt).unwrap();
    assert_eq!(controller.lock().unwrap().current_state, 94);
    let name = ActorGraphIntents::native_name;
    for (angle, selected, intent) in [
        (0.0, 83, b"OllieAnticipation".as_slice()),
        (std::f32::consts::PI, 88, b"NollieAnticipation".as_slice()),
    ] {
        {
            let mut state = actor.prefix().unwrap();
            let intents = state
                .graphs
                .as_mut()
                .unwrap()
                .action_mut()
                .unwrap()
                .intents
                .as_mut()
                .unwrap();
            intents.insert_or_assign(name(b"AnticMag"), 1.0).unwrap();
            intents
                .insert_or_assign(name(b"AnticAngle"), angle)
                .unwrap();
        }
        actor.try_update_action(dt).unwrap();
        assert_eq!(controller.lock().unwrap().current_state, selected);
        let token = {
            let retained = controller.lock().unwrap();
            match retained.instance_slots[32].instance.as_ref().unwrap() {
                crate::generated_action_load::GeneratedActionInstance::ConstIntent(i) => {
                    i.token().shallow_copy()
                }
                _ => panic!("actual Ollie const occurrence"),
            }
        };
        actor.try_update_action(dt).unwrap();
        {
            let retained = controller.lock().unwrap();
            assert_eq!(retained.current_state, selected);
            match retained.instance_slots[32].instance.as_ref().unwrap() {
                crate::generated_action_load::GeneratedActionInstance::ConstIntent(i) => {
                    assert!(token.is_same_issuance(i.token()));
                }
                _ => panic!("retained Ollie const occurrence"),
            }
        }
        {
            let mut state = actor.prefix().unwrap();
            let graphs = state.graphs.as_mut().unwrap();
            assert_eq!(
                graphs
                    .motion()
                    .unwrap()
                    .intents
                    .as_ref()
                    .unwrap()
                    .get(&name(intent)),
                Some(0.0)
            );
            graphs
                .action_mut()
                .unwrap()
                .intents
                .as_mut()
                .unwrap()
                .clear();
        }
        actor.try_update_action(dt).unwrap();
        assert_eq!(controller.lock().unwrap().current_state, 94);
        assert!(
            actor
                .prefix()
                .unwrap()
                .graphs
                .as_ref()
                .unwrap()
                .motion()
                .unwrap()
                .intents
                .as_ref()
                .unwrap()
                .get(&name(intent))
                .is_none()
        );
    }
    physical
        .owner()
        .phys_out()
        .lock()
        .unwrap()
        .try_store_conditioned(ConditionedStore::FilteredState(prior))
        .unwrap();
    eprintln!(
        "controlled original-data Action boundary: neutral94 -> Ollie83/Nollie88 -> held (same instance, zero-valued intent present) -> released94; full frame/gesture producer not claimed"
    );
}
