//! Controlled original-terrain support placement through actual `SetTransform`.
//! No contact flags, material words or velocities are injected. The actual drag
//! setter selects zero drag after earlier diagnostics left strong braking drag.
use super::{PhysicalAssemblyWorld, wheel_lines_corpus::OriginalPostCvars};
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    dynamic_assembly_session::{
        hook_pool::SimulationHookAssemblyHandle, skater_pool::SimulationSkaterAssemblyHandle,
    },
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        contact_feedback::ControlledNoFreezing, skateboard_ground::SkateboardGroundFrameInput,
    },
    skateboard_spawn::SkateboardSpawnServices,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use skate2_formats::collision::VolumeType;

struct Placement;
impl SkateboardSpawnServices for Placement {
    fn current_orthogonal_tolerance(&mut self) -> f32 {
        0.01
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("support fixture requested a nonorthogonal actual deck pose");
    }
}

/// Same actual pair after the preceding fixtures; deliberately selected schedule.
/// Failure is evidence that this support placement did not produce wheel contact.
#[expect(
    clippy::too_many_lines,
    reason = "one geometry-derived placement and actual solver-to-Up proof"
)]
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    hook: &SimulationHookAssemblyHandle,
    processed: &SkeletonProcessedContact,
    reckoning: &Reckoning<ReckoningVaultAttributes>,
    ground: &StaticGroundPool,
) {
    let frames: [_; 5] = [0, 1, 2, 3, 6].map(|part| {
        world
            .board(board)
            .unwrap()
            .try_live_part_transform_host(world.backend(), part)
            .unwrap()
    });
    let radii: [f32; 4] = std::array::from_fn(|part| {
        let volume = world
            .board(board)
            .unwrap()
            .template()
            .collision()
            .wheels
            .parts()[part]
            .volume();
        assert_eq!(
            volume.volume_type(),
            Some(VolumeType::Sphere),
            "support fixture currently proves original sphere wheels only"
        );
        assert!(volume.radius.is_finite() && volume.radius > 0.0);
        volume.radius
    });
    // Small, explicit fixture penetration, proportional to each REAL radius.
    // Maximum required lift supports one wheel while placing the others above
    // their measured planes. These queries inspect the mounted original world.
    let mut lift = f32::NEG_INFINITY;
    for part in 0..4 {
        let centre = [frames[part][12], frames[part][13], frames[part][14]];
        let point = |y| SkeletonRootVector {
            xyz: [centre[0], y, centre[2]],
            w: None,
        };
        let hit = world
            .try_world_line_test(
                ground,
                CollisionLineInput {
                    start: point(centre[1] + 2.0),
                    end: point(centre[1] - 2.0),
                    fatness: 0.0,
                },
                "original wheel support geometry",
                &mut HostCollisionLineNumeric,
            )
            .unwrap()
            .unwrap_or_else(|| panic!("no original terrain below wheel {part} at {centre:?}"));
        let normal_y = hit.normal.xyz[1];
        assert!(
            normal_y > 0.9,
            "support fixture requires upward original terrain: {:?}",
            hit.normal
        );
        let desired_y = hit.position.xyz[1] + radii[part] * 0.9 / normal_y;
        let candidate = desired_y - centre[1];
        assert!(candidate.is_finite());
        lift = lift.max(candidate);
        println!(
            "wheel support {part}: centre={centre:?}, radius={}, terrain={:?}, normal={:?}, requested lift={candidate}",
            radii[part], hit.position, hit.normal
        );
    }
    let mut requested = frames[4];
    requested[13] += lift;
    world
        .try_set_registered_board_transform(board, hook, requested, &mut Placement)
        .unwrap();
    // Explicit diagnostic free-motion setting through the actual seven-Part
    // source setter. This is not a runtime default or live control decision.
    world.try_set_board_linear_drag(board, 0.0).unwrap();
    let up = reckoning.up();
    world
        .try_start_board_wheel_lines(board, ground, up, &mut HostCollisionLineNumeric)
        .unwrap();
    let timestep = f32::from_bits(0x3d08_8889);
    let (counts, report) = world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            timestep,
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    world
        .try_end_board_wheel_lines(board, ground, &mut HostCollisionLineNumeric)
        .unwrap();
    let raw_parts: Vec<_> = world
        .try_get_physical_collisions(board)
        .unwrap()
        .iter()
        .map(crate::simulation_assembly_backend::constraint_session::physical_contact_list::PhysicalContactInfo::part)
        .collect();
    println!(
        "original support solve: lift={lift}, contacts={}, board parts={raw_parts:?}, report={report:?}",
        counts.contacts
    );
    assert!(
        raw_parts.iter().any(|part| *part < 4),
        "geometry-derived placement produced no actual wheel contact; retain this diagnostic failure"
    );
    let outcome = world
        .try_update_board_post_ground_normals(
            board,
            timestep,
            up,
            &mut OriginalPostCvars,
            &mut HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
        .unwrap();
    let wheel_count = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .post_fields()
        .unwrap()
        .num_wheels;
    assert!(
        wheel_count > 0,
        "actual contact selection produced zero wheels: {outcome:?}"
    );
    world
        .try_finish_board_postphysics(
            board,
            &mut crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
        )
        .unwrap();
    world
        .try_finish_board_ordinary_up_post(board, skeleton, processed, &mut HostReckoningNumeric)
        .unwrap();
    world.try_with_board_force_runtime(board, |runtime| {
        let contact = runtime.contact_surface.lock().unwrap();
        let state = runtime.up_state.lock().unwrap();
        assert_eq!(state.average_normal, contact.collision_state.average_normal);
        assert!(contact.up_post_completed.is_some());
        assert_eq!(runtime.queue.len(), 0);
        println!("actual nonzero-wheel Up endpoint: wheels={wheel_count}, dynamic_up={:?}, speed={:?}, average={:?}; same prior surface-speed provenance={:?}; selected ordinary schedule only", state.dynamic_up, state.dynamic_speed, state.average_normal, processed.prior_output().surface_speed());
    }).unwrap();
}
