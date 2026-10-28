//! Source-producing synthetic skeletons in the actual shared member pool.
//! These are ownership/failure fixtures, not original tuning or native ticks.
use super::*;
use crate::{
    dynamic_assembly_session::skeleton_normal::NORMAL_COLLISION_ENABLED,
    skeleton_body_attributes::{SkeletonBodyAttributeRoute, SkeletonBodyDebugRead},
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_culling::{SkeletonCullingInventory, SkeletonCullingKey},
    skeleton_runtime_state::tests::CollisionReader,
};

fn skeleton() -> SkeletonNormalAssembly {
    let (body, joints) = crate::skeleton_assembly::tests::fixture();
    let normal = SkeletonCullingInventory::default()
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    SkeletonNormalAssembly::try_create(
        body,
        joints,
        normal,
        SkeletonCollisionData::try_new(&mut CollisionReader).unwrap(),
    )
    .unwrap()
}

fn candidate() -> SimulationSkeletonAssemblyCandidate {
    SimulationSkeletonAssemblyCandidate::try_new(skeleton()).unwrap()
}

fn domain(
    bodies: usize,
    joints: usize,
) -> (SkateboardSimulationBackend, SimulationManagerContactDomain) {
    let mut backend = SkateboardSimulationBackend::try_new(bodies, joints, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    (backend, manager)
}

struct Reader {
    result: Result<u8, &'static str>,
    calls: usize,
}
impl SkeletonBodyDebugRead for Reader {
    type Error = &'static str;
    fn mode(&mut self) -> Result<u32, Self::Error> {
        panic!("normal setup has no debug read")
    }
    fn factor(&mut self) -> Result<f32, Self::Error> {
        panic!("normal setup has no debug read")
    }
}
impl SkeletonBodyAttributeRead for Reader {
    type Error = &'static str;
    type DebugInstance = Self;
    fn float(&mut self, _: SkeletonBodyAttributeRoute, _: u64) -> Result<f32, Self::Error> {
        panic!("normal setup has no scalar attribute read")
    }
    fn byte(&mut self, route: SkeletonBodyAttributeRoute, key: u64) -> Result<u8, Self::Error> {
        assert_eq!(route, SkeletonBodyAttributeRoute::LocalSkeleton);
        assert_eq!(key, NORMAL_COLLISION_ENABLED);
        self.calls += 1;
        self.result
    }
    fn debug_instance(&mut self) -> Result<Self, Self::Error> {
        panic!("no debug instance")
    }
}

#[test]
fn skeleton_members_swap_remove_complete_owner_and_never_recycle_world_identity() {
    let (mut backend, mut manager) = domain(49, 44);
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 0, false).unwrap();
    let first = pool
        .try_insert_skeleton(&mut backend, &mut manager, candidate(), 0.5)
        .unwrap();
    let last_candidate = candidate();
    let binding = Arc::clone(last_candidate.owner().session().template().collision());
    let last = pool
        .try_insert_skeleton(&mut backend, &mut manager, last_candidate, 0.5)
        .unwrap();
    assert_eq!(pool.len(), 2);
    assert_eq!(pool.board_count(), 0);
    assert_eq!(pool.world_index(&first).unwrap(), 0);
    assert_eq!(pool.world_index(&last).unwrap(), 1);
    let observations = pool
        .skeleton_publication(&last)
        .unwrap()
        .rigid_bodies()
        .to_vec();
    let full = pool
        .try_insert_skeleton(&mut backend, &mut manager, candidate(), 0.5)
        .unwrap_err();
    assert!(matches!(
        full.error,
        SimulationSkaterPoolError::Full { capacity: 2 }
    ));
    assert_eq!(full.candidate.world_index(), 0xffff);
    assert!(!full.candidate.owner().session().is_attached());
    let removed = pool.try_remove_skeleton(&mut backend, &first).unwrap();
    assert_eq!(removed.world_index(), 0);
    assert!(!removed.owner().session().is_attached());
    assert!(Arc::ptr_eq(
        pool.skeleton(&last)
            .unwrap()
            .session()
            .template()
            .collision(),
        &binding
    ));
    assert_eq!(
        pool.skeleton_publication(&last).unwrap().rigid_bodies(),
        observations
    );
    assert!(Arc::ptr_eq(&pool.handles().next().unwrap().0, &last.0));
    assert!(matches!(
        pool.skeleton(&first),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    let replacement = pool
        .try_insert_skeleton(&mut backend, &mut manager, full.candidate, 0.5)
        .unwrap();
    assert_eq!(pool.world_index(&replacement).unwrap(), 2);
    assert_eq!(pool.world_index(&last).unwrap(), 1);
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.active_ordinals(), Some([0, 1].as_slice()));
    assert!(
        pool.publications()
            .all(|entry| matches!(entry, AuthoredCollisionPublication::Skeleton(_)))
    );
    pool.try_remove_skeleton(&mut backend, &last).unwrap();
    pool.try_remove_skeleton(&mut backend, &replacement)
        .unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.free, 48);
    assert_eq!(backend.rigid_body_snapshot().counts.static_bodies, 1);
    assert_eq!(backend.joint_snapshot().counts.free, 44);
}

#[test]
fn skeleton_wrong_board_kind_and_foreign_backend_do_not_mutate_live_owner() {
    let (mut backend, mut manager) = domain(25, 22);
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 0, false).unwrap();
    let handle = pool
        .try_insert_skeleton(&mut backend, &mut manager, candidate(), 0.5)
        .unwrap();
    pool.try_refresh(&backend).unwrap();
    let before = backend.rigid_body_snapshot();
    let before_joints = backend.joint_snapshot();
    let frame = pool.active_frame();
    assert!(matches!(
        pool.session(&handle),
        Err(SimulationSkaterPoolError::WrongMemberKind)
    ));
    assert!(matches!(
        pool.publication(&handle),
        Err(SimulationSkaterPoolError::WrongMemberKind)
    ));
    assert!(matches!(
        pool.try_remove(&mut backend, &handle),
        Err(SimulationSkaterPoolError::WrongMemberKind)
    ));
    let (mut foreign, _) = domain(25, 22);
    assert!(matches!(
        pool.try_remove_skeleton(&mut foreign, &handle),
        Err(SimulationSkaterPoolError::ForeignBackend)
    ));
    assert_eq!(pool.active_frame(), frame);
    assert_eq!(pool.len(), 1);
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(backend.joint_snapshot(), before_joints);
    assert!(pool.skeleton(&handle).unwrap().session().is_attached());
    pool.try_remove_skeleton(&mut backend, &handle).unwrap();
}

#[test]
fn failed_normal_invalidates_same_frame_publications_before_retained_partial_writes() {
    let (mut backend, mut manager) = domain(49, 44);
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 0, false).unwrap();
    let first = pool
        .try_insert_skeleton(&mut backend, &mut manager, candidate(), 0.5)
        .unwrap();
    let second = pool
        .try_insert_skeleton(&mut backend, &mut manager, candidate(), 0.5)
        .unwrap();
    pool.try_set_skeleton_pre_normal_properties(&mut backend, &first)
        .unwrap();
    pool.try_refresh(&backend).unwrap();
    let frame = backend.collision_body_pool().frame_identity().unwrap();
    assert!(
        pool.publications()
            .all(|publication| publication.has_bounds_for_frame(frame))
    );
    let before_bodies = backend.rigid_body_snapshot();
    let before_joints = backend.joint_snapshot();
    let mut reader = Reader {
        result: Ok(1),
        calls: 0,
    };
    let mut calls = 0;
    let failed = pool.try_set_skeleton_normal(&mut backend, &first, &mut reader, &mut |_| {
        calls += 1;
        if calls == 1 {
            Ok(7.0_f32)
        } else {
            Err("second cosine")
        }
    });
    assert!(matches!(
        failed,
        Err(SimulationSkeletonMutationError::Setup(_))
    ));
    assert_eq!(calls, 2);
    assert_eq!(reader.calls, 0);
    assert_eq!(backend.collision_body_pool().frame_identity(), Some(frame));
    assert_eq!(pool.active_frame(), None);
    assert!(
        pool.publications()
            .all(|publication| !publication.has_bounds_for_frame(frame))
    );
    assert_eq!(backend.rigid_body_snapshot(), before_bodies);
    assert_eq!(backend.joint_snapshot(), before_joints);
    assert_eq!(
        pool.skeleton(&first)
            .unwrap()
            .session()
            .template()
            .collision()
            .joint_limits()[0]
            .try_read()
            .unwrap()
            .swing_c
            .to_bits(),
        7.0_f32.to_bits()
    );
    let empty_ground = crate::static_ground_pool::StaticGroundPool::try_new(0).unwrap();
    assert!(matches!(run_ground(&mut backend, &pool, &manager, &empty_ground),
        Err(crate::simulation_contact_jacobian::ground_preparation::cpu_job::SimulationStaticGroundJobError::Run {
            assembly: 0,
            source: crate::simulation_contact_jacobian::ground_preparation::SimulationGroundRunError::StaleAssemblyBounds,
        })));
    // Explicit refresh is necessary even within the unchanged body epoch.
    pool.try_refresh(&backend).unwrap();
    assert!(
        pool.publications()
            .all(|publication| publication.has_bounds_for_frame(frame))
    );
    let job = run_ground(&mut backend, &pool, &manager, &empty_ground).unwrap();
    assert_eq!(job.report.processed_assemblies, 2);
    assert_eq!(job.report.intersection_runs, 2); // one empty final Run per member
    assert!(job.into_records().is_empty());
    pool.try_remove_skeleton(&mut backend, &first).unwrap();
    pool.try_remove_skeleton(&mut backend, &second).unwrap();
}

#[test]
fn foreign_manager_refuses_before_id_and_failed_body_capacity_retains_consumed_id() {
    let (mut backend, mut manager) = domain(25, 22);
    let (_, mut foreign_manager) = domain(1, 0);
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 0, false).unwrap();
    let rejected = pool
        .try_insert_skeleton(&mut backend, &mut foreign_manager, candidate(), 0.5)
        .unwrap_err();
    assert!(matches!(
        rejected.error,
        SimulationSkaterPoolError::ForeignManager
    ));
    assert_eq!(rejected.candidate.world_index(), 0xffff);
    assert!(!rejected.candidate.owner().session().is_attached());
    let first = pool
        .try_insert_skeleton(&mut backend, &mut manager, rejected.candidate, 0.5)
        .unwrap();
    assert_eq!(pool.world_index(&first).unwrap(), 0);
    let before = backend.rigid_body_snapshot();
    let failed = pool
        .try_insert_skeleton(&mut backend, &mut manager, candidate(), 0.5)
        .unwrap_err();
    assert!(matches!(failed.error, SimulationSkaterPoolError::Attach(_)));
    assert_eq!(failed.candidate.world_index(), 1);
    assert!(!failed.candidate.owner().session().is_attached());
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(pool.len(), 1);
    pool.try_remove_skeleton(&mut backend, &first).unwrap();
    let retry = pool
        .try_insert_skeleton(&mut backend, &mut manager, failed.candidate, 0.5)
        .unwrap();
    assert_eq!(pool.world_index(&retry).unwrap(), 2);
    pool.try_remove_skeleton(&mut backend, &retry).unwrap();
}

#[test]
fn pool_placement_invalidates_all_same_frame_headers_without_reallocating_bodies() {
    let (mut backend, mut manager) = domain(49, 44);
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 0, false).unwrap();
    let first = pool
        .try_insert_skeleton(&mut backend, &mut manager, candidate(), 0.5)
        .unwrap();
    let second = pool
        .try_insert_skeleton(&mut backend, &mut manager, candidate(), 0.5)
        .unwrap();
    pool.try_refresh(&backend).unwrap();
    let frame = backend.collision_body_pool().frame_identity().unwrap();
    let observations: Vec<_> = pool
        .publications()
        .map(|publication| publication.rigid_bodies().to_vec())
        .collect();
    let joints = backend.joint_snapshot();
    let initial: Vec<_> = pool
        .skeleton(&first)
        .unwrap()
        .session()
        .part_transforms()
        .map(|matrix| matrix.expect("known fixture Part cache"))
        .map(|matrix| matrix.map(f32::to_bits))
        .collect();
    let (mut foreign, _) = domain(1, 0);
    assert!(matches!(
        pool.try_place_skeleton_parts(&mut foreign, &first),
        Err(SimulationSkaterPoolError::ForeignBackend)
    ));
    assert!(
        pool.publications()
            .all(|publication| publication.has_bounds_for_frame(frame))
    );
    pool.try_place_skeleton_parts(&mut backend, &first).unwrap();
    assert_eq!(backend.collision_body_pool().frame_identity(), Some(frame));
    assert_eq!(pool.active_frame(), None);
    assert!(
        pool.publications()
            .all(|publication| !publication.has_bounds_for_frame(frame))
    );
    assert_ne!(
        pool.skeleton(&first)
            .unwrap()
            .session()
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .next()
            .unwrap()
            .map(f32::to_bits),
        initial[0]
    );
    assert_eq!(backend.joint_snapshot(), joints);
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.active_frame(), Some(frame));
    for (publication, original) in pool.publications().zip(&observations) {
        assert!(publication.has_bounds_for_frame(frame));
        assert_eq!(publication.rigid_bodies(), original);
    }
    pool.try_remove_skeleton(&mut backend, &first).unwrap();
    pool.try_remove_skeleton(&mut backend, &second).unwrap();
}

fn run_ground(
    backend: &mut SkateboardSimulationBackend,
    pool: &SimulationSkaterAssemblyPool,
    manager: &SimulationManagerContactDomain,
    ground: &crate::static_ground_pool::StaticGroundPool,
) -> Result<
    crate::simulation_contact_jacobian::ground_preparation::cpu_job::SimulationStaticGroundJobOutput,
    crate::simulation_contact_jacobian::ground_preparation::cpu_job::SimulationStaticGroundJobError,
>{
    crate::simulation_assembly_backend::constraint_session::skateboard_pool::tests::run_pool_ground_job(
        backend, pool, manager, ground,
    )
}

/// Called by the existing original board corpus: no additional ignored test.
/// Geometry, mass, joints and Vault readers are real original-data producers.
/// Harness settings include board translation, bend threshold, timestep,
/// ground settings/contact capacity and zero solver iterations. Skeleton Parts
/// use recovered full Set with the retained original pose fixture and actual
/// shared-pool hooks/drives. Controlled cvars1/10 are not a live global service;
/// this does not prove humanoid terrain contact, native spawn or pair scheduling.
#[expect(
    clippy::too_many_lines,
    reason = "one original mixed-owner admission/ground/removal lifecycle"
)]
pub(crate) fn check_mounted_board_and_skeleton(
    template: &Arc<crate::skateboard_assembly::SkateboardRuntimeTemplate>,
    initial: &[[f32; 16]; 7],
) {
    use crate::{
        simulation_assembly_backend::constraint_session::{
            SimulationAssemblyConstraintSession, skateboard_ground::SkateboardGroundFrameInput,
        },
        skateboard_hook::SkateboardHookDefinition,
    };
    crate::skeleton_body_definition_corpus::with_mounted_original_normal_skeleton(
        |skeleton, reader| {
            let skeleton_binding = Arc::clone(skeleton.session().template().collision());
            let wheel = template.collision().wheels.parts()[0].volume();
            assert_eq!(
                wheel.volume_type(),
                Some(skate2_formats::collision::VolumeType::Sphere)
            );
            assert!(
                template.collision().wheels.parts()[0]
                    .inverse_body_transform()
                    .is_none()
            );
            let target = [588.8617, -54.431 + wheel.radius * 0.5, 996.5048];
            let offset: [f32; 3] = std::array::from_fn(|axis| target[axis] - initial[0][12 + axis]);
            let mut translated = *initial;
            for matrix in &mut translated {
                for axis in 0..3 {
                    matrix[12 + axis] += offset[axis];
                }
            }
            let board =
                SkateboardAssemblySession::try_new(Arc::clone(template), translated.to_vec())
                    .unwrap();
            board.board_collision_groups.try_initialize(7).unwrap();
            let mut world = SimulationAssemblyConstraintSession::<
                AssemblyPartMaterial,
                SkateboardHookDefinition,
            >::try_new(35, 28, 47, 256)
            .unwrap();
            world.try_initialize_skater_pool(3, false).unwrap();
            let board_handle = world
                .try_insert_board(
                    SimulationSkaterAssemblyCandidate::try_new(board).unwrap(),
                    4,
                    0.5,
                )
                .unwrap();
            let skeleton_handle = world
                .try_insert_skeleton(
                    SimulationSkeletonAssemblyCandidate::try_new(skeleton).unwrap(),
                    0.5,
                )
                .unwrap();
            assert_eq!(world.board_world_index(&board_handle).unwrap(), 0);
            // World index is common membership even for a non-board handle.
            assert_eq!(world.skater_world_index(&skeleton_handle).unwrap(), 1);
            assert_eq!(world.board_count(), 1);
            assert!(Arc::ptr_eq(
                world.board(&board_handle).unwrap().template(),
                template
            ));
            assert!(Arc::ptr_eq(
                world
                    .skeleton(&skeleton_handle)
                    .unwrap()
                    .session()
                    .template()
                    .collision(),
                &skeleton_binding
            ));
            assert!(matches!(
                world.board(&skeleton_handle),
                Err(SimulationSkaterPoolError::WrongMemberKind)
            ));
            assert!(matches!(
                world.skeleton(&board_handle),
                Err(SimulationSkaterPoolError::WrongMemberKind)
            ));
            let before = world.backend().rigid_body_snapshot();
            assert!(matches!(
                world.try_remove_board(&skeleton_handle),
                Err(SimulationSkaterPoolError::WrongMemberKind)
            ));
            assert!(matches!(
                world.try_remove_skeleton(&board_handle),
                Err(SimulationSkaterPoolError::WrongMemberKind)
            ));
            assert_eq!(world.backend().rigid_body_snapshot(), before);
            let before_boards = world
                .board_publication(&board_handle)
                .unwrap()
                .rigid_bodies()
                .to_vec();
            let before_skeletons = world
                .skeleton_publication(&skeleton_handle)
                .unwrap()
                .rigid_bodies()
                .to_vec();
            assert_eq!(before_boards.len(), 7);
            assert_eq!(before_skeletons.len(), 24);
            world
                .try_set_skeleton_pre_normal_properties(&skeleton_handle)
                .unwrap();
            world
                .try_set_skeleton_normal(&skeleton_handle, reader, &mut |angle| {
                    Ok::<_, std::convert::Infallible>(angle.cos())
                })
                .unwrap();
            world.try_refresh_skaters().unwrap();
            let frame = world
                .backend()
                .rigid_body_pool_for_test()
                .frame_identity()
                .unwrap();
            assert!(
                world
                    .skeleton_publication(&skeleton_handle)
                    .unwrap()
                    .has_bounds_for_frame(frame)
            );
            let drive_check = crate::dynamic_assembly_session::skeleton_normal::drive_owner::tests::initialize_mounted_drives(
                &mut world, &board_handle, &skeleton_handle,
            );
            assert_eq!(
                world.backend().rigid_body_pool_for_test().frame_identity(),
                Some(frame)
            );
            assert!(
                !world
                    .skeleton_publication(&skeleton_handle)
                    .unwrap()
                    .has_bounds_for_frame(frame)
            );
            assert!(
                !world
                    .board_publication(&board_handle)
                    .unwrap()
                    .has_bounds_for_frame(frame)
            );
            world.try_refresh_skaters().unwrap();
            let board_view =
                AuthoredCollisionPublication::from(world.board_publication(&board_handle).unwrap());
            let skeleton_view = AuthoredCollisionPublication::from(
                world.skeleton_publication(&skeleton_handle).unwrap(),
            );
            assert_eq!(board_view.group(), 4);
            assert_eq!(skeleton_view.group(), 6);
            assert_eq!(board_view.culling_table().rows, 7);
            assert_eq!(skeleton_view.culling_table().rows, 24);
            assert_eq!(board_view.records().len(), 19);
            assert_eq!(skeleton_view.records().len(), 24);
            assert_eq!(
                board_view.body_pool_authority(),
                skeleton_view.body_pool_authority()
            );
            let ground = crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_robs_park_ground_for_board_test();
            let (counts, report) = world
                .try_step_authored_ground(
                    SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                        ground_pool: &ground,
                        ground_group: 0,
                        pipeline_culling: None,
                        allow_multiple_writes: true,
                        spy_flags: 4,
                    },
                    f32::from_bits(0x3c88_8889),
                    0,
                    &mut crate::simulation_assembly_backend::constraint_session::contact_feedback::ControlledNoFreezing,
                )
                .unwrap();
            assert_eq!(report.processed_assemblies, 2);
            // The zero-iteration mixed preparation cannot emit positive solved
            // contact spies; the retained frame is nevertheless current/owned.
            assert!(world.contact_spies().unwrap().unwrap().records().is_empty());
            assert_eq!(report.skipped_assemblies, 0);
            assert!(report.visited_meshes > 0 && report.submitted_triangles > 0);
            assert!(
                counts.contacts > 0,
                "actual controlled wheel/terrain placement: {report:?}"
            );
            assert_eq!((counts.joints, counts.drives), (28, 47));
            assert!(world.active_rf_is_zero_for_test());
            assert_eq!(
                world
                    .board_publication(&board_handle)
                    .unwrap()
                    .rigid_bodies(),
                before_boards
            );
            assert_eq!(
                world
                    .skeleton_publication(&skeleton_handle)
                    .unwrap()
                    .rigid_bodies(),
                before_skeletons
            );
            crate::dynamic_assembly_session::skeleton_normal::drive_owner::tests::finish_mounted_drives(
                &mut world, &board_handle, &skeleton_handle, &drive_check,
            );
            let removed_board = world.try_remove_board(&board_handle).unwrap();
            assert!(!removed_board.session().is_attached());
            assert_eq!(world.board_count(), 0);
            assert!(Arc::ptr_eq(
                world
                    .skeleton(&skeleton_handle)
                    .unwrap()
                    .session()
                    .template()
                    .collision(),
                &skeleton_binding
            ));
            assert_eq!(world.skater_world_index(&skeleton_handle).unwrap(), 1);
            assert_eq!(
                world
                    .skeleton_publication(&skeleton_handle)
                    .unwrap()
                    .rigid_bodies(),
                before_skeletons
            );
            world.try_refresh_skaters().unwrap();
            let removed_skeleton = world.try_remove_skeleton(&skeleton_handle).unwrap();
            assert_eq!(removed_skeleton.world_index(), 1);
            assert!(!removed_skeleton.owner().session().is_attached());
            let returned_owner = removed_skeleton
                .into_owner()
                .unwrap_or_else(|_| panic!("legacy candidate has no construction owner"));
            assert!(Arc::ptr_eq(
                returned_owner.session().template().collision(),
                &skeleton_binding
            ));
            assert_eq!(world.backend().rigid_body_snapshot().counts.free, 34);
            assert_eq!(world.backend().joint_snapshot().counts.free, 28);
        },
    );
}
