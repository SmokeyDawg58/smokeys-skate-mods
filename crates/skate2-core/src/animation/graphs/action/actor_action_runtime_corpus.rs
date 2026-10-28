//! Original mounted Actor executes repeated generated Action updates after its
//! actual physical constructor returns. The selected schedule remains a probe.
use super::*;
#[path = "../../hands_face/actor_animation_hands_face_corpus.rs"]
mod hands_face;

/// Fixture camera manager whose camera skater is another Actor.
struct NotCameraSkater;
impl crate::actor_construction::animation_phase1::SkaterOpacityCamera for NotCameraSkater {
    fn camera_skater_opacity(&self, _: &ActorConstruction) -> Result<Option<f32>, String> {
        Ok(None)
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one ordered original Actor graph, animation and physical input integration fixture"
)]
pub(crate) fn check_published(
    actor: &ActorConstruction,
    cvars: &crate::input_cvar_runtime::InputCvarReferences,
    body_services: &mut crate::actor_construction::body_session::ActorBodyAnimationServices,
    input_services: &mut crate::actor_construction::input_generation::runtime::ActorInputRuntimeServices,
    motion_writer: &mut dyn super::super::motion_runtime_session::neutral_lifecycle::ForcePhysicsWriter,
) {
    actor.try_construct_physics_auxiliary().unwrap();
    let settings = crate::players_backend_settings::PlayersBackendSettings::construct_fields();
    let controller = {
        let prefix = actor.prefix().unwrap();
        prefix
            .graphs
            .as_ref()
            .unwrap()
            .action()
            .unwrap()
            .controller
            .as_ref()
            .unwrap()
            .generated_action()
            .unwrap()
    };
    let dt = 1.0_f32 / 60.0;
    let mut hand_tokens = crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool::new();
    actor
        .try_generate_runtime_action_intents(
            dt,
            input_services,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    actor
        .try_update_animation_phase1_prefix(
            dt,
            cvars,
            body_services,
            motion_writer,
            &NotCameraSkater,
        )
        .unwrap();
    actor.try_update_action(dt).unwrap();
    let tokens = |state: &crate::generated_action_load::GeneratedActionController<
        super::super::ActorIdentity,
    >| {
        state
            .instance_slots
            .iter()
            .map(|slot| match slot.instance.as_ref() {
                Some(crate::generated_action_load::GeneratedActionInstance::TransferIntent(
                    value,
                )) => Some(value.token().shallow_copy()),
                Some(crate::generated_action_load::GeneratedActionInstance::JuiceHook(value)) => {
                    Some(value.token().shallow_copy())
                }
                Some(crate::generated_action_load::GeneratedActionInstance::ConstIntent(value)) => {
                    Some(value.token().shallow_copy())
                }
                Some(crate::generated_action_load::GeneratedActionInstance::Gesture(value)) => {
                    Some(value.token().shallow_copy())
                }
                Some(crate::generated_action_load::GeneratedActionInstance::BodyFlip(value)) => {
                    Some(value.token().shallow_copy())
                }
                Some(crate::generated_action_load::GeneratedActionInstance::TimeIntent(value)) => {
                    Some(value.token().shallow_copy())
                }
                Some(crate::generated_action_load::GeneratedActionInstance::BoardAdjust(value)) => {
                    Some(value.token().shallow_copy())
                }
                None => None,
            })
            .collect::<Vec<_>>()
    };
    let (selected, count, initial_tokens) = {
        let state = controller.lock().unwrap();
        assert_ne!(state.current_state, u16::MAX);
        (
            state.current_state,
            state.current_behaviours.len(),
            tokens(&state),
        )
    };
    actor.try_clear_motion_attributes().unwrap();
    actor.try_update_motion(dt, motion_writer).unwrap();
    actor.try_update_animation_phase2(true, cvars).unwrap();
    actor
        .try_update_animation_phase3(dt, body_services, motion_writer)
        .unwrap();
    actor.try_sync_body_frame(body_services).unwrap();
    let first_hand_generations = hands_face::check(actor, body_services, &mut hand_tokens);
    actor
        .try_generate_runtime_action_intents(
            dt,
            input_services,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    actor
        .try_update_animation_phase1_prefix(
            dt,
            cvars,
            body_services,
            motion_writer,
            &NotCameraSkater,
        )
        .unwrap();
    actor.try_update_action(dt).unwrap();
    let state = controller.lock().unwrap();
    assert_eq!(state.current_state, selected);
    assert_eq!(state.last_state, selected);
    assert_eq!(state.current_behaviours.len(), count);
    for (before, after) in initial_tokens.iter().zip(tokens(&state)) {
        assert!(match (before, after) {
            (Some(before), Some(after)) => before.is_same_issuance(&after),
            (None, None) => true,
            _ => false,
        });
    }
    assert!(!state.allow_evaluations);
    drop(state);
    let motion = {
        let prefix = actor.prefix().unwrap();
        prefix
            .graphs
            .as_ref()
            .unwrap()
            .motion()
            .unwrap()
            .controller
            .as_ref()
            .unwrap()
            .generated_motion()
            .unwrap()
    };
    let motion_tokens =
        |state: &crate::generated_motion_load::controller::GeneratedMotionController<
            super::super::ActorIdentity,
        >| {
            state
                .instances
                .iter()
                .map(|slot| {
                    slot.instance
                        .as_ref()
                        .map(|instance| instance.token().shallow_copy())
                })
                .collect::<Vec<_>>()
        };
    let initial_motion_tokens = {
        let state = motion.lock().unwrap();
        assert_eq!(state.current_state, 6);
        assert_eq!(state.last_state, u16::MAX);
        assert_eq!(state.current_behaviours, [0, 1, 2, 3, 39, 40]);
        assert_eq!(
            state
                .time_in_state
                .iter()
                .map(|(&index, &time)| (index, time))
                .collect::<Vec<_>>(),
            [(0, 0.0), (6, 0.0)]
        );
        motion_tokens(&state)
    };
    actor.try_clear_motion_attributes().unwrap();
    actor.try_update_motion(dt, motion_writer).unwrap();
    actor.try_update_animation_phase2(true, cvars).unwrap();
    actor
        .try_update_animation_phase3(dt, body_services, motion_writer)
        .unwrap();
    actor.try_sync_body_frame(body_services).unwrap();
    let second_hand_generations = hands_face::check(actor, body_services, &mut hand_tokens);
    for (first, second) in first_hand_generations
        .into_iter()
        .zip(second_hand_generations)
    {
        assert_eq!(second, first + 1);
    }
    {
        let state = motion.lock().unwrap();
        assert_eq!(state.current_state, 6);
        assert_eq!(state.last_state, 6);
        assert_eq!(state.current_behaviours, [0, 1, 2, 3, 39, 40]);
        assert_eq!(
            state
                .time_in_state
                .iter()
                .map(|(&index, &time)| (index, time))
                .collect::<Vec<_>>(),
            [(0, dt), (6, dt)]
        );
        assert!(!state.allow_evaluations);
        for (before, after) in initial_motion_tokens.iter().zip(motion_tokens(&state)) {
            assert!(match (before, after) {
                (Some(before), Some(after)) => before.is_same_issuance(&after),
                (None, None) => true,
                _ => false,
            });
        }
    }
    eprintln!(
        "SAME published Actor executes two Motion updates in Default6, retains six behaviours and instance issuances, advances state timers and executes root/default callbacks"
    );
    actor
        .try_set_up_physics(
            dt,
            cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            &settings,
        )
        .unwrap();
    let before_flags = actor.prefix().unwrap().input_generation_flags.unwrap();
    let input = actor.published_animation_physics_input().unwrap();
    let published_reset = input.lock().unwrap().external_reset_active;
    actor.try_update_after_physics_setup(dt).unwrap();
    assert_eq!(
        actor.prefix().unwrap().input_generation_flags.unwrap(),
        before_flags & 0x97ff_ffff
    );
    assert_eq!(input.lock().unwrap().external_reset_active, published_reset);
    // Retained constructor defaults for this selected marquee fixture. Custom
    // CAC settings loading and the whole backend startup remain separate.
    assert!(std::sync::Arc::ptr_eq(
        actor
            .cached_physical_allocation()
            .unwrap()
            .owner()
            .animation_physics_input
            .lock()
            .unwrap()
            .as_ref()
            .unwrap(),
        &actor.published_animation_physics_input().unwrap(),
    ));
    actor.try_process_bound_animation_physics_input().unwrap();
    {
        let physical = actor.cached_physical_allocation().unwrap();
        let processed =
            std::sync::Arc::clone(physical.owner().reckoning().lock().unwrap().processed());
        let processed = processed.lock().unwrap();
        assert_eq!(processed.physics_state_fields().time_delta, Some(dt));
        assert_eq!(processed.animation_input().compression, Some(0.5));
        assert_eq!(
            processed
                .animation_input()
                .truck_tightness
                .unwrap()
                .to_bits(),
            0x3e99_999a
        );
        assert_eq!(
            processed
                .animation_input()
                .wheel_hardness
                .unwrap()
                .to_bits(),
            0x3f33_3333
        );
    }
    // Initial constructed Motion/tree owners contain no animation attributes.
    // The real transport must not fabricate a bail record for this frame.
    assert!(matches!(
        actor.try_process_selected_animation_wipeout_attribute(0),
        Err(crate::skeleton_processed_contact::jump_events::animation_wipeout::AnimationWipeoutError::MissingAttribute(0))
    ));
    eprintln!(
        "SAME published Actor generated Action selects {selected} and executes two updates, retaining {count} behaviour occurrences; controlled schedule, no full Actor frame"
    );
}
