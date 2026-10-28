//! Transactional runtime ownership for dynamic physics assemblies.
//!
//! `rw::physics::Assembly::AddToSimulation` (`0x82ae0df0..0x82ae1158`)
//! adds Parts, Joints, then Drives and has three distinct rollback orders.
//! `Assembly::RemoveFromSimulation` (`0x82ae0898..0x82ae09b4`) removes Parts,
//! Joints, then Drives in forward order. This module preserves that lifecycle
//! around an abstract backend without reconstructing `Simulation` allocation,
//! rigid-body integration, joints, drives, or application publication.
//!
//! The backend contract deliberately has no exceptional error channel after a
//! transaction begins. Native adds report an unavailable slot with a null
//! handle, while pose reads and removals are assumed valid and infallible.
//! Host validation and allocation errors are handled before any backend call.
//! The separate private post-admission extension has typed host errors and
//! retains earlier successful drives, unlike the definition transaction.

use crate::{
    dynamic_clustered_mesh_template::DynamicClusteredMeshTemplate,
    world::{compose_collision_affine, effective_part_body_state, part_attachment_transform},
};
use std::{
    error::Error,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[path = "../../board/collision/skateboard_collision_groups.rs"]
pub(crate) mod board_collision_groups;
#[cfg(test)]
#[path = "../parts/dynamic_assembly_part_material_tests.rs"]
mod part_material_tests;

#[path = "../parts/dynamic_assembly_part_activation.rs"]
pub(crate) mod part_activation;

#[path = "dynamic_assembly_post_drives.rs"]
mod post_drives;
pub(crate) use post_drives::DynamicAssemblyPostDriveError;

#[path = "dynamic_assembly_hook_pool.rs"]
pub(crate) mod hook_pool;

#[path = "../parts/dynamic_assembly_part_transform.rs"]
pub(crate) mod part_transform;

#[path = "../../../living_world/entities/living_world_character_collision_publication.rs"]
pub(crate) mod character_publication;
#[path = "dynamic_assembly_living_world_drive.rs"]
pub(crate) mod living_world_drive;
#[path = "dynamic_assembly_source_parts.rs"]
pub(crate) mod source_parts;

#[path = "../parts/dynamic_assembly_part_contact_spy.rs"]
pub(crate) mod part_contact_spy;
#[cfg(test)]
#[path = "dynamic_assembly_set_velocity_tests.rs"]
pub(crate) mod set_velocity_tests;

#[path = "../../board/collision/skateboard_collision_publication.rs"]
pub(crate) mod skateboard_publication;

#[path = "dynamic_assembly_skater_pool.rs"]
pub(crate) mod skater_pool;

#[path = "../skeleton/dynamic_assembly_skeleton_normal.rs"]
pub(crate) mod skeleton_normal;

#[path = "../../skeleton/collision/skeleton_collision_publication.rs"]
pub(crate) mod skeleton_collision_publication;

/// Backend-independent payload retained for one Part in assembly order.
///
/// The session passes only `inertia` to the rigid-body backend. `material` is
/// retained because the native Part points at it for later collision work; it
/// is intentionally uninterpreted here.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicAssemblyPartPayload<I, M> {
    pub inertia: I,
    pub material: M,
}

/// Host snapshot corresponding to the runtime Part state used by attachment.
///
/// Rust preserves the matrix and both source-word bit patterns, then lowers
/// their nonzero meanings at the attachment/copyback use sites. Native
/// `Part::Initialize` instead lowers the static word into an internal-flags bit
/// and the inverse-present word into either a Definition-backed matrix pointer
/// or null before attachment. This immutable snapshot policy preserves the
/// recovered value behavior, not native pointer identity or later alias
/// mutation visibility.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DynamicAssemblyPartAttachmentMetadata {
    pub(crate) inverse_body_transform: [f32; 16],
    pub(crate) has_inverse_body_transform: u32,
    pub(crate) is_static_body: u32,
}

/// Sealed, equal-length Part metadata and payloads in preserved positional order.
#[derive(Debug)]
pub(crate) struct DynamicAssemblyPartAttachmentTemplate<I, M> {
    metadata: Vec<DynamicAssemblyPartAttachmentMetadata>,
    payloads: Vec<DynamicAssemblyPartPayload<I, M>>,
}

impl<I, M> DynamicAssemblyPartAttachmentTemplate<I, M> {
    /// Seals arrays while preserving caller-supplied positional order.
    ///
    /// Only equal lengths are validated here. Definition order is a caller
    /// precondition rather than an invariant inferred by this constructor.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the SerialisedAssembly source-aware backend bridge"
        )
    )]
    pub(crate) fn try_from_parts(
        metadata: Vec<DynamicAssemblyPartAttachmentMetadata>,
        payloads: Vec<DynamicAssemblyPartPayload<I, M>>,
    ) -> Result<Self, DynamicAssemblyPartAttachmentTemplateError> {
        if metadata.len() != payloads.len() {
            return Err(
                DynamicAssemblyPartAttachmentTemplateError::MetadataPayloadCount {
                    metadata: metadata.len(),
                    payloads: payloads.len(),
                },
            );
        }
        Ok(Self { metadata, payloads })
    }

    fn from_validated_parts(
        metadata: Vec<DynamicAssemblyPartAttachmentMetadata>,
        payloads: Vec<DynamicAssemblyPartPayload<I, M>>,
    ) -> Self {
        assert_eq!(
            metadata.len(),
            payloads.len(),
            "validated attachment metadata and payload lengths remain equal"
        );
        Self { metadata, payloads }
    }

    fn len(&self) -> usize {
        self.metadata.len()
    }

    fn payloads(&self) -> &[DynamicAssemblyPartPayload<I, M>] {
        &self.payloads
    }

    #[cfg(test)]
    pub(crate) fn metadata_for_test(&self) -> &[DynamicAssemblyPartAttachmentMetadata] {
        &self.metadata
    }

    #[cfg(test)]
    pub(crate) fn payloads_for_test(&self) -> &[DynamicAssemblyPartPayload<I, M>] {
        &self.payloads
    }
}

/// Safe-host shape refusal before a positional Part template is sealed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DynamicAssemblyPartAttachmentTemplateError {
    MetadataPayloadCount { metadata: usize, payloads: usize },
}

impl fmt::Display for DynamicAssemblyPartAttachmentTemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetadataPayloadCount { metadata, payloads } => write!(
                formatter,
                "dynamic assembly has {metadata} Part metadata records but {payloads} backend payloads"
            ),
        }
    }
}

impl Error for DynamicAssemblyPartAttachmentTemplateError {}

/// Immutable definition of one `JointedPair` in native array order.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicAssemblyJointDefinition<JF, JL> {
    /// Signed serialized `AssemblyDefinition::PartPair` index.
    pub parent_part: i32,
    /// Signed serialized `AssemblyDefinition::PartPair` index.
    pub child_part: i32,
    pub frames: JF,
    pub limits: JL,
}

/// Immutable definition of one `DrivenPair` in native array order.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicAssemblyDriveDefinition<DF, DD> {
    /// Signed serialized `AssemblyDefinition::PartPair` index.
    pub parent_part: i32,
    /// Signed serialized `AssemblyDefinition::PartPair` index.
    pub child_part: i32,
    pub frames: DF,
    pub dynamics: DD,
}

/// Shared immutable definition storage for all instances of one assembly.
///
/// The collision source and shared attachment template retain copied host
/// metadata and payload values for every session lifetime. This ownership does
/// not claim the Definition-backed pointer identity or alias visibility used by
/// native Parts.
#[derive(Debug)]
pub struct DynamicAssemblyRuntimeTemplate<I, M, JF, JL, DF, DD, C = DynamicClusteredMeshTemplate> {
    collision: Arc<C>,
    part_attachment: Arc<DynamicAssemblyPartAttachmentTemplate<I, M>>,
    joints: Vec<DynamicAssemblyJointDefinition<JF, JL>>,
    drives: Vec<DynamicAssemblyDriveDefinition<DF, DD>>,
}

impl<I, M, JF, JL, DF, DD> DynamicAssemblyRuntimeTemplate<I, M, JF, JL, DF, DD> {
    /// Validates all ordinal relationships before publishing a template.
    ///
    /// # Errors
    ///
    /// Returns a typed host error when the payload count differs from the
    /// collision Part count or a Joint/Drive references an absent Part.
    pub fn try_new(
        collision: Arc<DynamicClusteredMeshTemplate>,
        part_payloads: Vec<DynamicAssemblyPartPayload<I, M>>,
        joints: Vec<DynamicAssemblyJointDefinition<JF, JL>>,
        drives: Vec<DynamicAssemblyDriveDefinition<DF, DD>>,
    ) -> Result<Self, DynamicAssemblyTemplateError> {
        let part_count = collision.parts().len();
        validate_u32_count(DynamicAssemblyRuntimeKind::Part, part_count)?;
        validate_u32_count(DynamicAssemblyRuntimeKind::Joint, joints.len())?;
        validate_u32_count(DynamicAssemblyRuntimeKind::Drive, drives.len())?;
        if part_payloads.len() != part_count {
            return Err(DynamicAssemblyTemplateError::PartPayloadCount {
                collision_parts: part_count,
                payloads: part_payloads.len(),
            });
        }
        for (ordinal, joint) in joints.iter().enumerate() {
            validate_pair_indices(
                DynamicAssemblyPairKind::Joint,
                ordinal,
                joint.parent_part,
                joint.child_part,
                part_count,
            )?;
        }
        for (ordinal, drive) in drives.iter().enumerate() {
            validate_pair_indices(
                DynamicAssemblyPairKind::Drive,
                ordinal,
                drive.parent_part,
                drive.child_part,
                part_count,
            )?;
        }
        let mut metadata = Vec::new();
        metadata.try_reserve_exact(part_count).map_err(|_| {
            DynamicAssemblyTemplateError::PartMetadataStorageAllocation {
                requested: part_count,
            }
        })?;
        metadata.extend(collision.parts().iter().map(|part| {
            DynamicAssemblyPartAttachmentMetadata {
                inverse_body_transform: part.inverse_body_transform,
                has_inverse_body_transform: part.has_inverse_body_transform,
                is_static_body: part.is_static_body,
            }
        }));
        let part_attachment = Arc::new(
            DynamicAssemblyPartAttachmentTemplate::from_validated_parts(metadata, part_payloads),
        );
        Ok(Self {
            collision,
            part_attachment,
            joints,
            drives,
        })
    }
}

impl<I, M, JF, JL, DF, DD, C> DynamicAssemblyRuntimeTemplate<I, M, JF, JL, DF, DD, C> {
    /// Internal authored-definition route. The actual collision owner is
    /// retained, without manufacturing a serialized mesh capability. Part
    /// metadata/payload correspondence is the producer's source obligation;
    /// all joint/drive indices are checked before any backend effect.
    pub(crate) fn try_from_owned_parts(
        collision: Arc<C>,
        part_attachment: Arc<DynamicAssemblyPartAttachmentTemplate<I, M>>,
        joints: Vec<DynamicAssemblyJointDefinition<JF, JL>>,
        drives: Vec<DynamicAssemblyDriveDefinition<DF, DD>>,
    ) -> Result<Self, DynamicAssemblyTemplateError> {
        let part_count = part_attachment.len();
        validate_u32_count(DynamicAssemblyRuntimeKind::Part, part_count)?;
        validate_u32_count(DynamicAssemblyRuntimeKind::Joint, joints.len())?;
        validate_u32_count(DynamicAssemblyRuntimeKind::Drive, drives.len())?;
        for (ordinal, joint) in joints.iter().enumerate() {
            validate_pair_indices(
                DynamicAssemblyPairKind::Joint,
                ordinal,
                joint.parent_part,
                joint.child_part,
                part_count,
            )?;
        }
        for (ordinal, drive) in drives.iter().enumerate() {
            validate_pair_indices(
                DynamicAssemblyPairKind::Drive,
                ordinal,
                drive.parent_part,
                drive.child_part,
                part_count,
            )?;
        }
        Ok(Self {
            collision,
            part_attachment,
            joints,
            drives,
        })
    }

    #[must_use]
    pub const fn collision(&self) -> &Arc<C> {
        &self.collision
    }

    #[must_use]
    pub fn part_payloads(&self) -> &[DynamicAssemblyPartPayload<I, M>] {
        self.part_attachment.payloads()
    }

    #[must_use]
    pub fn joints(&self) -> &[DynamicAssemblyJointDefinition<JF, JL>] {
        &self.joints
    }

    #[must_use]
    pub fn drives(&self) -> &[DynamicAssemblyDriveDefinition<DF, DD>] {
        &self.drives
    }
}

fn validate_pair_indices(
    kind: DynamicAssemblyPairKind,
    ordinal: usize,
    parent_part: i32,
    child_part: i32,
    part_count: usize,
) -> Result<(), DynamicAssemblyTemplateError> {
    if usize::try_from(parent_part).map_or(true, |index| index >= part_count) {
        return Err(DynamicAssemblyTemplateError::PartIndex {
            kind,
            ordinal,
            endpoint: DynamicAssemblyPairEndpoint::Parent,
            part_index: parent_part,
            part_count,
        });
    }
    if usize::try_from(child_part).map_or(true, |index| index >= part_count) {
        return Err(DynamicAssemblyTemplateError::PartIndex {
            kind,
            ordinal,
            endpoint: DynamicAssemblyPairEndpoint::Child,
            part_index: child_part,
            part_count,
        });
    }
    Ok(())
}

fn validate_u32_count(
    kind: DynamicAssemblyRuntimeKind,
    count: usize,
) -> Result<(), DynamicAssemblyTemplateError> {
    if u32::try_from(count).is_err() {
        return Err(DynamicAssemblyTemplateError::CountOutsideU32 { kind, count });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicAssemblyPairKind {
    Joint,
    Drive,
}

impl fmt::Display for DynamicAssemblyPairKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Joint => "joint",
            Self::Drive => "drive",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicAssemblyPairEndpoint {
    Parent,
    Child,
}

impl fmt::Display for DynamicAssemblyPairEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Parent => "parent",
            Self::Child => "child",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DynamicAssemblyTemplateError {
    CountOutsideU32 {
        kind: DynamicAssemblyRuntimeKind,
        count: usize,
    },
    PartPayloadCount {
        collision_parts: usize,
        payloads: usize,
    },
    PartIndex {
        kind: DynamicAssemblyPairKind,
        ordinal: usize,
        endpoint: DynamicAssemblyPairEndpoint,
        part_index: i32,
        part_count: usize,
    },
    PartMetadataStorageAllocation {
        requested: usize,
    },
}

impl fmt::Display for DynamicAssemblyTemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountOutsideU32 { kind, count } => write!(
                formatter,
                "dynamic assembly {kind} count {count} does not fit the native u32 field"
            ),
            Self::PartPayloadCount {
                collision_parts,
                payloads,
            } => write!(
                formatter,
                "dynamic assembly has {collision_parts} collision Parts but {payloads} physics payloads"
            ),
            Self::PartIndex {
                kind,
                ordinal,
                endpoint,
                part_index,
                part_count,
            } => write!(
                formatter,
                "dynamic assembly {kind} {ordinal} {endpoint} Part {part_index} is outside {part_count} Parts"
            ),
            Self::PartMetadataStorageAllocation { requested } => write!(
                formatter,
                "could not reserve {requested} dynamic assembly Part metadata records"
            ),
        }
    }
}

impl Error for DynamicAssemblyTemplateError {}

/// Minimal Simulation surface needed by the proven Assembly transaction.
///
/// `None` from an add must leave no backend object behind. Every handle passed
/// to a pose/removal method must remain valid through that call, and those
/// methods must be infallible. These are safe-host preconditions which keep the
/// transaction's only failure branch identical to the native null checks.
pub trait DynamicAssemblySimulationBackend {
    type Inertia;
    type Material;
    type JointFrames;
    type JointLimits;
    type DriveFrames;
    type DriveDynamics;

    /// Stable identity which uniquely names one concrete backend instance.
    ///
    /// The token must be ABA-safe: it cannot be reused while any session or
    /// handle from the prior instance could survive. Reading it has no side
    /// effects, and it must remain unchanged throughout every attach/detach
    /// call and for the complete interval in which a session is attached.
    type Authority: Copy + Eq;

    /// Copy-only capability tokens cannot run cleanup when a session is
    /// dropped; backend removal remains explicit and ordered.
    type RigidBodyHandle: Copy;
    type JointHandle: Copy;
    type DriveHandle: Copy;

    fn authority(&self) -> Self::Authority;

    fn add_rigid_body(
        &mut self,
        transform: &[f32; 16],
        inertia: &Self::Inertia,
        body_state: u32,
    ) -> Option<Self::RigidBodyHandle>;

    fn rigid_body_pose(&self, body: &Self::RigidBodyHandle) -> [f32; 16];

    /// Part copyback after the body pose and optional inverse-body adjustment.
    /// Source-qualified backends override this to retain packed full lanes;
    /// ordinary backends keep the existing explicitly canonical host policy.
    fn rigid_body_part_transform(
        &self,
        body: &Self::RigidBodyHandle,
        inverse: Option<&[f32; 16]>,
    ) -> [f32; 16] {
        let pose = self.rigid_body_pose(body);
        inverse.map_or(pose, |inverse| compose_collision_affine(pose, *inverse))
    }

    fn remove_rigid_body(&mut self, body: &Self::RigidBodyHandle);

    fn add_joint(
        &mut self,
        parent: &Self::RigidBodyHandle,
        child: &Self::RigidBodyHandle,
        frames: &Self::JointFrames,
        limits: &Self::JointLimits,
    ) -> Option<Self::JointHandle>;

    fn remove_joint(&mut self, joint: &Self::JointHandle);

    fn add_drive(
        &mut self,
        parent: &Self::RigidBodyHandle,
        child: &Self::RigidBodyHandle,
        frames: &Self::DriveFrames,
        dynamics: &Self::DriveDynamics,
    ) -> Option<Self::DriveHandle>;

    fn remove_drive(&mut self, drive: &Self::DriveHandle);
}

pub type DynamicAssemblyBackendTemplate<B, C = DynamicClusteredMeshTemplate> =
    DynamicAssemblyRuntimeTemplate<
        <B as DynamicAssemblySimulationBackend>::Inertia,
        <B as DynamicAssemblySimulationBackend>::Material,
        <B as DynamicAssemblySimulationBackend>::JointFrames,
        <B as DynamicAssemblySimulationBackend>::JointLimits,
        <B as DynamicAssemblySimulationBackend>::DriveFrames,
        <B as DynamicAssemblySimulationBackend>::DriveDynamics,
        C,
    >;

#[derive(Clone, Copy, Debug)]
enum DynamicAssemblyPartTransformCache {
    Known([f32; 16]),
    Partial(crate::skeleton_root_transforms::SkeletonRootMatrix),
}
impl DynamicAssemblyPartTransformCache {
    fn known(&self) -> Option<&[f32; 16]> {
        match self {
            Self::Known(matrix) => Some(matrix),
            Self::Partial(_) => None,
        }
    }
    fn typed(&self) -> crate::skeleton_root_transforms::SkeletonRootMatrix {
        match self {
            Self::Known(matrix) => {
                crate::skeleton_root_transforms::SkeletonRootMatrix::known(*matrix)
            }
            Self::Partial(matrix) => *matrix,
        }
    }
    fn from_typed(matrix: crate::skeleton_root_transforms::SkeletonRootMatrix) -> Self {
        matrix
            .known_lanes()
            .map_or(Self::Partial(matrix), Self::Known)
    }
}

struct DynamicAssemblyPartRuntime<H, M> {
    part_transform: DynamicAssemblyPartTransformCache,
    body: Option<H>,
    // Native Part+0x50 can change independently of the immutable Definition.
    // None keeps the retained template material, without requiring M: Clone.
    material_override: Option<Arc<M>>,
}

struct DynamicAssemblyJointRuntime<H> {
    joint: Option<H>,
}

struct DynamicAssemblyDriveRuntime<H> {
    drive: Option<H>,
}

/// Reusable host owner of the positionally ordered Part attachment transaction.
struct DynamicAssemblyPartAttachmentTransaction<B>
where
    B: DynamicAssemblySimulationBackend,
{
    template: Arc<DynamicAssemblyPartAttachmentTemplate<B::Inertia, B::Material>>,
    parts: Vec<DynamicAssemblyPartRuntime<B::RigidBodyHandle, B::Material>>,
    material_revision: Arc<AtomicU64>,
}

impl<B> DynamicAssemblyPartAttachmentTransaction<B>
where
    B: DynamicAssemblySimulationBackend,
{
    fn try_new(
        template: Arc<DynamicAssemblyPartAttachmentTemplate<B::Inertia, B::Material>>,
        initial_part_transforms: Vec<[f32; 16]>,
    ) -> Result<Self, DynamicAssemblySessionCreateError> {
        let part_count = template.len();
        if initial_part_transforms.len() != part_count {
            return Err(DynamicAssemblySessionCreateError::PartTransformCount {
                parts: part_count,
                transforms: initial_part_transforms.len(),
            });
        }
        let mut parts = Vec::new();
        parts.try_reserve_exact(part_count).map_err(|_| {
            DynamicAssemblySessionCreateError::RuntimeStorageAllocation {
                kind: DynamicAssemblyRuntimeKind::Part,
                requested: part_count,
            }
        })?;
        parts.extend(initial_part_transforms.into_iter().map(|part_transform| {
            DynamicAssemblyPartRuntime {
                part_transform: DynamicAssemblyPartTransformCache::Known(part_transform),
                body: None,
                material_override: None,
            }
        }));
        Ok(Self {
            template,
            parts,
            material_revision: Arc::new(AtomicU64::new(0)),
        })
    }

    fn part_transforms(&self) -> impl ExactSizeIterator<Item = Option<&[f32; 16]>> {
        self.parts.iter().map(|part| part.part_transform.known())
    }

    fn attached_body(&self, ordinal: usize) -> &B::RigidBodyHandle {
        self.parts[ordinal]
            .body
            .as_ref()
            .expect("validated attached Part")
    }

    fn try_attach_all(&mut self, backend: &mut B, requested_body_state: u32) -> Result<(), usize> {
        debug_assert!(self.parts.iter().all(|part| part.body.is_none()));
        for ordinal in 0..self.parts.len() {
            let metadata = self.template.metadata[ordinal];
            let inverse_body_transform = (metadata.has_inverse_body_transform != 0)
                .then_some(&metadata.inverse_body_transform);
            let transform = part_attachment_transform(
                *self.parts[ordinal].part_transform.known().expect("partial cache can only be written while attached; detach copies back known body pose"),
                inverse_body_transform,
            );
            let internal_flags = u16::from(metadata.is_static_body != 0);
            let body_state = effective_part_body_state(requested_body_state, internal_flags);
            self.parts[ordinal].body = backend.add_rigid_body(
                &transform,
                &self.template.payloads[ordinal].inertia,
                body_state,
            );
            if self.parts[ordinal].body.is_none() {
                for rollback_ordinal in (0..ordinal).rev() {
                    self.copy_back_and_remove(backend, rollback_ordinal);
                }
                return Err(ordinal);
            }
        }
        Ok(())
    }

    fn remove_all_forward(&mut self, backend: &mut B) {
        for ordinal in 0..self.parts.len() {
            self.copy_back_and_remove(backend, ordinal);
        }
    }

    fn copy_back_and_remove(&mut self, backend: &mut B, ordinal: usize) {
        let metadata = self.template.metadata[ordinal];
        let part = &mut self.parts[ordinal];
        let body = part
            .body
            .as_ref()
            .expect("transaction-owned live Part body");
        let inverse =
            (metadata.has_inverse_body_transform != 0).then_some(&metadata.inverse_body_transform);
        part.part_transform = DynamicAssemblyPartTransformCache::Known(
            backend.rigid_body_part_transform(body, inverse),
        );
        backend.remove_rigid_body(body);
        part.body = None;
    }
}

/// Attach refusal for the crate-private handle-free Part-only session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DynamicAssemblyBodyOnlyPartAttachError {
    AlreadyAttached,
    PartUnavailable { ordinal: usize },
}

impl fmt::Display for DynamicAssemblyBodyOnlyPartAttachError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyAttached => formatter.write_str("dynamic Assembly Parts are attached"),
            Self::PartUnavailable { ordinal } => {
                write!(
                    formatter,
                    "dynamic Assembly Part {ordinal} could not attach"
                )
            }
        }
    }
}

impl Error for DynamicAssemblyBodyOnlyPartAttachError {}

/// Detach refusal for the crate-private handle-free Part-only session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DynamicAssemblyBodyOnlyPartDetachError {
    NotAttached,
    BackendAuthorityMismatch,
}

impl fmt::Display for DynamicAssemblyBodyOnlyPartDetachError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAttached => formatter.write_str("dynamic Assembly Parts are not attached"),
            Self::BackendAuthorityMismatch => {
                formatter.write_str("dynamic Assembly Parts belong to another backend authority")
            }
        }
    }
}

impl Error for DynamicAssemblyBodyOnlyPartDetachError {}

/// Crate-private Part lifecycle with no body-handle or payload access.
///
/// Initial transforms and requested body state remain explicit caller inputs;
/// this staged owner does not claim their live DMO source. Dropping an attached
/// session performs no implicit cleanup.
pub(crate) struct DynamicAssemblyBodyOnlyPartSession<B>
where
    B: DynamicAssemblySimulationBackend,
{
    transaction: DynamicAssemblyPartAttachmentTransaction<B>,
    attached_authority: Option<B::Authority>,
}

impl<B> DynamicAssemblyBodyOnlyPartSession<B>
where
    B: DynamicAssemblySimulationBackend,
{
    pub(crate) fn try_new(
        template: Arc<DynamicAssemblyPartAttachmentTemplate<B::Inertia, B::Material>>,
        initial_part_transforms: Vec<[f32; 16]>,
    ) -> Result<Self, DynamicAssemblySessionCreateError> {
        Ok(Self {
            transaction: DynamicAssemblyPartAttachmentTransaction::try_new(
                template,
                initial_part_transforms,
            )?,
            attached_authority: None,
        })
    }

    pub(crate) const fn is_attached(&self) -> bool {
        self.attached_authority.is_some()
    }

    pub(crate) fn part_transforms(&self) -> impl ExactSizeIterator<Item = &[f32; 16]> {
        self.transaction
            .part_transforms()
            .map(|matrix| matrix.expect("body-only sessions have no partial-cache setter"))
    }

    /// `Part::GetTransform` 0x826c3480: an attached Part's body pose with the
    /// optional Definition inverse (the same rule as detach copyback), else
    /// the retained cache.
    pub(crate) fn world_part_transforms(&self, backend: &B) -> Vec<[f32; 16]> {
        (0..self.transaction.parts.len())
            .map(|ordinal| self.world_part_transform(backend, ordinal).expect("valid part ordinal"))
            .collect()
    }

    /// Single-Part view of the same getter, without allocating or reading other bodies.
    pub(crate) fn world_part_transform(&self, backend: &B, ordinal: usize) -> Option<[f32; 16]> {
        let transaction = &self.transaction;
        let part = transaction.parts.get(ordinal)?;
        Some(match &part.body {
                Some(body) => {
                    let metadata = transaction.template.metadata[ordinal];
                    let inverse = (metadata.has_inverse_body_transform != 0)
                        .then_some(&metadata.inverse_body_transform);
                    backend.rigid_body_part_transform(body, inverse)
                }
                None => *part
                    .part_transform
                    .known()
                    .expect("body-only sessions have no partial-cache setter"),
            })
    }

    pub(crate) fn attach(
        &mut self,
        backend: &mut B,
        requested_body_state: u32,
    ) -> Result<(), DynamicAssemblyBodyOnlyPartAttachError> {
        if self.attached_authority.is_some() {
            return Err(DynamicAssemblyBodyOnlyPartAttachError::AlreadyAttached);
        }
        let authority = backend.authority();
        self.transaction
            .try_attach_all(backend, requested_body_state)
            .map_err(
                |ordinal| DynamicAssemblyBodyOnlyPartAttachError::PartUnavailable { ordinal },
            )?;
        self.attached_authority = Some(authority);
        Ok(())
    }

    pub(crate) fn detach(
        &mut self,
        backend: &mut B,
    ) -> Result<(), DynamicAssemblyBodyOnlyPartDetachError> {
        let Some(authority) = self.attached_authority else {
            return Err(DynamicAssemblyBodyOnlyPartDetachError::NotAttached);
        };
        if authority != backend.authority() {
            return Err(DynamicAssemblyBodyOnlyPartDetachError::BackendAuthorityMismatch);
        }
        self.transaction.remove_all_forward(backend);
        self.attached_authority = None;
        Ok(())
    }
}

/// One mutable Assembly instance backed by a shared immutable definition.
///
/// Initial Part transforms are explicit because the live DMO placement path
/// overwrites definition identity transforms before attachment. This module
/// does not derive them from a catalog locator. Dropping an attached session
/// performs no implicit cleanup: its Simulation owner must call [`Self::detach`]
/// while the backend is still available.
///
/// Backend handle tokens are deliberately private:
///
/// ```compile_fail
/// use skate2_core::dynamic_assembly_session::{
///     DynamicAssemblySession, DynamicAssemblySimulationBackend,
/// };
/// fn cannot_extract_handle<B: DynamicAssemblySimulationBackend>(
///     session: &DynamicAssemblySession<B>,
/// ) {
///     let _ = session.part_body_handle(0);
/// }
/// ```
pub struct DynamicAssemblySession<B, C = DynamicClusteredMeshTemplate>
where
    B: DynamicAssemblySimulationBackend,
{
    template: Arc<DynamicAssemblyBackendTemplate<B, C>>,
    part_attachment: DynamicAssemblyPartAttachmentTransaction<B>,
    joints: Vec<DynamicAssemblyJointRuntime<B::JointHandle>>,
    drives: Vec<DynamicAssemblyDriveRuntime<B::DriveHandle>>,
    // Constructor-created drives are not definition drives and do not share
    // Assembly attachment's rollback-on-null policy. Their tokens stay private.
    post_attachment_drives: Vec<post_drives::DynamicAssemblyPostDrive<B::DriveHandle>>,
    // Cross-assembly drives owned by another session keep these bodies attached.
    // These guards are host lifetime policy, not source body IDs or constraints.
    incoming_cross_drives: Vec<post_drives::DynamicAssemblyCrossDriveGuard>,
    attached_authority: Option<B::Authority>,
    pub(crate) board_collision_groups: Arc<board_collision_groups::BoardCollisionGroups>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DynamicAssemblyPartMaterialError {
    NotAttached,
    BackendAuthorityMismatch,
    PartOutOfRange { ordinal: usize, count: usize },
    MissingBody { ordinal: usize },
    RevisionExhausted,
}
impl fmt::Display for DynamicAssemblyPartMaterialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "live Part material: {self:?}")
    }
}
impl Error for DynamicAssemblyPartMaterialError {}

impl<B, C> DynamicAssemblySession<B, C>
where
    B: DynamicAssemblySimulationBackend,
{
    /// Resolve the live Part pointer projection. The authored payload remains
    /// immutable; selected runtime owners are retained by the Part itself.
    pub(crate) fn current_part_material(
        &self,
        ordinal: usize,
    ) -> Result<&B::Material, DynamicAssemblyPartMaterialError> {
        let part = self.part_attachment.parts.get(ordinal).ok_or(
            DynamicAssemblyPartMaterialError::PartOutOfRange {
                ordinal,
                count: self.part_attachment.parts.len(),
            },
        )?;
        Ok(part
            .material_override
            .as_deref()
            .unwrap_or(&self.part_attachment.template.payloads[ordinal].material))
    }

    /// One source Part-pointer store, used four times by 0x82b29fb8. The caller
    /// validates its selected material's owning `SurfacePhysics` occurrence; this
    /// generic primitive preserves the existing private attached-body authority.
    /// No multi-Part preflight, rollback, coefficient copy or detach reset occurs.
    pub(crate) fn try_set_part_material(
        &mut self,
        backend: &B,
        ordinal: usize,
        material: Arc<B::Material>,
    ) -> Result<(), DynamicAssemblyPartMaterialError> {
        use DynamicAssemblyPartMaterialError::{
            BackendAuthorityMismatch, MissingBody, NotAttached, PartOutOfRange, RevisionExhausted,
        };
        let authority = self.attached_authority.ok_or(NotAttached)?;
        if authority != backend.authority() {
            return Err(BackendAuthorityMismatch);
        }
        let count = self.part_attachment.parts.len();
        let part = self
            .part_attachment
            .parts
            .get_mut(ordinal)
            .ok_or(PartOutOfRange { ordinal, count })?;
        if part.body.is_none() {
            return Err(MissingBody { ordinal });
        }
        let revision = self
            .part_attachment
            .material_revision
            .load(Ordering::Acquire);
        let next = revision.checked_add(1).ok_or(RevisionExhausted)?;
        part.material_override = Some(material);
        // Only this exclusive session can write the revision. Publications can
        // observe it without owning a mutable alias to the Part slots.
        self.part_attachment
            .material_revision
            .store(next, Ordering::Release);
        Ok(())
    }

    /// Constructs a detached session without touching a backend.
    ///
    /// # Errors
    ///
    /// Returns a typed error if there is not exactly one explicit initial
    /// transform per Part or if host runtime-state allocation fails. No
    /// partially constructed session is returned.
    pub fn try_new(
        template: Arc<DynamicAssemblyBackendTemplate<B, C>>,
        initial_part_transforms: Vec<[f32; 16]>,
    ) -> Result<Self, DynamicAssemblySessionCreateError> {
        let part_attachment = DynamicAssemblyPartAttachmentTransaction::try_new(
            Arc::clone(&template.part_attachment),
            initial_part_transforms,
        )?;
        let mut joints = Vec::new();
        joints
            .try_reserve_exact(template.joints.len())
            .map_err(
                |_| DynamicAssemblySessionCreateError::RuntimeStorageAllocation {
                    kind: DynamicAssemblyRuntimeKind::Joint,
                    requested: template.joints.len(),
                },
            )?;
        let mut drives = Vec::new();
        drives
            .try_reserve_exact(template.drives.len())
            .map_err(
                |_| DynamicAssemblySessionCreateError::RuntimeStorageAllocation {
                    kind: DynamicAssemblyRuntimeKind::Drive,
                    requested: template.drives.len(),
                },
            )?;

        joints.extend(
            (0..template.joints.len()).map(|_| DynamicAssemblyJointRuntime { joint: None }),
        );
        drives.extend(
            (0..template.drives.len()).map(|_| DynamicAssemblyDriveRuntime { drive: None }),
        );
        Ok(Self {
            template,
            part_attachment,
            joints,
            drives,
            post_attachment_drives: Vec::new(),
            incoming_cross_drives: Vec::new(),
            attached_authority: None,
            board_collision_groups: Arc::new(
                board_collision_groups::BoardCollisionGroups::default(),
            ),
        })
    }

    #[must_use]
    pub const fn template(&self) -> &Arc<DynamicAssemblyBackendTemplate<B, C>> {
        &self.template
    }

    #[must_use]
    pub const fn is_attached(&self) -> bool {
        self.attached_authority.is_some()
    }

    /// Raw cached transforms, preserving Part ordinals. A row is `None` when
    /// an attached typed setter retained unknown W lanes. Use the live Part
    /// getter for body poses; incomplete cache lanes must not become zeros.
    #[must_use]
    pub fn part_transforms(&self) -> impl ExactSizeIterator<Item = Option<&[f32; 16]>> {
        self.part_attachment.part_transforms()
    }

    /// Runs the recovered Part/Joint/Drive call and rollback orders at
    /// `0x82ae0df0..0x82ae1158` around the caller's opaque backend behavior.
    ///
    /// # Errors
    ///
    /// `AlreadyAttached` is a safe-host precondition. Backend-unavailable
    /// errors correspond to native null returns and leave the session fully
    /// detached after the stage-specific rollback.
    ///
    /// # Panics
    ///
    /// Panics only if this module's private, constructor-validated Part-pair
    /// or live-handle invariants have been broken internally.
    pub fn attach(
        &mut self,
        backend: &mut B,
        requested_body_state: u32,
    ) -> Result<(), DynamicAssemblyAttachError> {
        if self.attached_authority.is_some() {
            return Err(DynamicAssemblyAttachError::AlreadyAttached);
        }
        let authority = backend.authority();

        if let Err(ordinal) = self
            .part_attachment
            .try_attach_all(backend, requested_body_state)
        {
            return Err(DynamicAssemblyAttachError::PartUnavailable { ordinal });
        }

        for ordinal in 0..self.joints.len() {
            let definition = &self.template.joints[ordinal];
            let parent_part = usize::try_from(definition.parent_part)
                .expect("validated nonnegative Joint parent Part");
            let child_part = usize::try_from(definition.child_part)
                .expect("validated nonnegative Joint child Part");
            let parent = self.part_attachment.attached_body(parent_part);
            let child = self.part_attachment.attached_body(child_part);
            self.joints[ordinal].joint =
                backend.add_joint(parent, child, &definition.frames, &definition.limits);
            if self.joints[ordinal].joint.is_none() {
                for rollback_ordinal in (0..ordinal).rev() {
                    remove_joint(backend, &mut self.joints[rollback_ordinal]);
                }
                self.part_attachment.remove_all_forward(backend);
                return Err(DynamicAssemblyAttachError::JointUnavailable { ordinal });
            }
        }

        for ordinal in 0..self.drives.len() {
            let definition = &self.template.drives[ordinal];
            let parent_part = usize::try_from(definition.parent_part)
                .expect("validated nonnegative Drive parent Part");
            let child_part = usize::try_from(definition.child_part)
                .expect("validated nonnegative Drive child Part");
            let parent = self.part_attachment.attached_body(parent_part);
            let child = self.part_attachment.attached_body(child_part);
            self.drives[ordinal].drive =
                backend.add_drive(parent, child, &definition.frames, &definition.dynamics);
            if self.drives[ordinal].drive.is_none() {
                for rollback_ordinal in (0..ordinal).rev() {
                    remove_drive(backend, &mut self.drives[rollback_ordinal]);
                }
                self.part_attachment.remove_all_forward(backend);
                for rollback_ordinal in 0..self.joints.len() {
                    remove_joint(backend, &mut self.joints[rollback_ordinal]);
                }
                return Err(DynamicAssemblyAttachError::DriveUnavailable { ordinal });
            }
        }

        self.attached_authority = Some(authority);
        Ok(())
    }

    /// Removes Parts, Joints, then Drives in native forward order.
    ///
    /// # Errors
    ///
    /// Returns a safe-host precondition error without removal effects when the
    /// session is detached, belongs to another backend authority, or still has
    /// owned/incoming post-admission drives requiring explicit prior cleanup.
    pub fn detach(&mut self, backend: &mut B) -> Result<(), DynamicAssemblyDetachError> {
        self.try_validate_detach(backend)?;
        self.part_attachment.remove_all_forward(backend);
        for joint in &mut self.joints {
            remove_joint(backend, joint);
        }
        for drive in &mut self.drives {
            remove_drive(backend, drive);
        }
        self.incoming_cross_drives.clear();
        self.attached_authority = None;
        Ok(())
    }

    /// The same nonmutating host guards used by detach, so an enclosing Island
    /// cleanup can refuse before removing any actual Part spy registration.
    /// This does not execute native cleanup or sample retained payload locks.
    pub(crate) fn try_validate_detach(
        &self,
        backend: &B,
    ) -> Result<(), DynamicAssemblyDetachError> {
        if self.attached_authority.is_none() {
            return Err(DynamicAssemblyDetachError::NotAttached);
        }
        if self.attached_authority != Some(backend.authority()) {
            return Err(DynamicAssemblyDetachError::BackendAuthorityMismatch);
        }
        if !self.post_attachment_drives.is_empty() {
            return Err(DynamicAssemblyDetachError::PostAttachmentDrivesPresent);
        }
        if self
            .incoming_cross_drives
            .iter()
            .any(post_drives::DynamicAssemblyCrossDriveGuard::is_live)
        {
            return Err(DynamicAssemblyDetachError::IncomingCrossDrivesPresent);
        }
        Ok(())
    }
}

fn remove_joint<B>(backend: &mut B, runtime: &mut DynamicAssemblyJointRuntime<B::JointHandle>)
where
    B: DynamicAssemblySimulationBackend,
{
    let joint = runtime
        .joint
        .as_ref()
        .expect("transaction-owned live Joint");
    backend.remove_joint(joint);
    runtime.joint = None;
}

fn remove_drive<B>(backend: &mut B, runtime: &mut DynamicAssemblyDriveRuntime<B::DriveHandle>)
where
    B: DynamicAssemblySimulationBackend,
{
    let drive = runtime
        .drive
        .as_ref()
        .expect("transaction-owned live Drive");
    backend.remove_drive(drive);
    runtime.drive = None;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicAssemblyRuntimeKind {
    Part,
    Joint,
    Drive,
}

impl fmt::Display for DynamicAssemblyRuntimeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Part => "Part",
            Self::Joint => "Joint",
            Self::Drive => "Drive",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DynamicAssemblySessionCreateError {
    PartTransformCount {
        parts: usize,
        transforms: usize,
    },
    RuntimeStorageAllocation {
        kind: DynamicAssemblyRuntimeKind,
        requested: usize,
    },
}

impl fmt::Display for DynamicAssemblySessionCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PartTransformCount { parts, transforms } => write!(
                formatter,
                "dynamic assembly has {parts} Parts but {transforms} initial transforms"
            ),
            Self::RuntimeStorageAllocation { kind, requested } => write!(
                formatter,
                "failed to allocate runtime storage for {requested} dynamic assembly {kind}s"
            ),
        }
    }
}

impl Error for DynamicAssemblySessionCreateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicAssemblyAttachError {
    AlreadyAttached,
    PartUnavailable { ordinal: usize },
    JointUnavailable { ordinal: usize },
    DriveUnavailable { ordinal: usize },
}

impl fmt::Display for DynamicAssemblyAttachError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyAttached => formatter.write_str("dynamic assembly is already attached"),
            Self::PartUnavailable { ordinal } => {
                write!(formatter, "dynamic assembly Part {ordinal} was unavailable")
            }
            Self::JointUnavailable { ordinal } => {
                write!(
                    formatter,
                    "dynamic assembly Joint {ordinal} was unavailable"
                )
            }
            Self::DriveUnavailable { ordinal } => {
                write!(
                    formatter,
                    "dynamic assembly Drive {ordinal} was unavailable"
                )
            }
        }
    }
}

impl Error for DynamicAssemblyAttachError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicAssemblyDetachError {
    NotAttached,
    BackendAuthorityMismatch,
    PostAttachmentDrivesPresent,
    IncomingCrossDrivesPresent,
    SkeletonDrivesPresent,
}

impl fmt::Display for DynamicAssemblyDetachError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAttached => formatter.write_str("dynamic assembly is not attached"),
            Self::BackendAuthorityMismatch => {
                formatter.write_str("dynamic assembly belongs to a different backend authority")
            }
            Self::PostAttachmentDrivesPresent => formatter
                .write_str("constructor-created drives must be removed before assembly bodies"),
            Self::SkeletonDrivesPresent => {
                formatter.write_str("skeleton drive owner must be released before assembly bodies")
            }
            Self::IncomingCrossDrivesPresent => formatter.write_str(
                "another assembly's connecting drives must be removed before these bodies",
            ),
        }
    }
}

impl Error for DynamicAssemblyDetachError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dynamic_clustered_mesh_template::DynamicClusteredMeshTemplatePart,
        world::{CollisionArena, CollisionRoot},
    };
    use skate2_formats::collision::Volume;
    use std::collections::{BTreeMap, BTreeSet};

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 0.0,
    ];

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct BodyHandle(u32);

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct JointHandle(u32);

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct DriveHandle(u32);

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        AddBody {
            inertia: u32,
            state: u32,
            transform: [u32; 16],
        },
        BodyPose(u32),
        RemoveBody(u32),
        AddJoint {
            parent: u32,
            child: u32,
            frames: u32,
            limits: u32,
        },
        RemoveJoint(u32),
        AddDrive {
            parent: u32,
            child: u32,
            frames: u32,
            dynamics: u32,
        },
        RemoveDrive(u32),
    }

    #[derive(Default)]
    struct MockBackend {
        authority: u32,
        events: std::cell::RefCell<Vec<Event>>,
        fail_body: Option<usize>,
        fail_joint: Option<usize>,
        fail_drive: Option<usize>,
        body_calls: usize,
        joint_calls: usize,
        drive_calls: usize,
        body_poses: BTreeMap<u32, [f32; 16]>,
        live_bodies: BTreeSet<u32>,
        live_joints: BTreeSet<u32>,
        live_drives: BTreeSet<u32>,
    }

    impl MockBackend {
        fn events(&self) -> Vec<Event> {
            self.events.borrow().clone()
        }

        fn push(&self, event: Event) {
            self.events.borrow_mut().push(event);
        }
    }

    impl DynamicAssemblySimulationBackend for MockBackend {
        type Inertia = u32;
        type Material = u32;
        type JointFrames = u32;
        type JointLimits = u32;
        type DriveFrames = u32;
        type DriveDynamics = u32;
        type Authority = u32;
        type RigidBodyHandle = BodyHandle;
        type JointHandle = JointHandle;
        type DriveHandle = DriveHandle;

        fn authority(&self) -> u32 {
            self.authority
        }

        fn add_rigid_body(
            &mut self,
            transform: &[f32; 16],
            inertia: &u32,
            body_state: u32,
        ) -> Option<BodyHandle> {
            let call = self.body_calls;
            self.body_calls += 1;
            self.push(Event::AddBody {
                inertia: *inertia,
                state: body_state,
                transform: transform.map(f32::to_bits),
            });
            if self.fail_body == Some(call) {
                return None;
            }
            let handle = BodyHandle(u32::try_from(call).expect("test call fits u32") + 100);
            self.body_poses.entry(handle.0).or_insert(*transform);
            assert!(self.live_bodies.insert(handle.0));
            Some(handle)
        }

        fn rigid_body_pose(&self, body: &BodyHandle) -> [f32; 16] {
            assert!(self.live_bodies.contains(&body.0));
            self.push(Event::BodyPose(body.0));
            self.body_poses[&body.0]
        }

        fn remove_rigid_body(&mut self, body: &BodyHandle) {
            assert!(self.live_bodies.contains(&body.0));
            self.push(Event::RemoveBody(body.0));
            assert!(self.live_bodies.remove(&body.0));
        }

        fn add_joint(
            &mut self,
            parent: &BodyHandle,
            child: &BodyHandle,
            frames: &u32,
            limits: &u32,
        ) -> Option<JointHandle> {
            let call = self.joint_calls;
            self.joint_calls += 1;
            self.push(Event::AddJoint {
                parent: parent.0,
                child: child.0,
                frames: *frames,
                limits: *limits,
            });
            if self.fail_joint == Some(call) {
                return None;
            }
            let handle = JointHandle(u32::try_from(call).expect("test call fits u32") + 200);
            assert!(self.live_joints.insert(handle.0));
            Some(handle)
        }

        fn remove_joint(&mut self, joint: &JointHandle) {
            assert!(self.live_joints.contains(&joint.0));
            self.push(Event::RemoveJoint(joint.0));
            assert!(self.live_joints.remove(&joint.0));
        }

        fn add_drive(
            &mut self,
            parent: &BodyHandle,
            child: &BodyHandle,
            frames: &u32,
            dynamics: &u32,
        ) -> Option<DriveHandle> {
            let call = self.drive_calls;
            self.drive_calls += 1;
            self.push(Event::AddDrive {
                parent: parent.0,
                child: child.0,
                frames: *frames,
                dynamics: *dynamics,
            });
            if self.fail_drive == Some(call) {
                return None;
            }
            let handle = DriveHandle(u32::try_from(call).expect("test call fits u32") + 300);
            assert!(self.live_drives.insert(handle.0));
            Some(handle)
        }

        fn remove_drive(&mut self, drive: &DriveHandle) {
            assert!(self.live_drives.contains(&drive.0));
            self.push(Event::RemoveDrive(drive.0));
            assert!(self.live_drives.remove(&drive.0));
        }
    }

    type TestTemplate = DynamicAssemblyRuntimeTemplate<u32, u32, u32, u32, u32, u32>;

    fn marked_transform(marker: f32) -> [f32; 16] {
        let mut transform = IDENTITY;
        transform[12] = marker;
        transform[13] = marker + 1.0;
        transform[14] = marker + 2.0;
        transform
    }

    fn primitive_volume() -> Volume {
        Volume {
            transform: IDENTITY,
            vtable_offset: 1,
            shape_words: [0; 3],
            radius: 1.0,
            group_id: 0,
            surface_id: 0,
            flags: 1,
        }
    }

    fn collision_template(
        parts: Vec<DynamicClusteredMeshTemplatePart>,
    ) -> Arc<DynamicClusteredMeshTemplate> {
        let roots = (0..parts.len())
            .map(|index| CollisionRoot {
                dictionary_index: u32::try_from(index).expect("test index fits u32"),
                volume: primitive_volume(),
                aggregate: None,
            })
            .collect();
        let arena = Arc::new(CollisionArena {
            id: 1,
            grind_assets: Vec::new(),
            aggregate_roots: roots,
            primitive_sets: Vec::new(),
            simple_primitive_sets: Vec::new(),
            meshes: Vec::new(),
            collision_models: Vec::new(),
            instances: Vec::new(),
        });
        Arc::new(DynamicClusteredMeshTemplate::try_new(arena, parts, false).unwrap())
    }

    fn part(
        root_index: usize,
        inverse: Option<[f32; 16]>,
        is_static_raw: u32,
    ) -> DynamicClusteredMeshTemplatePart {
        DynamicClusteredMeshTemplatePart {
            root_index,
            inverse_body_transform: inverse.unwrap_or(IDENTITY),
            has_inverse_body_transform: inverse.map_or(0, |_| 7),
            is_static_body: is_static_raw,
        }
    }

    fn runtime_template(
        parts: Vec<DynamicClusteredMeshTemplatePart>,
        joints: Vec<DynamicAssemblyJointDefinition<u32, u32>>,
        drives: Vec<DynamicAssemblyDriveDefinition<u32, u32>>,
    ) -> Arc<TestTemplate> {
        let part_count = parts.len();
        let collision = collision_template(parts);
        Arc::new(
            DynamicAssemblyRuntimeTemplate::try_new(
                collision,
                (0..part_count)
                    .map(|index| DynamicAssemblyPartPayload {
                        inertia: u32::try_from(index).expect("test index fits u32") + 10,
                        material: u32::try_from(index).expect("test index fits u32") + 20,
                    })
                    .collect(),
                joints,
                drives,
            )
            .unwrap(),
        )
    }

    fn two_joints() -> Vec<DynamicAssemblyJointDefinition<u32, u32>> {
        vec![
            DynamicAssemblyJointDefinition {
                parent_part: 0,
                child_part: 1,
                frames: 30,
                limits: 40,
            },
            DynamicAssemblyJointDefinition {
                parent_part: 1,
                child_part: 2,
                frames: 31,
                limits: 41,
            },
        ]
    }

    fn two_drives() -> Vec<DynamicAssemblyDriveDefinition<u32, u32>> {
        vec![
            DynamicAssemblyDriveDefinition {
                parent_part: 0,
                child_part: 1,
                frames: 50,
                dynamics: 60,
            },
            DynamicAssemblyDriveDefinition {
                parent_part: 1,
                child_part: 2,
                frames: 51,
                dynamics: 61,
            },
        ]
    }

    fn three_part_session() -> DynamicAssemblySession<MockBackend> {
        let template = runtime_template(
            vec![part(0, None, 0), part(1, None, 0), part(2, None, 0)],
            two_joints(),
            two_drives(),
        );
        DynamicAssemblySession::try_new(
            template,
            vec![
                marked_transform(1.0),
                marked_transform(2.0),
                marked_transform(3.0),
            ],
        )
        .unwrap()
    }

    fn assert_no_runtime_handles(session: &DynamicAssemblySession<MockBackend>) {
        assert!(
            session
                .part_attachment
                .parts
                .iter()
                .all(|part| part.body.is_none())
        );
        assert!(session.joints.iter().all(|joint| joint.joint.is_none()));
        assert!(session.drives.iter().all(|drive| drive.drive.is_none()));
    }

    #[test]
    fn success_adds_parts_joints_and_drives_forward_and_detaches_forward() {
        let mut session = three_part_session();
        let mut backend = MockBackend::default();
        session.attach(&mut backend, 4).unwrap();
        assert!(session.is_attached());
        assert_eq!(session.attached_authority, Some(0));
        assert_eq!(
            backend.events(),
            vec![
                Event::AddBody {
                    inertia: 10,
                    state: 4,
                    transform: marked_transform(1.0).map(f32::to_bits)
                },
                Event::AddBody {
                    inertia: 11,
                    state: 4,
                    transform: marked_transform(2.0).map(f32::to_bits)
                },
                Event::AddBody {
                    inertia: 12,
                    state: 4,
                    transform: marked_transform(3.0).map(f32::to_bits)
                },
                Event::AddJoint {
                    parent: 100,
                    child: 101,
                    frames: 30,
                    limits: 40
                },
                Event::AddJoint {
                    parent: 101,
                    child: 102,
                    frames: 31,
                    limits: 41
                },
                Event::AddDrive {
                    parent: 100,
                    child: 101,
                    frames: 50,
                    dynamics: 60
                },
                Event::AddDrive {
                    parent: 101,
                    child: 102,
                    frames: 51,
                    dynamics: 61
                },
            ]
        );

        session.detach(&mut backend).unwrap();
        assert!(!session.is_attached());
        assert_eq!(session.attached_authority, None);
        assert_no_runtime_handles(&session);
        assert_eq!(
            &backend.events()[7..],
            &[
                Event::BodyPose(100),
                Event::RemoveBody(100),
                Event::BodyPose(101),
                Event::RemoveBody(101),
                Event::BodyPose(102),
                Event::RemoveBody(102),
                Event::RemoveJoint(200),
                Event::RemoveJoint(201),
                Event::RemoveDrive(300),
                Event::RemoveDrive(301),
            ]
        );
    }

    #[test]
    fn body_failure_rolls_prior_parts_back_in_reverse() {
        let mut session = three_part_session();
        let first_pose = marked_transform(40.0);
        let second_pose = marked_transform(50.0);
        let mut backend = MockBackend {
            fail_body: Some(2),
            ..MockBackend::default()
        };
        backend.body_poses.insert(100, first_pose);
        backend.body_poses.insert(101, second_pose);
        assert_eq!(
            session.attach(&mut backend, 2),
            Err(DynamicAssemblyAttachError::PartUnavailable { ordinal: 2 })
        );
        assert_eq!(
            backend
                .events()
                .iter()
                .map(|event| match event {
                    Event::AddBody { inertia, .. } => format!("add{inertia}"),
                    Event::BodyPose(body) => format!("pose{body}"),
                    Event::RemoveBody(body) => format!("remove{body}"),
                    _ => "unexpected".to_owned(),
                })
                .collect::<Vec<_>>(),
            [
                "add10",
                "add11",
                "add12",
                "pose101",
                "remove101",
                "pose100",
                "remove100"
            ]
        );
        assert!(!session.is_attached());
        assert_eq!(session.attached_authority, None);
        assert!(
            session
                .part_attachment
                .parts
                .iter()
                .all(|part| part.body.is_none())
        );
        assert!(backend.live_bodies.is_empty());
        assert!(backend.live_joints.is_empty());
        assert!(backend.live_drives.is_empty());
        assert_no_runtime_handles(&session);
        assert_eq!(
            session
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .map(|transform| transform.map(f32::to_bits))
                .collect::<Vec<_>>(),
            vec![
                first_pose.map(f32::to_bits),
                second_pose.map(f32::to_bits),
                marked_transform(3.0).map(f32::to_bits),
            ]
        );
    }

    #[test]
    fn joint_failure_reverses_prior_joints_then_parts_forward() {
        let mut session = three_part_session();
        let mut backend = MockBackend {
            fail_joint: Some(1),
            ..MockBackend::default()
        };
        assert_eq!(
            session.attach(&mut backend, 0),
            Err(DynamicAssemblyAttachError::JointUnavailable { ordinal: 1 })
        );
        assert_eq!(
            &backend.events()[3..],
            &[
                Event::AddJoint {
                    parent: 100,
                    child: 101,
                    frames: 30,
                    limits: 40
                },
                Event::AddJoint {
                    parent: 101,
                    child: 102,
                    frames: 31,
                    limits: 41
                },
                Event::RemoveJoint(200),
                Event::BodyPose(100),
                Event::RemoveBody(100),
                Event::BodyPose(101),
                Event::RemoveBody(101),
                Event::BodyPose(102),
                Event::RemoveBody(102),
            ]
        );
        assert_eq!(session.attached_authority, None);
        assert!(backend.live_bodies.is_empty());
        assert!(backend.live_joints.is_empty());
        assert!(backend.live_drives.is_empty());
        assert_no_runtime_handles(&session);
    }

    #[test]
    fn drive_failure_reverses_drives_then_parts_and_joints_forward() {
        let mut session = three_part_session();
        let mut backend = MockBackend {
            fail_drive: Some(1),
            ..MockBackend::default()
        };
        assert_eq!(
            session.attach(&mut backend, 0),
            Err(DynamicAssemblyAttachError::DriveUnavailable { ordinal: 1 })
        );
        assert_eq!(
            &backend.events()[5..],
            &[
                Event::AddDrive {
                    parent: 100,
                    child: 101,
                    frames: 50,
                    dynamics: 60
                },
                Event::AddDrive {
                    parent: 101,
                    child: 102,
                    frames: 51,
                    dynamics: 61
                },
                Event::RemoveDrive(300),
                Event::BodyPose(100),
                Event::RemoveBody(100),
                Event::BodyPose(101),
                Event::RemoveBody(101),
                Event::BodyPose(102),
                Event::RemoveBody(102),
                Event::RemoveJoint(200),
                Event::RemoveJoint(201),
            ]
        );
        assert_eq!(session.attached_authority, None);
        assert!(backend.live_bodies.is_empty());
        assert!(backend.live_joints.is_empty());
        assert!(backend.live_drives.is_empty());
        assert_no_runtime_handles(&session);
    }

    #[test]
    fn part_seed_state_and_copyback_use_template_metadata() {
        let inverse = marked_transform(4.0);
        let initial = marked_transform(8.0);
        let template = runtime_template(vec![part(0, Some(inverse), 9)], Vec::new(), Vec::new());
        let mut session = DynamicAssemblySession::try_new(template, vec![initial]).unwrap();
        let pose = marked_transform(12.0);
        let mut backend = MockBackend::default();
        backend.body_poses.insert(100, pose);
        session.attach(&mut backend, 4).unwrap();
        assert_eq!(
            backend.events()[0],
            Event::AddBody {
                inertia: 10,
                state: 1,
                transform: part_attachment_transform(initial, Some(&inverse)).map(f32::to_bits),
            }
        );
        session.detach(&mut backend).unwrap();
        assert_eq!(
            session
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .next()
                .unwrap()
                .map(f32::to_bits),
            compose_collision_affine(pose, inverse).map(f32::to_bits)
        );
    }

    #[test]
    fn attachment_template_copies_exact_bits_and_keeps_public_owners_and_payload_order() {
        let mut inverse = marked_transform(4.0);
        inverse[3] = f32::from_bits(0x8000_0000);
        inverse[7] = f32::from_bits(0x7fc0_1234);
        inverse[11] = f32::from_bits(0x0000_0001);
        let source_part = DynamicClusteredMeshTemplatePart {
            root_index: 0,
            inverse_body_transform: inverse,
            has_inverse_body_transform: 0x8000_0000,
            is_static_body: 0x8000_0000,
        };
        let collision = collision_template(vec![source_part]);
        let template = Arc::new(
            DynamicAssemblyRuntimeTemplate::try_new(
                Arc::clone(&collision),
                vec![DynamicAssemblyPartPayload {
                    inertia: 0x1122_3344,
                    material: 0x5566_7788,
                }],
                Vec::<DynamicAssemblyJointDefinition<u32, u32>>::new(),
                Vec::<DynamicAssemblyDriveDefinition<u32, u32>>::new(),
            )
            .unwrap(),
        );
        assert!(Arc::ptr_eq(template.collision(), &collision));
        assert_eq!(
            template.part_payloads(),
            [DynamicAssemblyPartPayload {
                inertia: 0x1122_3344,
                material: 0x5566_7788,
            }]
        );
        let metadata = template.part_attachment.metadata[0];
        assert_eq!(
            metadata.inverse_body_transform.map(f32::to_bits),
            inverse.map(f32::to_bits)
        );
        assert_eq!(metadata.has_inverse_body_transform, 0x8000_0000);
        assert_eq!(metadata.is_static_body, 0x8000_0000);

        let initial = marked_transform(8.0);
        let mut session = DynamicAssemblySession::try_new(template, vec![initial]).unwrap();
        let pose = marked_transform(12.0);
        let mut backend = MockBackend::default();
        backend.body_poses.insert(100, pose);
        session.attach(&mut backend, 4).unwrap();
        assert_eq!(
            backend.events()[0],
            Event::AddBody {
                inertia: 0x1122_3344,
                state: 1,
                transform: part_attachment_transform(initial, Some(&inverse)).map(f32::to_bits),
            }
        );
        session.detach(&mut backend).unwrap();
        assert_eq!(
            session
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .next()
                .unwrap()
                .map(f32::to_bits),
            compose_collision_affine(pose, inverse).map(f32::to_bits)
        );
    }

    #[test]
    fn attachment_template_try_from_parts_accepts_equal_nonempty_shape() {
        let metadata = vec![DynamicAssemblyPartAttachmentMetadata {
            inverse_body_transform: marked_transform(2.0),
            has_inverse_body_transform: 0x8000_0000,
            is_static_body: 0,
        }];
        let payloads = vec![DynamicAssemblyPartPayload {
            inertia: 0x1122_3344_u32,
            material: 0x5566_7788_u32,
        }];
        let sealed = DynamicAssemblyPartAttachmentTemplate::try_from_parts(
            metadata.clone(),
            payloads.clone(),
        )
        .unwrap();
        assert_eq!(sealed.metadata, metadata);
        assert_eq!(sealed.payloads(), payloads);
    }

    #[test]
    fn two_part_attachment_keeps_metadata_payload_and_add_order_paired() {
        let inverse0 = marked_transform(2.0);
        let inverse1 = marked_transform(6.0);
        let source_parts = vec![
            DynamicClusteredMeshTemplatePart {
                root_index: 0,
                inverse_body_transform: inverse0,
                has_inverse_body_transform: 0x8000_0000,
                is_static_body: 0,
            },
            DynamicClusteredMeshTemplatePart {
                root_index: 1,
                inverse_body_transform: inverse1,
                has_inverse_body_transform: 0x4000_0000,
                is_static_body: 0x8000_0000,
            },
        ];
        let payloads = vec![
            DynamicAssemblyPartPayload {
                inertia: 0x1111_1111,
                material: 0x2222_2222,
            },
            DynamicAssemblyPartPayload {
                inertia: 0x3333_3333,
                material: 0x4444_4444,
            },
        ];
        let template = Arc::new(
            DynamicAssemblyRuntimeTemplate::try_new(
                collision_template(source_parts),
                payloads.clone(),
                Vec::<DynamicAssemblyJointDefinition<u32, u32>>::new(),
                Vec::<DynamicAssemblyDriveDefinition<u32, u32>>::new(),
            )
            .unwrap(),
        );
        let metadata0 = template.part_attachment.metadata[0];
        let metadata1 = template.part_attachment.metadata[1];
        assert_eq!(
            metadata0.inverse_body_transform.map(f32::to_bits),
            inverse0.map(f32::to_bits)
        );
        assert_eq!(
            (
                metadata0.has_inverse_body_transform,
                metadata0.is_static_body
            ),
            (0x8000_0000, 0)
        );
        assert_eq!(
            metadata1.inverse_body_transform.map(f32::to_bits),
            inverse1.map(f32::to_bits)
        );
        assert_eq!(
            (
                metadata1.has_inverse_body_transform,
                metadata1.is_static_body
            ),
            (0x4000_0000, 0x8000_0000)
        );
        assert_eq!(template.part_payloads(), payloads);

        let initial0 = marked_transform(10.0);
        let initial1 = marked_transform(20.0);
        let mut session =
            DynamicAssemblySession::try_new(template, vec![initial0, initial1]).unwrap();
        let mut backend = MockBackend::default();
        session.attach(&mut backend, 4).unwrap();
        assert_eq!(
            backend.events(),
            vec![
                Event::AddBody {
                    inertia: 0x1111_1111,
                    state: 4,
                    transform: part_attachment_transform(initial0, Some(&inverse0))
                        .map(f32::to_bits),
                },
                Event::AddBody {
                    inertia: 0x3333_3333,
                    state: 1,
                    transform: part_attachment_transform(initial1, Some(&inverse1))
                        .map(f32::to_bits),
                },
            ]
        );
        session.detach(&mut backend).unwrap();
    }

    #[test]
    fn absent_inverse_copyback_preserves_pose_bits() {
        let initial = marked_transform(1.0);
        let template = runtime_template(vec![part(0, None, 0)], Vec::new(), Vec::new());
        let mut session = DynamicAssemblySession::try_new(template, vec![initial]).unwrap();
        let mut pose = marked_transform(5.0);
        pose[3] = f32::from_bits(0x8000_0000);
        pose[7] = f32::from_bits(0x7fc0_1234);
        let mut backend = MockBackend::default();
        backend.body_poses.insert(100, pose);
        session.attach(&mut backend, 4).unwrap();
        session.detach(&mut backend).unwrap();
        assert_eq!(
            session
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .next()
                .unwrap()
                .map(f32::to_bits),
            pose.map(f32::to_bits)
        );
    }

    #[test]
    fn body_only_part_session_keeps_lifecycle_copyback_and_order_opaque() {
        let inverse0 = marked_transform(2.0);
        let inverse1 = marked_transform(4.0);
        let template = runtime_template(
            vec![part(0, Some(inverse0), 0), part(1, Some(inverse1), 0)],
            Vec::new(),
            Vec::new(),
        );
        let initial = [marked_transform(6.0), marked_transform(8.0)];
        let mut session = DynamicAssemblyBodyOnlyPartSession::<MockBackend>::try_new(
            Arc::clone(&template.part_attachment),
            initial.to_vec(),
        )
        .unwrap();
        let poses = [marked_transform(10.0), marked_transform(12.0)];
        let mut backend = MockBackend {
            authority: 7,
            ..MockBackend::default()
        };
        backend.body_poses.insert(100, poses[0]);
        backend.body_poses.insert(101, poses[1]);

        session.attach(&mut backend, 4).unwrap();
        assert!(session.is_attached());
        let attached_events = backend.events().len();
        assert_eq!(
            session.attach(&mut backend, 4),
            Err(DynamicAssemblyBodyOnlyPartAttachError::AlreadyAttached)
        );
        assert_eq!(backend.events().len(), attached_events);
        session.detach(&mut backend).unwrap();
        assert!(!session.is_attached());
        assert_eq!(
            &backend.events()[attached_events..],
            &[
                Event::BodyPose(100),
                Event::RemoveBody(100),
                Event::BodyPose(101),
                Event::RemoveBody(101),
            ]
        );
        let transforms = session
            .part_transforms()
            .copied()
            .map(|transform| transform.map(f32::to_bits))
            .collect::<Vec<_>>();
        assert_eq!(
            transforms,
            [
                compose_collision_affine(poses[0], inverse0).map(f32::to_bits),
                compose_collision_affine(poses[1], inverse1).map(f32::to_bits),
            ]
        );
        assert_eq!(
            session.detach(&mut backend),
            Err(DynamicAssemblyBodyOnlyPartDetachError::NotAttached)
        );
    }

    #[test]
    fn detached_and_attached_host_preconditions_have_no_backend_effects() {
        let mut session = three_part_session();
        let mut backend = MockBackend::default();
        assert_eq!(
            session.detach(&mut backend),
            Err(DynamicAssemblyDetachError::NotAttached)
        );
        assert!(backend.events().is_empty());
        session.attach(&mut backend, 0).unwrap();
        let event_count = backend.events().len();
        assert_eq!(
            session.attach(&mut backend, 0),
            Err(DynamicAssemblyAttachError::AlreadyAttached)
        );
        assert_eq!(backend.events().len(), event_count);
        session.detach(&mut backend).unwrap();
    }

    #[test]
    fn foreign_backend_detach_is_rejected_before_every_effect() {
        let mut session = three_part_session();
        let mut owner = MockBackend {
            authority: 7,
            ..MockBackend::default()
        };
        session.attach(&mut owner, 0).unwrap();
        assert_eq!(session.attached_authority, Some(7));
        let owner_events = owner.events();

        let mut foreign = MockBackend {
            authority: 8,
            ..MockBackend::default()
        };
        assert_eq!(
            session.detach(&mut foreign),
            Err(DynamicAssemblyDetachError::BackendAuthorityMismatch)
        );
        assert!(foreign.events().is_empty());
        assert_eq!(owner.events(), owner_events);
        assert_eq!(session.attached_authority, Some(7));
        assert_eq!(owner.live_bodies.len(), 3);
        assert_eq!(owner.live_joints.len(), 2);
        assert_eq!(owner.live_drives.len(), 2);

        session.detach(&mut owner).unwrap();
        assert_eq!(session.attached_authority, None);
        assert!(owner.live_bodies.is_empty());
        assert!(owner.live_joints.is_empty());
        assert!(owner.live_drives.is_empty());
    }

    #[test]
    fn same_authority_can_detach_reattach_and_detach_in_native_order() {
        let mut session = three_part_session();
        let mut backend = MockBackend {
            authority: 17,
            ..MockBackend::default()
        };
        session.attach(&mut backend, 0).unwrap();
        session.detach(&mut backend).unwrap();
        assert_no_runtime_handles(&session);
        assert!(backend.live_bodies.is_empty());
        assert!(backend.live_joints.is_empty());
        assert!(backend.live_drives.is_empty());

        let second_cycle_start = backend.events().len();
        session.attach(&mut backend, 0).unwrap();
        session.detach(&mut backend).unwrap();
        assert_eq!(
            &backend.events()[second_cycle_start..],
            &[
                Event::AddBody {
                    inertia: 10,
                    state: 0,
                    transform: marked_transform(1.0).map(f32::to_bits)
                },
                Event::AddBody {
                    inertia: 11,
                    state: 0,
                    transform: marked_transform(2.0).map(f32::to_bits)
                },
                Event::AddBody {
                    inertia: 12,
                    state: 0,
                    transform: marked_transform(3.0).map(f32::to_bits)
                },
                Event::AddJoint {
                    parent: 103,
                    child: 104,
                    frames: 30,
                    limits: 40
                },
                Event::AddJoint {
                    parent: 104,
                    child: 105,
                    frames: 31,
                    limits: 41
                },
                Event::AddDrive {
                    parent: 103,
                    child: 104,
                    frames: 50,
                    dynamics: 60
                },
                Event::AddDrive {
                    parent: 104,
                    child: 105,
                    frames: 51,
                    dynamics: 61
                },
                Event::BodyPose(103),
                Event::RemoveBody(103),
                Event::BodyPose(104),
                Event::RemoveBody(104),
                Event::BodyPose(105),
                Event::RemoveBody(105),
                Event::RemoveJoint(202),
                Event::RemoveJoint(203),
                Event::RemoveDrive(302),
                Event::RemoveDrive(303),
            ]
        );
        assert!(!session.is_attached());
        assert_eq!(session.attached_authority, None);
        assert_no_runtime_handles(&session);
        assert!(backend.live_bodies.is_empty());
        assert!(backend.live_joints.is_empty());
        assert!(backend.live_drives.is_empty());
    }

    #[test]
    fn zero_component_assembly_attaches_and_detaches_without_backend_calls() {
        let template = runtime_template(Vec::new(), Vec::new(), Vec::new());
        let mut session = DynamicAssemblySession::try_new(template, Vec::new()).unwrap();
        let mut backend = MockBackend::default();
        session.attach(&mut backend, 4).unwrap();
        assert!(session.is_attached());
        assert_eq!(session.attached_authority, Some(0));
        session.detach(&mut backend).unwrap();
        assert!(!session.is_attached());
        assert_eq!(session.attached_authority, None);
        assert!(backend.events().is_empty());
    }

    #[test]
    fn attachment_template_rejects_metadata_payload_shape_mismatch() {
        assert_eq!(
            DynamicAssemblyPartAttachmentTemplate::try_from_parts(
                Vec::new(),
                vec![DynamicAssemblyPartPayload {
                    inertia: 1_u32,
                    material: 2_u32,
                }],
            )
            .unwrap_err(),
            DynamicAssemblyPartAttachmentTemplateError::MetadataPayloadCount {
                metadata: 0,
                payloads: 1,
            }
        );
        assert_eq!(
            DynamicAssemblyPartAttachmentTemplate::<u32, u32>::try_from_parts(
                vec![DynamicAssemblyPartAttachmentMetadata {
                    inverse_body_transform: marked_transform(1.0),
                    has_inverse_body_transform: 0,
                    is_static_body: 0,
                }],
                Vec::new(),
            )
            .unwrap_err(),
            DynamicAssemblyPartAttachmentTemplateError::MetadataPayloadCount {
                metadata: 1,
                payloads: 0,
            }
        );
    }

    #[test]
    fn template_and_session_prevalidation_reject_mismatches_and_bad_indices() {
        let collision = collision_template(vec![part(0, None, 0)]);
        assert_eq!(
            DynamicAssemblyRuntimeTemplate::<u32, u32, u32, u32, u32, u32>::try_new(
                Arc::clone(&collision),
                Vec::new(),
                Vec::new(),
                Vec::new()
            )
            .unwrap_err(),
            DynamicAssemblyTemplateError::PartPayloadCount {
                collision_parts: 1,
                payloads: 0
            }
        );
        let payload = vec![DynamicAssemblyPartPayload {
            inertia: 1,
            material: 2,
        }];
        assert_eq!(
            DynamicAssemblyRuntimeTemplate::try_new(
                Arc::clone(&collision),
                payload.clone(),
                vec![DynamicAssemblyJointDefinition {
                    parent_part: 1,
                    child_part: 0,
                    frames: 3,
                    limits: 4
                }],
                Vec::<DynamicAssemblyDriveDefinition<u32, u32>>::new(),
            )
            .unwrap_err(),
            DynamicAssemblyTemplateError::PartIndex {
                kind: DynamicAssemblyPairKind::Joint,
                ordinal: 0,
                endpoint: DynamicAssemblyPairEndpoint::Parent,
                part_index: 1,
                part_count: 1,
            }
        );
        assert_eq!(
            DynamicAssemblyRuntimeTemplate::try_new(
                Arc::clone(&collision),
                payload.clone(),
                vec![DynamicAssemblyJointDefinition {
                    parent_part: 0,
                    child_part: -1,
                    frames: 3,
                    limits: 4
                }],
                Vec::<DynamicAssemblyDriveDefinition<u32, u32>>::new(),
            )
            .unwrap_err(),
            DynamicAssemblyTemplateError::PartIndex {
                kind: DynamicAssemblyPairKind::Joint,
                ordinal: 0,
                endpoint: DynamicAssemblyPairEndpoint::Child,
                part_index: -1,
                part_count: 1,
            }
        );
        assert_eq!(
            DynamicAssemblyRuntimeTemplate::try_new(
                collision,
                payload,
                Vec::<DynamicAssemblyJointDefinition<u32, u32>>::new(),
                vec![DynamicAssemblyDriveDefinition {
                    parent_part: 0,
                    child_part: 1,
                    frames: 3,
                    dynamics: 4
                }],
            )
            .unwrap_err(),
            DynamicAssemblyTemplateError::PartIndex {
                kind: DynamicAssemblyPairKind::Drive,
                ordinal: 0,
                endpoint: DynamicAssemblyPairEndpoint::Child,
                part_index: 1,
                part_count: 1,
            }
        );

        let template = runtime_template(vec![part(0, None, 0)], Vec::new(), Vec::new());
        assert_eq!(
            DynamicAssemblySession::<MockBackend>::try_new(template, Vec::new()).err(),
            Some(DynamicAssemblySessionCreateError::PartTransformCount {
                parts: 1,
                transforms: 0
            })
        );
    }

    #[test]
    fn shared_template_sessions_keep_mutable_state_isolated() {
        let template = runtime_template(vec![part(0, None, 0)], Vec::new(), Vec::new());
        let mut first =
            DynamicAssemblySession::try_new(Arc::clone(&template), vec![marked_transform(1.0)])
                .unwrap();
        let second = DynamicAssemblySession::<MockBackend>::try_new(
            Arc::clone(&template),
            vec![marked_transform(9.0)],
        )
        .unwrap();
        assert!(Arc::ptr_eq(first.template(), second.template()));
        let mut backend = MockBackend::default();
        first.attach(&mut backend, 0).unwrap();
        assert!(first.is_attached());
        assert!(!second.is_attached());
        assert_eq!(
            second
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .next()
                .unwrap()
                .map(f32::to_bits),
            marked_transform(9.0).map(f32::to_bits)
        );
        first.detach(&mut backend).unwrap();
    }
}
