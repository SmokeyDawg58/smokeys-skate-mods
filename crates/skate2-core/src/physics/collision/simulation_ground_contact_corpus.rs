//! Opt-in real DMO/Rob's Park contact-path regression, not a native spawn trace.
//!
//! Assets remain external. The body is deliberately placed to penetrate the
//! known flat Rob's Park surface; its geometry, inertia and material are read
//! unchanged from the retained DMO bank. Address-image bases and the island
//! assignment are controlled harness inputs, not captured Xenon addresses.

use super::super::{
    HostContactNumericBackend,
    network_preparation::{
        SimulationContactIslandMapSource, SimulationManagerContactDomain,
        try_prepare_network_contact_island_batch_with_backend,
    },
};
use super::*;
use crate::{
    decoded_arena_asset::DecodedArenaAsset,
    serialised_assembly_create::{
        SerialisedAssemblyCreatePartInput, SerialisedAssemblyLiveAssemblyInput,
        SerialisedAssemblyParamsState,
        pool_update::{SerialisedAssemblyPoolUpdateInput, try_update_serialised_assembly_pool},
        try_prepare_serialised_assembly_create_from_xenon,
    },
    serialised_assembly_template::SerialisedAssemblyTemplate,
    simulation_assembly_ground_triangle_rip::AssemblyGroundTriangleRipper,
    simulation_collision_asset::SimulationCollisionAsset,
    simulation_drive_jacobian::prepare_drive_horus_island_batch_for_test,
    simulation_drive_pool::SimulationDrivePool,
    simulation_horus_pipeline::try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite,
    simulation_joint_pool::SimulationJointPool,
    simulation_joint_preparation::prepare_joint_horus_island_batch_for_test,
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_rigid_body_pool::SimulationRigidBodyPool,
    simulation_xenon_address_image::{
        SimulationArenaXenonAddressBinding, SimulationRigidBodyAddInertiaInput,
        SimulationRigidBodyAddInertiaOwner, SimulationXenonAddressImage,
    },
    static_ground_pool::STATIC_GROUND_IDENTITY_TRANSFORM,
    world::{compose_collision_affine, transform_collision_aabb},
};
use skate2_formats::{
    collision::{AssemblyPartMaterial, KdLeafRange},
    mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
    rw4::Arena,
    stream::{AssetToc, CollectionMap, StreamFile, decode_arena},
};
use std::{path::Path, sync::Arc};

const IDENTITY: [f32; 16] = STATIC_GROUND_IDENTITY_TRANSFORM;
const DMO_BASE: &str = "data\\content\\world\\dmo\\DMO\\DMO_Pres";
const TERRAIN_BASE: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim";
const TERRAIN_POINT: [f32; 3] = [588.8617, -54.431, 996.5048];

#[expect(
    clippy::too_many_lines,
    reason = "independent original geometry oracle and retained World query reuse"
)]
fn assert_ground_line_query(ground: &crate::static_ground_pool::StaticGroundPool) {
    use crate::{
        collision_line_collection::CollisionLineInput,
        collision_line_numeric::HostCollisionLineNumeric,
        skeleton_root_transforms::SkeletonRootVector,
    };
    use crate::{
        collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig},
        simulation_assembly_backend::constraint_session::{
            SimulationAssemblyConstraintSession,
            world_line_query::{
                SimulationWorldLineQueryConfig, corpus::assert_original_world_line,
            },
        },
    };

    let start = [TERRAIN_POINT[0], TERRAIN_POINT[1] + 2.0, TERRAIN_POINT[2]];
    let end = [TERRAIN_POINT[0], TERRAIN_POINT[1] - 2.0, TERRAIN_POINT[2]];
    let result = ground
        .try_run_line_test(
            CollisionLineInput {
                start: SkeletonRootVector::known([start[0], start[1], start[2], 3.0]),
                end: SkeletonRootVector::known([end[0], end[1], end[2], 7.0]),
                fatness: 0.0,
            },
            &mut HostCollisionLineNumeric,
        )
        .unwrap();
    // Independent existing geometric projection on this well-conditioned
    // original flat surface. Its simplified numerical graph is not the new
    // line implementation and does not establish native runtime trace parity.
    let mesh = ground.assembly_ground_meshes().next().unwrap().mesh;
    let reference = mesh
        .clustered_mesh
        .thin_segment_intersections(&mesh.object_bytes, start, end)
        .unwrap();
    let expected = reference
        .iter()
        .min_by(|a, b| {
            a.intersection
                .line_parameter
                .total_cmp(&b.intersection.line_parameter)
        })
        .unwrap();
    assert!(result.hit_count > 0);
    assert_eq!(result.surface_id, expected.triangle.surface_id.unwrap());
    assert_eq!(result.surface_id, 133);
    for axis in 0..3 {
        assert!(
            (result.position.xyz[axis] - expected.intersection.position[axis]).abs() < 0.000_02
        );
    }
    assert!((result.line_parameter - expected.intersection.line_parameter).abs() < 0.000_002);
    assert!(result.line_parameter > 0.0 && result.line_parameter < 1.0);
    assert!(
        expected.intersection.line_parameter > 0.0 && expected.intersection.line_parameter < 1.0
    );
    // The authored surface is slightly below the nominal probe centre, so
    // this intersection is not exactly halfway along the four-lane segment.
    assert_eq!(
        result.position.w.unwrap().to_bits(),
        4.0_f32.mul_add(result.line_parameter, 3.0).to_bits()
    );
    assert!(
        (result.position.w.unwrap() - 4.0_f32.mul_add(expected.intersection.line_parameter, 3.0))
            .abs()
            < 0.000_01
    );
    assert_eq!(result.owner, None);
    let provenance = result.provenance.unwrap();
    assert_eq!(provenance.occurrence_slot, 0);
    assert_eq!(provenance.cluster_index, 18);
    assert_eq!(provenance.unit_byte_offset, 351);
    println!(
        "original ground line count={} parameter={:08x} position={:08x?} normal={:08x?} provenance={provenance:?}",
        result.hit_count,
        result.line_parameter.to_bits(),
        result.position.xyz.map(f32::to_bits),
        result.normal.xyz.map(f32::to_bits)
    );
    // Dedicated query diagnostic with actual constructor-owned empty dynamic
    // pools. Batch counts are explicit small harness configuration, not a
    // reconstructed cvar bootstrap or a native BeginUpdate schedule.
    let family = CollisionQueryBatchCapacity {
        batches: 1,
        tests: 1,
    };
    let batches = CollisionQueryBatchConfig {
        line: family,
        boxes: family,
        trajectory: family,
        step_selector: family,
    };
    let mut query_world =
        SimulationAssemblyConstraintSession::<AssemblyPartMaterial>::try_new(1, 0, 0, 0).unwrap();
    query_world
        .try_initialize_world_line_queries(
            SimulationWorldLineQueryConfig {
                vehicle_update_in_parallel: false,
                ground: batches,
                dmo: batches,
                vehicle: batches,
            },
            false,
        )
        .unwrap();
    assert_original_world_line(
        &mut query_world,
        ground,
        CollisionLineInput {
            start: SkeletonRootVector::known([start[0], start[1], start[2], 3.0]),
            end: SkeletonRootVector::known([end[0], end[1], end[2], 7.0]),
            fatness: 0.0,
        },
    );
}

fn byte_signature(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

pub(crate) fn mounted_corpus() -> MountSet {
    let root = std::env::var("SKATE2_GAME_ROOT")
        .expect("set SKATE2_GAME_ROOT to the external original game-data directory");
    let mut mounts = MountSet::new();
    for archive in &SKATE2_ARCHIVES {
        let path = archive.path_under(Path::new(&root));
        if path.is_file() {
            mounts.push(MountedBig::open(path, archive.use_external_header).unwrap());
        }
    }
    mounts
}

/// Same original resident terrain producer reused by the authored board test.
/// No DMO assembly, inertia bridge or packed body image is constructed here.
pub(crate) fn mounted_robs_park_ground_for_board_test()
-> crate::static_ground_pool::StaticGroundPool {
    let mounts = mounted_corpus();
    let input_bytes = mounts.read_path("data\\config\\input.cfg").unwrap();
    assert_eq!(
        (input_bytes.len(), byte_signature(&input_bytes)),
        (10_121, 0x9922_ceaf_8912_81d6)
    );
    crate::input_map::tests::check_original(std::str::from_utf8(&input_bytes).unwrap());
    crate::input_pad::tests::check_original_map(std::str::from_utf8(&input_bytes).unwrap());
    crate::game_input_original_corpus::check(std::str::from_utf8(&input_bytes).unwrap());
    let patterns: Vec<_> = [
        "skater",
        "skater90",
        "skaterN90",
        "skater_air",
        "skater_fingerflip",
        "skaterls",
        "skaterstep",
    ]
    .into_iter()
    .map(|name| {
        mounts
            .read_path(&format!("data\\joystick\\{name}.pat"))
            .unwrap()
    })
    .collect();
    crate::input_pattern::tests::check_original(&patterns);
    let terrain = load_collection(&mounts, TERRAIN_BASE, "cSim_500_900_high", 21);
    let mesh_owner = &terrain[13];
    assert_eq!(
        mesh_owner.static_plan().asset().decoded.asset_id,
        0xa114_c754_0da1_8d0e
    );
    let mesh = &mesh_owner.static_plan().asset().collision.meshes[0];
    assert_eq!(byte_signature(&mesh.object_bytes), 0xa463_b422_29b8_fb97);
    let source = crate::static_ground_pool::StaticEnvironmentSource::from_operation(
        mesh_owner.static_plan_owner(),
        0,
    )
    .unwrap();
    assert_eq!(source.guid(), 0x8062_9fce_1595_539c);
    let mut ground = crate::static_ground_pool::StaticGroundPool::try_new(128).unwrap();
    ground.add(&source).unwrap();
    ground
}

fn load_collection(
    mounts: &MountSet,
    base: &str,
    name: &str,
    expected_assets: u32,
) -> Vec<Arc<SimulationNormalArenaPlan>> {
    let toc = AssetToc::parse(&mounts.read_path(&format!("{base}.xst")).unwrap()).unwrap();
    let map = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm")).unwrap()).unwrap();
    let collection = map
        .collections
        .iter()
        .find(|entry| entry.filename == name)
        .expect("frozen collection name exists");
    assert_eq!(collection.asset_count, expected_assets);
    let directory = base.rsplit_once('\\').unwrap().0;
    let path = format!("{directory}\\{name}.xsf");
    let bytes = mounts.read_path(&path).unwrap();
    let expected_signature = match name {
        "cPres_Global" => 0x3644_7f74_e467_5ca0,
        "cSim_500_900_high" => 0xaec6_0e47_9f36_d040,
        _ => panic!("unfrozen corpus collection"),
    };
    assert_eq!(byte_signature(&bytes), expected_signature);
    let stream = StreamFile::parse(&bytes, collection.asset_count).unwrap();
    println!(
        "real collection={path} id={:016x} assets={} bytes={} fnv1a64={:016x}",
        collection.id,
        stream.assets.len(),
        bytes.len(),
        byte_signature(&bytes)
    );
    stream
        .assets
        .iter()
        .map(|block| {
            let metadata = toc
                .assets
                .iter()
                .find(|asset| asset.id == block.id)
                .unwrap();
            let resources = decode_arena(&bytes[block.data_range.clone()]).unwrap();
            let arena = Arena::parse(&resources.resources).unwrap();
            Arc::new(
                SimulationNormalArenaPlan::from_collision_asset(
                    SimulationCollisionAsset::from_decoded(DecodedArenaAsset {
                        asset_id: block.id,
                        asset_type: metadata.asset_type,
                        asset_resource_descriptors: metadata.resources,
                        resources,
                        arena,
                    })
                    .unwrap(),
                )
                .unwrap(),
            )
        })
        .collect()
}

/// Native `BoxTest` 0x827cdaf0..0x827cdb64 writes and reloads three u16
/// descriptor fields. The generic KD API deliberately does not narrow them.
fn native_descriptor(range: KdLeafRange, max_clusters: u32) -> KdLeafRange {
    let shift = if max_clusters <= 0x1_0000 { 16 } else { 20 };
    let mask = (1_u32 << shift) - 1;
    let offset = (range.encoded_start & mask) & 0xffff;
    let cluster = (range.encoded_start >> shift) & 0xffff;
    KdLeafRange {
        encoded_start: (cluster << shift) | offset,
        unit_count: range.unit_count & 0xffff,
    }
}

fn movable_sphere_template(plans: &[Arc<SimulationNormalArenaPlan>]) -> SerialisedAssemblyTemplate {
    let templates = plans
        .iter()
        .flat_map(|owner| SerialisedAssemblyTemplate::all_in_plan(owner).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(templates.len(), 93);
    let mut candidates = templates
        .into_iter()
        .filter(|template| {
            template.parts().len() == 1
                && template.parts()[0].is_static_body == 0
                && template.assembly().joint_count == 0
                && template.assembly().drive_count == 0
                && template.volume_sources().iter().any(|source| {
                    source.volume_type == VolumeType::Sphere && source.volume.radius > 0.0
                })
                && template.volume_sources().iter().all(|source| {
                    matches!(
                        source.volume_type,
                        VolumeType::Aggregate | VolumeType::Sphere
                    )
                })
        })
        .collect::<Vec<_>>();
    // A small harness selection, not the game's DMO prioritization/spawn rule.
    candidates.sort_by_key(|template| template.volume_sources().len());
    assert!(
        !candidates.is_empty(),
        "real movable primitive/sphere template exists"
    );
    candidates.remove(0)
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT; real-data host arithmetic, not a Xenon tick oracle"]
#[allow(
    clippy::too_many_lines,
    reason = "one retained real-data producer-to-solver lifetime"
)]
fn real_dmo_primitive_and_robs_park_terrain_compile_solve_and_integrate() {
    let mounts = mounted_corpus();
    let dmo = load_collection(&mounts, DMO_BASE, "cPres_Global", 125);
    let terrain = load_collection(&mounts, TERRAIN_BASE, "cSim_500_900_high", 21);
    let template = movable_sphere_template(&dmo);
    let owner = template.owner();
    assert_eq!(
        owner.static_plan().asset().decoded.asset_id,
        0xd309_6f13_cf1c_0aab
    );
    assert_eq!(template.assembly_object_id().0, 0x219);
    assert_eq!(template.definition_object_id().0, 0x217);
    assert_eq!(template.volume_sources().len(), 2);
    assert_eq!(
        template.volume_sources()[0].volume_type,
        VolumeType::Aggregate
    );
    assert_eq!(template.volume_sources()[1].volume_type, VolumeType::Sphere);
    let mesh_owner = &terrain[13];
    let mesh = &mesh_owner.static_plan().asset().collision.meshes[0];
    let static_owner = mesh_owner.static_plan_owner();
    assert_eq!(
        static_owner.operations()[0],
        crate::simulation_static_arena_plan::SimulationStaticOperation::Environment {
            source_toc_index: 14,
            source_dictionary_index: 1,
            source_inline_index: 0,
            source_resource_index: 0,
            source_record_start: 576,
            instance_index: 0,
        }
    );
    let collision = &static_owner.asset().collision;
    assert_eq!(collision.instances[0].collision_model, Some(0));
    assert_eq!(collision.collision_models[0].dictionary_index, 4);
    assert_eq!(collision.collision_models[0].collision_roots, [0]);
    assert_eq!(collision.aggregate_roots[0].dictionary_index, 2);
    assert_eq!(
        collision.aggregate_roots[0].aggregate,
        Some(crate::world::CollisionAggregateRef::Mesh(0))
    );
    let environment = crate::static_ground_pool::StaticEnvironmentSource::from_operation(
        Arc::clone(&static_owner),
        0,
    )
    .unwrap();
    assert_eq!(environment.guid(), 0x8062_9fce_1595_539c);
    assert_eq!(environment.local_guid(), u64::MAX);
    assert_eq!(environment.root_count(), 1);
    println!(
        "real terrain Environment operation0={:?} guid={:016x} model0/dictionary4 root0/dictionary2 mesh0/dictionary3",
        static_owner.operations()[0],
        environment.guid()
    );
    assert_eq!(
        mesh_owner.static_plan().asset().decoded.asset_id,
        0xa114_c754_0da1_8d0e
    );
    assert_eq!(mesh.dictionary_index, 3);
    assert_eq!(byte_signature(&mesh.object_bytes), 0xa463_b422_29b8_fb97);
    let vertical_hits = mesh
        .clustered_mesh
        .thin_segment_intersections(
            &mesh.object_bytes,
            [TERRAIN_POINT[0], TERRAIN_POINT[1] + 2.0, TERRAIN_POINT[2]],
            [TERRAIN_POINT[0], TERRAIN_POINT[1] - 2.0, TERRAIN_POINT[2]],
        )
        .unwrap();
    assert!(vertical_hits.iter().any(|hit| {
        hit.triangle.cluster_index == 18
            && hit.triangle.unit_stream_offset == 351
            && hit.triangle.surface_id == Some(133)
    }));
    let sphere = template
        .volume_sources()
        .iter()
        .find(|source| source.volume_type == VolumeType::Sphere)
        .unwrap();
    let part = &template.parts()[0];
    let local_sphere = if part.has_inverse_body_transform == 0 {
        sphere.volume.transform
    } else {
        compose_collision_affine(part.inverse_body_transform, sphere.volume.transform)
    };
    // Controlled contact stimulus: sphere center half a radius above the real
    // terrain, retaining the complete authored template and every source lane.
    let mut placement = IDENTITY;
    placement[12] = TERRAIN_POINT[0] - local_sphere[12];
    placement[13] = TERRAIN_POINT[1] + sphere.volume.radius * 0.5 - local_sphere[13];
    placement[14] = TERRAIN_POINT[2] - local_sphere[14];
    let mut dmo_world = crate::simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession::<AssemblyPartMaterial>::try_new(8, 0, 0, 0).unwrap();
    crate::simulation_dmo_assembly_part_session::pool_corpus::assert_original_dmo_pool(
        &mut dmo_world,
        &dmo,
        placement,
    );
    println!(
        "real DMO outer={:016x} arena={:08x} assembly={:08x} definition={:08x} volumes={} sphere_radius={:?} controlled_pose_bits={:08x?}",
        owner.static_plan().asset().decoded.asset_id,
        owner.static_plan().asset().decoded.arena.id,
        template.assembly_object_id().0,
        template.definition_object_id().0,
        template.volume_sources().len(),
        sphere.volume.radius,
        placement.map(f32::to_bits)
    );
    println!(
        "real terrain outer={:016x} arena={:08x} mesh_dictionary={} mesh_bytes={} fnv1a64={:016x}",
        mesh_owner.static_plan().asset().decoded.asset_id,
        mesh_owner.static_plan().asset().decoded.arena.id,
        mesh.dictionary_index,
        mesh.object_bytes.len(),
        byte_signature(&mesh.object_bytes)
    );

    let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
    let mut domain = SimulationManagerContactDomain::try_new(&mut pool, 256).unwrap();
    let ground = domain.static_body_observation();
    let bases = std::array::from_fn(|index| {
        if owner.static_plan().asset().decoded.resources.resources[index].is_empty() {
            0
        } else {
            0x2000_0000 + u32::try_from(index).unwrap() * 0x0100_0000
        }
    });
    let binding =
        Arc::new(SimulationArenaXenonAddressBinding::try_new(Arc::clone(owner), bases).unwrap());
    let mut inertia_owner = SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
    let inertia = template.runtime_inertia(0).unwrap();
    let body = inertia_owner
        .try_add(
            &mut pool,
            placement,
            SimulationRigidBodyAddInertiaInput::Definition(&inertia),
            4,
        )
        .unwrap()
        .unwrap();
    let observation = body.observation();
    let joint_pool = SimulationJointPool::try_new(0).unwrap();
    let drive_pool = SimulationDrivePool::try_new(0, &pool).unwrap();
    let image = SimulationPackedBodyWordImageDomain::try_new(
        &pool,
        SimulationXenonAddressImage::try_new(0x1000_0080, 2, 0, 0).unwrap(),
    )
    .unwrap();
    let parts = [SerialisedAssemblyCreatePartInput {
        body: observation,
        part_transform: placement,
        material: part.material,
        user_data: 0,
    }];
    let volume_flags = template
        .update_volume_sources()
        .iter()
        .map(|source| source.volume.flags)
        .collect::<Vec<_>>();
    let live = SerialisedAssemblyLiveAssemblyInput {
        group: template.assembly().group_id,
        culling_table: template.culling_table(),
    };
    let mut prepared = try_prepare_serialised_assembly_create_from_xenon(
        &template,
        &pool,
        &inertia_owner,
        &image,
        &parts,
        live,
        SerialisedAssemblyParamsState::native_default(),
        &mut HostSerialisedAssemblyBBoxNumeric,
    )
    .unwrap();
    assert!(Arc::ptr_eq(prepared.owner(), owner));
    let before = pool
        .try_serialised_assembly_snapshot(observation)
        .unwrap()
        .center_of_mass;
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    domain.try_start_island_frame_labels(&pool).unwrap();
    let active = try_update_serialised_assembly_pool(
        &mut [SerialisedAssemblyPoolUpdateInput {
            assembly: &mut prepared,
            template: &template,
            inertia_owner: &inertia_owner,
            parts: &parts,
            volume_flags: &volume_flags,
            live,
        }],
        &pool,
        &image,
        &mut HostSerialisedAssemblyBBoxNumeric,
    )
    .unwrap();
    assert_eq!(active, [0]);
    let map = domain.try_island_map_snapshot(&pool).unwrap();
    assert_eq!(map.assignment(observation), Some((observation, 0)));
    assert_eq!(map.assignment(ground), Some((ground, -1)));
    let phase = pool.try_prepare_dynamic_update_phase().unwrap();
    let run = SimulationGroundAssemblyRun::try_new(
        &prepared,
        0,
        &phase,
        &image,
        SimulationGroundRunSettings {
            ground: Some(ground),
            ground_group: 0,
            pipeline_culling: None,
            is_object: false,
            draw_contacts: false,
        },
    )
    .unwrap()
    .expect("authored movable primitive produces a CPU ground Run");
    let minimum = prepared.header().aabb.min_f32();
    let maximum = prepared.header().aabb.max_f32();
    let world_minimum = [minimum[0], minimum[1], minimum[2]];
    let world_maximum = [maximum[0], maximum[1], maximum[2]];
    // Static GroundPool placement is identity; still execute the recovered
    // Transform midpoint/extent arithmetic before the KD query.
    let (local_minimum, local_maximum) =
        transform_collision_aabb(world_minimum, world_maximum, IDENTITY);
    let tree = mesh.clustered_mesh.kd_tree(&mesh.object_bytes).unwrap();
    // ClusteredMeshData::Reset (0x827c8378) snapshots the KD-tree bounds,
    // not the separate aggregate/mesh header bounds.
    assert!(!(0..3).any(|axis| {
        tree.bounding_box_min[axis] > local_maximum[axis]
            || local_minimum[axis] > tree.bounding_box_max[axis]
    }));
    let ranges = tree
        .leaf_ranges_for_aabb_limited(&mesh.object_bytes, local_minimum, local_maximum, 256)
        .unwrap();
    assert!(!ranges.is_empty());
    let mut output = SimulationGroundContactBuffer::try_new(domain.maximum(), true).unwrap();
    let mut ripper = AssemblyGroundTriangleRipper::new(world_minimum, world_maximum, false, false);
    let mut batches = 0;
    let mut triangles = 0;
    let mut consume = |batch: AssemblyGroundTriangleBatch<'_, AssemblyGroundTriangle>| {
        batches += 1;
        triangles += batch.iter().count();
        assert_eq!(
            run.try_run(batch, &mut output).unwrap(),
            SimulationGroundRunOutcome::Complete
        );
    };
    for &range in &ranges {
        ripper
            .rip_leaf_range(
                &mesh.clustered_mesh,
                &mesh.object_bytes,
                native_descriptor(range, mesh.clustered_mesh.maximum_cluster_count),
                IDENTITY,
                &mut |_| {},
                &mut consume,
            )
            .unwrap();
    }
    ripper.finish(consume);
    output.try_finish_assembly().unwrap();
    let expected_records = output.try_into_records().unwrap();
    // Exercise actual source-owned publication and the production CPU job,
    // independently compared with the prior explicit one-mesh regression path.
    let mut ground_pool = crate::static_ground_pool::StaticGroundPool::try_new(128).unwrap();
    let empty_job =
        cpu_job::try_run_static_ground_cpu_job(&cpu_job::SimulationStaticGroundJobInput {
            assemblies: std::slice::from_ref(&prepared),
            ground_pool: &ground_pool,
            phase: &phase,
            image: &image,
            run_settings: SimulationGroundRunSettings {
                ground: Some(ground),
                ground_group: 0,
                pipeline_culling: None,
                is_object: false,
                draw_contacts: false,
            },
            maximum_contacts: domain.maximum(),
            allow_multiple_writes: true,
        })
        .unwrap();
    assert_eq!(empty_job.report.processed_assemblies, 1);
    assert_eq!(empty_job.report.visited_meshes, 0);
    assert_eq!(
        empty_job.report.intersection_runs, 1,
        "native final Run executes even with no meshes"
    );
    assert_eq!(empty_job.report.submitted_triangles, 0);
    assert!(empty_job.into_records().is_empty());
    ground_pool.add(&environment).unwrap();
    assert_ground_line_query(&ground_pool);
    let job = cpu_job::try_run_static_ground_cpu_job(&cpu_job::SimulationStaticGroundJobInput {
        assemblies: std::slice::from_ref(&prepared),
        ground_pool: &ground_pool,
        phase: &phase,
        image: &image,
        run_settings: SimulationGroundRunSettings {
            ground: Some(ground),
            ground_group: 0,
            pipeline_culling: None,
            is_object: false,
            draw_contacts: false,
        },
        maximum_contacts: domain.maximum(),
        allow_multiple_writes: true,
    })
    .unwrap();
    assert_eq!(job.report.processed_assemblies, 1);
    assert_eq!(job.report.visited_meshes, 1);
    assert_eq!(job.report.gathered_ranges, 1);
    assert_eq!(job.report.intersection_runs, 1);
    assert_eq!(job.report.submitted_triangles, 1);
    assert_eq!(job.report.unavailable_runs, 0);
    let records = job.into_records();
    assert_eq!(records.len(), expected_records.len());
    for (record, expected) in records.iter().zip(expected_records) {
        assert_eq!(record.input.test_words(), expected.input.test_words());
        assert_eq!(record.source, expected.source);
    }
    // Duplicate published slots are native-valid. They distinguish one cache
    // spanning meshes from an incorrect fresh tester for every mesh.
    for _ in 1..21 {
        ground_pool.add(&environment).unwrap();
    }
    let repeated_job =
        cpu_job::try_run_static_ground_cpu_job(&cpu_job::SimulationStaticGroundJobInput {
            assemblies: std::slice::from_ref(&prepared),
            ground_pool: &ground_pool,
            phase: &phase,
            image: &image,
            run_settings: SimulationGroundRunSettings {
                ground: Some(ground),
                ground_group: 0,
                pipeline_culling: None,
                is_object: false,
                draw_contacts: false,
            },
            maximum_contacts: domain.maximum(),
            allow_multiple_writes: false,
        })
        .unwrap();
    assert_eq!(repeated_job.report.visited_meshes, 21);
    assert_eq!(repeated_job.report.gathered_ranges, 21);
    assert_eq!(repeated_job.report.intersection_runs, 2);
    assert_eq!(repeated_job.report.submitted_triangles, 21);
    assert_eq!(repeated_job.report.unavailable_runs, 0);
    let repeated_records = repeated_job.into_records();
    assert_eq!(repeated_records.len(), 21);
    assert!(
        repeated_records
            .iter()
            .all(|record| record.input.test_words() == records[0].input.test_words())
    );
    assert!(
        !records.is_empty(),
        "real source geometry generated ground contacts"
    );
    assert!(records.iter().all(|record| {
        record.table_a == observation
            && record.rf_a == observation
            && record.table_b == ground
            && record.rf_b == ground
    }));
    assert_eq!(
        (ranges.len(), batches, triangles, records.len()),
        (1, 1, 1, 1)
    );
    assert_eq!(records[0].source.volume_ordinal, 1);
    assert_eq!(
        records[0].source.target.triangle().unit_stream_offset(),
        351
    );
    let contact_bytes = records[0]
        .input
        .test_words()
        .into_iter()
        .flatten()
        .flat_map(u32::to_be_bytes)
        .collect::<Vec<_>>();
    // Frozen host regression only: this is not a captured console Contact.
    assert_eq!(byte_signature(&contact_bytes), 0x0e7c_704e_6a10_f08a);
    println!(
        "real contact_source={:?} words_fnv1a64={:016x} authored_inertia={:?} authored_material={:?}",
        records[0].source,
        byte_signature(&contact_bytes),
        part.inertia,
        part.material
    );
    println!(
        "real query ranges={} triangle_batches={batches} triangles={triangles} contacts={} bbox={world_minimum:?}..{world_maximum:?}",
        ranges.len(),
        records.len()
    );
    let mut open = domain.try_begin_contact_frame(&phase).unwrap();
    open.try_append_ground_contact_records(&records, &phase)
        .unwrap();
    assert_eq!(open.len(), records.len());
    let sealed = open
        .try_seal(&phase)
        .unwrap_or_else(|_| panic!("seal real ground frame"));
    let mut contacts = try_prepare_network_contact_island_batch_with_backend(
        sealed,
        &phase,
        SimulationContactIslandMapSource::Present(&map),
        &mut HostContactNumericBackend,
    )
    .unwrap_or_else(|_| panic!("compile real ground contacts"));
    let mut joints = prepare_joint_horus_island_batch_for_test(&joint_pool, &phase, &map);
    let mut drives = prepare_drive_horus_island_batch_for_test(&drive_pool, &phase, &map);
    try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite(
        &mut contacts,
        &mut joints,
        &mut drives,
        1,
        4,
        &drive_pool,
        phase,
    )
    .unwrap();
    let after = pool
        .try_serialised_assembly_snapshot(observation)
        .unwrap()
        .center_of_mass;
    assert!(after.iter().all(|value| value.is_finite()));
    assert!(
        after[1] > before[1],
        "real penetrating primitive receives upward correction: {before:?} -> {after:?}"
    );
    pool.try_with_solver_delta_row(observation, |rows| assert_eq!(*rows, [[0.0; 4]; 4]))
        .unwrap();
    println!(
        "real host solve center_before={before:?} center_after={after:?}; ordinary-finite host estimates, controlled placement/island/image, no native-tick parity claim"
    );

    // Keep the same actual arena, body occurrence, manager and image across
    // subsequent frames. Remove only the twenty duplicate stress registrations.
    for _ in 1..21 {
        ground_pool.remove(&environment);
    }
    let first_bounds = prepared.header().aabb;
    let mut subsequent_contacts = 0;
    for tick in 2..=8 {
        pool.begin_dynamic_update_frame(f32::from_bits(0x3c88_8889));
        domain.try_start_island_frame_labels(&pool).unwrap();
        assert!(!prepared.has_bounds_for_frame(pool.frame_identity().unwrap()));
        if tick == 2 {
            let stale_phase = pool.try_prepare_dynamic_update_phase().unwrap();
            assert!(matches!(
                SimulationGroundAssemblyRun::try_new(
                    &prepared,
                    0,
                    &stale_phase,
                    &image,
                    SimulationGroundRunSettings {
                        ground: Some(ground),
                        ground_group: 0,
                        pipeline_culling: None,
                        is_object: false,
                        draw_contacts: false,
                    },
                ),
                Err(SimulationGroundRunError::StaleAssemblyBounds)
            ));
        }
        let active = try_update_serialised_assembly_pool(
            &mut [SerialisedAssemblyPoolUpdateInput {
                assembly: &mut prepared,
                template: &template,
                inertia_owner: &inertia_owner,
                parts: &parts,
                volume_flags: &volume_flags,
                live,
            }],
            &pool,
            &image,
            &mut HostSerialisedAssemblyBBoxNumeric,
        )
        .unwrap();
        assert_eq!(active, [0]);
        assert!(prepared.has_bounds_for_frame(pool.frame_identity().unwrap()));
        if tick == 2 {
            assert_ne!(
                prepared.header().aabb,
                first_bounds,
                "integrated live motion reaches published Header bounds"
            );
        }
        let map = domain.try_island_map_snapshot(&pool).unwrap();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let job =
            cpu_job::try_run_static_ground_cpu_job(&cpu_job::SimulationStaticGroundJobInput {
                assemblies: std::slice::from_ref(&prepared),
                ground_pool: &ground_pool,
                phase: &phase,
                image: &image,
                run_settings: SimulationGroundRunSettings {
                    ground: Some(ground),
                    ground_group: 0,
                    pipeline_culling: None,
                    is_object: false,
                    draw_contacts: false,
                },
                maximum_contacts: domain.maximum(),
                allow_multiple_writes: false,
            })
            .unwrap();
        assert_eq!(job.report.visited_meshes, 1);
        let records = job.into_records();
        subsequent_contacts += records.len();
        let mut open = domain.try_begin_contact_frame(&phase).unwrap();
        open.try_append_ground_contact_records(&records, &phase)
            .unwrap();
        let sealed = open
            .try_seal(&phase)
            .unwrap_or_else(|_| panic!("seal repeated real ground frame"));
        let mut contacts = try_prepare_network_contact_island_batch_with_backend(
            sealed,
            &phase,
            SimulationContactIslandMapSource::Present(&map),
            &mut HostContactNumericBackend,
        )
        .unwrap_or_else(|_| panic!("compile repeated real ground frame"));
        let mut joints = prepare_joint_horus_island_batch_for_test(&joint_pool, &phase, &map);
        let mut drives = prepare_drive_horus_island_batch_for_test(&drive_pool, &phase, &map);
        try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite(
            &mut contacts,
            &mut joints,
            &mut drives,
            1,
            4,
            &drive_pool,
            phase,
        )
        .unwrap();
        let pose = pool
            .try_serialised_assembly_snapshot(observation)
            .unwrap()
            .center_of_mass;
        assert!(pose.iter().all(|value| value.is_finite()));
        pool.try_with_solver_delta_row(observation, |rows| assert_eq!(*rows, [[0.0; 4]; 4]))
            .unwrap();
        println!(
            "real retained frame={tick} contacts={} center={pose:?} bounds={:?}",
            records.len(),
            prepared.header().aabb
        );
    }
    assert!(
        subsequent_contacts > 0,
        "real ground producer still feeds later frames"
    );
    // The original one-contact oracle's library gravity and four iterations
    // remain controlled test settings, not recovered World/Island defaults.
}
