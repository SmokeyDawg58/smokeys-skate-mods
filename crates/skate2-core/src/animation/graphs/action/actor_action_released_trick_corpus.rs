//! Controlled original loaded-graph boundary; proposed runtime-corpus child.
//! This does not claim codec/recognizer or full physics scheduling produced the
//! test's AG intents and filtered Ground state.
use super::*;
use crate::{
    actor_graph_construction::ActorGraphIntents, generated_action_load::GeneratedActionInstance,
    physical_player_phys_out::construct::conditioned_output::ConditionedStore,
    physics::FilteredPhysicsState,
};

#[expect(
    clippy::too_many_lines,
    reason = "one same-Actor released/held/exit lifecycle with fixture restoration"
)]
pub(crate) fn check(actor: &ActorConstruction) {
    let name = ActorGraphIntents::native_name;
    let (controller, prior_action, prior_motion) = {
        let state = actor.prefix().unwrap();
        let graphs = state.graphs.as_ref().unwrap();
        let action = graphs.action().unwrap();
        let snapshot = |map: &ActorGraphIntents| {
            map.iter()
                .map(|entry| (entry.name, entry.value))
                .collect::<Vec<_>>()
        };
        // Actual SkaterAnim ctor/Init owns this bit. Do not manufacture a value
        // merely because DontMirrorTrick will later select the ordinary key.
        let animation = state.animation.as_ref().unwrap().lock().unwrap();
        let player = animation.player().unwrap();
        assert_ne!(player.flags_known_mask & 0x4000_0000, 0);
        (
            action
                .controller
                .as_ref()
                .unwrap()
                .generated_action()
                .unwrap(),
            snapshot(action.intents.as_ref().unwrap()),
            snapshot(graphs.motion().unwrap().intents.as_ref().unwrap()),
        )
    };
    // Invoke after the existing neutral/Ollie/Nollie boundary probe. Never
    // replace the controller state to arrange the transition.
    assert_eq!(controller.lock().unwrap().current_state, 94);
    let physical = actor.cached_physical_allocation().unwrap();
    let prior_filtered = {
        let mut output = physical.owner().phys_out().lock().unwrap();
        let prior = output.try_filtered_output().unwrap().filtered_physics_state;
        output
            .try_store_conditioned(ConditionedStore::FilteredState(
                FilteredPhysicsState::Ground,
            ))
            .unwrap();
        prior
    };
    let set_action = |entries: &[(&[u8], f32)]| {
        let mut state = actor.prefix().unwrap();
        let map = state
            .graphs
            .as_mut()
            .unwrap()
            .action_mut()
            .unwrap()
            .intents
            .as_mut()
            .unwrap();
        map.clear();
        for &(key, value) in entries {
            map.insert_or_assign(name(key), value).unwrap();
        }
    };
    let motion_has = |key: &[u8]| {
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
            .get(&name(key))
            .is_some()
    };
    // Loaded condition107 is presence of Trick; condition108 asks the actual
    // Square mapping. Zero remains a present gesture, not an absent signal.
    set_action(&[
        (b"Trick", 0.0),
        (b"Kickflip", 0.0),
        (b"DontMirrorTrick", 0.0),
    ]);
    let dt = 1.0_f32 / 60.0;
    actor.try_update_action(dt).unwrap();
    let token = {
        let state = controller.lock().unwrap();
        assert_eq!(state.current_state, 52);
        assert_eq!(state.current_behaviours.last(), Some(&131));
        match state.instance_slots[26].instance.as_ref().unwrap() {
            GeneratedActionInstance::Gesture(instance) => instance.token().shallow_copy(),
            _ => panic!("actual Square gesture instance"),
        }
    };
    assert!(motion_has(b"Kickflip"));
    assert!(!motion_has(b"KickflipHold"));

    // Remove the original gesture; sustain must now use HoldPattern and the
    // retained state52 timer. First Update already cleared mFirstUpdate.
    set_action(&[(b"HoldPattern", 0.0)]);
    actor.try_update_action(dt).unwrap();
    {
        let state = controller.lock().unwrap();
        assert_eq!(state.current_state, 52);
        match state.instance_slots[26].instance.as_ref().unwrap() {
            GeneratedActionInstance::Gesture(instance) => {
                assert!(token.is_same_issuance(instance.token()));
            }
            _ => panic!("retained Square gesture instance"),
        }
    }
    assert!(!motion_has(b"Kickflip"));
    assert!(motion_has(b"KickflipHold"));

    set_action(&[]);
    actor.try_update_action(dt).unwrap();
    assert_eq!(controller.lock().unwrap().current_state, 94);
    assert!(!motion_has(b"Kickflip"));
    assert!(!motion_has(b"KickflipHold"));
    assert!(!controller.lock().unwrap().current_behaviours.contains(&131));

    // Restore caller-owned stimuli after verifying actual End cleanup. The
    // controller has naturally returned to94; its elapsed times are not rewound.
    {
        let mut state = actor.prefix().unwrap();
        let graphs = state.graphs.as_mut().unwrap();
        let action = graphs.action_mut().unwrap().intents.as_mut().unwrap();
        action.clear();
        for (key, value) in prior_action {
            action.insert_or_assign(key, value).unwrap();
        }
        let motion = graphs.motion_mut().unwrap().intents.as_mut().unwrap();
        let extra_keys = motion
            .iter()
            .filter(|entry| !prior_motion.iter().any(|(key, _)| key == &entry.name))
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        for key in extra_keys {
            motion.erase(&key);
        }
        for (key, value) in prior_motion {
            motion.insert_or_assign(key, value).unwrap();
        }
    }
    physical
        .owner()
        .phys_out()
        .lock()
        .unwrap()
        .try_store_conditioned(ConditionedStore::FilteredState(prior_filtered))
        .unwrap();
    eprintln!(
        "controlled original Action: neutral94 -> Trick52 Kickflip -> same-instance KickflipHold -> neutral94 with End cleanup; no full input/frame claim"
    );
}
