//! Staged CPU `EA::Physics::SerialisedAssembly::Create` publication.
//!
//! The native function at `0x82e0c850..0x82e0cf2c` progressively mutates a
//! fixed 0x1780-byte object. This safe-host form resolves and validates every
//! arena/body occurrence, packed address-domain word, allocation, and numeric
//! callback before publishing one owner. Field values and source ordering are
//! recovered; native partial writes after invalid input are not reproduced.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the attached DMO session publication bridge"
    )
)]

use crate::{
    octree::AabbWords,
    serialised_assembly_bbox::{
        SerialisedAssemblyBodyMotion, SerialisedAssemblyPartBBoxNumeric,
        try_calculate_primitive_part_bbox,
    },
    serialised_assembly_template::{
        SerialisedAssemblyCullingTableSource, SerialisedAssemblyLivePartState,
        SerialisedAssemblyTemplate, SerialisedAssemblyVolumeDataSet,
        SerialisedAssemblyVolumeDataStateError,
    },
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_rigid_body_pool::{
        SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
        SimulationRigidBodySerialisedAssemblySnapshot,
    },
    simulation_xenon_address_image::{
        SimulationPackedBodyWordImageDomain, SimulationRigidBodyAddInertiaOutcome,
        SimulationRigidBodyAddInertiaOwner, SimulationRigidBodyAddInertiaOwnerError,
        SimulationXenonAddressImageError,
    },
};
use skate2_formats::{
    collision::{AssemblyPartMaterial, VolumeType},
    pegasus::ObjectId,
};
use std::{error::Error, fmt, num::NonZeroU64, sync::Arc};

const DEFAULT_EDGE_COS_BEND_NORMAL_THRESHOLD: f32 = f32::from_bits(0x3f7f_be77);

#[path = "serialised_assembly_pool_update.rs"]
pub(crate) mod pool_update;

/// Identity of the virtual-Xenon address domain supplying packed pointer words.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SerialisedAssemblyPackedBodyWordAuthority {
    image_domain_epoch: NonZeroU64,
    arena_binding_epoch: NonZeroU64,
}

impl SerialisedAssemblyPackedBodyWordAuthority {
    #[cfg(test)]
    pub(super) fn for_test(id: u64) -> Self {
        let id = NonZeroU64::new(id).expect("test packed-word authority must be nonzero");
        Self {
            image_domain_epoch: id,
            arena_binding_epoch: id,
        }
    }
}

/// Native pointer words adjacent to vectors loaded by `Part::GetTransform`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SerialisedAssemblyPackedBodyTransformWords {
    pub(super) stasis: u32,
    pub(super) inertia: u32,
    pub(super) solver_delta_row: u32,
}

/// Native pointer words adjacent to motion vectors in `CalculatePartBBox`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SerialisedAssemblyPackedBodyMotionWords {
    pub(super) right_link: u32,
    pub(super) left_link: u32,
}

/// Supplies nonportable native pointer images for one validated body
/// occurrence. These are virtual-Xenon words, not truncated host pointers.
/// No implicit zero is permitted for a value the source actually reads.
pub(super) trait SerialisedAssemblyPackedBodyWordProvider {
    type Error;

    fn authority(&self) -> SerialisedAssemblyPackedBodyWordAuthority;

    fn validate_context(
        &self,
        template_owner: &Arc<SimulationNormalArenaPlan>,
        body_pool: &SimulationRigidBodyPool,
    ) -> Result<(), Self::Error>;

    fn packed_transform_words(
        &mut self,
        body: SimulationRigidBodyObservation,
    ) -> Result<SerialisedAssemblyPackedBodyTransformWords, Self::Error>;

    fn packed_motion_words(
        &mut self,
        body: SimulationRigidBodyObservation,
    ) -> Result<SerialisedAssemblyPackedBodyMotionWords, Self::Error>;
}

/// Safe-host failure while pairing the live body pool, global Simulation
/// image generation, and one arena's Add-time Inertia binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SimulationPackedBodyWordProviderError {
    ForeignImageBodyPool,
    ForeignInertiaBodyPool,
    ForeignArenaOwner,
    BodyInertia(SimulationRigidBodyAddInertiaOwnerError),
    AddressImage(SimulationXenonAddressImageError),
}

impl fmt::Display for SimulationPackedBodyWordProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignImageBodyPool => {
                formatter.write_str("Simulation address image belongs to another body pool")
            }
            Self::ForeignInertiaBodyPool => {
                formatter.write_str("Inertia outcomes belong to another body pool")
            }
            Self::ForeignArenaOwner => {
                formatter.write_str("Inertia outcomes belong to another arena owner")
            }
            Self::BodyInertia(source) => write!(formatter, "body Inertia lookup failed: {source}"),
            Self::AddressImage(source) => {
                write!(formatter, "Simulation address lookup failed: {source}")
            }
        }
    }
}

impl Error for SimulationPackedBodyWordProviderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BodyInertia(source) => Some(source),
            Self::AddressImage(source) => Some(source),
            _ => None,
        }
    }
}

/// Short-lived, inseparable view of the three owners needed to publish the
/// five native address words. Production callers never receive this value.
struct SimulationPackedBodyWordProvider<'a> {
    image_domain: &'a SimulationPackedBodyWordImageDomain,
    inertia_owner: &'a SimulationRigidBodyAddInertiaOwner,
    body_pool: &'a SimulationRigidBodyPool,
}

impl SerialisedAssemblyPackedBodyWordProvider for SimulationPackedBodyWordProvider<'_> {
    type Error = SimulationPackedBodyWordProviderError;

    fn authority(&self) -> SerialisedAssemblyPackedBodyWordAuthority {
        SerialisedAssemblyPackedBodyWordAuthority {
            image_domain_epoch: self.image_domain.epoch(),
            arena_binding_epoch: self.inertia_owner.binding_epoch(),
        }
    }

    fn validate_context(
        &self,
        template_owner: &Arc<SimulationNormalArenaPlan>,
        body_pool: &SimulationRigidBodyPool,
    ) -> Result<(), Self::Error> {
        if !self.image_domain.is_same_pool(body_pool)
            || !self.image_domain.is_same_pool(self.body_pool)
        {
            return Err(Self::Error::ForeignImageBodyPool);
        }
        if !self.inertia_owner.is_same_pool(body_pool)
            || !self.inertia_owner.is_same_pool(self.body_pool)
        {
            return Err(Self::Error::ForeignInertiaBodyPool);
        }
        if !self.inertia_owner.is_same_arena_owner(template_owner) {
            return Err(Self::Error::ForeignArenaOwner);
        }
        Ok(())
    }

    fn packed_transform_words(
        &mut self,
        body: SimulationRigidBodyObservation,
    ) -> Result<SerialisedAssemblyPackedBodyTransformWords, Self::Error> {
        let (topology, inertia) = self
            .inertia_owner
            .try_packed_word_inputs(self.body_pool, body)
            .map_err(Self::Error::BodyInertia)?;
        let inertia = match inertia {
            SimulationRigidBodyAddInertiaOutcome::StaticNull
            | SimulationRigidBodyAddInertiaOutcome::NonStaticNull => 0,
            SimulationRigidBodyAddInertiaOutcome::Definition { address, .. } => address,
        };
        Ok(SerialisedAssemblyPackedBodyTransformWords {
            stasis: self.image_domain.simulation_address(),
            inertia,
            solver_delta_row: self
                .image_domain
                .solver_delta_row_address(topology.observation.identity().id())
                .map_err(Self::Error::AddressImage)?,
        })
    }

    fn packed_motion_words(
        &mut self,
        body: SimulationRigidBodyObservation,
    ) -> Result<SerialisedAssemblyPackedBodyMotionWords, Self::Error> {
        let (topology, _) = self
            .inertia_owner
            .try_packed_word_inputs(self.body_pool, body)
            .map_err(Self::Error::BodyInertia)?;
        Ok(SerialisedAssemblyPackedBodyMotionWords {
            right_link: self
                .image_domain
                .rigid_body_node_address(topology.right_link)
                .map_err(Self::Error::AddressImage)?,
            left_link: self
                .image_domain
                .rigid_body_node_address(topology.left_link)
                .map_err(Self::Error::AddressImage)?,
        })
    }
}

/// Live Part-owned fields sampled by CPU Create in definition order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SerialisedAssemblyCreatePartInput {
    pub(super) body: SimulationRigidBodyObservation,
    pub(super) part_transform: [f32; 16],
    pub(super) material: AssemblyPartMaterial,
    pub(super) user_data: u32,
}

/// Live Assembly fields sampled at the tail of Create and every Update.
#[derive(Clone, Copy, Debug)]
pub(super) struct SerialisedAssemblyLiveAssemblyInput<'a> {
    pub(super) group: u32,
    pub(super) culling_table: Option<&'a SerialisedAssemblyCullingTableSource>,
}

/// Exact one-word `SerialisedAssembly::Params` payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SerialisedAssemblyParamsState {
    pub(super) edge_cos_bend_normal_threshold: f32,
}

impl SerialisedAssemblyParamsState {
    /// `Params::Params`, `0x82e0b680..0x82e0b690`.
    pub(super) const fn native_default() -> Self {
        Self {
            edge_cos_bend_normal_threshold: DEFAULT_EDGE_COS_BEND_NORMAL_THRESHOLD,
        }
    }
}

/// Pointer-free semantic image of the exact 0x40-byte native Header.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SerialisedAssemblyHeaderState {
    pub(super) num_volumes: u32,
    pub(super) num_rigid_bodies: u32,
    pub(super) num_static: u32,
    pub(super) group: u32,
    pub(super) culling_table: Option<SerialisedAssemblyCullingTableSource>,
    pub(super) culling_table_size: u32,
    pub(super) all_parts_frozen: bool,
    pub(super) assembly_object_id: ObjectId,
    pub(super) aabb: AabbWords,
}

/// One fully staged CPU Create result retaining the arena/culling owner and
/// the validated Part-order body occurrences.
///
/// It does not retain the body pool that owns those occurrences and therefore
/// is not yet the production session-lifetime bridge.
#[derive(Debug)]
pub(super) struct PreparedSerialisedAssembly {
    owner: Arc<SimulationNormalArenaPlan>,
    definition_object_id: ObjectId,
    header: SerialisedAssemblyHeaderState,
    params: SerialisedAssemblyParamsState,
    volume_data: SerialisedAssemblyVolumeDataSet,
    rigid_bodies: Box<[SimulationRigidBodyObservation]>,
    packed_word_authority: SerialisedAssemblyPackedBodyWordAuthority,
    // Host stale-frame guards, not fields added to the native Header. They do
    // not detect arbitrary body mutation within the same frame; the scheduler
    // must keep refresh immediately before its exclusive collision phase.
    volume_frame: Option<crate::simulation_rigid_body_pool::SimulationRigidBodyFrameIdentity>,
    bounds_frame: Option<crate::simulation_rigid_body_pool::SimulationRigidBodyFrameIdentity>,
}

impl PreparedSerialisedAssembly {
    pub(crate) fn has_bounds_for_frame(
        &self,
        frame: crate::simulation_rigid_body_pool::SimulationRigidBodyFrameIdentity,
    ) -> bool {
        self.volume_frame == Some(frame) && self.bounds_frame == Some(frame)
    }

    /// The ground job must not reinterpret retained packed transform words
    /// through an equal-valued but different Simulation image generation.
    pub(crate) fn matches_ground_image(&self, image: &SimulationPackedBodyWordImageDomain) -> bool {
        self.packed_word_authority.image_domain_epoch == image.epoch()
    }

    pub(super) fn header(&self) -> &SerialisedAssemblyHeaderState {
        &self.header
    }

    pub(super) const fn params(&self) -> SerialisedAssemblyParamsState {
        self.params
    }

    pub(super) fn volume_data(&self) -> &SerialisedAssemblyVolumeDataSet {
        &self.volume_data
    }

    pub(super) fn rigid_bodies(&self) -> &[SimulationRigidBodyObservation] {
        &self.rigid_bodies
    }

    pub(super) fn owner(&self) -> &Arc<SimulationNormalArenaPlan> {
        &self.owner
    }

    pub(super) const fn definition_object_id(&self) -> ObjectId {
        self.definition_object_id
    }

    /// Applies exactly the Header/VolumeData field set written by
    /// `SerialisedAssembly::Update` (`0x82e0cf30..0x82e0d35c`).
    ///
    /// Counts, Assembly identity, Params, body occurrence array, and Create
    /// AABB remain unchanged. Native can observe concurrent state changes
    /// between its traversal and late frozen rescan; the host captures each
    /// validated body once and commits atomically as an explicit safety policy.
    fn try_update_with_provider<W>(
        &mut self,
        template: &SerialisedAssemblyTemplate,
        body_pool: &SimulationRigidBodyPool,
        parts: &[SerialisedAssemblyCreatePartInput],
        live_volume_flags: &[u32],
        live_assembly: SerialisedAssemblyLiveAssemblyInput<'_>,
        packed_words: &mut W,
    ) -> Result<(), SerialisedAssemblyUpdateError<W::Error>>
    where
        W: SerialisedAssemblyPackedBodyWordProvider,
    {
        if !Arc::ptr_eq(&self.owner, template.owner())
            || self.header.assembly_object_id != template.assembly_object_id()
            || self.definition_object_id != template.definition_object_id()
        {
            return Err(SerialisedAssemblyUpdateError::TemplateMismatch);
        }
        if parts.len() != template.parts().len() {
            return Err(SerialisedAssemblyUpdateError::LivePartCount {
                template: template.parts().len(),
                live: parts.len(),
            });
        }
        if self.packed_word_authority != packed_words.authority() {
            return Err(SerialisedAssemblyUpdateError::PackedBodyWordAuthorityMismatch);
        }
        validate_culling_table(template, live_assembly.culling_table).map_err(
            |error| match error {
                CullingValidationError::OwnerMismatch => {
                    SerialisedAssemblyUpdateError::CullingOwnerMismatch
                }
                CullingValidationError::WordCount { stored, dimensions } => {
                    SerialisedAssemblyUpdateError::CullingWordCount { stored, dimensions }
                }
            },
        )?;
        self.volume_data
            .validate_update_shape(template, live_volume_flags)
            .map_err(SerialisedAssemblyUpdateError::VolumeData)?;
        packed_words
            .validate_context(template.owner(), body_pool)
            .map_err(SerialisedAssemblyUpdateError::PackedBodyWords)?;

        let mut transform_required = [false; 26];
        for source in template.update_volume_sources() {
            transform_required[usize::from(source.part_ordinal)] = true;
        }

        let mut body_snapshots = Vec::new();
        body_snapshots.try_reserve_exact(parts.len()).map_err(|_| {
            SerialisedAssemblyUpdateError::BodySnapshotAllocation {
                requested: parts.len(),
            }
        })?;
        let mut live_parts = Vec::new();
        live_parts.try_reserve_exact(parts.len()).map_err(|_| {
            SerialisedAssemblyUpdateError::LivePartStorageAllocation {
                requested: parts.len(),
            }
        })?;
        for (ordinal, part) in parts.iter().enumerate() {
            if self.rigid_bodies.get(ordinal).copied() != Some(part.body) {
                return Err(SerialisedAssemblyUpdateError::BodyOccurrenceChanged { ordinal });
            }
            let body = body_pool
                .try_serialised_assembly_snapshot(part.body)
                .map_err(SerialisedAssemblyUpdateError::Body)?;
            let body_transform = if transform_required[ordinal] {
                let packed = packed_words
                    .packed_transform_words(part.body)
                    .map_err(SerialisedAssemblyUpdateError::PackedBodyWords)?;
                packed_body_transform(body, packed)
            } else {
                [0.0; 16]
            };
            live_parts.push(SerialisedAssemblyLivePartState {
                part_transform: part.part_transform,
                body_transform,
                material: part.material,
                user_data: part.user_data,
                body_state: body.state,
            });
            body_snapshots.push(body);
        }

        self.bounds_frame = None;
        self.volume_frame = None;
        self.volume_data
            .try_update(template, &live_parts, live_volume_flags)
            .map_err(SerialisedAssemblyUpdateError::VolumeData)?;
        self.header.num_static = body_snapshots
            .iter()
            .fold(0_u32, |count, body| count + u32::from(body.state & 1 != 0));
        self.header.group = live_assembly.group;
        self.header.culling_table = live_assembly.culling_table.cloned();
        self.header.culling_table_size = live_assembly.culling_table.map_or(0, |source| {
            serialised_culling_table_size(source.table().rows, source.table().columns)
        });
        self.header.all_parts_frozen = body_snapshots
            .iter()
            .fold(1_u8, |all, body| all & u8::from(body.state & 2 != 0))
            != 0;
        self.volume_frame = body_pool.frame_identity();
        Ok(())
    }

    /// Production-shaped Update entry point. The concrete provider cannot be
    /// retained or detached from the template passed to this call.
    #[allow(
        clippy::too_many_arguments,
        reason = "the source-shaped Update boundary keeps every live owner and input explicit"
    )]
    pub(super) fn try_update_from_xenon(
        &mut self,
        template: &SerialisedAssemblyTemplate,
        body_pool: &SimulationRigidBodyPool,
        inertia_owner: &SimulationRigidBodyAddInertiaOwner,
        image_domain: &SimulationPackedBodyWordImageDomain,
        parts: &[SerialisedAssemblyCreatePartInput],
        live_volume_flags: &[u32],
        live_assembly: SerialisedAssemblyLiveAssemblyInput<'_>,
    ) -> Result<(), SerialisedAssemblyUpdateError<SimulationPackedBodyWordProviderError>> {
        let mut packed_words = SimulationPackedBodyWordProvider {
            image_domain,
            inertia_owner,
            body_pool,
        };
        self.try_update_with_provider(
            template,
            body_pool,
            parts,
            live_volume_flags,
            live_assembly,
            &mut packed_words,
        )
    }

    #[cfg(test)]
    pub(super) fn try_update<W>(
        &mut self,
        template: &SerialisedAssemblyTemplate,
        body_pool: &SimulationRigidBodyPool,
        parts: &[SerialisedAssemblyCreatePartInput],
        live_volume_flags: &[u32],
        live_assembly: SerialisedAssemblyLiveAssemblyInput<'_>,
        packed_words: &mut W,
    ) -> Result<(), SerialisedAssemblyUpdateError<W::Error>>
    where
        W: SerialisedAssemblyPackedBodyWordProvider,
    {
        self.try_update_with_provider(
            template,
            body_pool,
            parts,
            live_volume_flags,
            live_assembly,
            packed_words,
        )
    }
}

/// Prepares one complete CPU Create snapshot without externally visible
/// partial writes.
///
/// Native `culling_table` is a borrowed const live-Assembly pointer. Here it is
/// an owner-qualified immutable parsed snapshot. Passing the template's current
/// snapshot models the ordinary DMO Create value, but does not model later
/// mutation or alias visibility of the native table allocation.
#[allow(
    clippy::too_many_lines,
    reason = "the staged function keeps the source-ordered Create transaction auditable"
)]
fn try_prepare_serialised_assembly_create_with_provider<N, W>(
    template: &SerialisedAssemblyTemplate,
    body_pool: &SimulationRigidBodyPool,
    parts: &[SerialisedAssemblyCreatePartInput],
    live_assembly: SerialisedAssemblyLiveAssemblyInput<'_>,
    params: SerialisedAssemblyParamsState,
    numeric: &mut N,
    packed_words: &mut W,
) -> Result<PreparedSerialisedAssembly, SerialisedAssemblyCreateError<N::Error, W::Error>>
where
    N: SerialisedAssemblyPartBBoxNumeric,
    W: SerialisedAssemblyPackedBodyWordProvider,
{
    let packed_word_authority = packed_words.authority();
    if parts.len() != template.parts().len() {
        return Err(SerialisedAssemblyCreateError::LivePartCount {
            template: template.parts().len(),
            live: parts.len(),
        });
    }
    validate_culling_table(template, live_assembly.culling_table).map_err(|error| match error {
        CullingValidationError::OwnerMismatch => {
            SerialisedAssemblyCreateError::CullingOwnerMismatch
        }
        CullingValidationError::WordCount { stored, dimensions } => {
            SerialisedAssemblyCreateError::CullingWordCount { stored, dimensions }
        }
    })?;
    if let Some(selected_ordinal) = template
        .volume_sources()
        .iter()
        .position(|source| source.volume_type == VolumeType::Null)
    {
        // Native would enter the non-Aggregate vtable path. A Null vtable has
        // no deterministic callable target, so safe-host publication refuses
        // this malformed input before callbacks.
        return Err(SerialisedAssemblyCreateError::NullVolume { selected_ordinal });
    }
    let producer_count = template
        .volume_sources()
        .iter()
        .filter(|source| source.volume_type != VolumeType::Aggregate)
        .count();
    if producer_count == 0 {
        return Err(SerialisedAssemblyCreateError::NoDeterministicCpuAabb {
            selected_volumes: template.volume_sources().len(),
        });
    }
    packed_words
        .validate_context(template.owner(), body_pool)
        .map_err(SerialisedAssemblyCreateError::PackedBodyWords)?;

    let mut transform_required = [false; 26];
    let mut motion_required = [false; 26];
    for source in template.volume_sources() {
        transform_required[source.part_ordinal] = true;
        if source.volume_type != VolumeType::Aggregate {
            motion_required[source.part_ordinal] = true;
        }
    }

    let mut body_snapshots = Vec::new();
    body_snapshots.try_reserve_exact(parts.len()).map_err(|_| {
        SerialisedAssemblyCreateError::BodySnapshotAllocation {
            requested: parts.len(),
        }
    })?;
    let mut observations = Vec::new();
    observations.try_reserve_exact(parts.len()).map_err(|_| {
        SerialisedAssemblyCreateError::BodyObservationAllocation {
            requested: parts.len(),
        }
    })?;
    let mut live_parts = Vec::new();
    live_parts.try_reserve_exact(parts.len()).map_err(|_| {
        SerialisedAssemblyCreateError::LivePartStorageAllocation {
            requested: parts.len(),
        }
    })?;
    let mut motions = Vec::new();
    motions.try_reserve_exact(parts.len()).map_err(|_| {
        SerialisedAssemblyCreateError::BodyMotionAllocation {
            requested: parts.len(),
        }
    })?;

    for part in parts {
        let body = body_pool
            .try_serialised_assembly_snapshot(part.body)
            .map_err(SerialisedAssemblyCreateError::Body)?;
        let ordinal = body_snapshots.len();
        let body_transform = if transform_required[ordinal] {
            let packed = packed_words
                .packed_transform_words(part.body)
                .map_err(SerialisedAssemblyCreateError::PackedBodyWords)?;
            packed_body_transform(body, packed)
        } else {
            [0.0; 16]
        };
        let motion = if motion_required[ordinal] {
            let packed = packed_words
                .packed_motion_words(part.body)
                .map_err(SerialisedAssemblyCreateError::PackedBodyWords)?;
            Some(packed_body_motion(body, packed))
        } else {
            None
        };
        observations.push(body.observation);
        live_parts.push(SerialisedAssemblyLivePartState {
            part_transform: part.part_transform,
            body_transform,
            material: part.material,
            user_data: part.user_data,
            body_state: body.state,
        });
        motions.push(motion);
        body_snapshots.push(body);
    }

    let volume_data = SerialisedAssemblyVolumeDataSet::try_create(template, &live_parts)
        .map_err(SerialisedAssemblyCreateError::VolumeData)?;
    let mut aabb: Option<AabbWords> = None;
    for (source, record) in template.volume_sources().iter().zip(volume_data.records()) {
        if source.volume_type == VolumeType::Aggregate {
            continue;
        }
        let candidate = try_calculate_primitive_part_bbox(
            &record.volume,
            &record.transform,
            motions[source.part_ordinal]
                .expect("primitive source Part was marked as requiring body motion"),
            numeric,
        )
        .map_err(SerialisedAssemblyCreateError::Numeric)?;
        let Some(candidate) = candidate else {
            continue;
        };
        aabb = Some(match aabb {
            None => candidate,
            Some(accumulator) => AabbWords::from_f32(
                numeric
                    .minimum(accumulator.min_f32(), candidate.min_f32())
                    .map_err(SerialisedAssemblyCreateError::Numeric)?,
                numeric
                    .maximum(accumulator.max_f32(), candidate.max_f32())
                    .map_err(SerialisedAssemblyCreateError::Numeric)?,
            ),
        });
    }
    let aabb = aabb.ok_or(SerialisedAssemblyCreateError::NoDeterministicCpuAabb {
        selected_volumes: template.volume_sources().len(),
    })?;
    let num_static = body_snapshots
        .iter()
        .fold(0_u32, |count, body| count + u32::from(body.state & 1 != 0));
    let all_parts_frozen = body_snapshots
        .iter()
        .fold(1_u8, |all, body| all & u8::from(body.state & 2 != 0))
        != 0;
    let header = SerialisedAssemblyHeaderState {
        num_volumes: u32::try_from(volume_data.records().len())
            .expect("SerialisedAssembly VolumeData capacity is below u32::MAX"),
        num_rigid_bodies: u32::try_from(observations.len())
            .expect("SerialisedAssembly Part capacity is below u32::MAX"),
        num_static,
        group: live_assembly.group,
        culling_table: live_assembly.culling_table.cloned(),
        culling_table_size: live_assembly.culling_table.map_or(0, |source| {
            serialised_culling_table_size(source.table().rows, source.table().columns)
        }),
        all_parts_frozen,
        assembly_object_id: template.assembly_object_id(),
        aabb,
    };

    Ok(PreparedSerialisedAssembly {
        owner: Arc::clone(template.owner()),
        definition_object_id: template.definition_object_id(),
        header,
        params,
        volume_data,
        rigid_bodies: observations.into_boxed_slice(),
        packed_word_authority,
        volume_frame: body_pool.frame_identity(),
        bounds_frame: body_pool.frame_identity(),
    })
}

/// Production-shaped Create entry point. The private provider is built from
/// the exact template, pool, arena outcome owner, and Simulation image domain
/// for only the duration of this staged call.
#[allow(
    clippy::too_many_arguments,
    reason = "the source-shaped Create boundary keeps every live owner and input explicit"
)]
pub(super) fn try_prepare_serialised_assembly_create_from_xenon<N>(
    template: &SerialisedAssemblyTemplate,
    body_pool: &SimulationRigidBodyPool,
    inertia_owner: &SimulationRigidBodyAddInertiaOwner,
    image_domain: &SimulationPackedBodyWordImageDomain,
    parts: &[SerialisedAssemblyCreatePartInput],
    live_assembly: SerialisedAssemblyLiveAssemblyInput<'_>,
    params: SerialisedAssemblyParamsState,
    numeric: &mut N,
) -> Result<
    PreparedSerialisedAssembly,
    SerialisedAssemblyCreateError<N::Error, SimulationPackedBodyWordProviderError>,
>
where
    N: SerialisedAssemblyPartBBoxNumeric,
{
    let mut packed_words = SimulationPackedBodyWordProvider {
        image_domain,
        inertia_owner,
        body_pool,
    };
    try_prepare_serialised_assembly_create_with_provider(
        template,
        body_pool,
        parts,
        live_assembly,
        params,
        numeric,
        &mut packed_words,
    )
}

#[cfg(test)]
pub(super) fn try_prepare_serialised_assembly_create<N, W>(
    template: &SerialisedAssemblyTemplate,
    body_pool: &SimulationRigidBodyPool,
    parts: &[SerialisedAssemblyCreatePartInput],
    live_assembly: SerialisedAssemblyLiveAssemblyInput<'_>,
    params: SerialisedAssemblyParamsState,
    numeric: &mut N,
    packed_words: &mut W,
) -> Result<PreparedSerialisedAssembly, SerialisedAssemblyCreateError<N::Error, W::Error>>
where
    N: SerialisedAssemblyPartBBoxNumeric,
    W: SerialisedAssemblyPackedBodyWordProvider,
{
    try_prepare_serialised_assembly_create_with_provider(
        template,
        body_pool,
        parts,
        live_assembly,
        params,
        numeric,
        packed_words,
    )
}

fn validate_culling_table(
    template: &SerialisedAssemblyTemplate,
    culling_table: Option<&SerialisedAssemblyCullingTableSource>,
) -> Result<(), CullingValidationError> {
    let Some(source) = culling_table else {
        return Ok(());
    };
    // Safe-host publication policy. Native Create/Update trusts this pointer,
    // reads rows/columns, and neither proves arena ownership nor consults the
    // stored array-size word.
    if !Arc::ptr_eq(source.owner(), template.owner()) {
        return Err(CullingValidationError::OwnerMismatch);
    }
    let expected = source
        .table()
        .rows
        .wrapping_mul(source.table().columns)
        .wrapping_add(31)
        >> 5;
    if source.table().array_size < expected {
        return Err(CullingValidationError::WordCount {
            stored: source.table().array_size,
            dimensions: expected,
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CullingValidationError {
    OwnerMismatch,
    WordCount { stored: u32, dimensions: u32 },
}

/// Checked owner failure before exposing an attached Part's native full lanes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationSourceBodyTransformError {
    Body(SimulationRigidBodyPoolError),
    Words(SimulationPackedBodyWordProviderError),
}

impl fmt::Display for SimulationSourceBodyTransformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Body(source) => write!(formatter, "source body transform: {source}"),
            Self::Words(source) => write!(formatter, "source body transform words: {source}"),
        }
    }
}

impl Error for SimulationSourceBodyTransformError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Body(source) => Some(source),
            Self::Words(source) => Some(source),
        }
    }
}

/// `Part::GetTransform` 0x826c3480 reads the same packed body columns as Create.
/// Reuse the existing inseparable provider and layout; no canonical W fallback.
pub(crate) fn try_source_body_transform(
    template_owner: &Arc<SimulationNormalArenaPlan>,
    body_pool: &SimulationRigidBodyPool,
    inertia_owner: &SimulationRigidBodyAddInertiaOwner,
    image_domain: &SimulationPackedBodyWordImageDomain,
    body: SimulationRigidBodyObservation,
) -> Result<[f32; 16], SimulationSourceBodyTransformError> {
    let mut packed_words = SimulationPackedBodyWordProvider {
        image_domain,
        inertia_owner,
        body_pool,
    };
    packed_words
        .validate_context(template_owner, body_pool)
        .map_err(SimulationSourceBodyTransformError::Words)?;
    let snapshot = body_pool
        .try_serialised_assembly_snapshot(body)
        .map_err(SimulationSourceBodyTransformError::Body)?;
    let words = packed_words
        .packed_transform_words(body)
        .map_err(SimulationSourceBodyTransformError::Words)?;
    Ok(packed_body_transform(snapshot, words))
}

/// Same packed body read used by Create, also read by DynamicObject's
/// GetPartVelocity 82d2aa80. Preserve the intrusive-link W from the actual
/// address image; never substitute the LivingWorld transform-delta velocity.
pub(crate) fn try_source_body_motion(
    template_owner: &Arc<SimulationNormalArenaPlan>,
    body_pool: &SimulationRigidBodyPool,
    inertia_owner: &SimulationRigidBodyAddInertiaOwner,
    image_domain: &SimulationPackedBodyWordImageDomain,
    body: SimulationRigidBodyObservation,
) -> Result<SerialisedAssemblyBodyMotion, SimulationSourceBodyTransformError> {
    let mut packed_words = SimulationPackedBodyWordProvider {
        image_domain,
        inertia_owner,
        body_pool,
    };
    packed_words
        .validate_context(template_owner, body_pool)
        .map_err(SimulationSourceBodyTransformError::Words)?;
    let snapshot = body_pool
        .try_serialised_assembly_snapshot(body)
        .map_err(SimulationSourceBodyTransformError::Body)?;
    let words = packed_words
        .packed_motion_words(body)
        .map_err(SimulationSourceBodyTransformError::Words)?;
    Ok(packed_body_motion(snapshot, words))
}

fn packed_body_transform(
    body: SimulationRigidBodySerialisedAssemblySnapshot,
    packed: SerialisedAssemblyPackedBodyTransformWords,
) -> [f32; 16] {
    [
        body.right[0],
        body.right[1],
        body.right[2],
        f32::from_bits(packed.stasis),
        body.up[0],
        body.up[1],
        body.up[2],
        f32::from_bits(packed.inertia),
        body.at[0],
        body.at[1],
        body.at[2],
        f32::from_bits(body.tag),
        body.center_of_mass[0],
        body.center_of_mass[1],
        body.center_of_mass[2],
        f32::from_bits(packed.solver_delta_row),
    ]
}

fn packed_body_motion(
    body: SimulationRigidBodySerialisedAssemblySnapshot,
    packed: SerialisedAssemblyPackedBodyMotionWords,
) -> SerialisedAssemblyBodyMotion {
    SerialisedAssemblyBodyMotion {
        velocity_and_right: append_word(body.velocity, packed.right_link),
        omega_and_left: append_word(body.omega, packed.left_link),
        force_and_kine: append_word(body.force, body.kinematic_value.to_bits()),
        torque_and_cool: append_word(body.torque, body.cooldown),
    }
}

fn append_word(vector: [f32; 3], word: u32) -> [f32; 4] {
    [vector[0], vector[1], vector[2], f32::from_bits(word)]
}

fn serialised_culling_table_size(rows: u32, columns: u32) -> u32 {
    rows.wrapping_mul(columns)
        .wrapping_add(31)
        .wrapping_shr(5)
        .wrapping_add(3)
        .wrapping_mul(4)
}

#[derive(Debug, PartialEq)]
pub(super) enum SerialisedAssemblyCreateError<N, W> {
    LivePartCount { template: usize, live: usize },
    NullVolume { selected_ordinal: usize },
    NoDeterministicCpuAabb { selected_volumes: usize },
    CullingOwnerMismatch,
    CullingWordCount { stored: u32, dimensions: u32 },
    BodySnapshotAllocation { requested: usize },
    BodyObservationAllocation { requested: usize },
    LivePartStorageAllocation { requested: usize },
    BodyMotionAllocation { requested: usize },
    Body(SimulationRigidBodyPoolError),
    PackedBodyWords(W),
    VolumeData(SerialisedAssemblyVolumeDataStateError),
    Numeric(N),
}

#[derive(Debug, PartialEq)]
pub(super) enum SerialisedAssemblyUpdateError<W> {
    TemplateMismatch,
    LivePartCount { template: usize, live: usize },
    BodyOccurrenceChanged { ordinal: usize },
    PackedBodyWordAuthorityMismatch,
    CullingOwnerMismatch,
    CullingWordCount { stored: u32, dimensions: u32 },
    BodySnapshotAllocation { requested: usize },
    LivePartStorageAllocation { requested: usize },
    Body(SimulationRigidBodyPoolError),
    PackedBodyWords(W),
    VolumeData(SerialisedAssemblyVolumeDataStateError),
}

impl<N: fmt::Display, W: fmt::Display> fmt::Display for SerialisedAssemblyCreateError<N, W> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LivePartCount { template, live } => write!(
                formatter,
                "SerialisedAssembly has {template} template Parts but {live} live Parts"
            ),
            Self::NullVolume { selected_ordinal } => write!(
                formatter,
                "CPU SerialisedAssembly Create selected Null Volume {selected_ordinal}"
            ),
            Self::NoDeterministicCpuAabb { selected_volumes } => write!(
                formatter,
                "CPU SerialisedAssembly Create selected {selected_volumes} Volumes but none has a deterministic bbox"
            ),
            Self::CullingOwnerMismatch => {
                formatter.write_str("BitTable belongs to a different arena owner")
            }
            Self::CullingWordCount { stored, dimensions } => write!(
                formatter,
                "BitTable stores {stored} words but its dimensions require {dimensions}"
            ),
            Self::BodySnapshotAllocation { requested } => {
                write!(formatter, "could not reserve {requested} body snapshots")
            }
            Self::BodyObservationAllocation { requested } => {
                write!(formatter, "could not reserve {requested} body observations")
            }
            Self::LivePartStorageAllocation { requested } => {
                write!(formatter, "could not reserve {requested} live Parts")
            }
            Self::BodyMotionAllocation { requested } => {
                write!(formatter, "could not reserve {requested} body motions")
            }
            Self::Body(source) => write!(formatter, "SerialisedAssembly body is invalid: {source}"),
            Self::PackedBodyWords(source) => {
                write!(formatter, "packed body-word provider failed: {source}")
            }
            Self::VolumeData(source) => {
                write!(
                    formatter,
                    "SerialisedAssembly VolumeData is invalid: {source}"
                )
            }
            Self::Numeric(source) => write!(formatter, "bbox numeric callback failed: {source}"),
        }
    }
}

impl<N, W> Error for SerialisedAssemblyCreateError<N, W>
where
    N: Error + 'static,
    W: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Body(source) => Some(source),
            Self::PackedBodyWords(source) => Some(source),
            Self::VolumeData(source) => Some(source),
            Self::Numeric(source) => Some(source),
            _ => None,
        }
    }
}

impl<W: fmt::Display> fmt::Display for SerialisedAssemblyUpdateError<W> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TemplateMismatch => {
                formatter.write_str("SerialisedAssembly Update template differs from Create")
            }
            Self::LivePartCount { template, live } => write!(
                formatter,
                "SerialisedAssembly has {template} template Parts but {live} live Parts"
            ),
            Self::BodyOccurrenceChanged { ordinal } => write!(
                formatter,
                "SerialisedAssembly Part {ordinal} body occurrence differs from Create"
            ),
            Self::PackedBodyWordAuthorityMismatch => {
                formatter.write_str("packed body-word address-domain authority changed")
            }
            Self::CullingOwnerMismatch => {
                formatter.write_str("BitTable belongs to a different arena owner")
            }
            Self::CullingWordCount { stored, dimensions } => write!(
                formatter,
                "BitTable stores {stored} words but its dimensions require {dimensions}"
            ),
            Self::BodySnapshotAllocation { requested } => {
                write!(formatter, "could not reserve {requested} body snapshots")
            }
            Self::LivePartStorageAllocation { requested } => {
                write!(formatter, "could not reserve {requested} live Parts")
            }
            Self::Body(source) => write!(formatter, "SerialisedAssembly body is invalid: {source}"),
            Self::PackedBodyWords(source) => {
                write!(formatter, "packed body-word provider failed: {source}")
            }
            Self::VolumeData(source) => {
                write!(
                    formatter,
                    "SerialisedAssembly VolumeData is invalid: {source}"
                )
            }
        }
    }
}

impl<W> Error for SerialisedAssemblyUpdateError<W>
where
    W: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Body(source) => Some(source),
            Self::PackedBodyWords(source) => Some(source),
            Self::VolumeData(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        serialised_assembly_template::{
            SerialisedAssemblyPartInertiaSource, SerialisedAssemblyRuntimeInertia,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyPoolError},
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        simulation_xenon_address_image::{
            SimulationArenaXenonAddressBinding, SimulationRigidBodyAddInertiaInput,
            SimulationXenonAddressImage,
        },
        world::{CollisionArena, CollisionPrimitiveSet},
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::{ArenaResources, BaseResourceDescriptor},
    };

    fn packed_word_arena_owner(resource_zero_size: usize) -> Arc<SimulationNormalArenaPlan> {
        let resources = std::array::from_fn(|index| {
            if index == 0 {
                vec![0; resource_zero_size]
            } else {
                Vec::new()
            }
        });
        let resource_descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: if index == 0 { 0x10 } else { 0 },
        });
        let embedded_descriptors = std::array::from_fn(|index| ResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let arena = Arena {
            file_header: FileHeader {
                magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                is_big_endian: true,
                pointer_size_bits: 32,
                pointer_alignment: 4,
                unused: 0,
                major_version: *b"454\0",
                minor_version: *b"000\0",
                build_number: 0,
            },
            id: 0x8646_a1f7,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 16,
            virtual_size: u32::try_from(resource_zero_size).unwrap(),
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: embedded_descriptors,
            resources_used: embedded_descriptors,
            resource_offsets: [0; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        let decoded = DecodedArenaAsset {
            asset_id: 0x0123_4567_89ab_cdef,
            asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
            asset_resource_descriptors: resource_descriptors,
            resources: ArenaResources { resources },
            arena,
        };
        let asset = SimulationCollisionAsset {
            decoded,
            collision: CollisionArena {
                id: 0x8646_a1f7,
                grind_assets: Vec::new(),
                aggregate_roots: Vec::new(),
                primitive_sets: Vec::<CollisionPrimitiveSet>::new(),
                simple_primitive_sets: Vec::new(),
                meshes: Vec::new(),
                collision_models: Vec::new(),
                instances: Vec::new(),
            },
        };
        let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            asset,
            Vec::new(),
        ));
        Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap())
    }

    fn identity() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ]
    }

    #[test]
    fn preserves_every_nonportable_body_word_in_its_native_fourth_lane() {
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let inertia = Arc::new(SimulationInertia::native_default());
        let transform = [
            1.0, 2.0, 3.0, 0.0, 4.0, 5.0, 6.0, 0.0, 7.0, 8.0, 9.0, 0.0, 10.0, 11.0, 12.0, 1.0,
        ];
        let handle = pool
            .try_add_rigid_body(transform, Some(&inertia), 3)
            .unwrap()
            .unwrap();
        let observation = pool.try_observe(handle).unwrap();
        let body = pool.try_serialised_assembly_snapshot(observation).unwrap();
        let transform_words = SerialisedAssemblyPackedBodyTransformWords {
            stasis: 0x7fc1_0001,
            inertia: 0xffc2_0002,
            solver_delta_row: 0x8000_0000,
        };
        let motion_words = SerialisedAssemblyPackedBodyMotionWords {
            right_link: 0x3f00_0001,
            left_link: 0xbf00_0002,
        };

        let matrix = packed_body_transform(body, transform_words);
        assert_eq!(
            [matrix[3], matrix[7], matrix[11], matrix[15]].map(f32::to_bits),
            [
                transform_words.stasis,
                transform_words.inertia,
                body.tag,
                transform_words.solver_delta_row
            ]
        );
        let motion = packed_body_motion(body, motion_words);
        assert_eq!(
            motion.velocity_and_right[3].to_bits(),
            motion_words.right_link
        );
        assert_eq!(motion.omega_and_left[3].to_bits(), motion_words.left_link);
        assert_eq!(
            motion.force_and_kine[3].to_bits(),
            body.kinematic_value.to_bits()
        );
        assert_eq!(motion.torque_and_cool[3].to_bits(), body.cooldown);
    }

    #[test]
    fn keeps_default_params_bits_and_wrapping_culling_size_graph() {
        assert_eq!(
            SerialisedAssemblyParamsState::native_default()
                .edge_cos_bend_normal_threshold
                .to_bits(),
            0x3f7f_be77
        );
        assert_eq!(serialised_culling_table_size(0, 99), 12);
        assert_eq!(serialised_culling_table_size(2, 33), 24);
        assert_eq!(serialised_culling_table_size(u32::MAX, u32::MAX - 1), 16);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one literal fixture keeps all five fields and three native body lists comparable"
    )]
    fn real_provider_projects_all_five_native_words_from_live_owners() {
        let owner = packed_word_arena_owner(0x200);
        let source_a =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x110);
        let source_b =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x150);
        let semantic = Arc::new(SimulationInertia::native_default());
        let inertia_a =
            SerialisedAssemblyRuntimeInertia::from_parts_for_test(Arc::clone(&semantic), source_a);
        let inertia_b = SerialisedAssemblyRuntimeInertia::from_parts_for_test(semantic, source_b);
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x2000_0080, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let mut pool = SimulationRigidBodyPool::try_new(5).unwrap();
        let mut inertia_owner =
            SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
        let bodies = [
            inertia_owner
                .try_add(
                    &mut pool,
                    identity(),
                    SimulationRigidBodyAddInertiaInput::Definition(&inertia_a),
                    1,
                )
                .unwrap()
                .unwrap(),
            inertia_owner
                .try_add(
                    &mut pool,
                    identity(),
                    SimulationRigidBodyAddInertiaInput::Definition(&inertia_a),
                    2,
                )
                .unwrap()
                .unwrap(),
            inertia_owner
                .try_add(
                    &mut pool,
                    identity(),
                    SimulationRigidBodyAddInertiaInput::Definition(&inertia_b),
                    4,
                )
                .unwrap()
                .unwrap(),
            inertia_owner
                .try_add(
                    &mut pool,
                    identity(),
                    SimulationRigidBodyAddInertiaInput::Null,
                    2,
                )
                .unwrap()
                .unwrap(),
            inertia_owner
                .try_add(
                    &mut pool,
                    identity(),
                    SimulationRigidBodyAddInertiaInput::Null,
                    4,
                )
                .unwrap()
                .unwrap(),
        ];
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 5, 0, 0).unwrap();
        let image_domain = SimulationPackedBodyWordImageDomain::try_new(&pool, image).unwrap();
        let mut provider = SimulationPackedBodyWordProvider {
            image_domain: &image_domain,
            inertia_owner: &inertia_owner,
            body_pool: &pool,
        };
        provider.validate_context(&owner, &pool).unwrap();

        let expected_inertia = [0, 0x2000_0190, 0x2000_01d0, 0, 0];
        let expected_rows = [
            0x1000_0380,
            0x1000_03c0,
            0x1000_0400,
            0x1000_0440,
            0x1000_0480,
        ];
        let expected_links = [
            (0x1000_0570, 0x1000_0570),
            (0x1000_0990, 0x1000_0620),
            (0x1000_0a40, 0x1000_06d0),
            (0x1000_0620, 0x1000_0830),
            (0x1000_06d0, 0x1000_08e0),
        ];
        for (index, body) in bodies.into_iter().enumerate() {
            let transform = provider.packed_transform_words(body.observation()).unwrap();
            assert_eq!(transform.stasis, 0x1000_0080);
            assert_eq!(transform.inertia, expected_inertia[index]);
            assert_eq!(transform.solver_delta_row, expected_rows[index]);
            let motion = provider.packed_motion_words(body.observation()).unwrap();
            assert_eq!((motion.right_link, motion.left_link), expected_links[index]);
        }
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one occurrence story keeps live append/remove, ABA reuse, and bypass refusals adjacent"
    )]
    fn real_provider_tracks_live_links_and_refuses_unowned_occurrences() {
        let owner = packed_word_arena_owner(0x200);
        let source_a =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x10);
        let source_b =
            SerialisedAssemblyPartInertiaSource::from_parts_for_test(Arc::clone(&owner), 0, 0x50);
        let semantic = Arc::new(SimulationInertia::native_default());
        let inertia_a =
            SerialisedAssemblyRuntimeInertia::from_parts_for_test(Arc::clone(&semantic), source_a);
        let inertia_b =
            SerialisedAssemblyRuntimeInertia::from_parts_for_test(Arc::clone(&semantic), source_b);
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x2000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let mut inertia_owner =
            SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
        let first = inertia_owner
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&inertia_a),
                4,
            )
            .unwrap()
            .unwrap();
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 2, 0, 0).unwrap();
        let image_domain = SimulationPackedBodyWordImageDomain::try_new(&pool, image).unwrap();
        {
            let mut provider = SimulationPackedBodyWordProvider {
                image_domain: &image_domain,
                inertia_owner: &inertia_owner,
                body_pool: &pool,
            };
            assert_eq!(
                provider.packed_motion_words(first.observation()).unwrap(),
                SerialisedAssemblyPackedBodyMotionWords {
                    right_link: 0x1000_0610,
                    left_link: 0x1000_0610,
                }
            );
        }
        let second = inertia_owner
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&inertia_a),
                4,
            )
            .unwrap()
            .unwrap();
        {
            let mut provider = SimulationPackedBodyWordProvider {
                image_domain: &image_domain,
                inertia_owner: &inertia_owner,
                body_pool: &pool,
            };
            assert_eq!(
                provider.packed_motion_words(first.observation()).unwrap(),
                SerialisedAssemblyPackedBodyMotionWords {
                    right_link: 0x1000_0770,
                    left_link: 0x1000_0610,
                }
            );
        }

        inertia_owner.try_remove(&mut pool, first).unwrap();
        {
            let mut provider = SimulationPackedBodyWordProvider {
                image_domain: &image_domain,
                inertia_owner: &inertia_owner,
                body_pool: &pool,
            };
            assert_eq!(
                provider.packed_transform_words(first.observation()),
                Err(SimulationPackedBodyWordProviderError::BodyInertia(
                    SimulationRigidBodyAddInertiaOwnerError::Pool(
                        SimulationRigidBodyPoolError::BodyIsFree { slot: 0 }
                    )
                ))
            );
            assert_eq!(
                provider.packed_motion_words(second.observation()).unwrap(),
                SerialisedAssemblyPackedBodyMotionWords {
                    right_link: 0x1000_0610,
                    left_link: 0x1000_0610,
                }
            );
        }
        let reused = inertia_owner
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Definition(&inertia_b),
                4,
            )
            .unwrap()
            .unwrap();
        assert_eq!(reused.id(), first.id());
        let mut provider = SimulationPackedBodyWordProvider {
            image_domain: &image_domain,
            inertia_owner: &inertia_owner,
            body_pool: &pool,
        };
        assert_eq!(
            provider.packed_transform_words(first.observation()),
            Err(SimulationPackedBodyWordProviderError::BodyInertia(
                SimulationRigidBodyAddInertiaOwnerError::Pool(
                    SimulationRigidBodyPoolError::StaleHandle { slot: 0 }
                )
            ))
        );
        assert_eq!(
            provider
                .packed_transform_words(reused.observation())
                .unwrap(),
            SerialisedAssemblyPackedBodyTransformWords {
                stasis: 0x1000_0080,
                inertia: 0x2000_0050,
                solver_delta_row: 0x1000_0380,
            }
        );

        let mut direct_pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let direct_binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x3000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let direct_owner =
            SimulationRigidBodyAddInertiaOwner::try_new(&direct_pool, direct_binding).unwrap();
        let direct = direct_pool
            .try_add_rigid_body(identity(), Some(&semantic), 4)
            .unwrap()
            .unwrap();
        let direct_image = SimulationXenonAddressImage::try_new(0x1100_0080, 1, 0, 0).unwrap();
        let direct_domain =
            SimulationPackedBodyWordImageDomain::try_new(&direct_pool, direct_image).unwrap();
        let mut direct_provider = SimulationPackedBodyWordProvider {
            image_domain: &direct_domain,
            inertia_owner: &direct_owner,
            body_pool: &direct_pool,
        };
        assert_eq!(
            direct_provider.packed_transform_words(direct.observation()),
            Err(SimulationPackedBodyWordProviderError::BodyInertia(
                SimulationRigidBodyAddInertiaOwnerError::MissingOccurrenceOutcome { slot: 0 }
            ))
        );
        assert_eq!(
            direct_provider.packed_motion_words(direct.observation()),
            Err(SimulationPackedBodyWordProviderError::BodyInertia(
                SimulationRigidBodyAddInertiaOwnerError::MissingOccurrenceOutcome { slot: 0 }
            ))
        );

        let mut foreign_pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let foreign = foreign_pool
            .try_add_rigid_body(identity(), Some(&semantic), 4)
            .unwrap()
            .unwrap();
        assert_eq!(
            direct_provider.packed_motion_words(foreign.observation()),
            Err(SimulationPackedBodyWordProviderError::BodyInertia(
                SimulationRigidBodyAddInertiaOwnerError::Pool(
                    SimulationRigidBodyPoolError::ForeignHandle
                )
            ))
        );
        assert_eq!(second.id().index(), 1);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one two-arena fixture keeps routing, cross-arena links, and context precedence adjacent"
    )]
    fn provider_routes_two_arenas_and_orders_context_refusals() {
        let owner_a = packed_word_arena_owner(0x100);
        let owner_b = packed_word_arena_owner(0x100);
        let binding_a = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner_a),
                [0x2000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let binding_b = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner_b),
                [0x3000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let mut inertia_a = SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding_a).unwrap();
        let mut inertia_b = SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding_b).unwrap();
        let body_a = inertia_a
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Null,
                4,
            )
            .unwrap()
            .unwrap();
        let body_b = inertia_b
            .try_add(
                &mut pool,
                identity(),
                SimulationRigidBodyAddInertiaInput::Null,
                4,
            )
            .unwrap()
            .unwrap();
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 2, 0, 0).unwrap();
        let image_domain = SimulationPackedBodyWordImageDomain::try_new(&pool, image).unwrap();
        let mut provider_a = SimulationPackedBodyWordProvider {
            image_domain: &image_domain,
            inertia_owner: &inertia_a,
            body_pool: &pool,
        };
        let provider_b = SimulationPackedBodyWordProvider {
            image_domain: &image_domain,
            inertia_owner: &inertia_b,
            body_pool: &pool,
        };

        assert_ne!(provider_a.authority(), provider_b.authority());
        assert_eq!(
            provider_a
                .packed_motion_words(body_a.observation())
                .unwrap(),
            SerialisedAssemblyPackedBodyMotionWords {
                right_link: 0x1000_0770,
                left_link: 0x1000_0610,
            },
            "arena A may project its live neighbor from arena B through the shared Simulation image"
        );
        assert_eq!(
            provider_a.packed_transform_words(body_b.observation()),
            Err(SimulationPackedBodyWordProviderError::BodyInertia(
                SimulationRigidBodyAddInertiaOwnerError::MissingOccurrenceOutcome { slot: 1 }
            )),
            "arena A may not authorize arena B's body outcome"
        );

        let foreign_pool = SimulationRigidBodyPool::try_new(2).unwrap();
        assert_eq!(
            provider_a.validate_context(&owner_a, &foreign_pool),
            Err(SimulationPackedBodyWordProviderError::ForeignImageBodyPool)
        );
        assert_eq!(
            provider_a.validate_context(&owner_b, &pool),
            Err(SimulationPackedBodyWordProviderError::ForeignArenaOwner)
        );
        let foreign_binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner_a),
                [0x4000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let foreign_inertia =
            SimulationRigidBodyAddInertiaOwner::try_new(&foreign_pool, foreign_binding).unwrap();
        let foreign_inertia_provider = SimulationPackedBodyWordProvider {
            image_domain: &image_domain,
            inertia_owner: &foreign_inertia,
            body_pool: &pool,
        };
        assert_eq!(
            foreign_inertia_provider.validate_context(&owner_a, &pool),
            Err(SimulationPackedBodyWordProviderError::ForeignInertiaBodyPool)
        );
    }
}
