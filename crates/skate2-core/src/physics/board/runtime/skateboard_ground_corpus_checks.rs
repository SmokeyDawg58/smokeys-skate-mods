//! Actual mounted authored board -> semantic-XYZ Create/refresh -> resident
//! Rob's Park terrain -> real C/J/D and integration. Controlled terrain
//! placement, cvar snapshots and contact capacity are harness inputs, not the
//! native spawn location or full flags3 pair scheduler. Actual `SkaterPool`
//! membership owns the board and the publication consumed by ground frames.
use super::*;
use crate::{
    dynamic_assembly_session::{
        hook_pool::SimulationHookAssemblyHandle,
        skateboard_publication::{
            SkateboardCollisionPublication, SkateboardCollisionPublicationError,
        },
    },
    simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameInput,
    simulation_contact_jacobian::ground_preparation::corpus::mounted_robs_park_ground_for_board_test,
    skateboard_spawn::SkateboardSpawnServices,
};
use skate2_formats::collision::{AssemblyPartMaterial, Volume};

const TERRAIN_POINT: [f32; 3] = [588.8617, -54.431, 996.5048];
const TIMESTEP: f32 = f32::from_bits(0x3c88_8889);
const BEND_THRESHOLD: f32 = f32::from_bits(0x3f7f_be77);

struct ControlledTolerance;
impl SkateboardSpawnServices for ControlledTolerance {
    fn current_orthogonal_tolerance(&mut self) -> f32 {
        0.01
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("controlled identity terrain placement must pass source orthogonality assertion");
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one retained mounted publication/terrain/solver lifetime with adjacent ownership checks"
)]
pub(super) fn check(
    world: &mut SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>,
    board: &SimulationSkaterAssemblyHandle,
    hook: &SimulationHookAssemblyHandle,
) -> [f32; 16] {
    let template = Arc::clone(world.board(board).unwrap().template());
    let definition = Arc::clone(template.collision());
    // This extra owner is only a diagnostic refusal discriminator. It never
    // supplies the runtime ground frame; that producer is retained in the pool.
    let mut publication = SkateboardCollisionPublication::try_create(
        world.board(board).unwrap(),
        world.backend(),
        BEND_THRESHOLD,
    )
    .unwrap();
    let original_bodies = publication.rigid_bodies().to_vec();
    let occurrences: Vec<_> = world
        .backend()
        .rigid_body_snapshot()
        .slots
        .iter()
        .map(|slot| (slot.id, slot.occurrence))
        .collect();
    assert_eq!(original_bodies.len(), 7);
    assert_eq!(
        original_bodies
            .iter()
            .map(|body| body.identity().id().index())
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6, 7]
    );
    assert_eq!(
        publication.body_pool_authority(),
        world.backend().authority()
    );
    assert_eq!(
        publication.bend_threshold().to_bits(),
        BEND_THRESHOLD.to_bits()
    );
    assert_eq!(publication.group(), 4);
    assert_eq!(publication.num_static(), 0);
    assert!(!publication.all_parts_frozen());
    assert!(std::ptr::eq(
        publication.culling_table(),
        &raw const definition.culling
    ));
    assert_eq!(
        publication.culling_table().words,
        [0xffff_ffff, 0x0001_ffff]
    );
    assert_eq!(
        (
            publication.culling_table().rows,
            publication.culling_table().columns
        ),
        (7, 7)
    );
    assert!(Arc::ptr_eq(
        definition.wheels.parts()[0].volume(),
        definition.wheels.parts()[3].volume()
    ));
    assert!(Arc::ptr_eq(
        definition.trucks.parts()[0].volume(),
        definition.trucks.parts()[1].volume()
    ));
    check_records(&publication, world.board(board).unwrap(), world.backend());
    let immutable: Vec<_> = publication
        .records()
        .iter()
        .map(|record| {
            (
                record.rigid_body_index,
                record.volume_type,
                record.volume.clone(),
                record.padding.to_bits(),
                record.surface_id,
            )
        })
        .collect();

    // Refusals do not use fake handles/arenas. They deliberately invalidate
    // freshness while retaining the old fields; refresh the real owner again.
    let frame = world
        .backend()
        .rigid_body_pool_for_test()
        .frame_identity()
        .unwrap();
    let before_records = publication.records().to_vec();
    let before_bounds = publication.bounds();
    let foreign = SkateboardSimulationBackend::try_new(0, 0, 0).unwrap();
    assert_eq!(
        publication.try_refresh(world.board(board).unwrap(), &foreign),
        Err(SkateboardCollisionPublicationError::ForeignPool)
    );
    assert!(!publication.has_bounds_for_frame(frame));
    assert_eq!(publication.records(), before_records);
    assert_eq!(publication.bounds(), before_bounds);
    let authored_joints = template
        .joints()
        .iter()
        .map(
            |joint| crate::dynamic_assembly_session::DynamicAssemblyJointDefinition {
                parent_part: joint.parent_part,
                child_part: joint.child_part,
                frames: Arc::clone(&joint.frames),
                limits: Arc::clone(
                    joint
                        .limits
                        .as_immutable()
                        .expect("board-only template retains immutable authored limits"),
                ),
            },
        )
        .collect();
    let equal_definition_template =
        try_bind_skateboard_assembly(Arc::clone(&definition), authored_joints).unwrap();
    let distinct_session = SkateboardAssemblySession::try_new(
        equal_definition_template,
        world
            .board(board)
            .unwrap()
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .copied()
            .collect(),
    )
    .unwrap();
    assert_eq!(
        publication.try_refresh(&distinct_session, world.backend()),
        Err(SkateboardCollisionPublicationError::DefinitionChanged)
    );
    assert_eq!(publication.records(), before_records);
    publication
        .try_refresh(world.board(board).unwrap(), world.backend())
        .unwrap();
    world.backend_mut().begin_dynamic_update_frame(TIMESTEP);
    let next_frame = world
        .backend()
        .rigid_body_pool_for_test()
        .frame_identity()
        .unwrap();
    assert_ne!(next_frame, frame);
    assert!(!publication.has_bounds_for_frame(next_frame));
    assert!(
        !world
            .board_publication(board)
            .unwrap()
            .has_bounds_for_frame(next_frame)
    );
    publication
        .try_refresh(world.board(board).unwrap(), world.backend())
        .unwrap();
    assert!(publication.has_bounds_for_frame(next_frame));
    world.try_refresh_boards().unwrap();
    assert!(
        world
            .board_publication(board)
            .unwrap()
            .has_bounds_for_frame(next_frame)
    );
    drop(publication);

    // Determine placement from the actual wheel pose AFTER prior solver and
    // relocation work. No wheel radius or deck-to-wheel offset is substituted.
    let ground_pool = mounted_robs_park_ground_for_board_test();
    let mut requested = FRESH_VOLUME_TRANSFORM;
    requested[12..15].copy_from_slice(&TERRAIN_POINT);
    world
        .try_set_registered_board_transform(board, hook, requested, &mut ControlledTolerance)
        .unwrap();
    let live_wheel = world
        .board(board)
        .unwrap()
        .try_live_part_transform_host(world.backend(), 0)
        .unwrap();
    let wheel = definition.wheels.parts()[0].volume();
    assert_eq!(wheel.volume_type(), Some(VolumeType::Sphere));
    assert_eq!(
        wheel.transform.map(f32::to_bits),
        FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
    );
    let target_center = [
        TERRAIN_POINT[0],
        TERRAIN_POINT[1] + wheel.radius * 0.5,
        TERRAIN_POINT[2],
    ];
    for axis in 0..3 {
        requested[12 + axis] += target_center[axis] - live_wheel[12 + axis];
    }
    world
        .try_set_registered_board_transform(board, hook, requested, &mut ControlledTolerance)
        .unwrap();
    let placed = world
        .board(board)
        .unwrap()
        .try_live_part_transform_host(world.backend(), 0)
        .unwrap();
    for axis in 0..3 {
        assert!((placed[12 + axis] - target_center[axis]).abs() < 0.000_25);
    }
    world.try_refresh_boards().unwrap();
    let publication = world.board_publication(board).unwrap();
    assert_ne!(
        publication.bounds(),
        before_bounds,
        "actual live relocation reaches swept Header bounds"
    );
    assert_eq!(publication.rigid_bodies(), original_bodies);
    check_records(publication, world.board(board).unwrap(), world.backend());
    let before = world.backend().rigid_body_snapshot();
    let joints_before = world.backend().joint_snapshot();
    let drives_before = world.backend().drive_snapshot();
    let mut total_contacts = 0;
    let mut frames_with_contacts = 0;
    for tick in 0..4 {
        let (counts, report) = world
            .try_step_authored_ground(
                SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                    ground_pool: &ground_pool,
                    ground_group: 0,
                    pipeline_culling: None,
                    allow_multiple_writes: true,
                    spy_flags: 4,
                },
                TIMESTEP,
                50,
                &mut crate::simulation_assembly_backend::constraint_session::contact_feedback::ControlledNoFreezing,
            )
            .unwrap();
        assert_eq!((counts.joints, counts.drives), (6, 3));
        // The real solved batch now reaches the source spy exporter. Actual
        // Island registration is still absent here: no body spy bit is set,
        // so even positive solved contacts correctly produce an empty list.
        assert!(world.contact_spies().unwrap().unwrap().records().is_empty());
        assert_eq!(report.processed_assemblies, 1);
        if tick == 0 {
            assert!(
                counts.contacts > 0,
                "controlled real wheel/terrain penetration generates contacts: {report:?}"
            );
            assert!(
                report.visited_meshes > 0
                    && report.submitted_triangles > 0
                    && report.intersection_runs > 0
            );
        }
        total_contacts += counts.contacts;
        frames_with_contacts += usize::from(counts.contacts != 0);
        assert!(world.active_rf_is_zero_for_test());
        let bodies = world.backend().rigid_body_snapshot();
        assert_eq!(
            bodies
                .slots
                .iter()
                .map(|slot| (slot.id, slot.occurrence))
                .collect::<Vec<_>>(),
            occurrences
        );
        assert_eq!((bodies.counts.active, bodies.counts.static_bodies), (7, 2));
        for body in &bodies.slots[1..8] {
            assert!(
                body.center_of_mass
                    .into_iter()
                    .chain(body.quaternion)
                    .chain(body.velocity)
                    .chain(body.omega)
                    .all(f32::is_finite)
            );
        }
        assert_eq!(
            bodies.slots[8], before.slots[8],
            "static hook is neither integrated nor recreated"
        );
        assert_eq!(world.backend().joint_snapshot(), joints_before);
        assert_eq!(world.backend().drive_snapshot(), drives_before);
        assert_eq!(world.board_count(), 1);
        assert_eq!(world.board_world_index(board).unwrap(), 0);
        assert!(Arc::ptr_eq(
            world.board(board).unwrap().template(),
            &template
        ));
        assert!(Arc::ptr_eq(
            world.board(board).unwrap().template().collision(),
            &definition
        ));
        let publication = world.board_publication(board).unwrap();
        assert_eq!(publication.rigid_bodies(), original_bodies);
        assert_eq!(
            publication
                .records()
                .iter()
                .map(|record| (
                    record.rigid_body_index,
                    record.volume_type,
                    record.volume.clone(),
                    record.padding.to_bits(),
                    record.surface_id,
                ))
                .collect::<Vec<_>>(),
            immutable
        );
        // Frame's Header is the pre-integration collision input. Recompute
        // from the now-live bodies before comparing output geometry; an epoch
        // alone is not a promise against same-frame movement after publication.
        let collision_bounds = publication.bounds();
        world.try_refresh_boards().unwrap();
        let publication = world.board_publication(board).unwrap();
        check_records(publication, world.board(board).unwrap(), world.backend());
        println!(
            "mounted authored19 ground tick={tick} C/J/D={}/{}/{} report={report:?} deck={:?} collision_bbox={collision_bounds:?} refreshed_bbox={:?}; real resident terrain/owners, controlled pair-free schedule, no native Tick claim",
            counts.contacts,
            counts.joints,
            counts.drives,
            bodies.slots[7].center_of_mass,
            publication.bounds()
        );
    }
    assert!(total_contacts > 0);
    assert!(
        frames_with_contacts >= 2,
        "contacts recur without resetting or replacing the retained board"
    );
    let final_bodies = world.backend().rigid_body_snapshot();
    assert!(
        final_bodies.slots[1..8]
            .iter()
            .zip(&before.slots[1..8])
            .any(|(after, before)| {
                after.center_of_mass.map(f32::to_bits) != before.center_of_mass.map(f32::to_bits)
            })
    );
    assert_eq!(
        world.try_hook_snapshot(hook).unwrap().world_index,
        1,
        "actual board then hook publication consumes shared manager IDs 0 and 1"
    );
    requested
}

fn check_records(
    publication: &SkateboardCollisionPublication,
    board: &SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
) {
    let definition = board.template().collision();
    let mut sources: Vec<(usize, &Volume)> = definition
        .wheels
        .parts()
        .iter()
        .enumerate()
        .map(|(part, wheel)| (part, wheel.volume().as_ref()))
        .collect();
    sources.extend(
        definition
            .trucks
            .parts()
            .iter()
            .enumerate()
            .map(|(part, truck)| (part + 4, truck.volume().as_ref())),
    );
    sources.extend(
        definition
            .deck
            .aggregate()
            .children()
            .iter()
            .map(|volume| (6, volume)),
    );
    assert_eq!(publication.records().len(), 19);
    assert_eq!(sources.len(), 19);
    for (ordinal, (record, (part, source))) in publication.records().iter().zip(sources).enumerate()
    {
        let expected_type = match ordinal {
            0..=3 => VolumeType::Sphere,
            4..=5 | 7..=8 => VolumeType::Capsule,
            6 => VolumeType::Box,
            _ => VolumeType::Triangle,
        };
        assert_eq!(record.volume_type, expected_type);
        assert_eq!(usize::from(record.rigid_body_index), part);
        assert_eq!(&record.volume, source);
        assert_eq!(record.padding.to_bits(), 0x3d4c_cccd);
        assert_eq!(record.surface_id, source.surface_id);
        assert_eq!(record.enabled, source.flags & 1 != 0);
        assert_eq!(record.material, *board.current_part_material(part).unwrap());
        assert_eq!(record.group, 4);
        assert!(!record.frozen);
        assert_eq!(record.volume.group_id, if ordinal < 4 { 5 } else { 0 });
        let live = board.try_live_part_transform_host(backend, part).unwrap();
        // This authored route proves geometric XYZ, not guest packed-W words.
        for column in 0..4 {
            for lane in 0..3 {
                assert_eq!(
                    record.transform[column * 4 + lane].to_bits(),
                    live[column * 4 + lane].to_bits()
                );
            }
        }
    }
}
