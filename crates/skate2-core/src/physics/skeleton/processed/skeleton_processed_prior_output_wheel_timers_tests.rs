use crate::physics_state_ids::PHYSICS_STATE_SKITCHING;
use super::{PriorOutputWheelTimersError, SkeletonProcessedContact, wheel_pairs};
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarManager, InputCvarReferences},
    physical_player_assembly::hilod_prefix::PhysicalHiLodPrefix,
    physical_player_phys_out::{
        PhysicalPlayerPhysOut,
        construct::{PhysOutAllocation, prior_input::PriorWheelContact},
    },
};

fn pair() -> (
    PhysicalPlayerPhysOut,
    SkeletonProcessedContact,
    PhysicalHiLodPrefix,
) {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    assert!(processed.prior_output.bind_output_occurrence(&output));
    processed.physics_state_fields_mut().time_delta = Some(0.25);
    (
        output,
        processed,
        PhysicalHiLodPrefix::construct_in_sim_allocation(),
    )
}

#[test]
fn unknown_storage_reset_and_foreign_output_remain_distinct() {
    let mut processed = SkeletonProcessedContact::new();
    assert_eq!(
        processed.prior_wheel_timer_fields().skitching_vehicle_id,
        None
    );
    assert_eq!(
        processed
            .prior_wheel_timer_fields()
            .skitch_delay_until_reskitchable,
        None
    );
    processed.reset();
    assert_eq!(
        processed.prior_wheel_timer_fields().skitching_vehicle_id,
        Some(0)
    );
    let (output, _, mut prefix) = pair();
    processed.flags_8c8 = Some(0x42);
    assert_eq!(
        processed.try_copy_wheel_timer_fields(
            &output,
            &mut prefix,
            &InputCvarReferences::new_static_initialized()
        ),
        Err(PriorOutputWheelTimersError::ForeignOutput)
    );
    assert_eq!(processed.flags_8c8, Some(0x42));
    assert!(!processed.prior_wheel_timer_fields().complete);
}

#[test]
fn pairs_guard_fresh_operands_in_native_short_circuit_order() {
    use PriorWheelContact::{Wheel0, Wheel1, Wheel2, Wheel3};
    let mut seen = Vec::new();
    let result = wheel_pairs(|wheel| {
        seen.push(wheel);
        Ok::<_, ()>(matches!(wheel, Wheel2))
    })
    .unwrap();
    assert_eq!(seen, [Wheel2, Wheel0, Wheel1, Wheel2, Wheel3, Wheel1]);
    assert_eq!(result, [true, false, true, false]);
    let mut reads = 0;
    assert_eq!(
        wheel_pairs(|_| {
            reads += 1;
            Err::<bool, _>(42)
        }),
        Err(42)
    );
    assert_eq!(reads, 1);
}

#[test]
fn board_flip_swaps_pairs_and_preserves_other_flags() {
    for flipped in [false, true] {
        let (mut output, mut processed, mut prefix) = pair();
        output.edit_prior_wheel_timer_test_outputs(|wheels, _, _| {
            *wheels = [false, false, true, false];
        });
        processed.write_control_flag8c4(0x0010_0000, flipped);
        processed.flags_8c8 = Some(0x8000_0042);
        processed
            .try_copy_wheel_timer_fields(
                &output,
                &mut prefix,
                &InputCvarReferences::new_static_initialized(),
            )
            .unwrap();
        assert_eq!(
            processed.flags_8c8,
            Some(0x8000_0042 | if flipped { 0x0a00_0000 } else { 0x1400_0000 })
        );
        assert!(processed.prior_output.wheel_timers.complete);
    }
}

#[test]
fn timer_notgt_gates_preserve_nan_and_expired_values_without_clamp() {
    for timer in [0.0, -0.0, -1.0, f32::from_bits(0x7fc0_1234), 0.125] {
        let (mut output, mut processed, mut prefix) = pair();
        output.edit_prior_wheel_timer_test_outputs(|_, _, ground| {
            ground.skitch_delay_until_reskitchable = f32::NAN;
            ground.pushing_delay_until_repushable = -1.0;
        });
        prefix.skitching_vehicle_id = 73;
        prefix.skitch_vehicle_collision_avoid_timer = timer;
        prefix.skitch_delay_until_reskitchable = timer;
        prefix.pushing_delay_until_repushable = timer;
        processed
            .try_copy_wheel_timer_fields(
                &output,
                &mut prefix,
                &InputCvarReferences::new_static_initialized(),
            )
            .unwrap();
        let expected = if timer > 0.0 { timer - 0.25 } else { timer };
        assert_eq!(
            prefix.skitch_vehicle_collision_avoid_timer.to_bits(),
            expected.to_bits()
        );
        assert_eq!(
            prefix.skitch_delay_until_reskitchable.to_bits(),
            expected.to_bits()
        );
        assert_eq!(
            prefix.pushing_delay_until_repushable.to_bits(),
            expected.to_bits()
        );
        let fields = &processed.prior_output.wheel_timers;
        assert_eq!(
            fields.skitching_vehicle_id,
            Some(if timer > 0.0 { 73 } else { 0 })
        );
        let published = if timer > 0.0 { expected } else { 0.0 };
        assert_eq!(
            fields.skitch_delay_until_reskitchable.unwrap().to_bits(),
            published.to_bits()
        );
        assert_eq!(
            fields.pushing_delay_until_repushable.unwrap().to_bits(),
            published.to_bits()
        );
    }
}

#[test]
fn skitch_state_samples_live_cvar_and_publishes_even_after_nan_decrement() {
    let (mut output, mut processed, mut prefix) = pair();
    output.edit_prior_wheel_timer_test_outputs(|_, state, ground| {
        state.physics_state = PHYSICS_STATE_SKITCHING;
        state.grab_spline_desc.object_id = -1;
        ground.skitch_delay_until_reskitchable = 3.0;
        ground.pushing_delay_until_repushable = 4.0;
    });
    let refs = InputCvarReferences::new_static_initialized();
    assert_eq!(
        processed.try_copy_wheel_timer_fields(&output, &mut prefix, &refs),
        Err(PriorOutputWheelTimersError::Cvar(InputCvarError::Unlinked(
            "kTimeToAvoidVehicleAfterSkitching"
        )))
    );
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    assert_eq!(
        refs.try_time_to_avoid_vehicle_after_skitching()
            .unwrap()
            .to_bits(),
        0x4000_0000
    );
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\nkTimeToAvoidVehicleAfterSkitching=12\r\n",
        ))
        .unwrap();
    processed.physics_state_fields_mut().time_delta = Some(f32::NAN);
    processed
        .try_copy_wheel_timer_fields(&output, &mut prefix, &refs)
        .unwrap();
    assert!(prefix.skitch_vehicle_collision_avoid_timer.is_nan());
    assert_eq!(
        processed.prior_output.wheel_timers.skitching_vehicle_id,
        Some(u32::MAX)
    );
    assert!(
        processed
            .prior_output
            .wheel_timers
            .skitch_delay_until_reskitchable
            .unwrap()
            .is_nan()
    );
    assert!(
        processed
            .prior_output
            .wheel_timers
            .pushing_delay_until_repushable
            .unwrap()
            .is_nan()
    );
    assert_eq!(
        refs.try_time_to_avoid_vehicle_after_skitching()
            .unwrap()
            .to_bits(),
        12.0f32.to_bits()
    );
}
