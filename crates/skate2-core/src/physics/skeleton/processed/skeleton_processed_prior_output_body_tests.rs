use super::{PriorOutputBodyError, SkeletonProcessedContact};
use crate::{
    physical_player_phys_out::{
        PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut,
        construct::{ConcretePhysOutError, PhysOutAllocation, prior_input::PriorInputReadError},
    },
    physics::FilteredPhysicsState,
};

fn pair() -> (PhysicalPlayerPhysOut, SkeletonProcessedContact) {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    assert!(processed.prior_output.bind_output_occurrence(&output));
    (output, processed)
}

#[test]
fn mixed_flags_copy_without_category_gate_and_preserve_neighbors() {
    let (mut output, mut processed) = pair();
    output.edit_prior_body_test_outputs(|skeleton, filtered, offboard| {
        skeleton.skateboard_dont_push = true;
        skeleton.is_left_foot_on_deck = false;
        skeleton.is_right_foot_on_deck = true;
        filtered.filtered_physics_state = FilteredPhysicsState::Air;
        offboard.is_grabbing_object = true;
        offboard.time_to_land = 1.25;
        offboard.is_holding_skateboard = false;
        offboard.on_deck = true;
        offboard.can_land_on_skateboard = false;
        offboard.land_on_skateboard_flip_board = true;
        offboard.is_board_mount_invalid = true;
    });
    processed.flags_8c8 = Some(0xa5a5_a5a5);
    processed.physics_state.flags_8cc = Some(u32::MAX);
    processed.physics_state.flags_8d0 = Some(u32::MAX);
    processed.physics_state.flags_8d4 = (0x42, 0x43);
    assert_eq!(
        output
            .try_guarded_state_output()
            .unwrap()
            .physics_state_category,
        0
    );
    processed.try_copy_body_fields(&output).unwrap();
    assert_eq!(processed.flags_8c8, Some(0xa5a5_ada5));
    assert_eq!(processed.physics_state.flags_8cc, Some(!0x0008_0200));
    assert_eq!(processed.physics_state.flags_8d0, Some(!0x0004_2000));
    assert_eq!(
        processed.physics_state.flags_8d4,
        (0x0080_0042, 0x0080_0043)
    );
    let fields = &processed.prior_output.body_fields;
    assert_eq!(
        fields.filtered_physics_state,
        Some(FilteredPhysicsState::Air)
    );
    assert_eq!(fields.offboard_time_to_land, Some(1.25));
    assert!(fields.complete);
}

#[test]
fn landing_comparison_includes_unordered_and_preserves_scalar_bits() {
    for (bits, expected) in [
        (0x0000_0000, true),
        (0x8000_0000, true),
        (0xbf80_0000, true),
        (0x3f80_0000, false),
        (0x7f80_0000, false),
        (0xff80_0000, true),
        (0x7fc1_2345, true),
    ] {
        let (mut output, mut processed) = pair();
        output.edit_prior_body_test_outputs(|_, _, offboard| {
            offboard.time_to_land = f32::from_bits(bits);
        });
        processed.try_copy_body_fields(&output).unwrap();
        assert_eq!(
            processed.physics_state.flags_8cc.unwrap() & 0x80000 != 0,
            expected
        );
        assert_eq!(
            processed
                .prior_output
                .body_fields
                .offboard_time_to_land
                .unwrap()
                .to_bits(),
            bits
        );
    }
}

#[test]
fn late_unknown_destination_retains_prior_stores_and_incomplete_boundary() {
    let (mut output, mut processed) = pair();
    output.edit_prior_body_test_outputs(|skeleton, _, _| {
        skeleton.skateboard_dont_push = true;
    });
    processed.flags_8c8 = Some(0);
    processed.physics_state.flags_8d0 = None;
    processed.prior_output.body_fields.offboard_time_to_land = Some(77.0);
    assert_eq!(
        processed.try_copy_body_fields(&output),
        Err(PriorOutputBodyError::UnknownWord("8d0"))
    );
    assert_eq!(processed.flags_8c8, Some(0x800));
    assert_eq!(
        processed.prior_output.body_fields.offboard_time_to_land,
        Some(77.0)
    );
    assert!(!processed.prior_output.body_fields.complete);
}

#[test]
fn null_filtered_component_preserves_three_skeleton_stores_before_failure() {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|component| {
            Ok::<_, ()>(if component == PhysOutComponent::Filtered {
                PhysOutAllocation::Null
            } else {
                PhysOutAllocation::Host
            })
        })
        .unwrap();
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    assert!(processed.prior_output.bind_output_occurrence(&output));
    processed.flags_8c8 = Some(u32::MAX);
    processed.physics_state.flags_8d0 = Some(u32::MAX);
    processed.physics_state.flags_8cc = Some(0x42);
    assert_eq!(
        processed.try_copy_body_fields(&output),
        Err(PriorOutputBodyError::Read(PriorInputReadError::Component(
            ConcretePhysOutError::MissingComponent(PhysOutComponent::Filtered)
        )))
    );
    assert_eq!(processed.flags_8c8, Some(!0x800));
    assert_eq!(processed.physics_state.flags_8d0, Some(!0x3000));
    assert_eq!(processed.physics_state.flags_8cc, Some(0x42));
    assert!(!processed.prior_output.body_fields.complete);
}

#[test]
fn latch_and_foreign_owner_fail_before_native_stores_and_reset_clears_readiness() {
    let (mut output, mut processed) = pair();
    processed.flags_8c8 = Some(0x1234);
    output.lock_after_update1().unwrap();
    assert_eq!(
        processed.try_copy_body_fields(&output),
        Err(PriorOutputBodyError::Read(PriorInputReadError::Access(
            PhysOutAccessError::LockedForWrite
        )))
    );
    assert_eq!(processed.flags_8c8, Some(0x1234));
    let (other, _) = pair();
    assert_eq!(
        processed.try_copy_body_fields(&other),
        Err(PriorOutputBodyError::ForeignOutput)
    );
    processed.prior_output.body_fields.complete = true;
    processed.prior_output.body_fields.offboard_time_to_land = Some(77.0);
    processed.reset();
    assert_eq!(
        processed.prior_output.body_fields.filtered_physics_state,
        Some(FilteredPhysicsState::Invalid)
    );
    assert_eq!(
        processed.prior_output.body_fields.offboard_time_to_land,
        Some(0.0)
    );
    assert!(!processed.prior_output.body_fields.complete);
}
