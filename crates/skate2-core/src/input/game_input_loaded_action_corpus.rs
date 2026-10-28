//! Child of `game_input_recognizer_corpus`. Retains the actual managers
//! created there through the later mounted-Actor graph probe.
use super::*;

pub(crate) type LoadedActionInputSession = crate::game_input_frame_session::GameInputFrameSession<
    crate::actor_construction::ActorIdentity,
>;

pub(crate) struct LoadedInputObservation {
    matched: Vec<Arc<crate::input_pattern::InputPattern>>,
    held: bool,
    action: Vec<crate::play_animation::FastString36>,
    selected: u16,
}

impl LoadedActionInputSession {
    /// Selected schedule: original sparse codec -> per-player recognition and
    /// event generation -> real shared Flush -> same Actor Fill -> loaded Action.
    /// No direct map insertion, substitute receiver, or diagnostic physics.
    #[expect(
        clippy::too_many_arguments,
        reason = "existing retained application collaborators"
    )]
    pub(crate) fn step_loaded(
        &mut self,
        actor: &ActorConstruction,
        point: [f32; 2],
        dt: f32,
        inputs: &mut crate::actor_construction::input_generation::runtime::ActorInputRuntimeServices,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        vault: &Arc<AttributeRuntimeVault>,
        overlay: &crate::debug_overlay::DebugOverlaySlot,
    ) -> LoadedInputObservation {
        let listener = actor.retained_listener().unwrap().unwrap();
        assert!(Arc::ptr_eq(
            listener.lock().unwrap().player(),
            self.gameplay.player(0).unwrap()
        ));
        let mut runtime =
            crate::game_input_recognizer_runtime_services::GameInputRuntimeServices::new(
                cvars, vault, overlay, None,
            );
        // Receiver10 is the actual Reset-created player0 command endpoint,
        // already used and checked by the parent corpus's retained session.
        let sample = super::sample_from_point(point);
        let delivery = self
            .try_step(
                crate::game_input_frame_session::GameInputFrameRequest {
                    input: &sample,
                    receiver_id: 10,
                    update_mask: 1,
                    add_to_crc: true,
                },
                &mut HostReckoningNumeric,
                &mut ControlledCommandSync,
                &mut runtime,
                overlay,
            )
            .unwrap();
        assert_eq!(
            delivery.dispatch,
            crate::game_input_manager::GameInputDispatch::Delivered
        );
        assert_eq!(self.gameplay.route_mut().pending(), 0);
        eprintln!(
            "retained input frame flushed {} events",
            delivery.flushed_events
        );
        // Flush has delivered to the real registered Actor. Observe before
        // successful Fill consumes the event list and its held latch.
        let (matched, held) = {
            let listener = actor.retained_listener().unwrap().unwrap();
            let listener = listener.lock().unwrap();
            (
                listener
                    .events()
                    .iter()
                    .filter_map(|event| event.pattern())
                    .map(|event| Arc::clone(&event.pattern))
                    .collect(),
                listener.held_pending(),
            )
        };
        actor
            .try_generate_runtime_action_intents(dt, inputs, &mut HostReckoningNumeric)
            .unwrap();
        let (action, controller) = {
            let state = actor.prefix().unwrap();
            let graph = state.graphs().unwrap().action().unwrap();
            (
                graph
                    .intents
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|entry| entry.name)
                    .collect(),
                graph
                    .controller
                    .as_ref()
                    .unwrap()
                    .generated_action()
                    .unwrap(),
            )
        };
        actor.try_update_action(dt).unwrap();
        let selected = controller.lock().unwrap().current_state;
        LoadedInputObservation {
            matched,
            held,
            action,
            selected,
        }
    }

    /// Feed the actual authored point order through the retained codec/history.
    /// No graph selection or physics output is changed to arrange recognition.
    pub(crate) fn check_original_kickflip(
        &mut self,
        actor: &ActorConstruction,
        inputs: &mut crate::actor_construction::input_generation::runtime::ActorInputRuntimeServices,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        vault: &Arc<AttributeRuntimeVault>,
        overlay: &crate::debug_overlay::DebugOverlaySlot,
    ) -> [u16; 3] {
        let pattern = self.original_right_pattern("Kickflip");
        let chronological = pattern.points().iter().rev().copied().collect::<Vec<_>>();
        assert!(chronological.len() >= 2);
        let waiting = vault
            .instance(0x7f08_6875_d621_c88c, InputPatternStick::Right.collection())
            .try_cached_layout(0x30)
            .unwrap()
            .expect("actual right recognizer layout")
            .bytes()[0x21];
        let dt = 1.0_f32 / 60.0;
        // Actual zero input expires old candidates by their source wait bound;
        // the extra two frames also pass matched-last-update's one-frame skip.
        // This is input replay, not an internal matcher reset or time overwrite.
        let mut states = Vec::new();
        for _ in 0..usize::from(waiting) + 2 {
            states.push(
                self.step_loaded(actor, [0.0, 0.0], dt, inputs, cvars, vault, overlay)
                    .selected,
            );
        }
        let mut observed = Vec::new();
        for point in chronological.iter().copied() {
            let result = self.step_loaded(actor, point, dt, inputs, cvars, vault, overlay);
            states.push(result.selected);
            observed.push(result);
        }
        let name = ActorGraphIntents::native_name;
        let matched = observed
            .iter()
            .find(|sample| {
                sample
                    .matched
                    .iter()
                    .any(|actual| Arc::ptr_eq(actual, &pattern))
            })
            .expect("authored codec-fed Kickflip must reach the SAME Actor listener");
        assert!(matched.action.contains(&name(b"Trick")));
        assert!(matched.action.contains(&name(b"Kickflip")));
        // The next actual endpoint sample requests Held from the same matcher;
        // listener accepts flips and Fill emits/consumes its real HoldPattern.
        let endpoint = *chronological.last().unwrap();
        let held = self.step_loaded(actor, endpoint, dt, inputs, cvars, vault, overlay);
        states.push(held.selected);
        assert!(
            held.held,
            "actual matcher Held must arm this Actor's flip latch"
        );
        assert!(held.action.contains(&name(b"HoldPattern")));
        let released = self.step_loaded(actor, [0.0, 0.0], dt, inputs, cvars, vault, overlay);
        states.push(released.selected);
        assert!(!released.held);
        assert!(!released.action.contains(&name(b"HoldPattern")));
        eprintln!(
            "actual retained input -> loaded Action: original Kickflip {} points, matched/held/release AG outputs; selected states {states:?}; no forced filtered state or complete Motion/frame claim",
            chronological.len()
        );
        [matched.selected, held.selected, released.selected]
    }
}
