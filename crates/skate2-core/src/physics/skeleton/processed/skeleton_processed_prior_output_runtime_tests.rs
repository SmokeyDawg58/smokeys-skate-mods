use crate::physics_state_ids::PHYSICS_STATE_OFFBOARD;
use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
fn make_output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}
fn input() -> AnimOutPhysInOwner {
    let mut input = AnimOutPhysInOwner::new_sim_allocation();
    input.try_construct(1).unwrap();
    input
}
#[test]
fn complete_copy_consumes_big_endian_flag_byte_and_actual_animation_payloads() {
    let output = make_output();
    let mut input = input();
    input.locomotion_flags = 0xa000_0040;
    input.flags.next_step_valid = true;
    input.center_of_mass_delta = WalkVector::known([1.0, 2.0, 3.0, -0.0]);
    input.foot_targets = [WalkVector::known([5.0; 4]), WalkVector::known([6.0; 4])];
    input.foot_ik = [-2.0, 4.0];
    input.next_step_position = WalkVector::known([7.0; 4]);
    input.next_step_normal = WalkVector::known([8.0; 4]);
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    processed.physics_state.flags_8cc = Some(0x1234_56fe);
    processed
        .try_copy_prior_input_runtime(&output, &input)
        .unwrap();
    assert_eq!(
        processed.physics_state.flags_8cc,
        Some((0x1234_56fe & !0xe1) | 0xa0)
    );
    assert_eq!(processed.physics_state.flags_8d0, Some(8));
    assert_eq!(
        processed
            .prior_output
            .input_fields
            .center_of_mass_delta
            .unwrap()
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        input
            .center_of_mass_delta
            .known_lanes()
            .unwrap()
            .map(f32::to_bits)
    );
    assert_eq!(
        processed.prior_output.input_fields.foot_targets[1]
            .unwrap()
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [6.0_f32.to_bits(); 4]
    );
    assert_eq!(
        processed.prior_output.input_fields.foot_ik[0]
            .unwrap()
            .to_bits(),
        (-2.0_f32).to_bits()
    );
    assert_eq!(
        processed
            .prior_output
            .input_fields
            .next_step_normal
            .unwrap()
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [8.0_f32.to_bits(); 4]
    );
    assert!(processed.prior_output.input_fields.complete);
    assert_eq!(output.locked_for_write(), Some(false));
}
#[test]
fn late_unknown_word_retains_the_exact_animation_copy_prefix() {
    let output = make_output();
    let mut input = input();
    input.foot_targets[1] = WalkVector::known([6.0; 4]);
    input.next_step_position = WalkVector::known([7.0; 4]);
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    processed.physics_state.flags_8d0 = None;
    assert_eq!(
        processed.try_copy_prior_input_runtime(&output, &input),
        Err(PriorInputRuntimeError::UnknownWord("8d0"))
    );
    assert_eq!(
        processed.prior_output.input_fields.foot_targets[1]
            .unwrap()
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [6.0_f32.to_bits(); 4]
    );
    assert_eq!(
        processed
            .prior_output
            .input_fields
            .next_step_position
            .unwrap()
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
    assert!(!processed.prior_output.input_fields.complete);
}

#[test]
fn prior_input_publication_preserves_unknown_lanes_without_replacing_xyz() {
    let output = make_output();
    let mut input = input();
    let unknown = WalkVector {
        xyz: [1.0, -0.0, 3.0],
        w: None,
    };
    input.center_of_mass_delta = unknown;
    input.center_of_mass_animation = unknown;
    input.foot_targets = [unknown; 2];
    input.next_step_position = unknown;
    input.next_step_normal = unknown;
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    processed
        .try_copy_prior_input_runtime(&output, &input)
        .unwrap();
    let fields = &processed.prior_output.input_fields;
    for vector in [
        fields.center_of_mass_delta,
        fields.center_of_mass_animation,
        fields.foot_targets[0],
        fields.foot_targets[1],
        fields.next_step_position,
        fields.next_step_normal,
    ] {
        let vector = vector.unwrap();
        assert_eq!(vector.w, None);
        assert_eq!(vector.xyz.map(f32::to_bits), unknown.xyz.map(f32::to_bits));
    }
}
#[test]
fn locked_output_fails_before_state_stores_and_foreign_owner_is_rejected() {
    let mut output = make_output();
    let input = input();
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    processed.physics_state.physics = Some(PHYSICS_STATE_OFFBOARD);
    output.lock_after_update1().unwrap();
    assert_eq!(
        processed.try_copy_prior_input_runtime(&output, &input),
        Err(PriorInputRuntimeError::Read(PriorInputReadError::Access(
            PhysOutAccessError::LockedForWrite
        )))
    );
    assert_eq!(processed.physics_state.physics, Some(PHYSICS_STATE_OFFBOARD));
    let foreign = make_output();
    assert_eq!(
        processed.try_copy_prior_input_runtime(&foreign, &input),
        Err(PriorInputRuntimeError::ForeignOutput)
    );
}
