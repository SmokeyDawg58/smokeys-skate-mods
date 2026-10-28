//! The skater DMO collision pipeline, `SkaterDMOCollisionJob` 0x827cf140.
//!
//! One job per skater assembly (A) runs against every DMO pool member (B):
//! an assembly AABB test, the pipeline culling table at assembly groups, then
//! `RunIntersectionTests` 0x827cef98. Only B's aggregate volumes collide: each
//! is taken as a ClusteredMesh (`GetClusteredMesh` 0x827ce4e8, forward
//! transform B's first volume transform, `ClusteredMeshData::Reset`'s
//! inverse), `BoxTest<TriangleCache,1>` 0x827ced60 rips every triangle of the
//! KD ranges A's AABB overlaps (`spew` is 1, so no per-triangle AABB gate), and
//! `RunIntersectionTests2` 0x827ce688 builds Contacts against B's own body in
//! place of ground. The job's ContactCache holds 0x50 Contacts and stores them
//! when the job ends.
//!
//! A pair that wrote a Contact against an all-frozen DMO pushes that DMO onto
//! the thaw list `Step_Solver2`'s `ThawAsNecessary` activates. Not reproduced:
//! native job order across skater assemblies is not deterministic. Admitted
//! assembly pairs feed the shared manager partitioning before the solve.

use super::super::{
    ContactBodyToken, SimulationAssemblyGroundContactInput, SimulationAssemblyGroundMaterialInput,
    SolverDeltaRowToken, build_assembly_ground_contact_input_with_material,
};
use super::{
    CONVEXITY_EPSILON, GENERATOR_PADDING, GroundAssemblyPublication, HostGroundContactNumeric,
    HostGroundPadding, SimulationGroundAssemblyRun, SimulationGroundContactRecord,
    SimulationGroundContactSource, SimulationGroundRunError, SimulationGroundRunSettings,
    binding_word, common_body, cpu_job::native_ground_descriptor, dmo_dmo::aabbs_separated,
    infallible, is_culled, material_input, triangle_overlaps,
};
use crate::{
    authored_collision_publication::AuthoredCollisionPublication,
    dynamic_assembly_session::character_publication::CharacterCollisionPublication,
    dynamic_clustered_mesh_template::{
        DynamicClusteredMeshTemplateError, resolve_aggregate_target,
    },
    narrow_phase::{GpInstanceTags, GpPrimitive, try_sk8_compute_contact_points},
    serialised_assembly_create::PreparedSerialisedAssembly,
    simulation_assembly_ground_run::try_assembly_ground_effective_padding,
    simulation_assembly_ground_triangle_cache::AssemblyGroundTriangleBatch,
    simulation_assembly_ground_triangle_rip::{
        AssemblyGroundTriangleRipError, AssemblyGroundTriangleRipper,
    },
    simulation_assembly_ground_triangle_topology::AssemblyGroundTriangle,
    simulation_rigid_body_pool::{
        SimulationRigidBodyDynamicUpdatePhase, SimulationRigidBodyObservation,
    },
    world::{CollisionAggregateRef, inverse_collision_affine_reset, transform_collision_aabb},
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable, CollisionError, VolumeType};
use std::fmt;

/// `ContactCache` capacity: `RunIntersectionTests2` 0x827ce914 stops
/// allocating at 0x50 Contacts.
const CONTACT_CACHE_CAPACITY: usize = 0x50;
/// `RunIntersectionTests` 0x827cefec..f00c ConstantData: the generator
/// padding (0.5) and convexity (0.01) of the ground Run, with its own bend
/// threshold `__real_3f7fbe77`.
const SKATER_DMO_BEND_THRESHOLD: f32 = f32::from_bits(0x3f7f_be77);
/// `SMALL_FLOAT` 0x8210af28 handed to `Contact::IsValid` at 0x827cebb0.
const CONTACT_VALID_TOLERANCE: f32 = f32::from_bits(0x3780_0000);
/// `BoxTest<TriangleCache,1>` stops gathering KD ranges at 0x100.
const MESH_RANGE_CAPACITY: usize = 0x100;
/// `SetForceWord`'s Active state written to a frozen B's side-1 word
/// (`RunIntersectionTests2` 0x827ceba8).
const ACTIVE_STATE_WORD: u32 = 4;

#[derive(Debug)]
pub(crate) enum SimulationSkaterDmoJobError {
    Run {
        assembly: usize,
        source: SimulationGroundRunError,
    },
    StaleDmoBounds {
        dmo: usize,
    },
    MissingMeshArena {
        dmo: usize,
        part: u16,
    },
    DmoBodyIndex {
        dmo: usize,
        volume: usize,
        index: u16,
    },
    Aggregate {
        dmo: usize,
        source: DynamicClusteredMeshTemplateError,
    },
    /// `GetClusteredMesh` reads the aggregate as a ClusteredMesh directly.
    NotClusteredMesh {
        dmo: usize,
        volume: usize,
    },
    Mesh {
        dmo: usize,
        source: CollisionError,
    },
    Rip {
        dmo: usize,
        source: AssemblyGroundTriangleRipError,
    },
}

impl fmt::Display for SimulationSkaterDmoJobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "skater DMO job: {self:?}")
    }
}

impl std::error::Error for SimulationSkaterDmoJobError {}

/// InitWorld827b42c0 installs the SAME SkaterDMOCollisionJob for skater-
/// vehicle and skater-DMO pipelines. Only the retained geometry owner differs.
/// This borrowed adapter supplies no new body, mesh or publication storage.
#[derive(Clone, Copy)]
pub(crate) enum MeshCollisionTarget<'a> {
    Source(&'a PreparedSerialisedAssembly),
    Character(&'a CharacterCollisionPublication),
}
impl<'a> MeshCollisionTarget<'a> {
    fn records(
        self,
    ) -> &'a [crate::serialised_assembly_template::SerialisedAssemblyVolumeDataState] {
        match self {
            Self::Source(value) => value.volume_data().records(),
            Self::Character(value) => value.records(),
        }
    }
    fn bodies(self) -> &'a [SimulationRigidBodyObservation] {
        match self {
            Self::Source(value) => value.rigid_bodies(),
            Self::Character(value) => value.rigid_bodies(),
        }
    }
    fn bounds(self) -> crate::octree::AabbWords {
        match self {
            Self::Source(value) => value.header().aabb,
            Self::Character(value) => value.bounds(),
        }
    }
    fn group(self) -> u32 {
        match self {
            Self::Source(value) => value.header().group,
            Self::Character(value) => value.group(),
        }
    }
    fn frozen(self) -> bool {
        match self {
            Self::Source(value) => value.header().all_parts_frozen,
            Self::Character(value) => value.all_parts_frozen(),
        }
    }
    fn has_bounds_for_frame(
        self,
        frame: crate::simulation_rigid_body_pool::SimulationRigidBodyFrameIdentity,
    ) -> bool {
        match self {
            Self::Source(value) => value.has_bounds_for_frame(frame),
            Self::Character(value) => value.has_bounds_for_frame(frame),
        }
    }
    fn arena(self, part: u16) -> Option<&'a crate::world::CollisionArena> {
        match self {
            Self::Source(value) => Some(&value.owner().static_plan().asset().collision),
            Self::Character(value) => value.part_mesh_arena(usize::from(part)),
        }
    }
}

/// `RunIntersectionTests2`'s GroundData for one B volume: its body and group.
/// The material words it also carries are never read.
#[derive(Clone, Copy)]
struct DmoGround {
    body: SimulationRigidBodyObservation,
    group: u32,
}

/// `RunIntersectionTests2` 0x827ce7b8..7f4: only A's material, restitution
/// through `fsel(res, 0, res)` and both frictions clamped at zero (`fcmpu` +
/// `ble`, so unordered selects zero).
fn skater_dmo_material(material: AssemblyPartMaterial) -> SimulationAssemblyGroundMaterialInput {
    let positive = |value: f32| if value > 0.0 { value } else { 0.0 };
    SimulationAssemblyGroundMaterialInput {
        static_friction: positive(material.static_friction),
        dynamic_friction: positive(material.dynamic_friction),
        restitution: if material.restitution >= 0.0 {
            0.0
        } else {
            material.restitution
        },
    }
}

impl SimulationGroundAssemblyRun<'_, '_> {
    /// `RunIntersectionTests2` 0x827ce688 for one DMO triangle against every
    /// skater volume. Unlike the ground Run it tests neither frozen nor
    /// static skater bodies. `Ok(true)` once any Contact slot was written,
    /// even one `Contact::IsValid` later releases.
    fn try_skater_dmo_triangle(
        &self,
        triangle: &AssemblyGroundTriangle,
        ground: DmoGround,
        ground_frozen: bool,
        cache: &mut Vec<SimulationGroundContactRecord>,
    ) -> Result<bool, SimulationGroundRunError> {
        let mut found = false;
        for (ordinal, volume) in self.volumes.iter().enumerate() {
            if volume.volume_type == VolumeType::Aggregate {
                continue;
            }
            if !volume.enabled && self.volumes.len() != 1 {
                continue;
            }
            if is_culled(self.pipeline_culling, volume.group, ground.group)? {
                continue;
            }
            let material = skater_dmo_material(volume.material);
            let extra = self.extra[ordinal]
                .ok_or(SimulationGroundRunError::MissingExtra { volume: ordinal })?;
            if !triangle_overlaps(triangle.primitive().points, extra.bounds) {
                continue;
            }
            let gp = GpPrimitive::from_volume(&volume.volume, Some(volume.transform))
                .ok_or(SimulationGroundRunError::UnsupportedVolume { volume: ordinal })?;
            self.phase
                .try_validate_contact_observation(ground.body)
                .map_err(SimulationGroundRunError::Body)?;
            self.phase
                .try_validate_contact_observation(extra.table)
                .map_err(SimulationGroundRunError::Body)?;
            let velocity = self
                .phase
                .try_contact_body_snapshot(extra.live)
                .map_err(SimulationGroundRunError::Body)?
                .velocity;
            let padding = infallible(try_assembly_ground_effective_padding(
                volume.padding,
                GENERATOR_PADDING,
                velocity,
                triangle.primitive().face_normals[0],
                &mut HostGroundPadding,
            ));
            // `SkaterDmoComputeContactPoints` 0x827ce158 is Sk8ComputeContactPoints
            // with isObject false and no fix-up output (instruction-compared).
            let Some(points) = try_sk8_compute_contact_points(
                &gp,
                GpInstanceTags::new(0, 0),
                triangle.primitive(),
                GpInstanceTags::new(triangle.volume_tag(), triangle.user_tag()),
                padding,
                SKATER_DMO_BEND_THRESHOLD,
                CONVEXITY_EPSILON,
                false,
                false,
            ) else {
                continue;
            };
            let table_a = binding_word(extra.table)?;
            let rf_a = binding_word(extra.live)?;
            let body_b = binding_word(ground.body)?;
            for (point_ordinal, pair) in points.point_pairs().iter().enumerate() {
                if cache.len() >= CONTACT_CACHE_CAPACITY {
                    break;
                }
                found = true;
                let a = self
                    .phase
                    .try_contact_body_snapshot(extra.live)
                    .map_err(SimulationGroundRunError::Body)?;
                let b = self
                    .phase
                    .try_contact_body_snapshot(ground.body)
                    .map_err(SimulationGroundRunError::Body)?;
                let mut side_b = common_body(b, SolverDeltaRowToken(body_b));
                if ground_frozen {
                    side_b.state_word = ACTIVE_STATE_WORD;
                }
                let input = infallible(build_assembly_ground_contact_input_with_material(
                    &SimulationAssemblyGroundContactInput {
                        body_a: common_body(a, SolverDeltaRowToken(rf_a)),
                        body_b: side_b,
                        position_a: pair.first(),
                        position_b: pair.second(),
                        normal: points.normal(),
                        table_body_a: ContactBodyToken(table_a),
                        table_body_b: ContactBodyToken(body_b),
                        volume_material: material_input(volume.material),
                        ground_material: material,
                        volume_surface_id: volume.surface_id,
                        triangle_user_tag: triangle.user_tag(),
                    },
                    material,
                    &mut HostGroundContactNumeric,
                ));
                if !input.is_valid_contact(CONTACT_VALID_TOLERANCE) {
                    continue;
                }
                cache.push(SimulationGroundContactRecord {
                    input,
                    table_a: extra.table,
                    table_b: ground.body,
                    rf_a: extra.live,
                    rf_b: ground.body,
                    source: SimulationGroundContactSource {
                        assembly_ordinal: self.assembly_ordinal,
                        volume_ordinal: ordinal,
                        target: triangle.source().into(),
                        point_ordinal,
                    },
                    frame: self.phase.frame_identity(),
                    phase: self.phase.phase_identity(),
                    pool: self.phase.body_pool_authority(),
                });
            }
        }
        Ok(found)
    }
}

/// One ripped batch, triangle by triangle; the first Run failure stops the
/// remaining triangles of the pair.
fn visit_batch(
    run: &SimulationGroundAssemblyRun<'_, '_>,
    batch: AssemblyGroundTriangleBatch<'_, AssemblyGroundTriangle>,
    ground: DmoGround,
    ground_frozen: bool,
    cache: &mut Vec<SimulationGroundContactRecord>,
    found: &mut bool,
    run_error: &mut Option<SimulationGroundRunError>,
) {
    if run_error.is_some() {
        return;
    }
    for triangle in batch.iter() {
        match run.try_skater_dmo_triangle(triangle, ground, ground_frozen, cache) {
            Ok(hit) => *found |= hit,
            Err(error) => {
                *run_error = Some(error);
                return;
            }
        }
    }
}

/// `RunIntersectionTests` 0x827cef98 for one A/B pair: every B aggregate
/// volume not culled against A's first volume group.
fn try_run_pair(
    run: &SimulationGroundAssemblyRun<'_, '_>,
    a_bounds: crate::octree::AabbWords,
    dmo: MeshCollisionTarget<'_>,
    dmo_ordinal: usize,
    cache: &mut Vec<SimulationGroundContactRecord>,
) -> Result<bool, SimulationSkaterDmoJobError> {
    let records = dmo.records();
    let bodies = dmo.bodies();
    let Some(first) = records.first() else {
        return Ok(false);
    };
    let Some(a_first) = run.volumes.first() else {
        return Ok(false);
    };
    // 0x827cf05c/0x827cf088: every aggregate uses B's FIRST volume transform
    // and frozen byte.
    let forward = first.transform;
    let inverse = inverse_collision_affine_reset(forward);
    let ground_frozen = first.frozen;
    let minimum = a_bounds.min_f32();
    let maximum = a_bounds.max_f32();
    let world_minimum = [minimum[0], minimum[1], minimum[2]];
    let world_maximum = [maximum[0], maximum[1], maximum[2]];
    let mut found = false;
    for (ordinal, volume) in records.iter().enumerate() {
        if is_culled(run.pipeline_culling, a_first.group, volume.group).map_err(|source| {
            SimulationSkaterDmoJobError::Run {
                assembly: run.assembly_ordinal,
                source,
            }
        })? {
            continue;
        }
        if volume.volume_type != VolumeType::Aggregate {
            continue;
        }
        let arena = dmo.arena(volume.rigid_body_index).ok_or(
            SimulationSkaterDmoJobError::MissingMeshArena {
                dmo: dmo_ordinal,
                part: volume.rigid_body_index,
            },
        )?;
        let target =
            resolve_aggregate_target(arena, volume.volume.shape_words[0]).map_err(|source| {
                SimulationSkaterDmoJobError::Aggregate {
                    dmo: dmo_ordinal,
                    source,
                }
            })?;
        let CollisionAggregateRef::Mesh(mesh_index) = target else {
            return Err(SimulationSkaterDmoJobError::NotClusteredMesh {
                dmo: dmo_ordinal,
                volume: ordinal,
            });
        };
        let mesh =
            arena
                .meshes
                .get(mesh_index)
                .ok_or(SimulationSkaterDmoJobError::NotClusteredMesh {
                    dmo: dmo_ordinal,
                    volume: ordinal,
                })?;
        let body = *bodies.get(usize::from(volume.rigid_body_index)).ok_or(
            SimulationSkaterDmoJobError::DmoBodyIndex {
                dmo: dmo_ordinal,
                volume: ordinal,
                index: volume.rigid_body_index,
            },
        )?;
        let ground = DmoGround {
            body,
            group: volume.group,
        };
        let tree = mesh
            .clustered_mesh
            .kd_tree(&mesh.object_bytes)
            .map_err(|source| SimulationSkaterDmoJobError::Mesh {
                dmo: dmo_ordinal,
                source,
            })?;
        let (local_minimum, local_maximum) =
            transform_collision_aabb(world_minimum, world_maximum, inverse);
        let separated = (0..3).any(|axis| {
            tree.bounding_box_min[axis] > local_maximum[axis]
                || local_minimum[axis] > tree.bounding_box_max[axis]
        });
        if separated {
            continue;
        }
        let ranges = tree
            .leaf_ranges_for_aabb_limited(
                &mesh.object_bytes,
                local_minimum,
                local_maximum,
                MESH_RANGE_CAPACITY,
            )
            .map_err(|source| SimulationSkaterDmoJobError::Mesh {
                dmo: dmo_ordinal,
                source,
            })?;
        // `spew` 1: every ripped triangle reaches RunIntersectionTests2 in rip
        // order; the twenty-triangle cache only delays, never reorders.
        let mut ripper =
            AssemblyGroundTriangleRipper::new(world_minimum, world_maximum, true, false);
        let mut run_error = None;
        for range in ranges {
            ripper
                .rip_leaf_range(
                    &mesh.clustered_mesh,
                    &mesh.object_bytes,
                    native_ground_descriptor(range, mesh.clustered_mesh.maximum_cluster_count),
                    forward,
                    &mut |_| {},
                    &mut |batch| {
                        visit_batch(
                            run,
                            batch,
                            ground,
                            ground_frozen,
                            cache,
                            &mut found,
                            &mut run_error,
                        );
                    },
                )
                .map_err(|source| SimulationSkaterDmoJobError::Rip {
                    dmo: dmo_ordinal,
                    source,
                })?;
        }
        ripper.finish(|batch| {
            visit_batch(
                run,
                batch,
                ground,
                ground_frozen,
                cache,
                &mut found,
                &mut run_error,
            );
        });
        if let Some(source) = run_error {
            return Err(SimulationSkaterDmoJobError::Run {
                assembly: run.assembly_ordinal,
                source,
            });
        }
    }
    Ok(found)
}

/// Inputs every skater DMO job shares.
pub(crate) struct SimulationSkaterDmoJobInput<'a, 'pool> {
    pub(crate) phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
    /// JobInput.cullingTable: the Island table.
    pub(crate) pipeline_culling: Option<&'a BitTable>,
    pub(crate) remaining_pairs: usize,
    pub(crate) remaining_thaws: usize,
}

/// The stored Contacts of every job and the DMO pool thaw list.
pub(crate) struct SimulationSkaterDmoJobOutput {
    pub(crate) records: Vec<SimulationGroundContactRecord>,
    /// Indices into the job's DMO list, in first-push order
    /// (0x827cf614..f660: `allPartsFrozen` and a written Contact).
    pub(crate) thaw: Vec<usize>,
    /// [skater ordinal, DMO ordinal], admitted before triangle narrow phase.
    pub(crate) pairs: Vec<[usize; 2]>,
}

/// Every skater assembly's job in order; each job's ContactCache is appended
/// in turn.
pub(crate) fn try_run_skater_dmo_cpu_jobs<'a, 'pool, P: Into<AuthoredCollisionPublication<'a>>>(
    skaters: impl Iterator<Item = P>,
    dmos: &[&'a PreparedSerialisedAssembly],
    input: &SimulationSkaterDmoJobInput<'a, 'pool>,
) -> Result<SimulationSkaterDmoJobOutput, SimulationSkaterDmoJobError> {
    let targets: Vec<_> = dmos
        .iter()
        .map(|assembly| MeshCollisionTarget::Source(*assembly))
        .collect();
    try_run_skater_mesh_cpu_jobs(skaters, &targets, input)
}

pub(crate) fn try_run_skater_mesh_cpu_jobs<'a, 'pool, P: Into<AuthoredCollisionPublication<'a>>>(
    skaters: impl Iterator<Item = P>,
    dmos: &[MeshCollisionTarget<'a>],
    input: &SimulationSkaterDmoJobInput<'a, 'pool>,
) -> Result<SimulationSkaterDmoJobOutput, SimulationSkaterDmoJobError> {
    let mut output = Vec::new();
    let mut thaw = Vec::new();
    let mut pairs = Vec::new();
    for (dmo_ordinal, dmo) in dmos.iter().enumerate() {
        if let MeshCollisionTarget::Character(publication) = dmo {
            if publication.body_pool_authority() != input.phase.body_pool_authority() {
                return Err(SimulationSkaterDmoJobError::Run {
                    assembly: dmo_ordinal,
                    source: SimulationGroundRunError::ForeignImage,
                });
            }
        }
        if !dmo.has_bounds_for_frame(input.phase.frame_identity()) {
            return Err(SimulationSkaterDmoJobError::StaleDmoBounds { dmo: dmo_ordinal });
        }
    }
    for (ordinal, skater) in skaters.enumerate() {
        let publication: AuthoredCollisionPublication<'a> = skater.into();
        let Some(run) = SimulationGroundAssemblyRun::try_from_publication(
            GroundAssemblyPublication::Authored(publication),
            ordinal,
            input.phase,
            SimulationGroundRunSettings {
                ground: None,
                ground_group: 0,
                pipeline_culling: input.pipeline_culling,
                is_object: false,
                draw_contacts: false,
            },
            // The native skater-DMO job never tests A.allPartsFrozen. Its only
            // frozen-header read (827cf614) is B's post-contact thaw decision.
            false,
        )
        .map_err(|source| SimulationSkaterDmoJobError::Run {
            assembly: ordinal,
            source,
        })?
        else {
            continue;
        };
        let a_bounds = publication.bounds();
        let a_group = publication.group();
        let mut cache = Vec::with_capacity(CONTACT_CACHE_CAPACITY);
        for (dmo_ordinal, dmo) in dmos.iter().enumerate() {
            if aabbs_separated(a_bounds, dmo.bounds()) {
                continue;
            }
            if is_culled(input.pipeline_culling, a_group, dmo.group()).map_err(|source| {
                SimulationSkaterDmoJobError::Run {
                    assembly: ordinal,
                    source,
                }
            })? {
                continue;
            }
            if pairs.len() < input.remaining_pairs {
                pairs.push([ordinal, dmo_ordinal]);
            }
            let found = try_run_pair(&run, a_bounds, *dmo, dmo_ordinal, &mut cache)?;
            if dmo.frozen() && found && thaw.len() < input.remaining_thaws {
                thaw.push(dmo_ordinal);
            }
        }
        output.extend(cache);
    }
    Ok(SimulationSkaterDmoJobOutput {
        records: output,
        thaw,
        pairs,
    })
}
