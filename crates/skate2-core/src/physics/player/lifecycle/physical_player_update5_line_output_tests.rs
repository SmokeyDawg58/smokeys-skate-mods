use super::*;
use crate::{
    physical_player_phys_out::{
        ZERO,
        construct::{ComponentStatus, PhysOutAllocation},
    },
    skeleton_root_transforms::SkeletonRootVector,
};

fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}
fn tail() -> Mutex<PhysicalHiLodTail> {
    let mut tail = PhysicalHiLodTail::from_sim_zeroed_allocation();
    tail.initialize_early_line_results();
    Mutex::new(tail)
}

#[test]
fn reset_unlocks_same_output_then_valid_line_copies_all_lanes_and_id_bits() {
    let mut output = output();
    let identity = Arc::clone(output.occurrence());
    let tail = tail();
    {
        let mut source = tail.lock().unwrap();
        let line = &mut source.line_records.as_mut().unwrap()[0];
        line.start_position = SkeletonRootVector::known([1.0, 2.0, 3.0, -0.0]);
        line.result.position = SkeletonRootVector::known([4.0, 5.0, 6.0, 7.0]);
        line.result.normal = SkeletonRootVector {
            xyz: [8.0, 9.0, 10.0],
            w: None,
        };
        line.result.surface_id = -2;
        line.result.valid = true;
    }
    output.lock_after_update1().unwrap();
    output.try_begin_update5_output(&tail).unwrap();
    assert!(Arc::ptr_eq(&identity, output.occurrence()));
    assert_eq!(output.locked_for_write(), Some(false));
    let ground = output.concrete.ground.as_ref().unwrap();
    assert!(ground.below_hips_ground_data_is_valid);
    assert_eq!(
        ground.below_hips_ground_data_start_pos.w.unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        ground.below_hips_ground_data_position,
        SkeletonRootVector::known([4.0, 5.0, 6.0, 7.0])
    );
    assert_eq!(ground.below_hips_ground_data_normal.w, None);
    assert_eq!(ground.below_hips_ground_data_surface_id, 0xffff_fffe);
    assert_eq!(output.concrete.state.as_ref().unwrap().physics_state, 0);
}

#[test]
fn invalid_result_still_copies_start_and_preserves_reset_payload() {
    let mut output = output();
    let tail = tail();
    {
        let mut source = tail.lock().unwrap();
        let line = &mut source.line_records.as_mut().unwrap()[0];
        line.start_position = SkeletonRootVector {
            xyz: [11.0, 12.0, 13.0],
            w: None,
        };
        line.result.position = SkeletonRootVector::known([f32::NAN; 4]);
        line.result.surface_id = 123;
    }
    output
        .concrete
        .ground
        .as_mut()
        .unwrap()
        .below_hips_ground_data_position = SkeletonRootVector::known([5.0; 4]);
    output.try_begin_update5_output(&tail).unwrap();
    let ground = output.concrete.ground.as_ref().unwrap();
    assert!(!ground.below_hips_ground_data_is_valid);
    assert_eq!(
        ground
            .below_hips_ground_data_start_pos
            .xyz
            .map(f32::to_bits),
        [11.0_f32, 12.0, 13.0].map(f32::to_bits)
    );
    assert_eq!(ground.below_hips_ground_data_start_pos.w, None);
    assert_eq!(ground.below_hips_ground_data_position, ZERO);
    assert_eq!(ground.below_hips_ground_data_normal, ZERO);
    assert_eq!(ground.below_hips_ground_data_surface_id, 0);
}

#[test]
fn absent_records_fail_after_completed_reset_but_guard_precedes_source_read() {
    let mut output = output();
    let tail = Mutex::new(PhysicalHiLodTail::from_sim_zeroed_allocation());
    output.lock_after_update1().unwrap();
    assert!(matches!(
        output.try_fill_update5_hips_line(&tail),
        Err(Update5LineOutputError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert!(matches!(
        output.try_begin_update5_output(&tail),
        Err(Update5LineOutputError::MissingLineRecords)
    ));
    assert_eq!(output.locked_for_write(), Some(false));
}

#[test]
fn later_reset_failure_keeps_old_latch_and_does_not_consume_line_source() {
    let mut output = output();
    output
        .concrete
        .motion
        .as_mut()
        .unwrap()
        .emperical_deck_velocity = SkeletonRootVector::known([9.0; 4]);
    output.concrete.status[PhysOutComponent::Air as usize] = ComponentStatus::AllocationNull;
    output.lock_after_update1().unwrap();
    let tail = Mutex::new(PhysicalHiLodTail::from_sim_zeroed_allocation());
    assert!(matches!(
        output.try_begin_update5_output(&tail),
        Err(Update5LineOutputError::Reset(
            ConcretePhysOutError::MissingComponent(PhysOutComponent::Air)
        ))
    ));
    assert_eq!(
        output
            .concrete
            .motion
            .as_ref()
            .unwrap()
            .emperical_deck_velocity,
        ZERO
    );
    assert_eq!(output.locked_for_write(), Some(true));
}

use std::sync::Arc;
