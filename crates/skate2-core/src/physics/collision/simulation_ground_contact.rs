//! CPU Assembly/ground Contact producer, `0x827cd1e0` and `0x827cdb98`.
//!
//! This connects retained `VolumeData`, live occurrence-qualified bodies, actual
//! primitive/triangle narrow phase and the recovered sixteen-row compositor.
//! Host arithmetic is ordinary-finite, not a Xenon estimate/FPSCR oracle.
//! Collection scheduling remains the caller's responsibility. In particular,
//! an unavailable local reservation ends one Run, not triangle ripping.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "ground producer awaiting outer collision-frame scheduling"
    )
)]

use super::{
    ContactBodyToken, ContactVector, SimulationAssemblyGroundContactEstimateSite,
    SimulationAssemblyGroundContactInput, SimulationAssemblyGroundContactNumericBackend,
    SimulationAssemblyGroundMaterialInput, SimulationContactBuildInput,
    SimulationContactCommonBodyInput, SolverDeltaRowToken,
    build_assembly_ground_contact_input_with_material, select_assembly_ground_material,
};
use crate::{
    authored_collision_publication::AuthoredCollisionPublication,
    narrow_phase::{GpInstanceTags, GpPrimitive, try_sk8_compute_contact_points},
    octree::AabbWords,
    serialised_assembly_bbox::{
        HostSerialisedAssemblyBBoxNumeric, SerialisedAssemblyBBoxMinMaxNumeric,
        SerialisedAssemblyBodyMotion, SerialisedAssemblyPartBBoxNumeric,
        try_calculate_primitive_part_bbox,
    },
    serialised_assembly_create::PreparedSerialisedAssembly,
    serialised_assembly_template::SerialisedAssemblyVolumeDataState,
    simulation_assembly_ground_run::{
        AssemblyGroundRunPaddingNumeric, try_assembly_ground_effective_padding,
    },
    simulation_assembly_ground_triangle_cache::AssemblyGroundTriangleBatch,
    simulation_assembly_ground_triangle_topology::{
        AssemblyGroundTriangle, AssemblyGroundTriangleSource,
    },
    simulation_contact_append::{
        AssemblyGroundContactWriteOutcome, SimulationGroundContactBuffer,
        SimulationGroundContactBufferError,
    },
    simulation_rigid_body_pool::{
        SimulationRigidBodyDynamicUpdatePhase, SimulationRigidBodyDynamicUpdatePhaseIdentity,
        SimulationRigidBodyFrameIdentity, SimulationRigidBodyNetworkContactSnapshot,
        SimulationRigidBodyObservation, SimulationRigidBodyPoolAuthority,
        SimulationRigidBodyPoolError,
    },
    simulation_xenon_address_image::{
        SimulationPackedBodyWordImageDomain, SimulationXenonAddressImageError,
    },
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable, VolumeType};
use std::convert::Infallible;
use std::fmt;

#[path = "simulation_assembly_ground_job.rs"]
pub(crate) mod cpu_job;
#[path = "simulation_dmo_dmo_contact.rs"]
pub(crate) mod dmo_dmo;
#[path = "simulation_skater_dmo_contact.rs"]
pub(crate) mod skater_dmo;

#[cfg(test)]
#[path = "simulation_ground_contact_corpus.rs"]
pub(crate) mod corpus;

const GENERATOR_PADDING: f32 = f32::from_bits(0x3f00_0000);
const CONVEXITY_EPSILON: f32 = f32::from_bits(0x3c23_d70a);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationContactTargetSource {
    Triangle(AssemblyGroundTriangleSource),
    Primitive {
        assembly_ordinal: usize,
        volume_ordinal: usize,
    },
}

impl From<AssemblyGroundTriangleSource> for SimulationContactTargetSource {
    fn from(source: AssemblyGroundTriangleSource) -> Self {
        Self::Triangle(source)
    }
}

#[cfg(test)]
impl SimulationContactTargetSource {
    fn triangle(self) -> AssemblyGroundTriangleSource {
        match self {
            Self::Triangle(source) => source,
            Self::Primitive { .. } => panic!("expected triangle contact provenance"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationGroundContactSource {
    pub(super) assembly_ordinal: usize,
    pub(super) volume_ordinal: usize,
    pub(super) target: SimulationContactTargetSource,
    pub(super) point_ordinal: usize,
}

/// Inert rows plus four independent body-role observations. No public caller
/// can construct this record or turn its binding words into RF authority.
#[derive(Clone, Copy)]
pub(crate) struct SimulationGroundContactRecord {
    pub(super) input: SimulationContactBuildInput,
    pub(super) table_a: SimulationRigidBodyObservation,
    pub(super) table_b: SimulationRigidBodyObservation,
    pub(super) rf_a: SimulationRigidBodyObservation,
    pub(super) rf_b: SimulationRigidBodyObservation,
    pub(super) source: SimulationGroundContactSource,
    pub(super) frame: SimulationRigidBodyFrameIdentity,
    pub(super) phase: SimulationRigidBodyDynamicUpdatePhaseIdentity,
    pub(super) pool: SimulationRigidBodyPoolAuthority,
}

impl SimulationGroundContactRecord {
    /// `Step_Solver2`'s `ThawAsNecessary` activates bodies after the jobs
    /// built their Contacts and before the solve; the rows stay as built and
    /// follow the same frame into the integration phase prepared afterwards.
    pub(crate) fn rebind_phase(&mut self, phase: &SimulationRigidBodyDynamicUpdatePhase<'_>) {
        debug_assert_eq!(self.frame, phase.frame_identity());
        self.phase = phase.phase_identity();
    }

    /// Read-only host diagnostic for a completed ground producer row. The
    /// observations remain occurrence-bound and grant no mutation authority.
    pub(crate) fn diagnostic_binding(
        &self,
    ) -> (
        usize,
        usize,
        usize,
        SimulationRigidBodyObservation,
        SimulationRigidBodyObservation,
    ) {
        (
            self.source.assembly_ordinal,
            self.source.volume_ordinal,
            self.source.point_ordinal,
            self.table_a,
            self.table_b,
        )
    }
}

#[derive(Debug)]
pub(crate) enum SimulationGroundRunError {
    ForeignImage,
    StaleAssemblyBounds,
    BodyIndex { volume: usize, index: u16 },
    Body(SimulationRigidBodyPoolError),
    Address(SimulationXenonAddressImageError),
    UnsupportedVolume { volume: usize },
    MissingGroundBody,
    MissingExtra { volume: usize },
    CullingWord { index: u32 },
    BindingOverflow,
    Allocation,
    Storage(SimulationGroundContactBufferError),
}

impl fmt::Display for SimulationGroundRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignImage => {
                f.write_str("ground assembly uses another Simulation image generation")
            }
            Self::StaleAssemblyBounds => {
                f.write_str("ground assembly bounds were not published for this body frame")
            }
            Self::BodyIndex { volume, index } => {
                write!(f, "ground volume {volume} has invalid body index {index}")
            }
            Self::Body(error) => write!(f, "ground body observation: {error}"),
            Self::Address(error) => write!(f, "ground packed motion address: {error}"),
            Self::UnsupportedVolume { volume } => {
                write!(f, "ground volume {volume} has no supported primitive")
            }
            Self::MissingGroundBody => f.write_str("ground body is null"),
            Self::MissingExtra { volume } => {
                write!(f, "ground volume {volume} has no initialized Extra record")
            }
            Self::CullingWord { index } => {
                write!(f, "ground culling bit {index} lies outside stored words")
            }
            Self::BindingOverflow => f.write_str("ground body binding word overflow"),
            Self::Allocation => f.write_str("ground Extra allocation failed"),
            Self::Storage(error) => write!(f, "ground Contact storage: {error:?}"),
        }
    }
}

/// Native early return is deliberately distinct from a typed host failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationGroundRunOutcome {
    Complete,
    ContactReservationUnavailable,
}

/// Closed provenance alternatives, never an arbitrary-record constructor.
/// The serialized branch retains its mandatory image validation; authored
/// publication proves real session/geometry ownership without native pointers.
#[derive(Clone, Copy)]
pub(crate) enum GroundAssemblyPublication<'a, 'image> {
    Serialized {
        assembly: &'a PreparedSerialisedAssembly,
        image: &'image SimulationPackedBodyWordImageDomain,
    },
    Authored(AuthoredCollisionPublication<'a>),
}

impl<'a> GroundAssemblyPublication<'a, '_> {
    fn bounds(self) -> AabbWords {
        match self {
            Self::Serialized { assembly, .. } => assembly.header().aabb,
            Self::Authored(assembly) => assembly.bounds(),
        }
    }

    fn records(self) -> &'a [SerialisedAssemblyVolumeDataState] {
        match self {
            Self::Serialized { assembly, .. } => assembly.volume_data().records(),
            Self::Authored(assembly) => assembly.records(),
        }
    }

    fn bodies(self) -> &'a [SimulationRigidBodyObservation] {
        match self {
            Self::Serialized { assembly, .. } => assembly.rigid_bodies(),
            Self::Authored(assembly) => assembly.rigid_bodies(),
        }
    }

    fn all_static(self) -> bool {
        match self {
            Self::Serialized { assembly, .. } => {
                assembly.header().num_static == assembly.header().num_rigid_bodies
            }
            Self::Authored(assembly) => {
                assembly.num_static() as usize == assembly.rigid_bodies().len()
            }
        }
    }

    fn frozen(self) -> bool {
        match self {
            Self::Serialized { assembly, .. } => assembly.header().all_parts_frozen,
            Self::Authored(assembly) => assembly.all_parts_frozen(),
        }
    }

    fn group(self) -> u32 {
        match self {
            Self::Serialized { assembly, .. } => assembly.header().group,
            Self::Authored(assembly) => assembly.group(),
        }
    }

    fn self_culling(self) -> Option<&'a BitTable> {
        match self {
            Self::Serialized { assembly, .. } => {
                assembly.header().culling_table.as_ref().map(|c| c.table())
            }
            Self::Authored(assembly) => Some(assembly.culling_table()),
        }
    }
}

#[derive(Clone, Copy)]
struct ExtraVolumeData {
    live: SimulationRigidBodyObservation,
    table: SimulationRigidBodyObservation,
    bounds: AabbWords,
}

/// One processed CPU assembly, retaining the exclusive phase's immutable view.
/// Construction performs the job's Extra/BBox pass, including disabled, frozen,
/// and static non-Aggregate volumes. Aggregate slots deliberately stay absent.
pub(crate) struct SimulationGroundAssemblyRun<'a, 'pool> {
    volumes: &'a [SerialisedAssemblyVolumeDataState],
    bend_threshold: f32,
    assembly_ordinal: usize,
    phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
    extra: Vec<Option<ExtraVolumeData>>,
    ground: Option<SimulationRigidBodyObservation>,
    ground_group: u32,
    pipeline_culling: Option<&'a BitTable>,
    is_object: bool,
    draw_contacts: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct SimulationGroundRunSettings<'a> {
    pub(crate) ground: Option<SimulationRigidBodyObservation>,
    pub(crate) ground_group: u32,
    /// JobInput.cullingTable, never the assembly's self-culling table.
    pub(crate) pipeline_culling: Option<&'a BitTable>,
    pub(crate) is_object: bool,
    pub(crate) draw_contacts: bool,
}

impl<'a, 'pool> SimulationGroundAssemblyRun<'a, 'pool> {
    /// CPU job preparation `0x827cdcc8..0x827cde60`. `None` is an assembly
    /// skipped by all-frozen or all-static header tests, before Extra reads.
    pub(crate) fn try_new(
        assembly: &'a PreparedSerialisedAssembly,
        assembly_ordinal: usize,
        phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
        image: &SimulationPackedBodyWordImageDomain,
        settings: SimulationGroundRunSettings<'a>,
    ) -> Result<Option<Self>, SimulationGroundRunError> {
        Self::try_from_publication(
            GroundAssemblyPublication::Serialized { assembly, image },
            assembly_ordinal,
            phase,
            settings,
            true,
        )
    }

    fn try_from_publication(
        publication: GroundAssemblyPublication<'a, '_>,
        assembly_ordinal: usize,
        phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
        settings: SimulationGroundRunSettings<'a>,
        skip_frozen: bool,
    ) -> Result<Option<Self>, SimulationGroundRunError> {
        let (records, bodies, bend_threshold) = match publication {
            GroundAssemblyPublication::Serialized { assembly, image } => {
                let header = assembly.header();
                if (skip_frozen && header.all_parts_frozen)
                    || header.num_static == header.num_rigid_bodies
                {
                    return Ok(None);
                }
                if !phase.ground_image_matches_pool(image) || !assembly.matches_ground_image(image)
                {
                    return Err(SimulationGroundRunError::ForeignImage);
                }
                if !assembly.has_bounds_for_frame(phase.frame_identity()) {
                    return Err(SimulationGroundRunError::StaleAssemblyBounds);
                }
                (
                    assembly.volume_data().records(),
                    assembly.rigid_bodies(),
                    assembly.params().edge_cos_bend_normal_threshold,
                )
            }
            GroundAssemblyPublication::Authored(assembly) => {
                if (skip_frozen && assembly.all_parts_frozen())
                    || assembly.num_static() as usize == assembly.rigid_bodies().len()
                {
                    return Ok(None);
                }
                if assembly.body_pool_authority() != phase.body_pool_authority() {
                    return Err(SimulationGroundRunError::ForeignImage);
                }
                if !assembly.has_bounds_for_frame(phase.frame_identity()) {
                    return Err(SimulationGroundRunError::StaleAssemblyBounds);
                }
                (
                    assembly.records(),
                    assembly.rigid_bodies(),
                    assembly.bend_threshold(),
                )
            }
        };
        let extra = prepare_volume_extras(records, bodies, phase, publication)?;
        Ok(Some(Self {
            volumes: records,
            bend_threshold,
            assembly_ordinal,
            phase,
            extra,
            ground: settings.ground,
            ground_group: settings.ground_group,
            pipeline_culling: settings.pipeline_culling,
            is_object: settings.is_object,
            draw_contacts: settings.draw_contacts,
        }))
    }

    /// Full triangle-major, volume-major, point-major Run at `0x827cd1e0`.
    /// No cache reset or tail flush belongs inside this invocation.
    pub(crate) fn try_run(
        &self,
        triangles: AssemblyGroundTriangleBatch<'_, AssemblyGroundTriangle>,
        output: &mut SimulationGroundContactBuffer,
    ) -> Result<SimulationGroundRunOutcome, SimulationGroundRunError> {
        let trace_wheels =
            self.assembly_ordinal == 0 && std::env::var_os("SKATE2_TRACE_WHEEL_GROUND").is_some();
        for (triangle_ordinal, triangle) in triangles.iter().enumerate() {
            for (ordinal, volume) in self.volumes.iter().enumerate() {
                let trace = trace_wheels && ordinal < 4;
                if trace {
                    eprintln!(
                        "wheel-ground triangle={triangle_ordinal} volume={ordinal} frozen={} aggregate={} enabled={} group={} ground-group={}",
                        volume.frozen,
                        volume.volume_type == VolumeType::Aggregate,
                        volume.enabled,
                        volume.group,
                        self.ground_group,
                    );
                }
                if volume.frozen || volume.volume_type == VolumeType::Aggregate {
                    continue;
                }
                let culled = is_culled(self.pipeline_culling, volume.group, self.ground_group)?;
                if trace {
                    eprintln!(
                        "wheel-ground triangle={triangle_ordinal} volume={ordinal} culled={culled}"
                    );
                }
                if culled {
                    continue;
                }
                if self.try_volume(triangle, ordinal, volume, output, trace, triangle_ordinal)?
                    == SimulationGroundRunOutcome::ContactReservationUnavailable
                {
                    return Ok(SimulationGroundRunOutcome::ContactReservationUnavailable);
                }
            }
        }
        Ok(SimulationGroundRunOutcome::Complete)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "keep the source-ordered gates and reserve-before-compose path adjacent"
    )]
    fn try_volume(
        &self,
        triangle: &AssemblyGroundTriangle,
        ordinal: usize,
        volume: &SerialisedAssemblyVolumeDataState,
        output: &mut SimulationGroundContactBuffer,
        trace: bool,
        triangle_ordinal: usize,
    ) -> Result<SimulationGroundRunOutcome, SimulationGroundRunError> {
        let extra = self.extra[ordinal]
            .ok_or(SimulationGroundRunError::MissingExtra { volume: ordinal })?;
        let volume_material = material_input(volume.material);
        // staticMaterial image at 0x82fef110: mus=0, mud=0, restitution=1.
        let ground_material = SimulationAssemblyGroundMaterialInput {
            static_friction: 0.0,
            dynamic_friction: 0.0,
            restitution: 1.0,
        };
        let selected_material = select_assembly_ground_material(volume_material, ground_material);
        let ground = self
            .ground
            .ok_or(SimulationGroundRunError::MissingGroundBody)?;
        self.phase
            .try_validate_contact_observation(ground)
            .map_err(SimulationGroundRunError::Body)?;
        let body_state = self
            .phase
            .try_raw_state_for_constraint(extra.live)
            .map_err(SimulationGroundRunError::Body)?;
        if trace {
            eprintln!(
                "wheel-ground triangle={triangle_ordinal} volume={ordinal} body-state={body_state} body={:?}",
                extra.live
            );
        }
        if body_state == 1 {
            return Ok(SimulationGroundRunOutcome::Complete);
        }
        let gp = GpPrimitive::from_volume(&volume.volume, Some(volume.transform))
            .ok_or(SimulationGroundRunError::UnsupportedVolume { volume: ordinal })?;
        if !volume.enabled && self.extra.len() != 1 {
            return Ok(SimulationGroundRunOutcome::Complete);
        }
        let overlaps = triangle_overlaps(triangle.primitive().points, extra.bounds);
        if trace {
            eprintln!(
                "wheel-ground triangle={triangle_ordinal} volume={ordinal} overlaps={overlaps} bounds={:?} triangle-points={:?} center={:?} radius={}",
                extra.bounds,
                triangle.primitive().points,
                [
                    volume.transform[12],
                    volume.transform[13],
                    volume.transform[14]
                ],
                volume.volume.radius,
            );
        }
        if !overlaps {
            return Ok(SimulationGroundRunOutcome::Complete);
        }
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
        if trace {
            eprintln!(
                "wheel-ground triangle={triangle_ordinal} volume={ordinal} velocity={velocity:?} padding={padding:?}"
            );
        }
        // These scratch tags are not consumed by Run's compositor. Zero is an
        // explicit inert host tag, not a fabricated native Volume pointer.
        let points = try_sk8_compute_contact_points(
            &gp,
            GpInstanceTags::new(0, 0),
            triangle.primitive(),
            GpInstanceTags::new(triangle.volume_tag(), triangle.user_tag()),
            padding,
            self.bend_threshold,
            CONVEXITY_EPSILON,
            self.is_object,
            self.draw_contacts,
        );
        if trace {
            eprintln!(
                "wheel-ground triangle={triangle_ordinal} volume={ordinal} contact-points={}",
                points
                    .as_ref()
                    .map_or(0, |points| points.point_pairs().len())
            );
        }
        let Some(points) = points else {
            return Ok(SimulationGroundRunOutcome::Complete);
        };
        let table_a = binding_word(extra.table)?;
        let table_b = binding_word(ground)?;
        let rf_a = binding_word(extra.live)?;
        let rf_b = binding_word(ground)?;
        self.phase
            .try_validate_contact_observation(extra.table)
            .map_err(SimulationGroundRunError::Body)?;
        for (point_ordinal, pair) in points.point_pairs().iter().enumerate() {
            let outcome = output
                .try_reserve_then_write(|| {
                    // Observation preflight plus immutable exclusive phase guarantees
                    // these post-reservation reads cannot become stale mid-Run.
                    let a = self
                        .phase
                        .try_contact_body_snapshot(extra.live)
                        .expect("validated ground volume body");
                    let b = self
                        .phase
                        .try_contact_body_snapshot(ground)
                        .expect("validated ground static body");
                    let input = infallible(build_assembly_ground_contact_input_with_material(
                        &SimulationAssemblyGroundContactInput {
                            body_a: common_body(a, SolverDeltaRowToken(rf_a)),
                            body_b: common_body(b, SolverDeltaRowToken(rf_b)),
                            position_a: pair.first(),
                            position_b: pair.second(),
                            normal: points.normal(),
                            table_body_a: ContactBodyToken(table_a),
                            table_body_b: ContactBodyToken(table_b),
                            volume_material,
                            ground_material,
                            volume_surface_id: volume.surface_id,
                            triangle_user_tag: triangle.user_tag(),
                        },
                        selected_material,
                        &mut HostGroundContactNumeric,
                    ));
                    SimulationGroundContactRecord {
                        input,
                        table_a: extra.table,
                        table_b: ground,
                        rf_a: extra.live,
                        rf_b: ground,
                        source: SimulationGroundContactSource {
                            assembly_ordinal: self.assembly_ordinal,
                            volume_ordinal: ordinal,
                            target: triangle.source().into(),
                            point_ordinal,
                        },
                        frame: self.phase.frame_identity(),
                        phase: self.phase.phase_identity(),
                        pool: self.phase.body_pool_authority(),
                    }
                })
                .map_err(SimulationGroundRunError::Storage)?;
            if outcome == AssemblyGroundContactWriteOutcome::Unavailable {
                return Ok(SimulationGroundRunOutcome::ContactReservationUnavailable);
            }
        }
        Ok(SimulationGroundRunOutcome::Complete)
    }
}

fn infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(never) => match never {},
    }
}

fn binding_word(body: SimulationRigidBodyObservation) -> Result<u32, SimulationGroundRunError> {
    body.identity()
        .id()
        .index()
        .checked_add(1)
        .ok_or(SimulationGroundRunError::BindingOverflow)
}

fn material_input(material: AssemblyPartMaterial) -> SimulationAssemblyGroundMaterialInput {
    SimulationAssemblyGroundMaterialInput {
        static_friction: material.static_friction,
        dynamic_friction: material.dynamic_friction,
        restitution: material.restitution,
    }
}

// Both CPU jobs prepare non-Aggregate volumes using the same live body and
// CalculatePartBBox path. Assembly skip rules remain with each job owner.
fn prepare_volume_extras(
    records: &[SerialisedAssemblyVolumeDataState],
    bodies: &[SimulationRigidBodyObservation],
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    publication: GroundAssemblyPublication<'_, '_>,
) -> Result<Vec<Option<ExtraVolumeData>>, SimulationGroundRunError> {
    let mut extra = Vec::new();
    extra
        .try_reserve_exact(records.len())
        .map_err(|_| SimulationGroundRunError::Allocation)?;
    let mut numeric = HostSerialisedAssemblyBBoxNumeric;
    for (ordinal, volume) in records.iter().enumerate() {
        if volume.volume_type == VolumeType::Aggregate {
            extra.push(None);
            continue;
        }
        let body = *bodies.get(usize::from(volume.rigid_body_index)).ok_or(
            SimulationGroundRunError::BodyIndex {
                volume: ordinal,
                index: volume.rigid_body_index,
            },
        )?;
        let motion = match publication {
            GroundAssemblyPublication::Serialized { image, .. } => body_motion(phase, image, body)?,
            GroundAssemblyPublication::Authored(_) => authored_body_motion(phase, body)?,
        };
        let bounds = infallible(try_calculate_primitive_part_bbox(
            &volume.volume,
            &volume.transform,
            motion,
            &mut numeric,
        ))
        .ok_or(SimulationGroundRunError::UnsupportedVolume { volume: ordinal })?;
        // CPU source independently loads both pointers from this same body
        // table entry. They retain separate semantic roles thereafter.
        extra.push(Some(ExtraVolumeData {
            live: body,
            table: body,
            bounds,
        }));
    }
    Ok(extra)
}

fn common_body(
    body: SimulationRigidBodyNetworkContactSnapshot,
    solver_delta: SolverDeltaRowToken,
) -> SimulationContactCommonBodyInput {
    SimulationContactCommonBodyInput {
        center_of_mass: body.center_of_mass,
        solver_delta,
        velocity: body.velocity,
        omega: body.omega,
        inertia_full: body.inverse_inertia_full,
        inverse_mass: body.inverse_mass,
        inertia_split: body.inverse_inertia_split,
        state_word: body.state,
        force: body.force,
        kinematic_value: body.kinematic_value,
        torque: body.torque,
        cooldown_word: body.cooldown,
    }
}

/// Only absent pointer-shaped motion W lanes use zero in this separate
/// semantic-XYZ profile. Real kine/cool remain intact. Independent source
/// audit proved no W-to-XYZ/control dependency in bounds/ground overlap.
fn authored_body_motion(
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    observation: SimulationRigidBodyObservation,
) -> Result<SerialisedAssemblyBodyMotion, SimulationGroundRunError> {
    let body = phase
        .try_contact_body_snapshot(observation)
        .map_err(SimulationGroundRunError::Body)?;
    Ok(SerialisedAssemblyBodyMotion {
        velocity_and_right: super::pack_xyz_word(body.velocity, 0.0),
        omega_and_left: super::pack_xyz_word(body.omega, 0.0),
        force_and_kine: super::pack_xyz_word(body.force, body.kinematic_value),
        torque_and_cool: super::pack_xyz_word(body.torque, f32::from_bits(body.cooldown)),
    })
}

fn body_motion(
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    image: &SimulationPackedBodyWordImageDomain,
    observation: SimulationRigidBodyObservation,
) -> Result<SerialisedAssemblyBodyMotion, SimulationGroundRunError> {
    let body = phase
        .try_contact_body_snapshot(observation)
        .map_err(SimulationGroundRunError::Body)?;
    let topology = phase
        .try_ground_motion_topology(observation)
        .map_err(SimulationGroundRunError::Body)?;
    let right = image
        .rigid_body_node_address(topology.right_link)
        .map_err(SimulationGroundRunError::Address)?;
    let left = image
        .rigid_body_node_address(topology.left_link)
        .map_err(SimulationGroundRunError::Address)?;
    Ok(SerialisedAssemblyBodyMotion {
        velocity_and_right: super::pack_xyz_word(body.velocity, f32::from_bits(right)),
        omega_and_left: super::pack_xyz_word(body.omega, f32::from_bits(left)),
        force_and_kine: super::pack_xyz_word(body.force, body.kinematic_value),
        torque_and_cool: super::pack_xyz_word(body.torque, f32::from_bits(body.cooldown)),
    })
}

fn is_culled(
    table: Option<&BitTable>,
    volume_group: u32,
    ground_group: u32,
) -> Result<bool, SimulationGroundRunError> {
    let Some(table) = table else {
        return Ok(false);
    };
    let index = table
        .columns
        .wrapping_mul(volume_group)
        .wrapping_add(ground_group);
    let word = table
        .words
        .get((index >> 5) as usize)
        .ok_or(SimulationGroundRunError::CullingWord { index })?;
    Ok(word & (1 << (index & 31)) != 0)
}

fn triangle_overlaps(points: [[f32; 3]; 3], bounds: AabbWords) -> bool {
    let points = points.map(|p| super::pack_xyz_word(p, 0.0));
    let mut numeric = HostSerialisedAssemblyBBoxNumeric;
    let min12 = infallible(numeric.minimum(points[1], points[2]));
    let max12 = infallible(numeric.maximum(points[1], points[2]));
    let minimum = infallible(numeric.minimum(points[0], min12));
    let maximum = infallible(numeric.maximum(points[0], max12));
    let lo = bounds.min_f32();
    let hi = bounds.max_f32();
    !(0..3).any(|axis| minimum[axis] > hi[axis] || lo[axis] > maximum[axis])
}

struct HostGroundPadding;
impl AssemblyGroundRunPaddingNumeric for HostGroundPadding {
    type Error = Infallible;
    fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> Result<f32, Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.sum_three_products(
            super::pack_xyz_word(left, 0.0),
            super::pack_xyz_word(right, 0.0),
        )
    }
}

struct HostGroundContactNumeric;
impl SimulationAssemblyGroundContactNumericBackend for HostGroundContactNumeric {
    type Error = Infallible;
    fn reciprocal_sqrt_estimate(
        &mut self,
        _site: SimulationAssemblyGroundContactEstimateSite,
        operand: ContactVector,
    ) -> Result<ContactVector, Self::Error> {
        Ok(operand.map(|value| value.sqrt().recip()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        simulation_assembly_ground_triangle_rip::{AssemblyGroundTriangleRipper, tests::fixture},
        simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyPool},
    };
    use skate2_formats::collision::{KdLeafRange, Volume};
    use std::sync::Arc;

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    #[test]
    fn cpu_preparation_uses_retained_template_and_refuses_replacement_image_generation() {
        use crate::{
            serialised_assembly_create::{
                SerialisedAssemblyCreatePartInput, SerialisedAssemblyLiveAssemblyInput,
                SerialisedAssemblyParamsState, try_prepare_serialised_assembly_create_from_xenon,
            },
            serialised_assembly_template::{SerialisedAssemblyTemplate, tests::source_plan},
            simulation_xenon_address_image::{
                SimulationArenaXenonAddressBinding, SimulationRigidBodyAddInertiaInput,
                SimulationRigidBodyAddInertiaOwner, SimulationXenonAddressImage,
            },
        };
        use skate2_formats::pegasus::ObjectId;
        let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let template =
            SerialisedAssemblyTemplate::try_new(Arc::clone(&owner), ObjectId(3)).unwrap();
        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let ground = pool.try_add_rigid_body(IDENTITY, None, 1).unwrap().unwrap();
        let ground = pool.try_observe(ground).unwrap();
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x2000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let mut inertia_owner =
            SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
        let runtime_inertia = template.runtime_inertia(0).unwrap();
        let body = inertia_owner
            .try_add(
                &mut pool,
                IDENTITY,
                SimulationRigidBodyAddInertiaInput::Definition(&runtime_inertia),
                4,
            )
            .unwrap()
            .unwrap();
        let parts = [SerialisedAssemblyCreatePartInput {
            body: body.observation(),
            part_transform: IDENTITY,
            material: sphere(0).material,
            user_data: 0,
        }];
        let raw_image = SimulationXenonAddressImage::try_new(0x1000_0080, 2, 0, 0).unwrap();
        let image = SimulationPackedBodyWordImageDomain::try_new(&pool, raw_image).unwrap();
        let replacement = SimulationPackedBodyWordImageDomain::try_new(&pool, raw_image).unwrap();
        let prepared = try_prepare_serialised_assembly_create_from_xenon(
            &template,
            &pool,
            &inertia_owner,
            &image,
            &parts,
            SerialisedAssemblyLiveAssemblyInput {
                group: 4,
                culling_table: template.culling_table(),
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut HostSerialisedAssemblyBBoxNumeric,
        )
        .unwrap();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let settings = || SimulationGroundRunSettings {
            ground: Some(ground),
            ground_group: 0,
            pipeline_culling: None,
            is_object: false,
            draw_contacts: false,
        };
        let run = SimulationGroundAssemblyRun::try_new(&prepared, 3, &phase, &image, settings())
            .unwrap()
            .unwrap();
        assert_eq!(run.extra.len(), prepared.volume_data().records().len());
        assert_eq!(run.extra[0].unwrap().live, body.observation());
        assert_eq!(run.extra[0].unwrap().table, body.observation());
        assert!(matches!(
            SimulationGroundAssemblyRun::try_new(&prepared, 3, &phase, &replacement, settings()),
            Err(SimulationGroundRunError::ForeignImage)
        ));
    }

    fn bodies() -> (
        SimulationRigidBodyPool,
        SimulationRigidBodyObservation,
        SimulationRigidBodyObservation,
    ) {
        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let ground = pool.try_add_rigid_body(IDENTITY, None, 1).unwrap().unwrap();
        let inertia = Arc::new(SimulationInertia::native_default());
        let body = pool
            .try_add_rigid_body(IDENTITY, Some(&inertia), 4)
            .unwrap()
            .unwrap();
        let ground = pool.try_observe(ground).unwrap();
        let body = pool.try_observe(body).unwrap();
        (pool, body, ground)
    }

    fn sphere(surface_id: u32) -> SerialisedAssemblyVolumeDataState {
        let mut transform = IDENTITY;
        transform[13] = 0.5;
        SerialisedAssemblyVolumeDataState {
            rigid_body_index: 0,
            volume_type: VolumeType::Sphere,
            volume: Volume {
                transform,
                vtable_offset: VolumeType::Sphere as u32,
                shape_words: [0; 3],
                radius: 1.0,
                group_id: 0,
                surface_id,
                flags: 1,
            },
            padding: 0.0,
            material: AssemblyPartMaterial {
                static_friction: 0.8,
                dynamic_friction: 0.4,
                restitution: 0.2,
            },
            surface_id,
            transform: IDENTITY,
            enabled: true,
            group: 0,
            frozen: false,
        }
    }

    fn run_fixture<'a, 'pool>(
        volumes: &'a [SerialisedAssemblyVolumeDataState],
        phase: &'a SimulationRigidBodyDynamicUpdatePhase<'pool>,
        body: SimulationRigidBodyObservation,
        ground: Option<SimulationRigidBodyObservation>,
    ) -> SimulationGroundAssemblyRun<'a, 'pool> {
        SimulationGroundAssemblyRun {
            volumes,
            bend_threshold: f32::from_bits(0x3f7f_be77),
            assembly_ordinal: 7,
            phase,
            extra: volumes
                .iter()
                .map(|_| {
                    Some(ExtraVolumeData {
                        live: body,
                        table: body,
                        bounds: AabbWords::from_f32([-2.0; 4], [2.0; 4]),
                    })
                })
                .collect(),
            ground,
            ground_group: 0,
            pipeline_culling: None,
            is_object: false,
            draw_contacts: false,
        }
    }

    fn rip_repeated(
        run: &SimulationGroundAssemblyRun<'_, '_>,
        count: u16,
        output: &mut SimulationGroundContactBuffer,
    ) -> Vec<Result<SimulationGroundRunOutcome, SimulationGroundRunError>> {
        let units = [0x01, 0, 1, 2].repeat(usize::from(count));
        let (mesh, bytes) = fixture(
            &[[-10.0, 0.0, -10.0], [0.0, 0.0, 10.0], [10.0, 0.0, -10.0]],
            count,
            &units,
        );
        let mut ripper = AssemblyGroundTriangleRipper::new([-2.0; 3], [2.0; 3], false, false);
        let mut outcomes = Vec::new();
        ripper
            .rip_leaf_range(
                &mesh,
                &bytes,
                KdLeafRange {
                    encoded_start: 0,
                    unit_count: u32::from(count),
                },
                IDENTITY,
                &mut |_| {},
                &mut |batch| outcomes.push(run.try_run(batch, output)),
            )
            .unwrap();
        ripper.finish(|batch| outcomes.push(run.try_run(batch, output)));
        outcomes
    }

    #[test]
    fn real_gp_rip_narrow_phase_and_compositor_preserve_volume_and_point_order() {
        let (mut pool, body, ground) = bodies();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let volumes = [sphere(0x1234), sphere(0x5678)];
        let run = run_fixture(&volumes, &phase, body, Some(ground));
        let mut output = SimulationGroundContactBuffer::try_new(20, true).unwrap();
        let outcomes = rip_repeated(&run, 2, &mut output);
        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(outcome, Ok(SimulationGroundRunOutcome::Complete)))
        );
        output.try_finish_assembly().unwrap();
        let records = output.try_into_records().unwrap();
        assert_eq!(
            records.len(),
            4,
            "two actual sphere/triangle intersections per triangle"
        );
        for (ordinal, record) in records.iter().enumerate() {
            assert_eq!(record.source.assembly_ordinal, 7);
            assert_eq!(record.source.volume_ordinal, ordinal % 2);
            assert_eq!(
                record.source.target.triangle().unit_stream_offset(),
                (ordinal / 2) * 4
            );
            assert_eq!(record.source.point_ordinal, 0);
            assert_eq!((record.table_a, record.rf_a), (body, body));
            assert_eq!((record.table_b, record.rf_b), (ground, ground));
            let words = record.input.test_words();
            assert_eq!(words[0][3], binding_word(body).unwrap());
            assert_eq!(words[1][3], binding_word(ground).unwrap());
            assert_eq!(
                words[5][3] >> 16,
                if ordinal % 2 == 0 { 0x1234 } else { 0x5678 }
            );
            assert_eq!(words[2][3], 0.2_f32.to_bits());
            assert_eq!(words[3][3], 0.8_f32.to_bits());
            assert_eq!(words[4][3], 0.4_f32.to_bits());
        }
    }

    #[test]
    fn nonflush_exhaustion_ends_run_but_ripper_still_visits_later_batches() {
        let (mut pool, body, ground) = bodies();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let volumes = [sphere(3)];
        let run = run_fixture(&volumes, &phase, body, Some(ground));
        let mut output = SimulationGroundContactBuffer::try_new(100, false).unwrap();
        let outcomes = rip_repeated(&run, 81, &mut output);
        assert_eq!(outcomes.len(), 5);
        for (ordinal, outcome) in outcomes.into_iter().enumerate() {
            assert_eq!(
                outcome.unwrap(),
                if ordinal < 2 {
                    SimulationGroundRunOutcome::Complete
                } else {
                    SimulationGroundRunOutcome::ContactReservationUnavailable
                }
            );
        }
        output.try_finish_assembly().unwrap();
        let records = output.try_into_records().unwrap();
        assert_eq!(records.len(), 50);
        assert_eq!(
            records
                .last()
                .unwrap()
                .source
                .target
                .triangle()
                .unit_stream_offset(),
            49 * 4
        );
    }

    #[test]
    fn disabled_single_volume_runs_but_disabled_multiple_volumes_skip() {
        let (mut pool, body, ground) = bodies();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let mut disabled = sphere(3);
        disabled.enabled = false;
        for count in [1, 2] {
            let volumes = vec![disabled.clone(); count];
            let run = run_fixture(&volumes, &phase, body, Some(ground));
            let mut output = SimulationGroundContactBuffer::try_new(10, true).unwrap();
            assert!(rip_repeated(&run, 1, &mut output).pop().unwrap().is_ok());
            output.try_finish_assembly().unwrap();
            assert_eq!(
                output.try_into_records().unwrap().len(),
                usize::from(count == 1)
            );
        }
    }

    #[test]
    fn ripped_contacts_compile_solve_and_integrate_the_live_body() {
        use super::super::{
            HostContactNumericBackend,
            network_preparation::{
                SimulationContactIslandMapSource, SimulationManagerContactDomain,
                try_prepare_network_contact_island_batch_with_backend,
            },
        };
        use crate::{
            simulation_drive_jacobian::prepare_drive_horus_island_batch_for_test,
            simulation_drive_pool::SimulationDrivePool,
            simulation_horus_pipeline::try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite,
            simulation_island_map::SimulationRigidBodyIslandMapSnapshot,
            simulation_joint_pool::SimulationJointPool,
            simulation_joint_preparation::prepare_joint_horus_island_batch_for_test,
        };
        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let mut domain = SimulationManagerContactDomain::try_new(&mut pool, 20).unwrap();
        let ground = domain.static_body_observation();
        let body = pool
            .try_add_rigid_body(
                IDENTITY,
                Some(&Arc::new(SimulationInertia::native_default())),
                4,
            )
            .unwrap()
            .unwrap();
        let body = pool.try_observe(body).unwrap();
        let joint_pool = SimulationJointPool::try_new(0).unwrap();
        let drive_pool = SimulationDrivePool::try_new(0, &pool).unwrap();
        pool.begin_dynamic_update_frame(1.0 / 60.0);
        let map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &pool,
            pool.frame_identity().unwrap(),
            [(body, 0), (ground, -1)],
        )
        .unwrap();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let volumes = [sphere(0x1234)];
        let run = run_fixture(&volumes, &phase, body, Some(ground));
        let mut output = SimulationGroundContactBuffer::try_new(domain.maximum(), true).unwrap();
        assert!(
            rip_repeated(&run, 1, &mut output)
                .into_iter()
                .all(|outcome| matches!(outcome, Ok(SimulationGroundRunOutcome::Complete)))
        );
        output.try_finish_assembly().unwrap();
        let records = output.try_into_records().unwrap();
        assert_eq!(records.len(), 1);
        let mut open = domain.try_begin_contact_frame(&phase).unwrap();
        assert_eq!(open.maximum(), 20);
        assert_eq!(open.static_body_observation(), ground);
        open.try_append_ground_contact_records(&records, &phase)
            .unwrap();
        assert_eq!(open.len(), 1);
        let sealed = open
            .try_seal(&phase)
            .unwrap_or_else(|_| panic!("sealed ground frame"));
        let mut contacts = try_prepare_network_contact_island_batch_with_backend(
            sealed,
            &phase,
            SimulationContactIslandMapSource::Present(&map),
            &mut HostContactNumericBackend,
        )
        .unwrap_or_else(|_| panic!("compiled ground frame"));
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
        let body_state = pool.try_serialised_assembly_snapshot(body).unwrap();
        assert!(
            body_state
                .center_of_mass
                .iter()
                .all(|value| value.is_finite())
        );
        assert!(
            body_state.center_of_mass[1] > 0.0,
            "penetrating sphere receives upward correction: {:?}",
            body_state.center_of_mass
        );
        pool.try_with_solver_delta_row(body, |rows| assert_eq!(*rows, [[0.0; 4]; 4]))
            .unwrap();
    }

    #[test]
    fn assembly_tail_flush_resets_nonflush_buffer_and_global_capacity_keeps_dense_prefix() {
        let (mut pool, body, ground) = bodies();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let volumes = [sphere(3)];
        let run = run_fixture(&volumes, &phase, body, Some(ground));
        let mut output = SimulationGroundContactBuffer::try_new(53, false).unwrap();
        for _ in 0..2 {
            let outcomes = rip_repeated(&run, 30, &mut output);
            assert!(
                outcomes
                    .into_iter()
                    .all(|outcome| matches!(outcome, Ok(SimulationGroundRunOutcome::Complete)))
            );
            output.try_finish_assembly().unwrap();
        }
        let records = output.try_into_records().unwrap();
        assert_eq!(records.len(), 53);
        assert_eq!(
            records[29].source.target.triangle().unit_stream_offset(),
            29 * 4
        );
        assert_eq!(records[30].source.target.triangle().unit_stream_offset(), 0);
        assert_eq!(
            records[52].source.target.triangle().unit_stream_offset(),
            22 * 4
        );
    }

    #[test]
    fn frozen_aggregate_and_culling_gates_do_not_read_absent_extra_or_ground() {
        let (mut pool, body, _) = bodies();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        for gate in 0..3 {
            let mut volume = sphere(3);
            volume.frozen = gate == 0;
            if gate == 1 {
                volume.volume_type = VolumeType::Aggregate;
            }
            let volumes = [volume];
            let table = BitTable {
                rows: 1,
                columns: 1,
                array_size: 1,
                words: vec![1],
            };
            let mut run = run_fixture(&volumes, &phase, body, None);
            run.extra[0] = None;
            if gate == 2 {
                run.pipeline_culling = Some(&table);
            }
            let mut output = SimulationGroundContactBuffer::try_new(10, true).unwrap();
            assert_eq!(
                rip_repeated(&run, 1, &mut output).pop().unwrap().unwrap(),
                SimulationGroundRunOutcome::Complete
            );
            output.try_finish_assembly().unwrap();
            assert!(output.try_into_records().unwrap().is_empty());
        }
    }

    #[test]
    fn ground_validation_precedes_static_gate_and_gp_precedes_disabled_gate() {
        let (mut pool, _, ground) = bodies();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let volumes = [sphere(3)];
        let run = run_fixture(&volumes, &phase, ground, None);
        let mut output = SimulationGroundContactBuffer::try_new(10, true).unwrap();
        assert!(matches!(
            rip_repeated(&run, 1, &mut output).pop().unwrap(),
            Err(SimulationGroundRunError::MissingGroundBody)
        ));
        drop(phase);
        let (mut pool, body, ground) = bodies();
        let phase = pool.try_prepare_dynamic_update_phase().unwrap();
        let mut invalid = sphere(3);
        invalid.enabled = false;
        invalid.volume.vtable_offset = 0;
        let volumes = [invalid.clone(), invalid];
        let run = run_fixture(&volumes, &phase, body, Some(ground));
        assert!(matches!(
            rip_repeated(&run, 1, &mut output).pop().unwrap(),
            Err(SimulationGroundRunError::UnsupportedVolume { volume: 0 })
        ));
    }

    #[test]
    fn culling_is_asymmetric_wrapping_and_bounds_overlap_is_inclusive_xyz_only() {
        let table = BitTable {
            rows: 8,
            columns: 5,
            array_size: 2,
            words: vec![1 << 13, 0],
        };
        assert!(is_culled(Some(&table), 2, 3).unwrap());
        assert!(!is_culled(Some(&table), 3, 2).unwrap());
        assert!(matches!(
            is_culled(Some(&table), 100, 2),
            Err(SimulationGroundRunError::CullingWord { .. })
        ));
        let wrapped = BitTable {
            rows: 0,
            columns: u32::MAX,
            array_size: 1,
            words: vec![1 << 1],
        };
        assert!(is_culled(Some(&wrapped), 1, 2).unwrap());
        let bounds = AabbWords::from_f32(
            [0.0, 0.0, 0.0, f32::NAN],
            [1.0, 1.0, 1.0, f32::NEG_INFINITY],
        );
        assert!(triangle_overlaps(
            [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 1.0]],
            bounds
        ));
        assert!(!triangle_overlaps(
            [[2.0, 0.0, 0.0], [2.0, 1.0, 0.0], [2.0, 0.0, 1.0]],
            bounds
        ));
    }
}
