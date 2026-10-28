//! Complete constructor output ownership and selected contact publication on the
//! original pair. The assembly mounted this same output before either Body;
//! prior-output gaps remain explicit diagnostic collaborators.
use super::PhysicalAssemblyWorld;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_phys_out::{PhysOutAccessError, PhysOutOrigin, PhysOutPublicationError},
    simulation_assembly_backend::constraint_session::physical_phys_out::PhysicalPlayerSurfaceOutputError,
    skateboard_contact_surface::SkateboardContactSurfaceCvars,
    skateboard_processed_surface::{SkateboardProcessedSurface, SkateboardSurfaceProjection},
    skeleton_processed_contact::{
        SkeletonProcessedContact,
        prior_output::{SkeletonPriorOutputGap, SkeletonPriorOutputGaps},
    },
};
use std::{convert::Infallible, sync::Arc};
#[derive(Default)]
pub(super) struct Gaps {
    count: usize,
}
impl SkeletonPriorOutputGaps for Gaps {
    type Error = Infallible;
    fn run(
        &mut self,
        _: SkeletonPriorOutputGap,
        _: &mut crate::physical_player_phys_out::PhysicalPlayerPhysOut,
    ) -> Result<(), Self::Error> {
        self.count += 1;
        Ok(())
    }
}
struct ForceSurfaceDefault;
/// Verify the already mounted constructor, before the first output write.
pub(super) fn construct(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    world
        .try_with_pair_phys_out(board, skeleton, |output| {
            assert!(output.complete_construction());
            assert!(output.component_statuses().iter().all(|status| {
                *status == crate::physical_player_phys_out::construct::ComponentStatus::Constructed
            }));
            assert_eq!(
                output.collision_physics_type().unwrap().origin,
                PhysOutOrigin::Constructor
            );
            assert_eq!(
                output
                    .try_animation_output()
                    .unwrap()
                    .time_since_last_input
                    .to_bits(),
                0
            );
        })
        .unwrap();
}
impl SkateboardContactSurfaceCvars for ForceSurfaceDefault {
    type Error = Infallible;
    fn forced_world_surface(&mut self) -> Result<u32, Self::Error> {
        Ok(0)
    }
}
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    processed: &mut SkeletonProcessedContact,
) {
    let occurrence = world
        .try_with_pair_phys_out(board, skeleton, |output| {
            assert!(output.selected_construction_complete());
            Arc::clone(output.occurrence())
        })
        .unwrap();
    world
        .try_update_board_contact_surface(board, &mut ForceSurfaceDefault)
        .unwrap();
    world
        .try_publish_pair_board_surface(board, skeleton)
        .unwrap();
    let (surface, expected) = world
        .try_with_board_force_runtime(board, |runtime| {
            let mut surface = SkateboardProcessedSurface::bind(&runtime.slide_collision_attributes);
            surface
                .try_initialize_materials(&mut runtime.slide_collision_attributes)
                .unwrap();
            (
                surface,
                runtime.contact_surface.lock().unwrap().overall().unwrap(),
            )
        })
        .unwrap();
    let mut gaps = Gaps::default();
    world
        .try_with_pair_phys_out(board, skeleton, |output| {
            assert!(Arc::ptr_eq(&occurrence, output.occurrence()));
            let published = output.collision_physics_type().unwrap();
            assert_eq!(published.value, expected);
            assert_eq!(published.origin, PhysOutOrigin::Publication);
            let mut selected = SkateboardSurfaceProjection::default();
            output
                .try_dispatch_surface(&surface, &mut selected)
                .unwrap();
            let normalized = match expected {
                1..=5 => expected,
                6 => 3,
                _ => 1,
            };
            assert_eq!(selected.physics_type, Some(normalized));
            assert!(selected.selected.is_some() && selected.material.is_some());
            processed.try_copy_prior_output(output, &mut gaps).unwrap();
            assert_eq!(gaps.count, 10);
            assert!(processed.prior_output().selected_copies_complete());
            // Only collision was published by this selected probe. Motion/ground
            // remain proven constructor data, not mislabeled current engine output.
            assert_eq!(
                processed.prior_output().velocity().unwrap().origin,
                PhysOutOrigin::Constructor
            );
            assert_eq!(
                processed.prior_output().z_speed().unwrap().value.to_bits(),
                0
            );
            assert_eq!(
                processed
                    .prior_output()
                    .surface_speed()
                    .unwrap()
                    .value
                    .to_bits(),
                0
            );
            assert_eq!(
                processed
                    .prior_output()
                    .deck_speed()
                    .unwrap()
                    .value
                    .to_bits(),
                0
            );
            assert_eq!(
                processed.prior_output().ground_normal().unwrap().origin,
                PhysOutOrigin::Constructor
            );
            assert!(processed.ready(0).is_none());
            output.lock_after_update1().unwrap();
        })
        .unwrap();
    assert!(matches!(
        world.try_publish_pair_board_surface(board, skeleton),
        Err(PhysicalPlayerSurfaceOutputError::Publication(
            PhysOutPublicationError::Access(PhysOutAccessError::LockedForWrite)
        ))
    ));
    world
        .try_with_pair_phys_out(board, skeleton, |output| {
            output.try_reset_complete().unwrap();
            assert!(Arc::ptr_eq(&occurrence, output.occurrence()));
            assert_eq!(
                output.collision_physics_type().unwrap().origin,
                PhysOutOrigin::Reset
            );
        })
        .unwrap();
    world
        .try_publish_pair_board_surface(board, skeleton)
        .unwrap();
}
