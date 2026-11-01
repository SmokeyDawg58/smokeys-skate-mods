//! Single serialized CPU ground job over source-owned resident static meshes.
//!
//! `AssemblyGroundCollisionJob` (`0x827cdb98`) and its `BoxTest<...,1>`
//! specialization (`0x827cd960`) provide the complete sequence here. This
//! profile handles the actual static `GroundPool` collection; it does not
//! schedule native workers, merge competing jobs, or supply other collision
//! producers. All bindings and the source array remain borrowed for the job.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "CPU ground job awaiting persistent simulation-frame caller"
    )
)]

use super::{
    GroundAssemblyPublication, PreparedSerialisedAssembly, SimulationGroundAssemblyRun,
    SimulationGroundContactBuffer, SimulationGroundContactBufferError,
    SimulationGroundContactRecord, SimulationGroundRunError, SimulationGroundRunOutcome,
    SimulationGroundRunSettings, SimulationPackedBodyWordImageDomain,
    SimulationRigidBodyDynamicUpdatePhase,
};
use crate::{
    authored_collision_publication::AuthoredCollisionPublication,
    simulation_assembly_ground_triangle_cache::AssemblyGroundTriangleBatch,
    simulation_assembly_ground_triangle_rip::{
        AssemblyGroundTriangleRipError, AssemblyGroundTriangleRipper,
    },
    simulation_assembly_ground_triangle_topology::AssemblyGroundTriangle,
    static_ground_pool::{STATIC_GROUND_IDENTITY_TRANSFORM, StaticGroundPool},
    world::transform_collision_aabb,
};
use skate2_formats::collision::{CollisionError, KdLeafRange};
use std::fmt;

const MESH_RANGE_CAPACITY: usize = 256;

/// Explicit caller values from the native job/pipeline, never hidden defaults.
pub(crate) struct SimulationStaticGroundJobInput<'a, 'pool> {
    pub(crate) assemblies: &'a [PreparedSerialisedAssembly],
    pub(crate) ground_pool: &'a StaticGroundPool,
    pub(crate) phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
    pub(crate) image: &'a SimulationPackedBodyWordImageDomain,
    pub(crate) run_settings: SimulationGroundRunSettings<'a>,
    pub(crate) maximum_contacts: u32,
    pub(crate) allow_multiple_writes: bool,
}

#[derive(Debug)]
pub(crate) enum SimulationStaticGroundJobError {
    Storage(SimulationGroundContactBufferError),
    Run {
        assembly: usize,
        source: SimulationGroundRunError,
    },
    Mesh {
        assembly: usize,
        mesh: usize,
        source: CollisionError,
    },
    Rip {
        assembly: usize,
        mesh: usize,
        source: AssemblyGroundTriangleRipError,
    },
}

impl fmt::Display for SimulationStaticGroundJobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(source) => write!(f, "ground job Contact storage: {source}"),
            Self::Run { assembly, source } => write!(f, "ground job assembly {assembly}: {source}"),
            Self::Mesh {
                assembly,
                mesh,
                source,
            } => write!(f, "ground job assembly {assembly}, mesh {mesh}: {source}"),
            Self::Rip {
                assembly,
                mesh,
                source,
            } => write!(
                f,
                "ground job assembly {assembly}, mesh {mesh} ripping: {source:?}"
            ),
        }
    }
}

/// Inert host diagnostics, not native counters or collision authority.
#[derive(Default, Debug, Eq, PartialEq)]
pub(crate) struct SimulationStaticGroundJobReport {
    pub(crate) processed_assemblies: usize,
    pub(crate) skipped_assemblies: usize,
    pub(crate) visited_meshes: usize,
    pub(crate) gathered_ranges: usize,
    pub(crate) intersection_runs: usize,
    pub(crate) submitted_triangles: usize,
    pub(crate) unavailable_runs: usize,
}

/// Consuming result prevents this job owner itself from publishing twice.
/// Rows remain inert until the common compiler validates their phase/roles.
pub(crate) struct SimulationStaticGroundJobOutput {
    records: Vec<SimulationGroundContactRecord>,
    pub(crate) report: SimulationStaticGroundJobReport,
}

impl SimulationStaticGroundJobOutput {
    /// A pipeline may partition its retained pool into several jobs. Restore
    /// pool-relative provenance after a job received only its contiguous slice.
    pub(crate) fn offset_assembly_ordinals(&mut self, offset: usize) {
        for record in &mut self.records { record.source.assembly_ordinal += offset; }
    }
    pub(crate) fn into_parts(
        self,
    ) -> (
        Vec<SimulationGroundContactRecord>,
        SimulationStaticGroundJobReport,
    ) {
        (self.records, self.report)
    }

    pub(crate) fn into_records(self) -> Vec<SimulationGroundContactRecord> {
        self.records
    }
}

/// Executes one isolated serialized job with an initially empty global prefix.
/// All temporary output is abandoned on a checked host failure; no body, RF,
/// resident pool or external manager is mutated. This does not claim native
/// malformed-memory failure atomicity or inter-job atomic reservation order.
pub(crate) fn try_run_static_ground_cpu_job(
    input: &SimulationStaticGroundJobInput<'_, '_>,
) -> Result<SimulationStaticGroundJobOutput, SimulationStaticGroundJobError> {
    try_run_publications(
        input
            .assemblies
            .iter()
            .map(|assembly| GroundAssemblyPublication::Serialized {
                assembly,
                image: input.image,
            }),
        input.ground_pool,
        input.phase,
        input.run_settings,
        input.maximum_contacts,
        input.allow_multiple_writes,
    )
}

/// Separate sealed authored source, sharing the SAME mesh traversal and Run.
/// This is one isolated CPU job, not full SkaterPool/pair scheduling.
pub(crate) struct SimulationAuthoredGroundJobInput<'a, 'pool> {
    pub(crate) ground_pool: &'a StaticGroundPool,
    pub(crate) phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
    pub(crate) run_settings: SimulationGroundRunSettings<'a>,
    pub(crate) maximum_contacts: u32,
    pub(crate) allow_multiple_writes: bool,
}

pub(crate) fn try_run_authored_ground_cpu_job<
    'a,
    'pool,
    P: Into<AuthoredCollisionPublication<'a>>,
>(
    assemblies: impl Iterator<Item = P>,
    input: &SimulationAuthoredGroundJobInput<'a, 'pool>,
) -> Result<SimulationStaticGroundJobOutput, SimulationStaticGroundJobError> {
    try_run_publications(
        assemblies.map(|assembly| GroundAssemblyPublication::Authored(assembly.into())),
        input.ground_pool,
        input.phase,
        input.run_settings,
        input.maximum_contacts,
        input.allow_multiple_writes,
    )
}

/// The DMO pool's serialized publications in pool order through the SAME
/// mesh traversal and Run (`Island`'s DMO `AssemblyGroundCollisionPipeline`).
pub(crate) fn try_run_serialized_ground_cpu_job<'a, 'pool, 'image>(
    assemblies: impl Iterator<Item = &'a PreparedSerialisedAssembly>,
    image: &'image SimulationPackedBodyWordImageDomain,
    input: &SimulationAuthoredGroundJobInput<'a, 'pool>,
) -> Result<SimulationStaticGroundJobOutput, SimulationStaticGroundJobError> {
    try_run_publications(
        assemblies.map(|assembly| GroundAssemblyPublication::Serialized { assembly, image }),
        input.ground_pool,
        input.phase,
        input.run_settings,
        input.maximum_contacts,
        input.allow_multiple_writes,
    )
}

fn try_run_publications<'a, 'pool, 'image>(
    assemblies: impl Iterator<Item = GroundAssemblyPublication<'a, 'image>>,
    ground_pool: &StaticGroundPool,
    phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
    settings: SimulationGroundRunSettings<'a>,
    maximum_contacts: u32,
    allow_multiple_writes: bool,
) -> Result<SimulationStaticGroundJobOutput, SimulationStaticGroundJobError> {
    let mut output =
        SimulationGroundContactBuffer::try_new(maximum_contacts, allow_multiple_writes)
            .map_err(SimulationStaticGroundJobError::Storage)?;
    let mut report = SimulationStaticGroundJobReport::default();
    for (ordinal, assembly) in assemblies.enumerate() {
        let Some(run) = SimulationGroundAssemblyRun::try_from_publication(
            assembly, ordinal, phase, settings, true,
        )
        .map_err(|source| SimulationStaticGroundJobError::Run {
            assembly: ordinal,
            source,
        })?
        else {
            report.skipped_assemblies += 1;
            continue;
        };
        report.processed_assemblies += 1;
        let trace_board_query =
            ordinal == 0 && std::env::var_os("SKATE2_TRACE_WHEEL_GROUND").is_some();
        if trace_board_query {
            eprintln!(
                "wheel-ground-query frame={:?} assembly={ordinal} bounds={:?}",
                phase.frame_identity(),
                assembly.bounds()
            );
        }
        let triangles_before = report.submitted_triangles;
        try_run_assembly_meshes(
            ground_pool,
            assembly.bounds(),
            ordinal,
            &run,
            &mut output,
            &mut report,
            trace_board_query,
        )?;
        if trace_board_query {
            eprintln!(
                "wheel-ground-query frame={:?} assembly={ordinal} submitted-triangles={}",
                phase.frame_identity(),
                report.submitted_triangles - triangles_before
            );
        }
        output
            .try_finish_assembly()
            .map_err(SimulationStaticGroundJobError::Storage)?;
    }
    let records = output
        .try_into_records()
        .map_err(SimulationStaticGroundJobError::Storage)?;
    Ok(SimulationStaticGroundJobOutput { records, report })
}

fn try_run_assembly_meshes(
    ground_pool: &StaticGroundPool,
    bounds: crate::octree::AabbWords,
    assembly_ordinal: usize,
    run: &SimulationGroundAssemblyRun<'_, '_>,
    output: &mut SimulationGroundContactBuffer,
    report: &mut SimulationStaticGroundJobReport,
    trace_board_query: bool,
) -> Result<(), SimulationStaticGroundJobError> {
    let minimum = bounds.min_f32();
    let maximum = bounds.max_f32();
    let world_minimum = [minimum[0], minimum[1], minimum[2]];
    let world_maximum = [maximum[0], maximum[1], maximum[2]];
    // One tester/cache survives the entire collection, including mesh boundaries.
    // CPU caller passes spew=false; this supported profile disables debug output.
    let mut ripper = AssemblyGroundTriangleRipper::new(world_minimum, world_maximum, false, false);
    let mut run_error = None;
    for (mesh_ordinal, view) in ground_pool.assembly_ground_meshes().enumerate() {
        report.visited_meshes += 1;
        // Reset retains identity forward/inverse transforms for static ground.
        // Still perform the source midpoint/extent Transform per mesh.
        let (local_minimum, local_maximum) = transform_collision_aabb(
            world_minimum,
            world_maximum,
            STATIC_GROUND_IDENTITY_TRANSFORM,
        );
        let &(mesh_minimum, mesh_maximum) =
            view.kd_bounds
                .as_ref()
                .map_err(|source| SimulationStaticGroundJobError::Mesh {
                    assembly: assembly_ordinal,
                    mesh: mesh_ordinal,
                    source: source.clone(),
                })?;
        let intersects_mesh = !(0..3).any(|axis| {
            mesh_minimum[axis] > local_maximum[axis] || local_minimum[axis] > mesh_maximum[axis]
        });
        if trace_board_query {
            eprintln!(
                "wheel-ground-query assembly={assembly_ordinal} mesh={mesh_ordinal} mesh-bounds={mesh_minimum:?}..{mesh_maximum:?} intersects={intersects_mesh}"
            );
        }
        if !intersects_mesh {
            continue;
        }
        let mesh = view.mesh;
        let tree = mesh
            .clustered_mesh
            .kd_tree(&mesh.object_bytes)
            .map_err(|source| SimulationStaticGroundJobError::Mesh {
                assembly: assembly_ordinal,
                mesh: mesh_ordinal,
                source,
            })?;
        let ranges = tree
            .leaf_ranges_for_aabb_limited(
                &mesh.object_bytes,
                local_minimum,
                local_maximum,
                MESH_RANGE_CAPACITY,
            )
            .map_err(|source| SimulationStaticGroundJobError::Mesh {
                assembly: assembly_ordinal,
                mesh: mesh_ordinal,
                source,
            })?;
        report.gathered_ranges += ranges.len();
        if trace_board_query {
            eprintln!(
                "wheel-ground-query assembly={assembly_ordinal} mesh={mesh_ordinal} kd-ranges={}",
                ranges.len()
            );
        }
        // Native gathers the complete capped range array BEFORE the first rip.
        let mut clusters = view.cluster_decode.memo();
        for range in ranges {
            ripper
                .rip_mesh_leaf_range(
                    &mesh.clustered_mesh,
                    &mesh.object_bytes,
                    native_ground_descriptor(range, mesh.clustered_mesh.maximum_cluster_count),
                    &mut clusters,
                    STATIC_GROUND_IDENTITY_TRANSFORM,
                    &mut |_| {},
                    &mut |batch| run_batch(run, output, report, &mut run_error, batch),
                )
                .map_err(|source| SimulationStaticGroundJobError::Rip {
                    assembly: assembly_ordinal,
                    mesh: mesh_ordinal,
                    source,
                })?;
            if let Some(source) = run_error.take() {
                return Err(SimulationStaticGroundJobError::Run {
                    assembly: assembly_ordinal,
                    source,
                });
            }
        }
    }
    // PC park editor: placed mission props after the streamed meshes.
    let host: Vec<_> = crate::static_ground_pool::mission_prop_ground()
        .iter()
        .flat_map(|prop| prop.triangles_near(world_minimum, world_maximum).map(|triangle| triangle.gp))
        .collect();
    ripper.rip_host_triangles(&host, &mut |batch| run_batch(run, output, report, &mut run_error, batch));
    if let Some(source) = run_error.take() {
        return Err(SimulationStaticGroundJobError::Run {
            assembly: assembly_ordinal,
            source,
        });
    }
    // Unconditional final Run, even with zero meshes or an empty triangle cache.
    ripper.finish(|batch| run_batch(run, output, report, &mut run_error, batch));
    if let Some(source) = run_error {
        return Err(SimulationStaticGroundJobError::Run {
            assembly: assembly_ordinal,
            source,
        });
    }
    Ok(())
}

fn run_batch(
    run: &SimulationGroundAssemblyRun<'_, '_>,
    output: &mut SimulationGroundContactBuffer,
    report: &mut SimulationStaticGroundJobReport,
    run_error: &mut Option<SimulationGroundRunError>,
    batch: AssemblyGroundTriangleBatch<'_, AssemblyGroundTriangle>,
) {
    if run_error.is_some() {
        return;
    }
    report.intersection_runs += 1;
    report.submitted_triangles += batch.len();
    match run.try_run(batch, output) {
        Ok(SimulationGroundRunOutcome::Complete) => {}
        Ok(SimulationGroundRunOutcome::ContactReservationUnavailable) => {
            report.unavailable_runs += 1;
        }
        Err(error) => *run_error = Some(error),
    }
}

/// Native stores/reloads THREE u16 fields at `0x827cdaf0..0x827cdb64`.
/// Preserve wrapped-zero counts; they still occupy a collected descriptor.
pub(super) fn native_ground_descriptor(
    range: KdLeafRange,
    maximum_cluster_count: u32,
) -> KdLeafRange {
    let shift = if maximum_cluster_count <= 0x1_0000 {
        16
    } else {
        20
    };
    let offset = range.encoded_start & 0xffff;
    let cluster = (range.encoded_start >> shift) & 0xffff;
    KdLeafRange {
        encoded_start: (cluster << shift) | offset,
        unit_count: range.unit_count & 0xffff,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_test_descriptors_narrow_offset_cluster_and_count_without_dropping_zero() {
        let range = KdLeafRange {
            encoded_start: 0x123a_bcde,
            unit_count: 0x1_0000,
        };
        assert_eq!(
            native_ground_descriptor(range, 65536),
            KdLeafRange {
                encoded_start: 0x123a_bcde,
                unit_count: 0
            }
        );
        assert_eq!(
            native_ground_descriptor(range, 65537),
            KdLeafRange {
                encoded_start: 0x1230_bcde,
                unit_count: 0
            }
        );
        assert_eq!(
            native_ground_descriptor(
                KdLeafRange {
                    encoded_start: 0xffff_ffff,
                    unit_count: 0xffff_ffff
                },
                u32::MAX
            ),
            KdLeafRange {
                encoded_start: 0xfff0_ffff,
                unit_count: 65535
            }
        );
    }
}
