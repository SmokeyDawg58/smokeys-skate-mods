//! Actual Skeleton owner -> source `SerialisedAssembly` Create/Update/bounds.
//!
//! Create 0x82e0c850, Update 0x82e0cf30, bounds job 0x827cdf78. This child reads
//! the real session's private body handles; no arena, packed inertia address
//! or caller-created collision record establishes authority. One immutable
//! snapshot per Part is an explicit host substitute for repeated native reads.
//! Semantic body XYZ/canonical pose W and zero motion-link W use the already
//! audited authored numeric boundary; real force/kine/torque/cool are retained.
//!
//! Hat `SimpleMappedArray` children are flattened in source order. Child flags
//! are independent of the outer Aggregate flags; no outer transform is added.
//! Immutable geometry header words are initial snapshots. Create overlays the
//! actual live header only for primitive top-level Volumes; Update never
//! rewrites inline Volume words, even after live flags or group change.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "skeleton publication awaits mixed SkaterPool ownership"
    )
)]

use super::{DynamicAssemblyBackendTemplate, skeleton_normal::SkeletonNormalAssembly};
use crate::{
    authored_collision_publication::{semantic_motion, semantic_part_transform},
    octree::AabbWords,
    ragdoll::SKELETON_PART_COUNT,
    serialised_assembly_bbox::{
        HostSerialisedAssemblyBBoxNumeric, SerialisedAssemblyBodyMotion,
        try_calculate_primitive_part_bbox, try_union_part_bboxes,
    },
    serialised_assembly_template::{
        SERIALISED_ASSEMBLY_VOLUME_CAPACITY, SERIALISED_ASSEMBLY_VOLUME_PADDING,
        SerialisedAssemblyVolumeDataState,
    },
    simulation_rigid_body_pool::{
        SimulationRigidBodyFrameIdentity, SimulationRigidBodyObservation,
        SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
    },
    skeleton_assembly::{SkeletonAssemblyBinding, SkeletonPreNormalSimulationBackend},
    skeleton_body_definition::{SkeletonBodyDefinition, SkeletonPartGeometry},
    skeleton_culling::SkeletonCullingResource,
};
use skate2_formats::collision::{BitTable, Volume, VolumeType};
use std::{convert::Infallible, fmt, sync::Arc};

#[cfg(test)]
#[path = "skeleton_collision_publication_tests.rs"]
pub(crate) mod tests;

type SkeletonRuntimeTemplate =
    DynamicAssemblyBackendTemplate<SkeletonPreNormalSimulationBackend, SkeletonAssemblyBinding>;

#[derive(Debug)]
pub(crate) struct SkeletonCollisionPublication {
    template: Arc<SkeletonRuntimeTemplate>,
    binding: Arc<SkeletonAssemblyBinding>,
    body_pool: SimulationRigidBodyPoolAuthority,
    bodies: [SimulationRigidBodyObservation; SKELETON_PART_COUNT],
    records: Vec<SerialisedAssemblyVolumeDataState>,
    bounds: AabbWords,
    num_static: u32,
    all_parts_frozen: bool,
    bend_threshold: f32,
    group: u32,
    culling: Arc<SkeletonCullingResource>,
    volume_frame: Option<SimulationRigidBodyFrameIdentity>,
    bounds_frame: Option<SimulationRigidBodyFrameIdentity>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SkeletonCollisionPublicationError {
    Detached,
    ForeignPool,
    DefinitionChanged,
    PartCount { actual: usize },
    MissingBody { part: usize },
    BodyOccurrenceChanged { part: usize },
    Body(SimulationRigidBodyPoolError),
    UnsupportedVolume { part: usize, kind: u32 },
    SelectionChanged,
    FrameUnavailable,
    StaleVolumes,
    Allocation,
}

impl fmt::Display for SkeletonCollisionPublicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Detached => f.write_str("Skeleton collision owner is detached"),
            Self::ForeignPool => {
                f.write_str("Skeleton collision owner belongs to another body pool")
            }
            Self::DefinitionChanged => f.write_str("Skeleton collision template/binding changed"),
            Self::PartCount { actual } => {
                write!(f, "Skeleton collision needs 24 Parts, found {actual}")
            }
            Self::MissingBody { part } => write!(f, "Skeleton Part {part} has no body"),
            Self::BodyOccurrenceChanged { part } => {
                write!(f, "Skeleton Part {part} body occurrence changed")
            }
            Self::Body(source) => write!(f, "Skeleton collision body: {source}"),
            Self::UnsupportedVolume { part, kind } => {
                write!(f, "Skeleton Part {part} has unsupported Volume {kind}")
            }
            Self::SelectionChanged => f.write_str("Skeleton Create/Update selections differ"),
            Self::FrameUnavailable => f.write_str("Skeleton collision frame is unavailable"),
            Self::StaleVolumes => {
                f.write_str("Skeleton collision Volumes are not current for bounds")
            }
            Self::Allocation => f.write_str("Skeleton publication storage allocation failed"),
        }
    }
}
impl std::error::Error for SkeletonCollisionPublicationError {}

#[derive(Clone, Copy)]
struct LivePart {
    body: SimulationRigidBodyObservation,
    transform: [f32; 16],
    state: u32,
    motion: SerialisedAssemblyBodyMotion,
}

impl SkeletonCollisionPublication {
    pub(crate) fn try_create(
        owner: &SkeletonNormalAssembly,
        backend: &SkeletonPreNormalSimulationBackend,
        edge_cos_bend_normal_threshold: f32,
    ) -> Result<Self, SkeletonCollisionPublicationError> {
        use SkeletonCollisionPublicationError as Error;
        let parts = read_live_parts(owner, backend)?;
        let frame = backend
            .collision_body_pool()
            .frame_identity()
            .ok_or(Error::FrameUnavailable)?;
        let state = owner.state();
        let mut records = Vec::new();
        records
            .try_reserve_exact(SERIALISED_ASSEMBLY_VOLUME_CAPACITY)
            .map_err(|_| Error::Allocation)?;
        let mut numeric = HostSerialisedAssemblyBBoxNumeric;
        let mut bounds = None;
        for (part, source, top_level) in source_volumes(state.binding.body()) {
            if records.len() == SERIALISED_ASSEMBLY_VOLUME_CAPACITY {
                continue;
            }
            let volume_type = require_primitive(part, source)?;
            let mut volume = source.clone();
            if top_level {
                volume.flags = state.volume_headers[part].flags;
                volume.group_id = state.volume_headers[part].group_id;
            }
            let record = SerialisedAssemblyVolumeDataState {
                rigid_body_index: u16::try_from(part).expect("24 actual Parts fit u16"),
                volume_type,
                enabled: volume.flags & 1 != 0,
                volume,
                padding: SERIALISED_ASSEMBLY_VOLUME_PADDING,
                material: *state.material(part),
                surface_id: source.surface_id,
                transform: parts[part].transform,
                group: state.part_user_data[part],
                frozen: parts[part].state & 2 != 0,
            };
            // Create calculates and unions each primitive immediately, unlike
            // the later job's all-boxes-then-union schedule.
            let candidate = record_bounds(&record, &parts, &mut numeric)?;
            bounds = Some(match bounds {
                None => candidate,
                Some(previous) => infallible(try_union_part_bboxes(
                    &[(previous, 1), (candidate, 1)],
                    &mut numeric,
                )),
            });
            records.push(record);
        }
        let (num_static, all_parts_frozen) = header_state(&parts);
        Ok(Self {
            template: Arc::clone(owner.session().template()),
            binding: Arc::clone(&state.binding),
            body_pool: backend.authority(),
            bodies: parts.map(|part| part.body),
            records,
            bounds: bounds.expect("source Skeleton owns a primitive root Part"),
            num_static,
            all_parts_frozen,
            bend_threshold: edge_cos_bend_normal_threshold,
            group: state.assembly_group,
            culling: Arc::clone(&state.current_culling),
            volume_frame: Some(frame),
            bounds_frame: Some(frame),
        })
    }

    /// Single-member convenience, not the pool's required outer phase order.
    pub(crate) fn try_refresh(
        &mut self,
        owner: &SkeletonNormalAssembly,
        backend: &SkeletonPreNormalSimulationBackend,
    ) -> Result<(), SkeletonCollisionPublicationError> {
        self.try_update_volumes(owner, backend)?;
        let bounds = self.try_stage_bounds(owner, backend)?;
        let frame = self
            .volume_frame
            .expect("successful Update established the frame");
        self.publish_bounds(bounds, frame);
        Ok(())
    }

    /// Invalidate BEFORE same-frame mutations or any attempted refresh, including
    /// paths that may fail after partial source writes. The owning mixed pool
    /// must schedule this before modifying its actual `SkeletonNormalAssembly`.
    pub(super) fn invalidate_refresh(&mut self) {
        self.volume_frame = None;
        self.bounds_frame = None;
    }

    fn matching_parts(
        &self,
        owner: &SkeletonNormalAssembly,
        backend: &SkeletonPreNormalSimulationBackend,
    ) -> Result<[LivePart; SKELETON_PART_COUNT], SkeletonCollisionPublicationError> {
        use SkeletonCollisionPublicationError as Error;
        if self.body_pool != backend.authority() {
            return Err(Error::ForeignPool);
        }
        if !Arc::ptr_eq(&self.template, owner.session().template())
            || !Arc::ptr_eq(&self.binding, &owner.state().binding)
        {
            return Err(Error::DefinitionChanged);
        }
        let parts = read_live_parts(owner, backend)?;
        for (part, live) in parts.iter().enumerate() {
            if live.body != self.bodies[part] {
                return Err(Error::BodyOccurrenceChanged { part });
            }
        }
        Ok(parts)
    }

    pub(super) fn try_update_volumes(
        &mut self,
        owner: &SkeletonNormalAssembly,
        backend: &SkeletonPreNormalSimulationBackend,
    ) -> Result<(), SkeletonCollisionPublicationError> {
        use SkeletonCollisionPublicationError as Error;
        self.invalidate_refresh();
        let parts = self.matching_parts(owner, backend)?;
        let frame = backend
            .collision_body_pool()
            .frame_identity()
            .ok_or(Error::FrameUnavailable)?;
        let state = owner.state();
        let selected =
            source_volumes(state.binding.body()).take(SERIALISED_ASSEMBLY_VOLUME_CAPACITY);
        if selected.clone().count() != self.records.len() {
            return Err(Error::SelectionChanged);
        }
        for (record, (part, volume, top_level)) in self.records.iter_mut().zip(selected) {
            let flags = if top_level {
                state.volume_headers[part].flags
            } else {
                volume.flags
            };
            // Direct primitive and direct-child Update perform these stores in
            // this order. No inline Volume/group/surface/Part ID/body table copy.
            record.enabled = flags & 1 != 0;
            record.transform = parts[part].transform;
            record.material = *state.material(part);
            record.group = state.part_user_data[part];
            record.frozen = parts[part].state & 2 != 0;
        }
        self.group = state.assembly_group;
        self.culling = Arc::clone(&state.current_culling);
        (self.num_static, self.all_parts_frozen) = header_state(&parts);
        self.volume_frame = Some(frame);
        Ok(())
    }

    pub(super) fn try_stage_bounds(
        &self,
        owner: &SkeletonNormalAssembly,
        backend: &SkeletonPreNormalSimulationBackend,
    ) -> Result<AabbWords, SkeletonCollisionPublicationError> {
        use SkeletonCollisionPublicationError as Error;
        let parts = self.matching_parts(owner, backend)?;
        let frame = backend.collision_body_pool().frame_identity();
        if frame.is_none() || self.volume_frame != frame {
            return Err(Error::StaleVolumes);
        }
        let mut boxes = Vec::new();
        boxes
            .try_reserve_exact(self.records.len())
            .map_err(|_| Error::Allocation)?;
        let mut numeric = HostSerialisedAssemblyBBoxNumeric;
        for record in &self.records {
            boxes.push((record_bounds(record, &parts, &mut numeric)?, 1));
        }
        Ok(infallible(try_union_part_bboxes(&boxes, &mut numeric)))
    }

    pub(super) fn publish_bounds(
        &mut self,
        bounds: AabbWords,
        frame: SimulationRigidBodyFrameIdentity,
    ) {
        self.bounds = bounds;
        self.bounds_frame = Some(frame);
    }

    pub(super) fn try_all_parts_frozen(
        &self,
        owner: &SkeletonNormalAssembly,
        backend: &SkeletonPreNormalSimulationBackend,
    ) -> Result<bool, SkeletonCollisionPublicationError> {
        Ok(header_state(&self.matching_parts(owner, backend)?).1)
    }

    pub(crate) fn records(&self) -> &[SerialisedAssemblyVolumeDataState] {
        &self.records
    }
    pub(crate) fn rigid_bodies(&self) -> &[SimulationRigidBodyObservation] {
        &self.bodies
    }
    pub(crate) const fn bounds(&self) -> AabbWords {
        self.bounds
    }
    pub(crate) const fn num_static(&self) -> u32 {
        self.num_static
    }
    pub(crate) const fn all_parts_frozen(&self) -> bool {
        self.all_parts_frozen
    }
    pub(crate) const fn bend_threshold(&self) -> f32 {
        self.bend_threshold
    }
    pub(crate) const fn body_pool_authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.body_pool
    }
    pub(crate) fn has_bounds_for_frame(&self, frame: SimulationRigidBodyFrameIdentity) -> bool {
        self.volume_frame == Some(frame) && self.bounds_frame == Some(frame)
    }
    pub(crate) const fn group(&self) -> u32 {
        self.group
    }
    pub(crate) fn culling_table(&self) -> &BitTable {
        self.culling.table()
    }
}

fn source_volumes(
    definition: &SkeletonBodyDefinition,
) -> impl Iterator<Item = (usize, &Volume, bool)> + Clone {
    definition
        .parts
        .iter()
        .enumerate()
        .flat_map(|(part, definition)| {
            let (volumes, top_level): (&[Volume], bool) = match &definition.geometry {
                SkeletonPartGeometry::Primitive(volume) => {
                    (std::slice::from_ref(volume.as_ref()), true)
                }
                SkeletonPartGeometry::Hat(hat) => (hat.children().as_slice(), false),
            };
            volumes.iter().map(move |volume| (part, volume, top_level))
        })
}

fn require_primitive(
    part: usize,
    volume: &Volume,
) -> Result<VolumeType, SkeletonCollisionPublicationError> {
    match volume.volume_type() {
        Some(
            kind @ (VolumeType::Sphere
            | VolumeType::Capsule
            | VolumeType::Box
            | VolumeType::Triangle
            | VolumeType::Cylinder),
        ) => Ok(kind),
        _ => Err(SkeletonCollisionPublicationError::UnsupportedVolume {
            part,
            kind: volume.vtable_offset,
        }),
    }
}

fn read_live_parts(
    owner: &SkeletonNormalAssembly,
    backend: &SkeletonPreNormalSimulationBackend,
) -> Result<[LivePart; SKELETON_PART_COUNT], SkeletonCollisionPublicationError> {
    use SkeletonCollisionPublicationError as Error;
    let session = owner.session();
    match session.attached_authority {
        None => return Err(Error::Detached),
        Some(authority) if authority != backend.authority() => return Err(Error::ForeignPool),
        Some(_) => {}
    }
    if !Arc::ptr_eq(session.template().collision(), &owner.state().binding) {
        return Err(Error::DefinitionChanged);
    }
    let attachment = &session.part_attachment;
    if attachment.parts.len() != SKELETON_PART_COUNT {
        return Err(Error::PartCount {
            actual: attachment.parts.len(),
        });
    }
    let pool = backend.collision_body_pool();
    let mut parts = [None; SKELETON_PART_COUNT];
    for (ordinal, destination) in parts.iter_mut().enumerate() {
        let body = attachment.parts[ordinal]
            .body
            .ok_or(Error::MissingBody { part: ordinal })?;
        let observation = pool.try_observe(body).map_err(Error::Body)?;
        let snapshot = pool
            .try_serialised_assembly_snapshot(observation)
            .map_err(Error::Body)?;
        let transform = semantic_part_transform(&snapshot, &attachment.template.metadata[ordinal]);
        *destination = Some(LivePart {
            body: observation,
            transform,
            state: snapshot.state,
            motion: semantic_motion(
                snapshot.velocity,
                snapshot.omega,
                snapshot.force,
                snapshot.kinematic_value,
                snapshot.torque,
                snapshot.cooldown,
            ),
        });
    }
    Ok(parts.map(|part| part.expect("all 24 actual attached Parts were read")))
}

fn record_bounds(
    record: &SerialisedAssemblyVolumeDataState,
    parts: &[LivePart; SKELETON_PART_COUNT],
    numeric: &mut HostSerialisedAssemblyBBoxNumeric,
) -> Result<AabbWords, SkeletonCollisionPublicationError> {
    let part = usize::from(record.rigid_body_index);
    infallible(try_calculate_primitive_part_bbox(
        &record.volume,
        &record.transform,
        parts[part].motion,
        numeric,
    ))
    .ok_or(SkeletonCollisionPublicationError::UnsupportedVolume {
        part,
        kind: record.volume.vtable_offset,
    })
}

fn header_state(parts: &[LivePart; SKELETON_PART_COUNT]) -> (u32, bool) {
    let num_static = parts
        .iter()
        .fold(0, |count, part| count + u32::from(part.state & 1 != 0));
    let all_frozen = parts
        .iter()
        .fold(1_u8, |all, part| all & u8::from(part.state & 2 != 0))
        != 0;
    (num_static, all_frozen)
}

fn infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(never) => match never {},
    }
}
