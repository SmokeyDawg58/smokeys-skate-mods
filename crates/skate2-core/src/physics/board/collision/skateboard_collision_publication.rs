//! Authored board `SerialisedAssembly` Create/Update and swept bounds.
//!
//! Sources: Create `0x82e0c850`, Update `0x82e0cf30`, pool bounds job
//! `0x827cdf78`, `EndUpdate` `0x82e0bb10`. This child of the real assembly
//! session reads its private, attached body handles; no caller-created record,
//! arena ID, serialized Inertia address or Xenon image establishes authority.
//!
//! Board Init's Assembly group4 / Part userdata4 setup is scoped explicitly
//! here. This is not Island/SkaterPool membership or a complete native Tick.
//! Immutable retained definitions/materials and one live snapshot per Part
//! replace concurrent alias visibility. All observations remain occurrence
//! qualified; a publication does not itself keep a session attached.
//!
//! The independently audited semantic-XYZ branch uses real body XYZ with
//! canonical affine W, and zero W for velocity/omega. Those W lanes are NOT
//! native pointer words. Real kine/cool and all definition Volume words remain
//! retained. Existing host bbox numeric limits apply; raw packed-W parity is
//! neither claimed nor required from the unchanged arena-image route.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "authored collision awaits live skater publication"
    )
)]

use crate::{
    authored_collision_publication::{semantic_motion, semantic_part_transform},
    octree::AabbWords,
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
    skateboard_assembly::{
        SkateboardAssemblySession, SkateboardCollisionDefinition, SkateboardRuntimeTemplate,
        SkateboardSimulationBackend,
    },
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable, Volume, VolumeType};
use std::{
    convert::Infallible,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

const BOARD_PART_COUNT: usize = 7;
use super::board_collision_groups::{BoardCollisionGroupError, BoardCollisionGroups};

/// Retains the exact template/definition and seven actual allocation
/// occurrences, not merely equal-valued geometry or constructor transforms.
#[derive(Debug)]
pub(crate) struct SkateboardCollisionPublication {
    template: Arc<SkateboardRuntimeTemplate>,
    definition: Arc<SkateboardCollisionDefinition>,
    body_pool: SimulationRigidBodyPoolAuthority,
    bodies: [SimulationRigidBodyObservation; BOARD_PART_COUNT],
    records: Vec<SerialisedAssemblyVolumeDataState>,
    bounds: AabbWords,
    num_static: u32,
    all_parts_frozen: bool,
    bend_threshold: f32,
    volume_frame: Option<SimulationRigidBodyFrameIdentity>,
    bounds_frame: Option<SimulationRigidBodyFrameIdentity>,
    material_revision: Arc<AtomicU64>,
    volume_material_revision: Option<u64>,
    volume_flags_revision: Option<u64>,
    collision_groups: Arc<BoardCollisionGroups>,
    volume_group_revision: Option<u64>,
    group: u32,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SkateboardCollisionPublicationError {
    Detached,
    ForeignPool,
    DefinitionChanged,
    MaterialOwnerChanged,
    GroupOwnerChanged,
    Groups(BoardCollisionGroupError),
    Material(super::DynamicAssemblyPartMaterialError),
    VolumeFlags(crate::skateboard_assembly::volume_flags::SkateboardVolumeFlagError),
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

impl fmt::Display for SkateboardCollisionPublicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Detached => f.write_str("board collision source is not attached"),
            Self::ForeignPool => f.write_str("board collision source belongs to another body pool"),
            Self::DefinitionChanged => f.write_str("board collision definition/template changed"),
            Self::MaterialOwnerChanged => {
                f.write_str("board collision Part material owner changed")
            }
            Self::GroupOwnerChanged => f.write_str("board collision group owner changed"),
            Self::Groups(error) => write!(f, "{error}"),
            Self::Material(error) => write!(f, "board collision material: {error}"),
            Self::VolumeFlags(error) => write!(f, "board collision volume flags: {error:?}"),
            Self::PartCount { actual } => {
                write!(f, "board collision needs seven Parts, found {actual}")
            }
            Self::MissingBody { part } => write!(f, "board Part {part} has no admitted body"),
            Self::BodyOccurrenceChanged { part } => {
                write!(f, "board Part {part} allocation occurrence changed")
            }
            Self::Body(error) => write!(f, "board collision body: {error}"),
            Self::UnsupportedVolume { part, kind } => {
                write!(f, "board Part {part} has unsupported Volume type {kind}")
            }
            Self::SelectionChanged => {
                f.write_str("board collision Create/Update selections differ")
            }
            Self::FrameUnavailable => f.write_str("board collision body frame is unavailable"),
            Self::StaleVolumes => f.write_str("board collision volumes are not current for bounds"),
            Self::Allocation => {
                f.write_str("board collision publication storage allocation failed")
            }
        }
    }
}

impl std::error::Error for SkateboardCollisionPublicationError {}

#[derive(Clone, Copy)]
struct LivePart {
    body: SimulationRigidBodyObservation,
    transform: [f32; 16],
    material: AssemblyPartMaterial,
    state: u32,
    motion: SerialisedAssemblyBodyMotion,
}

impl SkateboardCollisionPublication {
    /// Actual attached board -> source Create fields and initial bounds.
    /// All variable deck children are visited with the native per-emission
    /// 26-record cap; 19 records is only the shipped five-segment deck case.
    pub(crate) fn try_create(
        session: &SkateboardAssemblySession,
        backend: &SkateboardSimulationBackend,
        edge_cos_bend_normal_threshold: f32,
    ) -> Result<Self, SkateboardCollisionPublicationError> {
        let parts = read_live_parts(session, backend)?;
        let groups = session
            .board_collision_groups
            .try_snapshot()
            .map_err(SkateboardCollisionPublicationError::Groups)?;
        let pool = backend.collision_body_pool();
        let frame = pool
            .frame_identity()
            .ok_or(SkateboardCollisionPublicationError::FrameUnavailable)?;
        let definition = Arc::clone(session.template.collision());
        let mut records = Vec::new();
        records
            .try_reserve_exact(SERIALISED_ASSEMBLY_VOLUME_CAPACITY)
            .map_err(|_| SkateboardCollisionPublicationError::Allocation)?;
        let flags = definition
            .volume_flags
            .read()
            .map_err(SkateboardCollisionPublicationError::VolumeFlags)?;
        for (emission, (part_ordinal, volume)) in source_volumes(&definition).enumerate() {
            if records.len() == SERIALISED_ASSEMBLY_VOLUME_CAPACITY {
                continue;
            }
            let volume_type = require_primitive(part_ordinal, volume)?;
            let part = parts[part_ordinal];
            records.push(SerialisedAssemblyVolumeDataState {
                rigid_body_index: u16::try_from(part_ordinal).expect("seven board Parts fit u16"),
                volume_type,
                volume: volume.clone(),
                padding: SERIALISED_ASSEMBLY_VOLUME_PADDING,
                material: part.material,
                surface_id: volume.surface_id,
                transform: part.transform,
                enabled: flags
                    .emission_flags(emission)
                    .ok_or(SkateboardCollisionPublicationError::SelectionChanged)?
                    & 1
                    != 0,
                group: groups.parts[part_ordinal],
                frozen: part.state & 2 != 0,
            });
        }
        // Every authored board has six non-Aggregate wheel/truck occurrences,
        // so Create's otherwise undefined empty/all-Aggregate AABB is absent.
        let bounds = calculate_create_bounds(&records, &parts)?;
        let (num_static, all_parts_frozen) = header_state(&parts);
        let volume_flags_revision = Some(flags.revision());
        drop(flags);
        Ok(Self {
            template: Arc::clone(&session.template),
            definition,
            body_pool: pool.authority(),
            bodies: parts.map(|part| part.body),
            records,
            bounds,
            num_static,
            all_parts_frozen,
            bend_threshold: edge_cos_bend_normal_threshold,
            volume_frame: Some(frame),
            bounds_frame: Some(frame),
            material_revision: Arc::clone(&session.part_attachment.material_revision),
            volume_material_revision: Some(
                session
                    .part_attachment
                    .material_revision
                    .load(Ordering::Acquire),
            ),
            volume_flags_revision,
            collision_groups: Arc::clone(&session.board_collision_groups),
            volume_group_revision: Some(groups.revision),
            group: groups.assembly,
        })
    }

    /// Source Update -> separate pool bounds job -> Header bounds publication
    /// for this one real board. A multi-member pool must preserve its outer
    /// all-Update/all-bounds/all-publish order rather than interleave this call.
    ///
    /// Invalidate freshness even on rejected refresh. Checked host failures
    /// after `VolumeData` Update may leave its new records/header state visible,
    /// but publish no new bounds stamp. This is explicit host failure policy.
    /// Same-frame body mutation after success remains a scheduler obligation:
    /// refresh immediately before taking the exclusive collision phase.
    pub(crate) fn try_refresh(
        &mut self,
        session: &SkateboardAssemblySession,
        backend: &SkateboardSimulationBackend,
    ) -> Result<(), SkateboardCollisionPublicationError> {
        self.try_update_volumes(session, backend)?;
        let bounds = self.try_stage_bounds(session, backend)?;
        let frame = backend
            .collision_body_pool()
            .frame_identity()
            .ok_or(SkateboardCollisionPublicationError::FrameUnavailable)?;
        self.publish_bounds(bounds, frame);
        Ok(())
    }

    /// Host invalidation precedes all source Update work across the pool.
    pub(super) fn invalidate_refresh(&mut self) {
        self.bounds_frame = None;
        self.volume_frame = None;
        self.volume_material_revision = None;
        self.volume_flags_revision = None;
        self.volume_group_revision = None;
    }

    fn matching_live_parts(
        &self,
        session: &SkateboardAssemblySession,
        backend: &SkateboardSimulationBackend,
    ) -> Result<[LivePart; BOARD_PART_COUNT], SkateboardCollisionPublicationError> {
        if backend.authority() != self.body_pool {
            return Err(SkateboardCollisionPublicationError::ForeignPool);
        }
        if !Arc::ptr_eq(&self.template, &session.template)
            || !Arc::ptr_eq(&self.definition, session.template.collision())
        {
            return Err(SkateboardCollisionPublicationError::DefinitionChanged);
        }
        if !Arc::ptr_eq(
            &self.material_revision,
            &session.part_attachment.material_revision,
        ) {
            return Err(SkateboardCollisionPublicationError::MaterialOwnerChanged);
        }
        let parts = read_live_parts(session, backend)?;
        if !Arc::ptr_eq(&self.collision_groups, &session.board_collision_groups) {
            return Err(SkateboardCollisionPublicationError::GroupOwnerChanged);
        }
        for (ordinal, part) in parts.iter().enumerate() {
            if part.body != self.bodies[ordinal] {
                return Err(SkateboardCollisionPublicationError::BodyOccurrenceChanged {
                    part: ordinal,
                });
            }
        }
        Ok(parts)
    }

    /// `SerialisedAssembly::Update` 0x82e0cf30, without the later pool job.
    pub(super) fn try_update_volumes(
        &mut self,
        session: &SkateboardAssemblySession,
        backend: &SkateboardSimulationBackend,
    ) -> Result<(), SkateboardCollisionPublicationError> {
        self.invalidate_refresh();
        let parts = self.matching_live_parts(session, backend)?;
        let groups = self
            .collision_groups
            .try_snapshot()
            .map_err(SkateboardCollisionPublicationError::Groups)?;
        let frame = backend
            .collision_body_pool()
            .frame_identity()
            .ok_or(SkateboardCollisionPublicationError::FrameUnavailable)?;
        let selected = source_volumes(&self.definition).take(SERIALISED_ASSEMBLY_VOLUME_CAPACITY);
        if selected.clone().count() != self.records.len() {
            return Err(SkateboardCollisionPublicationError::SelectionChanged);
        }
        // In this concrete graph the root is a SimpleMappedArray of actual
        // primitives; Create and Update select the same occurrences. No
        // ClusteredMesh wrapper or nested aggregate is invented for the deck.
        let flags = self
            .definition
            .volume_flags
            .read()
            .map_err(SkateboardCollisionPublicationError::VolumeFlags)?;
        for (emission, (record, (ordinal, _volume))) in
            self.records.iter_mut().zip(selected).enumerate()
        {
            let part = parts[ordinal];
            record.enabled = flags
                .emission_flags(emission)
                .ok_or(SkateboardCollisionPublicationError::SelectionChanged)?
                & 1
                != 0;
            record.material = part.material;
            record.transform = part.transform;
            record.group = groups.parts[ordinal];
            record.frozen = part.state & 2 != 0;
        }
        (self.num_static, self.all_parts_frozen) = header_state(&parts);
        self.volume_frame = Some(frame);
        self.volume_material_revision = Some(self.material_revision.load(Ordering::Acquire));
        self.volume_flags_revision = Some(flags.revision());
        self.volume_group_revision = Some(groups.revision);
        self.group = groups.assembly;
        Ok(())
    }

    /// Separate bounds output; never modifies the Header or freshness stamp.
    pub(super) fn try_stage_bounds(
        &self,
        session: &SkateboardAssemblySession,
        backend: &SkateboardSimulationBackend,
    ) -> Result<AabbWords, SkateboardCollisionPublicationError> {
        let parts = self.matching_live_parts(session, backend)?;
        let frame = backend.collision_body_pool().frame_identity();
        if frame.is_none() || self.volume_frame != frame || !self.material_is_current() {
            return Err(SkateboardCollisionPublicationError::StaleVolumes);
        }
        calculate_bounds(&self.records, &parts)
    }

    /// Owner-only `EndUpdate` publication after the complete bounds job.
    pub(super) fn publish_bounds(
        &mut self,
        bounds: AabbWords,
        frame: SimulationRigidBodyFrameIdentity,
    ) {
        self.bounds = bounds;
        self.bounds_frame = Some(frame);
    }

    /// `EndUpdate` rescans live Part state rather than filtering by static state.
    pub(super) fn try_all_parts_frozen(
        &self,
        session: &SkateboardAssemblySession,
        backend: &SkateboardSimulationBackend,
    ) -> Result<bool, SkateboardCollisionPublicationError> {
        let parts = self.matching_live_parts(session, backend)?;
        Ok(header_state(&parts).1)
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
        self.volume_frame == Some(frame)
            && self.bounds_frame == Some(frame)
            && self.material_is_current()
    }
    fn material_is_current(&self) -> bool {
        self.volume_material_revision == Some(self.material_revision.load(Ordering::Acquire))
            && self.volume_group_revision == Some(self.collision_groups.revision())
            && self
                .definition
                .volume_flags
                .read()
                .is_ok_and(|flags| self.volume_flags_revision == Some(flags.revision()))
    }
    pub(crate) const fn group(&self) -> u32 {
        self.group
    }
    pub(crate) fn culling_table(&self) -> &BitTable {
        &self.definition.culling
    }
}

fn source_volumes(
    definition: &SkateboardCollisionDefinition,
) -> impl Iterator<Item = (usize, &Volume)> + Clone {
    definition
        .wheels
        .parts()
        .iter()
        .enumerate()
        .map(|(ordinal, part)| (ordinal, part.volume().as_ref()))
        .chain(
            definition
                .trucks
                .parts()
                .iter()
                .enumerate()
                .map(|(ordinal, part)| (ordinal + 4, part.volume().as_ref())),
        )
        .chain(
            definition
                .deck
                .aggregate()
                .children()
                .iter()
                .map(|volume| (6, volume)),
        )
}

fn require_primitive(
    part: usize,
    volume: &Volume,
) -> Result<VolumeType, SkateboardCollisionPublicationError> {
    match volume.volume_type() {
        Some(
            kind @ (VolumeType::Sphere
            | VolumeType::Capsule
            | VolumeType::Box
            | VolumeType::Triangle
            | VolumeType::Cylinder),
        ) => Ok(kind),
        _ => Err(SkateboardCollisionPublicationError::UnsupportedVolume {
            part,
            kind: volume.vtable_offset,
        }),
    }
}

fn read_live_parts(
    session: &SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
) -> Result<[LivePart; BOARD_PART_COUNT], SkateboardCollisionPublicationError> {
    match session.attached_authority {
        None => return Err(SkateboardCollisionPublicationError::Detached),
        Some(authority) if authority != backend.authority() => {
            return Err(SkateboardCollisionPublicationError::ForeignPool);
        }
        Some(_) => {}
    }
    let attachment = &session.part_attachment;
    if attachment.parts.len() != BOARD_PART_COUNT {
        return Err(SkateboardCollisionPublicationError::PartCount {
            actual: attachment.parts.len(),
        });
    }
    let pool = backend.collision_body_pool();
    let mut parts = [None; BOARD_PART_COUNT];
    for (ordinal, destination) in parts.iter_mut().enumerate() {
        let handle = attachment.parts[ordinal]
            .body
            .ok_or(SkateboardCollisionPublicationError::MissingBody { part: ordinal })?;
        let observation = pool
            .try_observe(handle)
            .map_err(SkateboardCollisionPublicationError::Body)?;
        let snapshot = pool
            .try_serialised_assembly_snapshot(observation)
            .map_err(SkateboardCollisionPublicationError::Body)?;
        let transform = semantic_part_transform(&snapshot, &attachment.template.metadata[ordinal]);
        *destination = Some(LivePart {
            body: observation,
            transform,
            material: *session
                .current_part_material(ordinal)
                .map_err(SkateboardCollisionPublicationError::Material)?,
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
    Ok(parts.map(|part| part.expect("all seven attached Parts were filled")))
}

fn calculate_bounds(
    records: &[SerialisedAssemblyVolumeDataState],
    parts: &[LivePart; BOARD_PART_COUNT],
) -> Result<AabbWords, SkateboardCollisionPublicationError> {
    let mut boxes = Vec::new();
    boxes
        .try_reserve_exact(records.len())
        .map_err(|_| SkateboardCollisionPublicationError::Allocation)?;
    let mut numeric = HostSerialisedAssemblyBBoxNumeric;
    for record in records {
        // This owner emits only real primitives. No enabled/frozen/static
        // filter belongs in either Create's bounds or the later bounds job.
        let bounds = calculate_record_bounds(record, parts, &mut numeric)?;
        boxes.push((bounds, 1));
    }
    Ok(infallible(try_union_part_bboxes(&boxes, &mut numeric)))
}

/// Create unions immediately after each primitive calculation, unlike the
/// later pool job which first computes the complete separate bounds array.
fn calculate_create_bounds(
    records: &[SerialisedAssemblyVolumeDataState],
    parts: &[LivePart; BOARD_PART_COUNT],
) -> Result<AabbWords, SkateboardCollisionPublicationError> {
    let mut numeric = HostSerialisedAssemblyBBoxNumeric;
    let mut bounds = None;
    for record in records {
        let candidate = calculate_record_bounds(record, parts, &mut numeric)?;
        bounds = Some(match bounds {
            None => candidate,
            Some(previous) => infallible(try_union_part_bboxes(
                &[(previous, 1), (candidate, 1)],
                &mut numeric,
            )),
        });
    }
    // Construction always selected the six wheel/truck primitives first.
    Ok(bounds.expect("authored board Create has at least six primitive bounds"))
}

fn calculate_record_bounds(
    record: &SerialisedAssemblyVolumeDataState,
    parts: &[LivePart; BOARD_PART_COUNT],
    numeric: &mut HostSerialisedAssemblyBBoxNumeric,
) -> Result<AabbWords, SkateboardCollisionPublicationError> {
    let ordinal = usize::from(record.rigid_body_index);
    let part = &parts[ordinal];
    infallible(try_calculate_primitive_part_bbox(
        &record.volume,
        &record.transform,
        part.motion,
        numeric,
    ))
    .ok_or(SkateboardCollisionPublicationError::UnsupportedVolume {
        part: ordinal,
        kind: record.volume.vtable_offset,
    })
}

fn header_state(parts: &[LivePart; BOARD_PART_COUNT]) -> (u32, bool) {
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

#[cfg(test)]
#[path = "skateboard_collision_publication_tests.rs"]
mod kernel_tests;

#[cfg(test)]
pub(crate) use kernel_tests::check_material_replacement;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_motion_keeps_real_kine_cool_but_never_manufactures_link_words() {
        let kine = f32::from_bits(0x8000_0000);
        let cool = 0x7fc0_1234;
        let motion = semantic_motion(
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
            kine,
            [10.0, 11.0, 12.0],
            cool,
        );
        assert_eq!(
            motion.velocity_and_right.map(f32::to_bits),
            [1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits(), 0]
        );
        assert_eq!(motion.omega_and_left[3].to_bits(), 0);
        assert_eq!(motion.force_and_kine[3].to_bits(), kine.to_bits());
        assert_eq!(motion.torque_and_cool[3].to_bits(), cool);
    }
}
