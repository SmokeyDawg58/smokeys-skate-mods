//! Proposed child of the existing original collision corpus; no asset reload.
use super::*;
use crate::{
    input_cvar_runtime::InputCvarReferences, reckoning_numeric::HostReckoningNumeric,
    simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame,
};

/// Run on a dedicated instance of the existing original completed-frame setup,
/// at the first call site after `post_records.prepare`, BEFORE any history update.
/// Do not append on the already-consumed same frame or rewrite old response
/// oracles to accept different live inputs silently.
pub(super) fn check_live_postphysics(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    references: &InputCvarReferences,
    frame: &Arc<SimulationContactSpyFrame>,
    actual_partial_ragdoll_argument: bool,
) {
    world
        .try_update_skeleton_postphysics_live(
            board,
            skeleton,
            references,
            actual_partial_ragdoll_argument,
            &mut HostReckoningNumeric,
        )
        .unwrap();
    let normal = world.skeleton(skeleton).unwrap();
    assert!(Arc::ptr_eq(
        normal.physics_history_frame_state().frame().unwrap(),
        frame
    ));
    assert!(Arc::ptr_eq(
        normal.collision_frame_state().frame().unwrap(),
        frame
    ));
    assert_eq!(normal.collision_frame_state().progress(),
        crate::dynamic_assembly_session::skeleton_normal::collision_owner::SkeletonCollisionFrameProgress::Complete);
    let collision = &normal.state().collision;
    assert!(collision.specific_parts.contact.iter().all(Option::is_some));
    assert!(
        collision
            .specific_parts
            .recent_contact
            .iter()
            .all(Option::is_some)
    );
    let positions = *normal.physics_history().record().positions();
    let com = normal.post_physics_records().centre_of_mass();
    // Same completed frame cannot repeat source mutations. This protects the
    // retained history used by mandatory Audio.Fill, rather than merely testing
    // a token flag or accepting a newly allocated collision owner.
    assert!(
        world
            .try_update_skeleton_postphysics_live(
                board,
                skeleton,
                references,
                actual_partial_ragdoll_argument,
                &mut HostReckoningNumeric
            )
            .is_err()
    );
    let normal = world.skeleton(skeleton).unwrap();
    assert_eq!(
        normal
            .physics_history()
            .record()
            .positions()
            .map(|v| v.map(f32::to_bits)),
        positions.map(|v| v.map(f32::to_bits))
    );
    assert_eq!(
        normal
            .post_physics_records()
            .centre_of_mass()
            .map(f32::to_bits),
        com.map(f32::to_bits)
    );
}
