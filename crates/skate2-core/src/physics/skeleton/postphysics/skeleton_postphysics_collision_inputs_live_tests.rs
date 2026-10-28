use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GROUND, PHYSICS_CATEGORY_GROUNDTRICK, PHYSICS_CATEGORY_OFFBOARD,
    PHYSICS_STATE_BONELESS, PHYSICS_STATE_FOOTPLANT, PHYSICS_STATE_LANDINGONDECK,
    PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_PHYSICS_GROUND,
};
use super::*;
use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
use crate::{
    attribute_runtime::tests::graph,
    reckoning_attributes::{RECKONING_CLASS, ReckoningVaultAttributes},
    skeleton_post_physics_records::SkeletonPostPhysicsRecords,
};
use std::sync::Arc;

fn reckoning() -> Reckoning<ReckoningVaultAttributes> {
    Reckoning::try_new(ReckoningVaultAttributes::bind(&Arc::new(graph(
        RECKONING_CLASS,
        vec![],
    ))))
    .unwrap()
}
fn processed() -> SkeletonProcessedContact {
    let mut result = SkeletonProcessedContact::new();
    result.try_initialize_constructor_fields().unwrap();
    result
}
fn input(
    p: &SkeletonProcessedContact,
) -> Result<SkeletonCollisionUpdateInput, CollisionInputError> {
    p.try_postphysics_collision_input(
        SkeletonRootVector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        },
        Some(&reckoning()),
        &SkeletonPostPhysicsRecords::new(),
        0.125,
        true,
    )
}

#[test]
fn constructor_null_grab_and_selected_ground_do_not_consume_offboard_payloads() {
    let mut p = processed();
    p.physics_state.physics = Some(PHYSICS_STATE_PHYSICS_GROUND);
    p.physics_state.category = Some(PHYSICS_CATEGORY_GROUND);
    p.physics_state.flags_8d0 = None;
    p.prior_output.input_fields.next_step_position = None;
    p.prior_output.input_fields.next_step_normal = None;
    p.flags_8c8 = Some(0x4001_0000);
    let actual = input(&p).unwrap();
    assert!(actual.ignore_rigid_body.is_none());
    assert_eq!(
        (
            actual.ai_skater,
            actual.danger_zone,
            actual.request_partial_ragdoll
        ),
        (1, 1, 1)
    );
    assert_eq!(
        actual.system_velocity.map(f32::to_bits),
        [2.0_f32, 3.0, 4.0].map(f32::to_bits)
    );
}

#[test]
fn reset_without_actual_embedded_constructor_does_not_fabricate_grab_status() {
    let mut p = SkeletonProcessedContact::new();
    p.reset();
    assert!(matches!(
        input(&p),
        Err(CollisionInputError::Missing(
            "constructed processed primary GrabSpline"
        ))
    ));
}

#[test]
fn boneless_and_footplant_read_selected_ground_then_offboard_override_is_late() {
    let mut p = processed();
    p.ground_position.ground_position = Some(SkeletonRootVector::known([7.0; 4]));
    p.prior_output
        .publish_ground_position_all_states(SkeletonRootVector::known([11.0; 4]));
    for state in [PHYSICS_STATE_FOOTPLANT, PHYSICS_STATE_BONELESS] {
        p.physics_state.physics = Some(state);
        p.physics_state.category = Some(PHYSICS_CATEGORY_GROUNDTRICK);
        let result = input(&p).unwrap();
        assert_eq!(
            result.ground_position.map(f32::to_bits),
            [7.0_f32; 3].map(f32::to_bits)
        );
        assert_eq!(result.ground_trick, 1);
    }
    p.physics_state.physics = Some(PHYSICS_STATE_OFFBOARD_PUSHING);
    p.physics_state.category = Some(PHYSICS_CATEGORY_OFFBOARD);
    p.physics_state.flags_8d0 = Some(8);
    p.prior_output.input_fields.next_step_position = Some(WalkVector::known([13.0; 4]));
    p.prior_output.input_fields.next_step_normal = Some(WalkVector::known([17.0; 4]));
    let result = input(&p).unwrap();
    assert_eq!(
        result.ground_position.map(f32::to_bits),
        [13.0_f32; 3].map(f32::to_bits)
    );
    assert_eq!(
        result.ground_normal.map(f32::to_bits),
        [17.0_f32; 3].map(f32::to_bits)
    );
    // Air and landing-on-deck do not perform the offboard flag read at all.
    p.physics_state.flags_8d0 = None;
    for state in [PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_LANDINGONDECK] {
        p.physics_state.physics = Some(state);
        let result = input(&p).unwrap();
        assert_eq!((result.offboard, result.offboard_air), (0, 1));
    }
}

#[test]
fn enabled_grab_status_with_actual_null_body_still_ignores_nothing() {
    let mut p = processed();
    p.primary_grab.as_mut().unwrap().status |= 0x40;
    assert!(input(&p).unwrap().ignore_rigid_body.is_none());
}
