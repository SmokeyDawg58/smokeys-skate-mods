use super::*;
use crate::attribute_runtime::tests::graph;
use crate::input_cvar_runtime::InputCvarManager;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}
fn cvars() -> InputCvarReferences {
    let references = InputCvarReferences::new_static_initialized();
    InputCvarManager::new().try_link(&references).unwrap();
    references
}
#[test]
fn full_constructor_retains_same_output_and_changes_scoring_reset_flag() {
    let mut output = output();
    assert!(!output.concrete.scoring.as_ref().unwrap().has_been_reset);
    let mut owner = Conditioners::before_constructor(&output);
    let vault = Arc::new(graph(0, vec![]));
    let mut sites = Vec::new();
    owner
        .try_construct(&mut output, &vault, &cvars(), &mut |site| {
            sites.push(site);
            Ok::<_, ()>(true)
        })
        .unwrap();
    assert!(owner.is_complete());
    assert!(owner.matches_output(&output));
    assert_eq!(sites.len(), 29);
    assert_eq!(sites[0], AllocationSite::Intents);
    assert_eq!(sites.last(), Some(&AllocationSite::GameCam));
    assert!(output.concrete.scoring.as_ref().unwrap().has_been_reset);
    let score = owner
        .scoring
        .published_value()
        .unwrap()
        .payload
        .module
        .published_value()
        .unwrap();
    assert!(Arc::ptr_eq(&score.output, output.occurrence()));
    let score_module::CurrentCollector::Other(current) = score.current_collector.as_ref().unwrap() else { panic!("Reset must bind OtherCollector") };
    assert!(Arc::ptr_eq(current, score.other.published_value().unwrap()));
    let air = score.air.published_value().unwrap();
    assert_eq!(
        air.takeoff_deck_position,
        output
            .concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .deck_position
    );
    assert_eq!(
        air.player_spin_tracker.last_frame_matrix,
        Some(
            output
                .concrete
                .skeleton
                .as_ref()
                .unwrap()
                .skeleton_root_matrix
        )
    );
    assert_eq!(air.in_air_point_scalar.to_bits(), 1f32.to_bits());
    let ground = score.ground.published_value().unwrap();
    assert_eq!(ground.total_trick_score, None);
    assert_eq!(
        ground
            .manual_scorable
            .trick_scoring_settings
            .switch_variant_id,
        None
    );
    assert_eq!(
        ground.manual_scorable.trick_scoring_settings.trick_inputs[0].connector,
        None
    );
    assert_eq!(ground.manual_tracker.preserved_low_flags, None);
    assert_eq!(score.reset_suffix.as_ref().unwrap().collector_type, 4);
    assert_eq!(score.reset_suffix.as_ref().unwrap().current_gesture, 35);
    assert_eq!(
        owner
            .grinds
            .published_value()
            .unwrap()
            .payload
            .consecutive_non_grind_frames,
        31
    );
}
#[test]
fn unlinked_grind_cvar_retains_partial_allocation_without_pointer_publication() {
    let mut output = output();
    let mut owner = Conditioners::before_constructor(&output);
    let vault = Arc::new(graph(0, vec![]));
    let unlinked = InputCvarReferences::new_static_initialized();
    assert!(matches!(
        owner.try_construct(&mut output, &vault, &unlinked, &mut |_| Ok::<_, ()>(true)),
        Err(ConstructionError::Cvar(InputCvarError::Unlinked(
            "kMinApproachNonGrindFrames"
        )))
    ));
    assert!(owner.intents.published);
    assert!(owner.grinds.value.is_some());
    assert!(!owner.grinds.published);
    assert!(!owner.mission.published);
    assert!(!owner.is_complete());
    assert!(matches!(
        owner.try_construct(&mut output, &vault, &cvars(), &mut |_| Ok::<_, ()>(true)),
        Err(ConstructionError::AlreadyStarted)
    ));
}
#[test]
fn null_nested_sequence_faults_at_real_holder_reset_and_preserves_prefix() {
    let mut output = output();
    let mut owner = Conditioners::before_constructor(&output);
    let vault = Arc::new(graph(0, vec![]));
    let mut visited = Vec::new();
    let result = owner.try_construct(&mut output, &vault, &cvars(), &mut |site| {
        visited.push(site);
        Ok::<_, ()>(site != AllocationSite::Sequence)
    });
    assert!(matches!(
        result,
        Err(ConstructionError::Missing(AllocationSite::Sequence))
    ));
    assert_eq!(visited.last(), Some(&AllocationSite::Sequence));
    let scoring = &owner.scoring.value.as_ref().unwrap().payload;
    assert!(!owner.scoring.published);
    assert!(!scoring.module.published);
    let score = scoring.module.value.as_ref().unwrap();
    assert!(score.metrics.published);
    assert!(score.holder.value.is_some());
    assert!(!score.holder.published);
    assert!(!score.air.published);
    assert!(!output.concrete.scoring.as_ref().unwrap().has_been_reset);
}
#[test]
fn sequence_reset_preserves_stored_tricks_patterns_and_inactive_payloads() {
    let output = output();
    let mut sequence = sequence::Sequence::new_sim(output.occurrence().clone(), None);
    sequence.scorables[0].score = 123.0;
    sequence.active_scorables.elements[1].score = 456.0;
    sequence.active_scorables.num_entries = 2;
    sequence.pattern_matchers[0].is_active = true;
    sequence.pattern_matchers[0].length = 1;
    sequence.pattern_matchers[0].matches = [true; 3];
    sequence.flags_word = 0xf234_5678;
    sequence.reset();
    assert_eq!(sequence.scorables[0].score.to_bits(), 123f32.to_bits());
    assert_eq!(
        sequence.active_scorables.elements[1].score.to_bits(),
        456f32.to_bits()
    );
    assert_eq!(sequence.active_scorables.num_entries, 0);
    assert!(sequence.pattern_matchers[0].is_active);
    assert_eq!(sequence.pattern_matchers[0].length, 1);
    assert_eq!(sequence.pattern_matchers[0].matches, [false; 3]);
    assert_eq!(sequence.flags_word, 0x1234_5678);
}

#[test]
fn locked_audio_constructor_retains_zero_allocation_before_reset() {
    let mut output = output();
    output.lock_after_update1().unwrap();
    let mut owner = Conditioners::before_constructor(&output);
    let vault = Arc::new(graph(0, vec![]));
    let result = owner.try_construct(&mut output, &vault, &cvars(), &mut |_| Ok::<_, ()>(true));
    assert!(matches!(
        result,
        Err(ConstructionError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert!(owner.mission.published);
    assert!(!owner.audio.published);
    let audio = &owner.audio.value.as_ref().unwrap().payload;
    assert_eq!(audio.wheel_in_contact, [false; 4]);
    assert_eq!(audio.deck_transform.columns(), &[ZERO; 4]);
    assert_eq!(audio.grind_type, crate::physics::GrindType::FiftyFifty);
    assert!(!owner.camera.published);
}
