use crate::physics_state_ids::{
    PHYSICS_STATE_BONELESS, PHYSICS_STATE_KNOWN_AIR, PHYSICS_STATE_OFFBOARD,
    PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_PHYSICS_AIR,
};
use super::*;
fn source() -> SkeletonProcessedContact {
    let mut source = SkeletonProcessedContact::new();
    source.try_initialize_constructor_fields().unwrap();
    source
}
#[test]
fn boneless_precedes_offboard_and_unknown_jump_fields() {
    let mut source = source();
    let state = source.physics_state_fields_mut();
    state.flags_8d0 = Some(0x20000);
    state.flags_8cc = None;
    state.frames_since_jump = None;
    assert_eq!(
        select_ground_animation::<()>(&source, false).unwrap(),
        Selection::State(PHYSICS_STATE_BONELESS)
    );
}
#[test]
fn offboard_priorities_precede_frames_and_animation_queries() {
    let mut source = source();
    source.physics_state_fields_mut().frames_since_jump = None;
    source.physics_state_fields_mut().flags_8cc = Some(0x20100);
    assert_eq!(
        select_ground_animation::<()>(&source, false).unwrap(),
        Selection::State(PHYSICS_STATE_OFFBOARD)
    );
    source.physics_state_fields_mut().flags_8cc = Some(0x100);
    assert_eq!(
        select_ground_animation::<()>(&source, true).unwrap(),
        Selection::State(PHYSICS_STATE_OFFBOARD_AIR)
    );
}
#[test]
fn jump_and_ordinary_results_require_distinct_live_continuations() {
    let mut source = source();
    source.physics_state_fields_mut().frames_since_jump = Some(1);
    source.publish_trajectory_selector_ready(false);
    assert_eq!(
        select_ground_animation::<()>(&source, false).unwrap(),
        Selection::State(PHYSICS_STATE_PHYSICS_AIR)
    );
    source.publish_trajectory_selector_ready(true);
    assert_eq!(
        select_ground_animation::<()>(&source, true).unwrap(),
        Selection::State(PHYSICS_STATE_KNOWN_AIR)
    );
    source.physics_state_fields_mut().frames_since_jump = Some(2);
    assert_eq!(
        select_ground_animation::<()>(&source, true).unwrap(),
        Selection::GroundAnimation { colliding: true }
    );
}
