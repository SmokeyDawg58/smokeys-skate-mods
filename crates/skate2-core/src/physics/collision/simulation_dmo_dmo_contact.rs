//! AssemblyAssemblyCollisionJob827cc410 / RunIntersectionTests827cbdc8.
//! Serial CPU execution of intra-pool and cross-pool pipelines. Contacts use the same
//! sealed compositor, temporary buffer and solver as the other producers.

use super::super::{
    ContactBodyToken, SimulationAssemblyGroundContactInput, SimulationAssemblyGroundMaterialInput,
    SolverDeltaRowToken, build_assembly_ground_contact_input_with_material,
};
use super::{
    CONVEXITY_EPSILON, ExtraVolumeData, GroundAssemblyPublication, HostGroundContactNumeric,
    PreparedSerialisedAssembly, SimulationContactTargetSource, SimulationGroundContactBuffer,
    SimulationGroundContactRecord, SimulationGroundContactSource, SimulationGroundRunError,
    SimulationPackedBodyWordImageDomain, SimulationRigidBodyDynamicUpdatePhase, binding_word,
    common_body, infallible, is_culled, material_input, prepare_volume_extras,
};
use crate::{
    narrow_phase::{GpInstanceTags, GpPrimitive, try_compute_contact_points},
    simulation_contact_append::AssemblyBodyContactWriteOutcome,
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable, VolumeType};

// InitWorld827b4e64..4ea8. These override the pipeline Params defaults.
const ASSEMBLIES_PER_JOB: usize = 5;
const ADDITIONAL_PADDING: f32 = f32::from_bits(0x3e19_999a);
const BEND_THRESHOLD: f32 = f32::from_bits(0x3f7f_be77);
const VALID_TOLERANCE: f32 = f32::from_bits(0x3780_0000);

pub(crate) struct SimulationDmoDmoJobInput<'a, 'pool> {
    pub(crate) phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
    pub(crate) image: &'a SimulationPackedBodyWordImageDomain,
    pub(crate) pipeline_culling: Option<&'a BitTable>,
    pub(crate) maximum_contacts: u32,
    pub(crate) remaining_pairs: usize,
    pub(crate) remaining_thaws: usize,
}

pub(crate) struct SimulationDmoDmoJobOutput {
    pub(crate) records: Vec<SimulationGroundContactRecord>,
    /// Pool ordinals, preserving job/pair admission order before narrow phase.
    /// Owner-side partitioning supplies the actual assemblies and Part bodies.
    pub(crate) pairs: Vec<[usize; 2]>,
    pub(crate) thaw: Vec<usize>,
}

/// InitWorld827b42c0 uses the ordinary AssemblyAssembly worker for DMO-
/// vehicle, ten A assemblies per job, multiple writes, default padding zero.
/// No mesh worker or vehicle-ground pipeline is substituted for this route.
pub(crate) struct SimulationAssemblyPairJobInput<'a, 'pool> {
    pub(crate) phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
    pub(crate) pipeline_culling: Option<&'a BitTable>,
    pub(crate) maximum_contacts: u32,
    pub(crate) remaining_pairs: usize,
    pub(crate) remaining_thaws: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssemblyPairSide {
    A,
    B,
}

pub(crate) struct SimulationAssemblyPairJobOutput {
    pub(crate) records: Vec<SimulationGroundContactRecord>,
    pub(crate) pairs: Vec<[usize; 2]>,
    pub(crate) thaw: Vec<(AssemblyPairSide, usize)>,
}

pub(crate) fn aabbs_separated(a: crate::octree::AabbWords, b: crate::octree::AabbWords) -> bool {
    let (amin, amax, bmin, bmax) = (a.min_f32(), a.max_f32(), b.min_f32(), b.max_f32());
    (0..3).any(|axis| bmin[axis] > amax[axis] || amin[axis] > bmax[axis])
}

fn pair_material(
    a: AssemblyPartMaterial,
    b: AssemblyPartMaterial,
) -> SimulationAssemblyGroundMaterialInput {
    // 827cbfb8..bff8: keep the three independent ordered comparisons.
    SimulationAssemblyGroundMaterialInput {
        // bge is !LT, including unordered; ble below is !GT.
        restitution: if a.restitution < b.restitution {
            a.restitution
        } else {
            b.restitution
        },
        dynamic_friction: if a.dynamic_friction > b.dynamic_friction {
            a.dynamic_friction
        } else {
            b.dynamic_friction
        },
        static_friction: if a.static_friction > b.static_friction {
            a.static_friction
        } else {
            b.static_friction
        },
    }
}

fn prepare(
    publication: GroundAssemblyPublication<'_, '_>,
    input: &SimulationAssemblyPairJobInput<'_, '_>,
) -> Result<Vec<Option<ExtraVolumeData>>, SimulationGroundRunError> {
    let has_bounds = match publication {
        GroundAssemblyPublication::Serialized { assembly, image } => {
            if !input.phase.ground_image_matches_pool(image)
                || !assembly.matches_ground_image(image)
            {
                return Err(SimulationGroundRunError::ForeignImage);
            }
            assembly.has_bounds_for_frame(input.phase.frame_identity())
        }
        GroundAssemblyPublication::Authored(assembly) => {
            if assembly.body_pool_authority() != input.phase.body_pool_authority() {
                return Err(SimulationGroundRunError::ForeignImage);
            }
            assembly.has_bounds_for_frame(input.phase.frame_identity())
        }
    };
    if !has_bounds {
        return Err(SimulationGroundRunError::StaleAssemblyBounds);
    }
    prepare_volume_extras(
        publication.records(),
        publication.bodies(),
        input.phase,
        publication,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_pair(
    a: GroundAssemblyPublication<'_, '_>,
    b: GroundAssemblyPublication<'_, '_>,
    a_ordinal: usize,
    b_ordinal: usize,
    a_extra: &[Option<ExtraVolumeData>],
    b_extra: &[Option<ExtraVolumeData>],
    input: &SimulationAssemblyPairJobInput<'_, '_>,
    buffer: &mut SimulationGroundContactBuffer,
    self_pair: bool,
    additional_padding: f32,
) -> Result<bool, SimulationGroundRunError> {
    let a_volumes = a.records();
    let b_volumes = b.records();
    let mut found = false;
    for (ai, av) in a_volumes.iter().enumerate() {
        for (bi, bv) in b_volumes.iter().enumerate() {
            if (self_pair && ai == bi)
                || av.volume_type == VolumeType::Aggregate
                || bv.volume_type == VolumeType::Aggregate
                || !av.enabled
                || (!bv.enabled && b_volumes.len() != 1)
            {
                continue;
            }
            let culled = if self_pair {
                av.rigid_body_index == bv.rigid_body_index
                    || is_culled(
                        a.self_culling(),
                        u32::from(av.rigid_body_index),
                        u32::from(bv.rigid_body_index),
                    )?
            } else {
                is_culled(input.pipeline_culling, av.group, bv.group)?
            };
            if culled {
                continue;
            }
            let material = pair_material(av.material, bv.material);
            let ae = a_extra[ai].ok_or(SimulationGroundRunError::MissingExtra { volume: ai })?;
            let be = b_extra[bi].ok_or(SimulationGroundRunError::MissingExtra { volume: bi })?;
            let ab = input
                .phase
                .try_contact_body_snapshot(ae.live)
                .map_err(SimulationGroundRunError::Body)?;
            let bb = input
                .phase
                .try_contact_body_snapshot(be.live)
                .map_err(SimulationGroundRunError::Body)?;
            if ab.state == 1 && bb.state == 1 {
                continue;
            }
            let padding = (bv.padding + av.padding) + additional_padding;
            let agp = GpPrimitive::from_volume(&av.volume, Some(av.transform))
                .ok_or(SimulationGroundRunError::UnsupportedVolume { volume: ai })?;
            let bgp = GpPrimitive::from_volume(&bv.volume, Some(bv.transform))
                .ok_or(SimulationGroundRunError::UnsupportedVolume { volume: bi })?;
            if aabbs_separated(ae.bounds, be.bounds) {
                continue;
            }
            let Some(points) = try_compute_contact_points(
                &agp,
                GpInstanceTags::new(0, 0),
                &bgp,
                GpInstanceTags::new(0, 0),
                padding,
                BEND_THRESHOLD,
                CONVEXITY_EPSILON,
                false,
            ) else {
                continue;
            };
            let table_a = binding_word(ae.table)?;
            let table_b = binding_word(be.table)?;
            let rf_a = binding_word(ae.live)?;
            let rf_b = binding_word(be.live)?;
            for (point_ordinal, pair) in points.point_pairs().iter().enumerate() {
                let outcome = buffer
                    .try_reserve_then_validate(|| {
                        let contact =
                            infallible(build_assembly_ground_contact_input_with_material(
                                &SimulationAssemblyGroundContactInput {
                                    body_a: common_body(ab, SolverDeltaRowToken(rf_a)),
                                    body_b: common_body(bb, SolverDeltaRowToken(rf_b)),
                                    position_a: pair.first(),
                                    position_b: pair.second(),
                                    normal: points.normal(),
                                    table_body_a: ContactBodyToken(table_a),
                                    table_body_b: ContactBodyToken(table_b),
                                    volume_material: material_input(av.material),
                                    ground_material: material,
                                    volume_surface_id: av.surface_id,
                                    // Shared compositor extracts B's surface from
                                    // the high half of this packed material word.
                                    triangle_user_tag: (bv.surface_id & 0xffff) << 16,
                                },
                                material,
                                &mut HostGroundContactNumeric,
                            ));
                        contact.is_valid_contact(VALID_TOLERANCE).then_some(
                            SimulationGroundContactRecord {
                                input: contact,
                                table_a: ae.table,
                                table_b: be.table,
                                rf_a: ae.live,
                                rf_b: be.live,
                                source: SimulationGroundContactSource {
                                    assembly_ordinal: a_ordinal,
                                    volume_ordinal: ai,
                                    point_ordinal,
                                    target: SimulationContactTargetSource::Primitive {
                                        assembly_ordinal: b_ordinal,
                                        volume_ordinal: bi,
                                    },
                                },
                                frame: input.phase.frame_identity(),
                                phase: input.phase.phase_identity(),
                                pool: input.phase.body_pool_authority(),
                            },
                        )
                    })
                    .map_err(SimulationGroundRunError::Storage)?;
                match outcome {
                    AssemblyBodyContactWriteOutcome::Unavailable => return Ok(found),
                    AssemblyBodyContactWriteOutcome::Written
                    | AssemblyBodyContactWriteOutcome::Discarded => found = true,
                }
            }
        }
    }
    Ok(found)
}

pub(crate) fn try_run_dmo_dmo_cpu_jobs(
    dmos: &[&PreparedSerialisedAssembly],
    input: &SimulationDmoDmoJobInput<'_, '_>,
) -> Result<SimulationDmoDmoJobOutput, SimulationGroundRunError> {
    let targets: Vec<_> = dmos
        .iter()
        .map(|assembly| GroundAssemblyPublication::Serialized {
            assembly,
            image: input.image,
        })
        .collect();
    let output = try_run_assembly_pair_jobs(
        &targets,
        &targets,
        &SimulationAssemblyPairJobInput {
            phase: input.phase,
            pipeline_culling: input.pipeline_culling,
            maximum_contacts: input.maximum_contacts,
            remaining_pairs: input.remaining_pairs,
            remaining_thaws: input.remaining_thaws,
        },
        true,
        ASSEMBLIES_PER_JOB,
        ADDITIONAL_PADDING,
        true,
    )?;
    Ok(SimulationDmoDmoJobOutput {
        records: output.records,
        pairs: output.pairs,
        thaw: output
            .thaw
            .into_iter()
            .map(|(_, ordinal)| ordinal)
            .collect(),
    })
}

pub(crate) fn try_run_dmo_vehicle_cpu_jobs(
    dmos: &[&PreparedSerialisedAssembly],
    vehicles: &[&crate::dynamic_assembly_session::character_publication::CharacterCollisionPublication],
    image: &SimulationPackedBodyWordImageDomain,
    input: &SimulationAssemblyPairJobInput<'_, '_>,
) -> Result<SimulationAssemblyPairJobOutput, SimulationGroundRunError> {
    let a: Vec<_> = dmos
        .iter()
        .map(|assembly| GroundAssemblyPublication::Serialized { assembly, image })
        .collect();
    let b: Vec<_> = vehicles
        .iter()
        .map(|assembly| GroundAssemblyPublication::Authored((*assembly).into()))
        .collect();
    // Params82e0ace8 sets padding0; InitWorld changes only job count and writes.
    try_run_assembly_pair_jobs(&a, &b, input, false, 10, 0.0, true)
}

/// `Island::InitWorld` 0x827b4bf4..4c28: one SkaterPool on both sides,
/// two assemblies per job. Params 0x82e0ace8 retains padding zero and
/// allowMultipleContactWrites=false: the native 50-contact temporary buffer
/// stops accepting contacts until the worker flushes after the current A.
/// The same worker includes self-collision through each assembly's own table.
pub(crate) fn try_run_skater_skater_cpu_jobs(
    skaters: &[GroundAssemblyPublication<'_, '_>],
    input: &SimulationAssemblyPairJobInput<'_, '_>,
) -> Result<SimulationAssemblyPairJobOutput, SimulationGroundRunError> {
    try_run_assembly_pair_jobs(skaters, skaters, input, true, 2, 0.0, false)
}

/// PC Object Dropper (a port addition): riderless boards against the skaters'
/// pool members, the skater-skater worker's settings across two pools.
pub(crate) fn try_run_riderless_board_cpu_jobs(
    boards: &[GroundAssemblyPublication<'_, '_>],
    skaters: &[GroundAssemblyPublication<'_, '_>],
    input: &SimulationAssemblyPairJobInput<'_, '_>,
) -> Result<SimulationAssemblyPairJobOutput, SimulationGroundRunError> {
    try_run_assembly_pair_jobs(boards, skaters, input, false, 2, 0.0, false)
}

/// InitWorld827b42c0 assigns the ordinary AssemblyAssembly worker to these
/// four pipelines. They all enable multiple writes and retain padding zero.
#[derive(Clone, Copy)]
pub(crate) enum PedestrianPairPipeline {
    DmoPedestrian,
    PedestrianVehicle,
    PedestrianPedestrian,
    SkaterPedestrian,
}
pub(crate) fn try_run_pedestrian_pair_cpu_jobs(
    a: &[GroundAssemblyPublication<'_, '_>],
    b: &[GroundAssemblyPublication<'_, '_>],
    input: &SimulationAssemblyPairJobInput<'_, '_>,
    pipeline: PedestrianPairPipeline,
) -> Result<SimulationAssemblyPairJobOutput, SimulationGroundRunError> {
    let (intra, count) = match pipeline {
        PedestrianPairPipeline::DmoPedestrian => (false, 5),
        PedestrianPairPipeline::PedestrianVehicle => (false, 10),
        PedestrianPairPipeline::PedestrianPedestrian => (true, 5),
        PedestrianPairPipeline::SkaterPedestrian => (false, 2),
    };
    try_run_assembly_pair_jobs(a, b, input, intra, count, 0.0, true)
}

fn try_run_assembly_pair_jobs(
    a_targets: &[GroundAssemblyPublication<'_, '_>],
    b_targets: &[GroundAssemblyPublication<'_, '_>],
    input: &SimulationAssemblyPairJobInput<'_, '_>,
    intra_pool: bool,
    assemblies_per_job: usize,
    additional_padding: f32,
    allow_multiple_writes: bool,
) -> Result<SimulationAssemblyPairJobOutput, SimulationGroundRunError> {
    let mut buffer = SimulationGroundContactBuffer::try_new(input.maximum_contacts, allow_multiple_writes)
        .map_err(SimulationGroundRunError::Storage)?;
    let mut pairs = Vec::new();
    let mut thaw = Vec::new();
    // Pipeline Run82e0bf68..80 issues no jobs if either pool is empty.
    if !b_targets.is_empty() {
        for job_start in (0..a_targets.len()).step_by(assemblies_per_job) {
            let job_end = (job_start + assemblies_per_job).min(a_targets.len());
            if a_targets[job_start..job_end].iter().all(|a| a.all_static()) {
                continue;
            }
            for ai in job_start..job_end {
                let a = a_targets[ai];
                let a_extra = prepare(a, input)?;
                // Run82e0c158..168 advances B only for an intra-pool job.
                // Job827cc68c..6b8 skips A>B only for that same-pool case.
                let b_start = if intra_pool { ai } else { 0 };
                for bi in b_start..b_targets.len() {
                    let b = b_targets[bi];
                    let self_pair = intra_pool && ai == bi;
                    if self_pair {
                        if a.self_culling().is_none() {
                            continue;
                        }
                    } else {
                        if aabbs_separated(a.bounds(), b.bounds()) {
                            continue;
                        }
                        if (a.frozen() && b.frozen())
                            || (a.all_static() && b.all_static())
                            || (a.frozen() && b.all_static())
                            || (b.frozen() && a.all_static())
                        {
                            continue;
                        }
                        if is_culled(input.pipeline_culling, a.group(), b.group())? {
                            continue;
                        }
                    }
                    if pairs.len() < input.remaining_pairs {
                        pairs.push([ai, bi]);
                    }
                    if self_pair {
                        run_pair(
                            a,
                            a,
                            ai,
                            ai,
                            &a_extra,
                            &a_extra,
                            input,
                            &mut buffer,
                            true,
                            additional_padding,
                        )?;
                    } else {
                        let b_extra = prepare(b, input)?;
                        if run_pair(
                            a,
                            b,
                            ai,
                            bi,
                            &a_extra,
                            &b_extra,
                            input,
                            &mut buffer,
                            false,
                            additional_padding,
                        )? {
                            // 827cc950..9f0: append A then B, no deduplication.
                            if a.frozen() && thaw.len() < input.remaining_thaws {
                                thaw.push((AssemblyPairSide::A, ai));
                            }
                            if b.frozen() && thaw.len() < input.remaining_thaws {
                                thaw.push((AssemblyPairSide::B, bi));
                            }
                        }
                    }
                }
                // 827ccaf0..b1c flushes after EACH A, not just once per job.
                buffer
                    .try_finish_assembly()
                    .map_err(SimulationGroundRunError::Storage)?;
            }
        }
    }
    Ok(SimulationAssemblyPairJobOutput {
        records: buffer
            .try_into_records()
            .map_err(SimulationGroundRunError::Storage)?,
        pairs,
        thaw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_material_keeps_native_unordered_and_signed_zero_choices() {
        let a = AssemblyPartMaterial {
            restitution: 0.8,
            dynamic_friction: 0.4,
            static_friction: 0.3,
        };
        let b = AssemblyPartMaterial {
            restitution: 0.2,
            dynamic_friction: 0.5,
            static_friction: 0.6,
        };
        let selected = pair_material(a, b);
        assert_eq!(
            [
                selected.restitution,
                selected.dynamic_friction,
                selected.static_friction
            ],
            [0.2, 0.5, 0.6]
        );
        let nan = f32::from_bits(0x7fc0_1234);
        let selected = pair_material(
            AssemblyPartMaterial {
                restitution: nan,
                dynamic_friction: nan,
                static_friction: nan,
            },
            b,
        );
        assert_eq!(selected.restitution, b.restitution);
        assert_eq!(selected.dynamic_friction, b.dynamic_friction);
        assert_eq!(selected.static_friction, b.static_friction);
        let selected = pair_material(
            AssemblyPartMaterial {
                restitution: 0.0,
                dynamic_friction: 0.0,
                static_friction: 0.0,
            },
            AssemblyPartMaterial {
                restitution: -0.0,
                dynamic_friction: -0.0,
                static_friction: -0.0,
            },
        );
        assert_eq!(
            [
                selected.restitution.to_bits(),
                selected.dynamic_friction.to_bits(),
                selected.static_friction.to_bits()
            ],
            [0x8000_0000; 3]
        );
    }
}
