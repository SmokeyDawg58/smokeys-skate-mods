use super::*;
use crate::{
    simulation_assembly_backend::constraint_session::skateboard_ground::{
        SkateboardGroundFrameError, SkateboardGroundFrameInput,
    },
    static_ground_pool::StaticGroundPool,
};

type World = SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>;

/// Test-only readback from the real ground job and exclusive body phase. It
/// neither refreshes a publication nor manufactures contact rows or labels.
pub(crate) fn run_pool_ground_job(
    backend: &mut crate::skateboard_assembly::SkateboardSimulationBackend,
    pool: &SimulationSkaterAssemblyPool,
    manager: &crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactDomain,
    ground: &StaticGroundPool,
) -> Result<
    crate::simulation_contact_jacobian::ground_preparation::cpu_job::SimulationStaticGroundJobOutput,
    crate::simulation_contact_jacobian::ground_preparation::cpu_job::SimulationStaticGroundJobError,
>{
    use crate::simulation_contact_jacobian::ground_preparation::{
        SimulationGroundRunSettings,
        cpu_job::{SimulationAuthoredGroundJobInput, try_run_authored_ground_cpu_job},
    };
    let phase = backend
        .rigid_bodies
        .try_prepare_dynamic_update_phase()
        .unwrap();
    try_run_authored_ground_cpu_job(
        pool.publications(),
        &SimulationAuthoredGroundJobInput {
            ground_pool: ground,
            phase: &phase,
            maximum_contacts: 256,
            allow_multiple_writes: true,
            run_settings: SimulationGroundRunSettings {
                ground: Some(manager.static_body_observation()),
                ground_group: 0,
                pipeline_culling: None,
                is_object: false,
                draw_contacts: false,
            },
        },
    )
}

#[test]
fn skater_pool_initialization_preserves_shared_world_flags_and_cannot_replace_membership() {
    let mut world = World::try_new(1, 0, 0, 0).unwrap();
    let before = world.backend().rigid_body_snapshot();
    assert!(matches!(
        world.try_initialize_skater_pool(0, false),
        Err(SimulationSkaterPoolError::WorldFlagsMismatch {
            expected: 3,
            actual: 0
        })
    ));
    assert!(matches!(
        world.try_initialize_skater_pool(3, true),
        Err(SimulationSkaterPoolError::UnsupportedParallel)
    ));
    assert!(world.skaters.is_none());
    world.try_initialize_skater_pool(3, false).unwrap();
    assert_eq!(world.skater_pool().unwrap().capacity(), 16);
    assert!(matches!(
        world.try_initialize_skater_pool(3, false),
        Err(SimulationSkaterPoolError::AlreadyInitialized)
    ));
    assert_eq!(world.backend().rigid_body_snapshot(), before);
}

#[test]
fn alternate_explicit_world_flags_reach_both_real_pool_constructors() {
    let mut world = World::try_new_with_island_flags(1, 0, 0, 0, 0).unwrap();
    world.try_initialize_skater_pool(0, false).unwrap();
    assert_eq!(world.skater_pool().unwrap().capacity(), 2);
    assert_eq!(world.island_flags, 0);
    assert_eq!(world.hook_count(), 0);
    assert_eq!(world.board_count(), 0);
}

#[test]
fn uninitialized_ground_owner_refuses_before_frame_effects() {
    let mut world = World::try_new(1, 0, 0, 0).unwrap();
    assert!(world.contact_spies().unwrap().is_none());
    let before = world.backend().rigid_body_snapshot();
    let frame = world.backend().collision_body_pool().frame_identity();
    let ground = StaticGroundPool::try_new(0).unwrap();
    assert!(matches!(
        world.try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: &ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            f32::from_bits(0x3c88_8889),
            50,
            &mut super::super::contact_feedback::ControlledNoFreezing,
        ),
        Err(SkateboardGroundFrameError::Pool(
            SimulationSkaterPoolError::Uninitialized
        ))
    ));
    assert_eq!(world.backend().rigid_body_snapshot(), before);
    assert_eq!(
        world.backend().collision_body_pool().frame_identity(),
        frame
    );
}

#[test]
fn gameplay_schedule_rejects_non_both_flags_before_frame_effects() {
    use crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule;
    let mut world = World::try_new_with_island_flags(1, 0, 0, 0, 0).unwrap();
    world.try_initialize_skater_pool(0, false).unwrap();
    let frame = world.backend().collision_body_pool().frame_identity();
    let before = world.backend().rigid_body_snapshot();
    let ground = StaticGroundPool::try_new(0).unwrap();
    let result = world.try_step_authored_ground(
        SkateboardGroundFrameInput {
            schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
            ground_pool: &ground,
            ground_group: 0,
            pipeline_culling: None,
            allow_multiple_writes: true,
            spy_flags: 4,
        },
        f32::from_bits(0x3c88_8889),
        50,
        &mut super::super::contact_feedback::ControlledNoFreezing,
    );
    assert!(matches!(
        result,
        Err(SkateboardGroundFrameError::UnsupportedPredictorSchedule { island_flags: 0 })
    ));
    assert_eq!(
        world.backend().collision_body_pool().frame_identity(),
        frame
    );
    assert_eq!(world.backend().rigid_body_snapshot(), before);
}

#[test]
fn gameplay_missing_predictor_fails_at_harvest_after_frame_begin_and_refresh() {
    use crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule;
    let mut world = World::try_new(1, 0, 0, 0).unwrap();
    world.try_initialize_skater_pool(3, false).unwrap();
    let frame = world.backend().collision_body_pool().frame_identity();
    let ground = StaticGroundPool::try_new(0).unwrap();
    let result = world.try_step_authored_ground(
        SkateboardGroundFrameInput {
            schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
            ground_pool: &ground,
            ground_group: 0,
            pipeline_culling: None,
            allow_multiple_writes: true,
            spy_flags: 4,
        },
        f32::from_bits(0x3c88_8889),
        50,
        &mut super::super::contact_feedback::ControlledNoFreezing,
    );
    assert!(matches!(
        result,
        Err(SkateboardGroundFrameError::MissingPredictor)
    ));
    assert_ne!(
        world.backend().collision_body_pool().frame_identity(),
        frame
    );
    assert!(world.contact_spies().unwrap().is_none());
    assert!(world.prediction_owner().is_none());
}
