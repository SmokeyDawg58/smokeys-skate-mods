//! CheckAirCameraCollision82e3ec80 uses the same rig collision service.
use super::CameraTrajectoryError as E;
use crate::{
    camera_collision::CameraCollision, camera_positioner::CameraPositioner,
    camera_subject::CameraSubject, collision_line_collection::CollisionLineInput,
    input_cvar_runtime::InputCvarReferences,
    simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices,
    skeleton_root_transforms::SkeletonRootVector as V,
};
use std::sync::{Arc, Mutex};

fn end_position(camera: V, board: V, landing: V) -> V {
    // Raw3ed08 VD61 VA62 VB63, then3ed44 VD58 VA59 VB60.
    // Keep the two rounded vector operations and all retained W lanes.
    camera
        .zip(board, |camera, board| camera - board)
        .zip(landing, |offset, landing| offset + landing)
}
pub(super) fn check_air_collision<N>(
    positioner: &CameraPositioner,
    collision: &Arc<Mutex<CameraCollision>>,
    subject: &CameraSubject,
    cvars: &InputCvarReferences,
    services: &mut CameraLineServices<'_>,
) -> Result<bool, E<N>> {
    if !cvars.try_camera_enable_air_collision().map_err(E::Cvar)? {
        return Ok(false);
    }
    let start = positioner.position;
    let radius = positioner.collision_radius;
    let end = end_position(
        start,
        subject.board_position,
        subject.trajectory_end_position,
    );
    let mut owner = collision.lock().map_err(|_| E::OwnerLock)?;
    let result = owner
        .try_test_line(
            CollisionLineInput {
                start,
                end,
                fatness: radius,
            },
            services,
        )
        .map_err(E::Query)?;
    // The source reads collided only AFTER this live cvar and overlay assertion.
    if cvars.try_camera_show_air_collision().map_err(E::Cvar)? {
        services.overlay.require().map_err(E::DebugOverlay)?;
    }
    Ok(result.collided)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn end_position_preserves_unknown_w_and_separate_subtract_add_rounding() {
        let camera = V {
            xyz: [16777216.0, 2.0, -0.0],
            w: None,
        };
        let board = V::known([-1.0, 5.0, 0.0, 8.0]);
        let landing = V::known([-16777216.0, 8.0, 0.0, 10.0]);
        let end = end_position(camera, board, landing);
        assert_eq!(
            end.xyz.map(f32::to_bits),
            [0.0_f32, 5.0, 0.0].map(f32::to_bits)
        );
        assert_eq!(end.w, None);
        let end = end_position(V::known([0.0, 0.0, 0.0, 3.0]), board, landing);
        assert_eq!(end.w, Some(5.0));
    }
    #[test]
    fn disabled_gate_is_lazy_but_enabled_path_requires_real_query_owner() {
        use crate::{
            camera_attributes::CameraAttributes, collision_line_numeric::HostCollisionLineNumeric,
            debug_overlay::DebugOverlaySlot, input_cvar_runtime::InputCvarManager,
            static_ground_pool::StaticGroundPool,
        };
        let collision = Arc::new(Mutex::new(CameraCollision::constructed(
            std::sync::Weak::new(),
        )));
        let positioner = CameraPositioner::constructed(
            Arc::clone(&collision),
            Arc::new(Mutex::new(CameraAttributes::begin_init())),
            true,
        );
        let subject = CameraSubject::try_construct().unwrap();
        let cvars = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&cvars).unwrap();
        let ground = StaticGroundPool::try_new(1).unwrap();
        let mut queries = None;
        let mut numeric = HostCollisionLineNumeric;
        let overlay = DebugOverlaySlot::default();
        let grinds = crate::simulation_assembly_backend::constraint_session::grind_world::PhysicalGrindServices::new();
        let mut services = CameraLineServices {
            queries: &mut queries,
            dmos: None,
            ground: &ground,
            numeric: &mut numeric,
            overlay: &overlay,
            grinds: &grinds,
        };
        assert!(
            !check_air_collision::<()>(&positioner, &collision, &subject, &cvars, &mut services)
                .unwrap()
        );
        manager
            .try_deserialize_startup(Some(b"[Config]\nkbEnableAirCameraCollision=1\n"))
            .unwrap();
        assert!(matches!(
            check_air_collision::<()>(&positioner, &collision, &subject, &cvars, &mut services),
            Err(E::Query(
                crate::camera_collision::query::CameraQueryError::MissingQueries
            ))
        ));
    }
}
