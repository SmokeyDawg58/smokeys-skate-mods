//! Shared numerical-body tests with the actual retained processed reader.
use super::{Fixture, SkeletonCurrentAnimationError};
use crate::{
    skeleton_current_animation::continuation::{AnimationInputError, ControlledGrind},
    skeleton_processed_contact::SkeletonProcessedContact,
};

#[test]
fn missing_offboard_and_landing_fields_do_not_block_inactive_consumers() {
    let mut fixture = Fixture::new();
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    // No current physical toolkit is published. The inactive adjustment paths
    // never consume deck/landing inputs, while COM and up come from Reset.
    let mut grind = ControlledGrind {
        activated: false,
        adjusting: true,
    };
    processed
        .with_process_data_read(|input, delta| {
            fixture.current.try_process_read(
                &mut fixture.targets,
                &fixture.pose,
                input,
                delta,
                &fixture.masses,
                &fixture.root,
                &super::TEST_ANIMATION_TO_WORLD,
                &fixture.world_to_animation,
                fixture.retained_com_y,
                &mut fixture.attributes,
                &mut fixture.cvars,
                &mut fixture.numeric,
                &mut grind,
            )
        })
        .unwrap();
    assert!(!grind.adjusting);
    assert!(processed.process_data_output().deck_zy_delta.is_some());
}

#[test]
fn missing_input_fails_after_mapping_and_preserves_old_delta() {
    let mut fixture = Fixture::new();
    let mut processed = SkeletonProcessedContact::new();
    let mut grind = ControlledGrind {
        activated: false,
        adjusting: true,
    };
    let result = processed.with_process_data_read(|input, delta| {
        fixture.current.try_process_read(
            &mut fixture.targets,
            &fixture.pose,
            input,
            delta,
            &fixture.masses,
            &fixture.root,
            &super::TEST_ANIMATION_TO_WORLD,
            &fixture.world_to_animation,
            fixture.retained_com_y,
            &mut fixture.attributes,
            &mut fixture.cvars,
            &mut fixture.numeric,
            &mut grind,
        )
    });
    assert!(matches!(
        result,
        Err(SkeletonCurrentAnimationError::Input(
            AnimationInputError::Missing("category+8ec")
        ))
    ));
    assert!(fixture.targets.completed_volumes().is_some());
    assert!(processed.process_data_output().deck_zy_delta.is_none());
    assert!(grind.adjusting);
}

#[test]
fn late_landing_input_does_not_precede_required_attribute_lookup() {
    let mut fixture = Fixture::new();
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    fixture.attributes.value = Err("missing source attribute");
    processed.physics_state_fields_mut().balance = None;
    let mut grind = ControlledGrind {
        activated: false,
        adjusting: true,
    };
    let result = processed.with_process_data_read(|input, delta| {
        fixture.current.try_process_read(
            &mut fixture.targets,
            &fixture.pose,
            input,
            delta,
            &fixture.masses,
            &fixture.root,
            &super::TEST_ANIMATION_TO_WORLD,
            &fixture.world_to_animation,
            fixture.retained_com_y,
            &mut fixture.attributes,
            &mut fixture.cvars,
            &mut fixture.numeric,
            &mut grind,
        )
    });
    assert!(matches!(
        result,
        Err(SkeletonCurrentAnimationError::Attribute(
            "missing source attribute"
        ))
    ));
    assert!(!grind.adjusting);
    assert_eq!(&*fixture.events.borrow(), &["attribute"]);
    assert_eq!(
        processed
            .process_data_output()
            .deck_zy_delta
            .unwrap()
            .to_bits(),
        0
    );
}
