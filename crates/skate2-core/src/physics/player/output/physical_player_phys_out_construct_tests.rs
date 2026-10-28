use super::construct::{ComponentStatus, ConcretePhysOutError, PhysOutAllocation};
use super::*;

fn constructed() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one source-ordered nineteen-component constructor discriminator"
)]
fn complete_constructor_visits_all_nineteen_and_retains_nonzero_native_defaults() {
    let mut output = PhysicalPlayerPhysOut::new();
    let mut order = Vec::new();
    output
        .try_construct_complete(|component| {
            order.push(component);
            Ok::<_, ()>(PhysOutAllocation::Host)
        })
        .unwrap();
    assert_eq!(order, COMPONENTS);
    assert!(output.complete_construction());
    assert_eq!(
        output.component_statuses(),
        &[ComponentStatus::Constructed; 19]
    );
    assert_eq!(output.locked_for_write(), Some(false));
    assert_eq!(
        output
            .try_animation_output()
            .unwrap()
            .time_since_last_input
            .to_bits(),
        0
    );
    assert_eq!(
        output.try_filtered_output().unwrap().filtered_physics_state,
        crate::physics::FilteredPhysicsState::Invalid
    );
    assert!(
        !output
            .try_skeleton_output()
            .unwrap()
            .tell_anim_dont_push_or_brake
    );
    assert_eq!(output.try_state_output().unwrap().physics_state, 0);
    assert_eq!(
        output
            .concrete
            .filtered
            .as_ref()
            .unwrap()
            .filtered_physics_state,
        crate::physics::FilteredPhysicsState::Invalid
    );
    assert_eq!(
        output
            .concrete
            .animation
            .as_ref()
            .unwrap()
            .time_since_last_input
            .to_bits(),
        0
    );
    assert_eq!(
        output
            .concrete
            .animation
            .as_ref()
            .unwrap()
            .wipeout_estimated_time_to_land
            .to_bits(),
        f32::MAX.to_bits()
    );
    assert!(
        output
            .concrete
            .offboard
            .as_ref()
            .unwrap()
            .is_locomotion_viable
    );
    assert!(
        !output
            .concrete
            .offboard
            .as_ref()
            .unwrap()
            .is_starting_standing_jump
    );
    assert_eq!(
        output
            .concrete
            .audio
            .as_ref()
            .unwrap()
            .off_board_jump_landing,
        1
    );
    assert_eq!(
        output.concrete.grinds.as_ref().unwrap().anim_grind_name,
        crate::play_animation::ANDALE_NULL_STRING_NAME_30
    );
    assert_eq!(
        output.concrete.grinds.as_ref().unwrap().skating_grind_name,
        crate::play_animation::EMPTY_FAST_STRING_30
    );
    let scoring = output.concrete.scoring.as_ref().unwrap();
    let settings = &scoring.tricks_begun.elements[0].trick_scoring_settings;
    assert_eq!(settings.difficulty, 3);
    assert_eq!(settings.fakie_variant_id, -1);
    assert_eq!(settings.switch_variant_id, Some(0));
    assert_eq!(
        settings.skater_intent_tuples[0].value.to_bits(),
        1f32.to_bits()
    );
    assert_eq!(settings.skater_intent_tuples[0].end_frame, -1);
    assert_eq!(scoring.current_grind_info.spline_pathed_guid, u64::MAX);
    assert_eq!(
        output
            .concrete
            .path
            .as_ref()
            .unwrap()
            .new_node
            .board_orientation,
        [127; 4]
    );
}

#[test]
fn failure_before_scoring_allocation_retains_earlier_constructor_fields_without_filtered_admission()
{
    let mut output = PhysicalPlayerPhysOut::new();
    let result = output.try_construct_complete(|component| {
        if component == PhysOutComponent::Scoring2 {
            Err(37)
        } else {
            Ok(PhysOutAllocation::Host)
        }
    });
    assert_eq!(result, Err(ConcretePhysOutError::Allocation(37)));
    assert!(!output.complete_construction());
    assert_eq!(output.locked_for_write(), None);
    assert!(output.concrete.animation.is_some());
    assert!(matches!(
        output.try_animation_output(),
        Err(ConcretePhysOutError::NotConstructed)
    ));
    assert!(output.concrete.scoring.is_none());
    assert!(output.concrete.filtered.is_none());
    assert_eq!(
        output.component_statuses()[15],
        ComponentStatus::Unattempted
    );
    assert_eq!(
        output.try_reset_complete(),
        Err(ConcretePhysOutError::NotConstructed)
    );
    assert_eq!(
        output.try_construct_complete(|_| Ok::<_, i32>(PhysOutAllocation::Host)),
        Err(ConcretePhysOutError::AlreadyStarted)
    );
}

#[test]
fn reset_preserves_inactive_scoring_payloads_context_uids_and_source_history_fields() {
    let mut output = constructed();
    output.lock_after_update1().unwrap();
    assert!(output.try_filtered_output().is_ok());
    output
        .concrete
        .air
        .as_mut()
        .unwrap()
        .anim_hand_plant_info_struct
        .flags = u32::MAX;
    output
        .concrete
        .audio
        .as_mut()
        .unwrap()
        .wheel_landing
        .impact_strength[1] = 9.0;
    output.concrete.path.as_mut().unwrap().new_node.extra = [1, 2, 3];
    let scoring = output.concrete.scoring.as_mut().unwrap();
    scoring.tricks_begun.num_entries = 1;
    scoring.tricks_begun.elements[0].point_scalar = 7.0;
    let context = &mut scoring.current_context_data.context_sensitive_data[0].uid_data[0];
    context.count = 1;
    context.uids[0].unique_id = 77;
    scoring
        .current_air_data
        .initial_trick
        .trick_scoring_settings
        .base_score = 42;
    scoring
        .current_air_data
        .initial_trick
        .trick_scoring_settings
        .is_valid = true;
    output.try_reset_complete().unwrap();
    assert_eq!(output.locked_for_write(), Some(false));
    assert_eq!(
        output
            .concrete
            .air
            .as_ref()
            .unwrap()
            .anim_hand_plant_info_struct
            .flags,
        0x4fff_ffff
    );
    assert_eq!(
        output
            .concrete
            .audio
            .as_ref()
            .unwrap()
            .wheel_landing
            .impact_strength[1]
            .to_bits(),
        9f32.to_bits()
    );
    assert_eq!(
        output.concrete.path.as_ref().unwrap().new_node.extra,
        [1, 2, 3]
    );
    let scoring = output.concrete.scoring.as_ref().unwrap();
    assert_eq!(scoring.tricks_begun.num_entries, 0);
    assert_eq!(
        scoring.tricks_begun.elements[0].point_scalar.to_bits(),
        7f32.to_bits()
    );
    let context = &scoring.current_context_data.context_sensitive_data[0].uid_data[0];
    assert_eq!(context.count, 0);
    assert_eq!(context.uids[0].unique_id, 77);
    assert_eq!(
        scoring
            .current_air_data
            .initial_trick
            .trick_scoring_settings
            .base_score,
        42
    );
    assert!(
        !scoring
            .current_air_data
            .initial_trick
            .trick_scoring_settings
            .is_valid
    );
}

#[test]
fn null_child_is_retained_and_reset_stops_at_its_actual_position() {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|component| {
            Ok::<_, ()>(if component == PhysOutComponent::Scoring2 {
                PhysOutAllocation::Null
            } else {
                PhysOutAllocation::Host
            })
        })
        .unwrap();
    assert!(output.complete_construction());
    assert_eq!(
        output.component_statuses()[15],
        ComponentStatus::AllocationNull
    );
    assert!(output.concrete.filtered.is_some());
    output.lock_after_update1().unwrap();
    output
        .concrete
        .animation
        .as_mut()
        .unwrap()
        .time_since_last_input = 13.0;
    output
        .concrete
        .filtered
        .as_mut()
        .unwrap()
        .filtered_physics_state = crate::physics::FilteredPhysicsState::Air;
    assert_eq!(
        output.try_reset_complete(),
        Err(ConcretePhysOutError::MissingComponent(
            PhysOutComponent::Scoring2
        ))
    );
    assert_eq!(
        output
            .concrete
            .animation
            .as_ref()
            .unwrap()
            .time_since_last_input
            .to_bits(),
        0
    );
    assert_eq!(
        output
            .concrete
            .filtered
            .as_ref()
            .unwrap()
            .filtered_physics_state,
        crate::physics::FilteredPhysicsState::Air
    );
    assert_eq!(output.locked_for_write(), Some(true));
}
