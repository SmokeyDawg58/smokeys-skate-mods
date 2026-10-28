use super::*;

#[derive(Debug, PartialEq, Eq)]
enum RegistryError {
    Foreign,
    Duplicate,
    AfterAdd,
}
struct Registry {
    entity: MotionGraphEntityAuthority<u32>,
    entries: Vec<(ActorGraphInterface, Arc<()>)>,
    fail_after: Option<ActorGraphInterface>,
}
impl Registry {
    fn new(authority: &ActorGraphAuthority<u32>) -> Self {
        Self {
            entity: authority.entity_handle(),
            entries: Vec::new(),
            fail_after: None,
        }
    }
}
impl ActorGraphRegistry<u32> for Registry {
    type Error = RegistryError;
    fn register_graph_interface(
        &mut self,
        entity: &MotionGraphEntityAuthority<u32>,
        kind: ActorGraphInterface,
        component: &Arc<()>,
    ) -> Result<(), Self::Error> {
        if !self.entity.is_same_issuance(entity) {
            return Err(RegistryError::Foreign);
        }
        if self.entries.iter().any(|(existing, _)| *existing == kind) {
            return Err(RegistryError::Duplicate);
        }
        self.entries.push((kind, Arc::clone(component)));
        if self.fail_after == Some(kind) {
            return Err(RegistryError::AfterAdd);
        }
        Ok(())
    }
}

#[test]
fn constructors_retain_same_entity_distinct_wrappers_and_interface_occurrences() {
    let authority = ActorGraphAuthority::new(1);
    let mut registry = Registry::new(&authority);
    let mut graphs = ActorGraphComponents::new(&authority);
    graphs.try_construct_action(&mut registry).unwrap();
    graphs.try_construct_skater_motion(&mut registry).unwrap();
    assert!(graphs.action_finished() && graphs.motion_finished());
    let action = graphs.action().unwrap();
    let motion = graphs.motion().unwrap();
    assert!(action.is_complete() && motion.is_complete());
    assert_eq!(
        registry
            .entries
            .iter()
            .map(|(kind, _)| kind.source_name())
            .collect::<Vec<_>>(),
        [0x825e_be88, 0x825e_c090, 0x825e_c0a0]
    );
    assert!(Arc::ptr_eq(&registry.entries[0].1, action.occurrence()));
    assert!(Arc::ptr_eq(&registry.entries[1].1, motion.occurrence()));
    assert!(Arc::ptr_eq(&registry.entries[2].1, motion.occurrence()));
    assert!(!Arc::ptr_eq(action.occurrence(), motion.occurrence()));
    let a = action.controller.as_ref().unwrap();
    let m = motion.controller.as_ref().unwrap();
    assert!(
        !a.controller_identity()
            .is_same_issuance(m.controller_identity())
    );
    for wrapper in [a, m] {
        assert!(
            wrapper
                .context()
                .entity()
                .unwrap()
                .is_same_issuance(&authority.entity_handle())
        );
        assert!(
            wrapper
                .context()
                .controller()
                .unwrap()
                .is_same_issuance(wrapper.controller_identity())
        );
        assert!(wrapper.context().instance().is_none());
        assert!(wrapper.context().camera_man().is_none());
        assert!(wrapper.context().behaviour_source().is_none());
        assert!(wrapper.context().behaviour_sink().is_none());
        assert_eq!(wrapper.root_name_hash(), 0);
        assert!(wrapper.is_unloaded());
    }
    for intents in [
        action.intents.as_ref().unwrap(),
        motion.intents.as_ref().unwrap(),
        motion.filtered_intents.as_ref().unwrap(),
    ] {
        assert_eq!(intents.len(), 0);
        assert_eq!(intents.maximum(), 64);
    }
}

#[test]
fn derived_constructor_defaults_preserve_unknown_packed_bits_and_original_random_words() {
    let authority = ActorGraphAuthority::new(1);
    let mut registry = Registry::new(&authority);
    let mut graphs = ActorGraphComponents::new(&authority);
    graphs.try_construct_action(&mut registry).unwrap();
    graphs.try_construct_skater_motion(&mut registry).unwrap();
    let motion = graphs.motion().unwrap();
    let state = motion.state.as_ref().unwrap();
    assert!(state.attributes().is_empty());
    assert_eq!(state.attribute_maximum(), 20);
    assert_eq!(state.last_good_landing_velocity.to_bits(), 0);
    assert_eq!(state.grab_busy, [0; 2]);
    assert_eq!(state.push.map(f32::to_bits), [0; 8]);
    assert_eq!(state.push_bits, ActorGraphKnownBits::clear(0x8000_0000));
    assert_eq!(state.slide_time.map(f32::to_bits), [0; 2]);
    assert_eq!(
        state.slide_bits,
        [ActorGraphKnownBits::clear(0xe000_0000); 2]
    );
    assert_eq!(
        state.combined_slide_bits,
        ActorGraphKnownBits::clear(0xc000_0000)
    );
    assert_eq!(
        [
            state.time_since_kickturn,
            state.time_since_teleport,
            state.manual_out_timer
        ]
        .map(f32::to_bits),
        [0; 3]
    );
    assert_eq!(
        state.flags,
        ActorGraphKnownBits {
            known_mask: 0xff00_0000,
            bits: 0x0300_0000
        }
    );
    assert_eq!(state.trick_name, EMPTY_FAST_STRING_36);
    assert_eq!(state.trick_gesture, EMPTY_FAST_STRING_36);
    assert_eq!(state.trick_tweak.map(f32::to_bits), [0; 2]);
    assert_eq!(state.trick_bits, 0);
    assert_eq!(state.gesture, 0x23);
    assert!(!state.is_gesturing);
    assert_eq!(state.wipeout.map(f32::to_bits), [0; 2]);
    assert_eq!((state.random_index, state.random_seed), (0, 0));
    assert_eq!(
        state.random_values,
        [
            0xf22d_0e56,
            0x8831_26e9,
            0xc624_dd2f,
            0x0702_c49c,
            0x9e35_3f7d,
            0x6fdf_3b64
        ]
    );
    let transition = motion.transition.as_ref().unwrap();
    assert_eq!((transition.transition_type, transition.placement), (0, 0));
    assert_eq!(transition.blend_length.to_bits(), 0x3e4c_cccd);
    assert_eq!(transition.flags, ActorGraphKnownBits::clear(0xc000_0000));
}

#[test]
fn registration_failure_keeps_each_constructor_prefix_and_consumes_attempt() {
    for fail in [
        ActorGraphInterface::Action,
        ActorGraphInterface::Motion,
        ActorGraphInterface::SkaterMotion,
    ] {
        let authority = ActorGraphAuthority::new(1);
        let mut registry = Registry::new(&authority);
        registry.fail_after = Some(fail);
        let mut graphs = ActorGraphComponents::new(&authority);
        if fail == ActorGraphInterface::Action {
            assert!(matches!(
                graphs.try_construct_action(&mut registry),
                Err(ActorGraphConstructionError::Registry(
                    RegistryError::AfterAdd
                ))
            ));
            let action = graphs.action().unwrap();
            assert!(!action.is_complete());
            assert!(action.intents.is_none() && action.controller.is_none());
            assert!(Arc::ptr_eq(action.occurrence(), &registry.entries[0].1));
            assert!(!graphs.action_finished());
            assert!(matches!(
                graphs.try_construct_action(&mut registry),
                Err(ActorGraphConstructionError::AlreadyAttempted)
            ));
            assert!(matches!(
                graphs.try_construct_skater_motion(&mut registry),
                Err(ActorGraphConstructionError::ActionRequired)
            ));
            assert!(graphs.motion().is_none());
        } else {
            graphs.try_construct_action(&mut registry).unwrap();
            assert!(matches!(
                graphs.try_construct_skater_motion(&mut registry),
                Err(ActorGraphConstructionError::Registry(
                    RegistryError::AfterAdd
                ))
            ));
            let motion = graphs.motion().unwrap();
            assert!(!motion.is_complete());
            assert!(!graphs.motion_finished());
            assert_eq!(
                motion.intents.is_some(),
                fail == ActorGraphInterface::SkaterMotion
            );
            assert_eq!(
                motion.controller.is_some(),
                fail == ActorGraphInterface::SkaterMotion
            );
            assert_eq!(
                motion.transition.is_some(),
                fail == ActorGraphInterface::SkaterMotion
            );
            assert!(motion.filtered_intents.is_none() && motion.state.is_none());
            assert!(Arc::ptr_eq(motion.occurrence(), &registry.entries[1].1));
            assert!(matches!(
                graphs.try_construct_skater_motion(&mut registry),
                Err(ActorGraphConstructionError::AlreadyAttempted)
            ));
        }
    }
}

#[test]
fn equal_key_foreign_registry_rejects_before_add() {
    let authority = ActorGraphAuthority::new(7);
    let foreign = ActorGraphAuthority::new(7);
    let mut graphs = ActorGraphComponents::new(&authority);
    let mut registry = Registry::new(&foreign);
    assert!(matches!(
        graphs.try_construct_action(&mut registry),
        Err(ActorGraphConstructionError::Registry(
            RegistryError::Foreign
        ))
    ));
    assert!(registry.entries.is_empty());
    assert!(graphs.action().unwrap().intents.is_none());
}

#[test]
fn null_outer_allocations_finish_stages_without_registering_or_constructing() {
    let authority = ActorGraphAuthority::new(1);
    let mut registry = Registry::new(&authority);
    let mut graphs = ActorGraphComponents::new(&authority);
    assert!(!graphs.action_finished() && !graphs.motion_finished());
    graphs
        .try_construct_action_with_allocations(
            &mut registry,
            ActorGraphAllocation::Null,
            ActorGraphAllocation::Host,
        )
        .unwrap();
    assert!(graphs.action_finished() && graphs.action().is_none());
    graphs
        .try_construct_skater_motion_with_allocations(
            &mut registry,
            ActorGraphAllocation::Null,
            ActorGraphAllocation::Host,
        )
        .unwrap();
    assert!(graphs.motion_finished() && graphs.motion().is_none());
    assert!(registry.entries.is_empty());
    assert!(matches!(
        graphs.try_construct_action(&mut registry),
        Err(ActorGraphConstructionError::AlreadyAttempted)
    ));
    assert!(matches!(
        graphs.try_construct_skater_motion(&mut registry),
        Err(ActorGraphConstructionError::AlreadyAttempted)
    ));
}

#[test]
fn motion_can_construct_after_null_action_and_null_wrappers_still_complete() {
    for action_outer in [ActorGraphAllocation::Null, ActorGraphAllocation::Host] {
        let authority = ActorGraphAuthority::new(1);
        let mut registry = Registry::new(&authority);
        let mut graphs = ActorGraphComponents::new(&authority);
        graphs
            .try_construct_action_with_allocations(
                &mut registry,
                action_outer,
                ActorGraphAllocation::Null,
            )
            .unwrap();
        if let Some(action) = graphs.action() {
            assert!(action.is_complete() && action.controller.is_none());
            assert!(action.intents.is_some());
        }
        graphs
            .try_construct_skater_motion_with_allocations(
                &mut registry,
                ActorGraphAllocation::Host,
                ActorGraphAllocation::Null,
            )
            .unwrap();
        let motion = graphs.motion().unwrap();
        assert!(motion.is_complete() && motion.controller.is_none());
        assert!(motion.state.is_some() && motion.filtered_intents.is_some());
        assert!(graphs.action_finished() && graphs.motion_finished());
    }
}

#[test]
fn reset_preserves_nonreset_fields_and_known_low_bits_but_transition_discards_them() {
    let mut state = ActorSkaterGraphState::try_new::<()>().unwrap();
    state.time_since_teleport = 17.0;
    state.manual_out_timer = -4.0;
    state.gesture = 9;
    state.is_gesturing = true;
    state.random_seed = 99;
    state.trick_bits = 0x1234;
    state.push_bits = ActorGraphKnownBits {
        known_mask: u32::MAX,
        bits: u32::MAX,
    };
    state.flags = ActorGraphKnownBits {
        known_mask: u32::MAX,
        bits: 0xff12_3456,
    };
    state.reset();
    assert_eq!(state.time_since_teleport.to_bits(), 17.0_f32.to_bits());
    assert_eq!(state.manual_out_timer.to_bits(), (-4.0_f32).to_bits());
    assert_eq!(
        (
            state.gesture,
            state.is_gesturing,
            state.random_seed,
            state.trick_bits
        ),
        (9, true, 99, 0x1234)
    );
    assert_eq!(
        state.push_bits,
        ActorGraphKnownBits {
            known_mask: u32::MAX,
            bits: 0x7fff_ffff
        }
    );
    assert_eq!(
        state.flags,
        ActorGraphKnownBits {
            known_mask: u32::MAX,
            bits: 0x0312_3456
        }
    );
    let mut transition = ActorGraphTransition::new();
    transition.flags = ActorGraphKnownBits {
        known_mask: u32::MAX,
        bits: u32::MAX,
    };
    transition.clear_override();
    assert_eq!(transition.flags, ActorGraphKnownBits::clear(0xc000_0000));
}
