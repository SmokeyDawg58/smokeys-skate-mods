//! Prepared skeleton mass and inertia orchestration from the debug reference.
//!
//! This module deliberately stops at the boundaries which are not yet safe to
//! express as host arithmetic. In particular, hat-volume construction, the
//! VMX shape-integral calculations, `rw::physics::ComputeMassProperties`, and
//! the reference single-precision divisions are supplied by collaborators.

use crate::ragdoll::{SKELETON_PART_COUNT, SkeletonPartIndex};

/// The positive root-part mass used by `SkeletonBody::Init`.
///
/// Reference: raw float `0x3a83126f` loaded at `0x82b61920` and selected for
/// the root path at `0x82b61d50`.
pub const ROOT_PART_MASS: f32 = f32::from_bits(0x3a83_126f);

/// The density passed to both native mass-property calls for every part.
///
/// Reference: raw float `0x3f800000` loaded at `0x82b61918`, then passed at
/// both `0x82b61ea8` and `0x82b61f00`.
pub const MASS_PROPERTIES_DENSITY: f32 = f32::from_bits(0x3f80_0000);

/// The fixed numerator used for each animated inverse-mass cache entry.
///
/// Reference: raw float `0x3f800000` used by the animated pass beginning at
/// `0x82b61ef0`.
pub const ANIMATED_INVERSE_MASS_NUMERATOR: f32 = f32::from_bits(0x3f80_0000);

/// `rw::collision::VOLUMEFLAG_ISENABLED`, cleared on every root capsule.
///
/// Reference: `lwz`/`clrrwi`/`stw` at `0x82b61de4..0x82b61dec`.
pub const VOLUMEFLAG_ISENABLED: u32 = 1;

/// Canonical high-word-first key for the capsule-only scale sampled before
/// the native part loop.
///
/// The supplied debug-reference data resolves this key to raw `0x3f99999a`.
/// That observation is not a compiled default: callers pass one already-
/// sampled value, which is forwarded unchanged throughout the part loop.
/// Reference: key construction at `0x82b6178c..0x82b6179c`, lookup at
/// `0x82b617a4`, default handling at `0x82b617b0..0x82b617b4`, load at
/// `0x82b617bc`, and the capsule-path use at `0x82b61aa0`.
pub const CAPSULE_DIMENSION_SCALE_KEY: u64 = 0x0720_4f10_dd93_7907;

/// Canonical high-word-first VLT key for the root capsule half-length.
pub const ROOT_CAPSULE_HALF_LENGTH_KEY: u64 = 0x5e43_a9eb_d2be_bbde;

/// Canonical high-word-first VLT key for the root capsule radius.
pub const ROOT_CAPSULE_RADIUS_KEY: u64 = 0x7e90_113b_05ba_cda6;

/// Bit-preserving form of `Andale::PhysicsParamBoneData::mSize` at `+0x20`.
///
/// The PDB type is a 16-byte `rw::math::vpu::Vector3`. `SkeletonBody::Init`
/// loads all four lanes at `0x82b61a74`, `0x82b61bc0`, or `0x82b61c90`.
/// Lane four is therefore retained without assigning it a geometric meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicsBoneSizeBits {
    pub lanes: [u32; 4],
}

impl PhysicsBoneSizeBits {
    #[must_use]
    pub const fn from_float_lanes(lanes: [f32; 4]) -> Self {
        Self {
            lanes: [
                lanes[0].to_bits(),
                lanes[1].to_bits(),
                lanes[2].to_bits(),
                lanes[3].to_bits(),
            ],
        }
    }

    #[must_use]
    pub const fn float_lane(self, lane: usize) -> f32 {
        f32::from_bits(self.lanes[lane])
    }
}

/// Per-part authored and Andale inputs used by the recovered dynamics slice.
///
/// `mass_factor` and `ragdoll_mass_factor` are the PDB fields at
/// `SkeletonBoneData +0x00/+0x04`. `physics_bone_id` comes from
/// `SkeletonData +0x1270`; the native root test compares each value with the
/// value at index zero rather than comparing the part index itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonBoneDynamicsInput {
    pub physics_bone_id: i32,
    pub physics_bone_size: PhysicsBoneSizeBits,
    pub mass_factor: f32,
    pub ragdoll_mass_factor: f32,
    pub volume_type_raw: u32,
    pub volume_scalar: f32,
}

/// One pass's cached inverse mass and four inverse-inertia lanes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassAndInertiaCache {
    pub inverse_mass: f32,
    pub inverse_inertia: [f32; 4],
}

/// Animated and ragdoll caches produced for one skeleton part.
///
/// The fields correspond to `SkeletonBody::mInverseMassAnimated` (`+0x7a4`),
/// `mInverseMassRagdoll` (`+0x804`), `mInverseInertiaAnimated` (`+0x870`),
/// and `mInverseInertiaRagdoll` (`+0x9f0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonBoneDynamics {
    pub animated: MassAndInertiaCache,
    pub ragdoll: MassAndInertiaCache,
}

/// The three valid authored `SkeletonBoneData::VolumeType` values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SkeletonVolumeShape {
    Box = 0,
    Sphere = 1,
    Capsule = 2,
}

impl SkeletonVolumeShape {
    #[must_use]
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Box),
            1 => Some(Self::Sphere),
            2 => Some(Self::Capsule),
            _ => None,
        }
    }
}

/// Input forwarded to the unrecovered VMX general-shape preparation boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneralShapeRequest {
    pub part: SkeletonPartIndex,
    pub shape: SkeletonVolumeShape,
    pub physics_bone_size: PhysicsBoneSizeBits,
    pub volume_scalar: f32,
    /// Key [`CAPSULE_DIMENSION_SCALE_KEY`], sampled once before the part loop.
    /// Native code consumes it only for `Capsule`; retaining it on every
    /// request keeps that pre-loop dependency explicit without resampling it.
    pub capsule_dimension_scale_snapshot: f32,
}

/// Caller-defined identity for one prepared native collision volume.
///
/// This is deliberately not represented as a host pointer. Its only contract
/// is that shape preparation, root flag mutation, and part installation refer
/// to the same stable volume.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedVolumeIdentity(pub u64);

/// Caller-defined identity for one stable native `PartDefinition`.
///
/// This remains distinct from [`PreparedVolumeIdentity`]. The mass-property
/// and debug-helper calls receive this installed part definition, whose
/// `m_volume` points at the prepared volume.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedPartDefinitionIdentity(pub u64);

/// Caller-defined identity for the inner two-child `SimpleMappedArray` owned
/// by the hat helper.
///
/// This is not the [`PreparedVolumeIdentity`] returned from the helper. Native
/// stores this inner identity in `SkeletonBody::mHatAggregate` at `+0x98`
/// (`0x82b4fbcc`), then later creates and returns a distinct outer
/// `AggregateVolume` wrapper at `0x82b4ff3c..0x82b4ffb4`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedHatInnerIdentity(pub u64);

/// One observed result of either independent live hat-collision lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HatCollisionState {
    Disabled,
    Enabled,
}

/// The fixed child kind installed at each inner hat-array index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HatChildShape {
    Cylinder,
    Sphere,
}

/// Observable facts produced inside one successful `CreateHatVolume` call.
///
/// This records identity, flags, child order, and mass values without exposing
/// or reconstructing the opaque geometry/VMX calculations. The two collision
/// samples are intentionally separate and may differ.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HatPreparationObservation {
    /// Inner two-child `SimpleMappedArray` stored at `mHatAggregate +0x98` by
    /// `0x82b4fbcc`.
    pub inner_mapped_array: PreparedHatInnerIdentity,
    /// First `kEnableHatCollision` sample, stored in
    /// `mLastEnableHatCollision +0x9d` at `0x82b4fb28..0x82b4fb34`.
    pub first_enable_collision_sample: HatCollisionState,
    /// Independent second sample at `0x82b4fd54..0x82b4fd70`.
    pub cylinder_enable_collision_sample: HatCollisionState,
    /// Cylinder bit-0 result written from the second sample.
    pub cylinder_enabled_flag: HatCollisionState,
    /// Sphere bit 0, unconditionally set at `0x82b4fefc..0x82b4ff00`.
    pub sphere_enabled_flag: HatCollisionState,
    /// Exactly cylinder at index 0, then sphere at index 1.
    pub child_order: [HatChildShape; 2],
    /// Unscaled out mass produced at `0x82b4fdd4`.
    pub unscaled_out_mass_bits: u32,
    /// Raw `SkeletonState::mPartMass` write at `0x82b4fe0c`.
    pub part_mass_write_bits: u32,
    /// Live internal hash-`0x9f4a03c75637e7e9` scale sample.
    pub mass_scale_bits: u32,
    /// Scaled final out mass produced at `0x82b4fe48..0x82b4fe50`.
    pub final_out_mass_bits: u32,
}

/// Root capsule operands after their exact live lookup order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootCapsuleRequest {
    pub part: SkeletonPartIndex,
    pub radius: f32,
    pub half_length: f32,
}

/// A volume prepared far enough to install into one native part definition.
///
/// General-shape integrals are the pre-`SkeletonMass` values held in `f31` at
/// `0x82b61cdc`. Their VMX derivation is not implemented here. The root mass
/// is the executable constant, while the hat mass is the out-parameter of
/// `SkeletonBody::CreateHatVolume` at `0x82b619a4..0x82b619bc`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreparedShape {
    HatAggregate {
        /// The returned outer `AggregateVolume` wrapper, never the inner
        /// `SimpleMappedArray` stored in `mHatAggregate`.
        volume: PreparedVolumeIdentity,
        effective_mass: f32,
        observation: HatPreparationObservation,
    },
    RootCapsule {
        volume: PreparedVolumeIdentity,
        radius: f32,
        half_length: f32,
    },
    Box {
        volume: PreparedVolumeIdentity,
        half_extents: [f32; 3],
        unscaled_mass_integral: f32,
    },
    Sphere {
        volume: PreparedVolumeIdentity,
        radius: f32,
        unscaled_mass_integral: f32,
    },
    Capsule {
        volume: PreparedVolumeIdentity,
        radius: f32,
        half_length: f32,
        unscaled_mass_integral: f32,
    },
}

/// Discriminant used to validate collaborator results without comparing
/// floating-point shape payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreparedShapeClass {
    HatAggregate,
    RootCapsule,
    Box,
    Sphere,
    Capsule,
}

impl PreparedShape {
    #[must_use]
    pub const fn volume_identity(self) -> PreparedVolumeIdentity {
        match self {
            Self::HatAggregate { volume, .. }
            | Self::RootCapsule { volume, .. }
            | Self::Box { volume, .. }
            | Self::Sphere { volume, .. }
            | Self::Capsule { volume, .. } => volume,
        }
    }

    #[must_use]
    pub const fn class(self) -> PreparedShapeClass {
        match self {
            Self::HatAggregate { .. } => PreparedShapeClass::HatAggregate,
            Self::RootCapsule { .. } => PreparedShapeClass::RootCapsule,
            Self::Box { .. } => PreparedShapeClass::Box,
            Self::Sphere { .. } => PreparedShapeClass::Sphere,
            Self::Capsule { .. } => PreparedShapeClass::Capsule,
        }
    }

    const fn general_kind(self) -> Option<SkeletonVolumeShape> {
        match self {
            Self::Box { .. } => Some(SkeletonVolumeShape::Box),
            Self::Sphere { .. } => Some(SkeletonVolumeShape::Sphere),
            Self::Capsule { .. } => Some(SkeletonVolumeShape::Capsule),
            Self::HatAggregate { .. } | Self::RootCapsule { .. } => None,
        }
    }

    const fn unscaled_mass_integral(self) -> Option<f32> {
        match self {
            Self::Box {
                unscaled_mass_integral,
                ..
            }
            | Self::Sphere {
                unscaled_mass_integral,
                ..
            }
            | Self::Capsule {
                unscaled_mass_integral,
                ..
            } => Some(unscaled_mass_integral),
            Self::HatAggregate { .. } | Self::RootCapsule { .. } => None,
        }
    }
}

/// A live debug-inertia selection from the per-part post-processing lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugInertiaAdjustment {
    Mode1,
    Mode2,
}

/// Identifies which of the two native mass-property passes is running.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MassPropertyPass {
    Ragdoll,
    Animated,
}

/// Input for either boolean-result `rw::physics::ComputeMassProperties` call.
///
/// Reference: calls at `0x82b61ea8` and `0x82b61f00`. Both receive the same
/// installed part definition, a pass-specific mass, exact density `1.0`, and
/// a null optional output pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartMassRequest {
    pub part: SkeletonPartIndex,
    pub pass: MassPropertyPass,
    pub part_definition: PreparedPartDefinitionIdentity,
    pub mass: f32,
    pub density: f32,
}

/// Live authored controls read by `SkeletonBody::Init`.
///
/// The methods preserve conditional timing: root dimensions are read only on
/// the root path; the general mass scale is read only after a general shape is
/// prepared; and the debug multiplier is read only for raw modes 1 or 2.
pub trait RagdollDynamicsControlSource {
    /// Hash `0x3689013d7adbdd2a`, read once at `0x82b618a4..0x82b618d8`.
    fn read_ragdoll_inverse_mass_numerator(&mut self) -> f32;

    /// Live hat type, read only at part 1 after part 0 has completed.
    fn read_hat_type_nonzero(&mut self) -> bool;

    /// Root capsule half-length key `0x5e43a9ebd2bebbde`, read first.
    fn read_root_capsule_half_length(&mut self) -> f32;

    /// Root capsule radius key `0x7e90113b05bacda6`, read second.
    fn read_root_capsule_radius(&mut self) -> f32;

    /// Hash `0x9f4a03c75637e7e9`, read once for each general part.
    fn read_general_mass_scale(&mut self, part: SkeletonPartIndex) -> f32;

    /// Raw mode hash `0xbe03a6ea6ddd9384`, read after the animated pass.
    fn read_debug_inertia_mode(&mut self, part: SkeletonPartIndex) -> u32;

    /// Multiplier hash `0x54441f41425c5017`, conditionally read for modes 1/2.
    fn read_debug_inertia_multiplier(&mut self, part: SkeletonPartIndex) -> f32;
}

/// Stateful volume and part-definition operations used by the orchestration.
///
/// `divide_inverse_mass` is intentionally explicit: substituting host `/`
/// would silently claim PPC `fdivs` parity for exceptional values and
/// rounding. The backend owns stable native-equivalent volume and
/// `PartDefinition` state; identities make their distinct lifetimes explicit
/// without exposing host pointers.
pub trait MassPropertiesBackend {
    /// Runs the complete `CreateHatVolume` boundary at
    /// `0x82b4fae8..0x82b4ffb4`.
    ///
    /// The boundary sets `mHatPresent` before allocation; samples
    /// `kEnableHatCollision` once for `mLastEnableHatCollision` and later,
    /// independently, for the cylinder's enabled bit; creates an inner
    /// two-child mapped array with cylinder then sphere; sets the sphere's
    /// enabled bit; updates the aggregate after both children; writes the
    /// unscaled mass to `SkeletonState::mPartMass`; samples and applies the
    /// internal mass-scale key; and creates the returned outer aggregate
    /// wrapper. Geometry and VMX arithmetic remain opaque.
    ///
    /// A successful result must be [`PreparedShape::HatAggregate`]. Its
    /// `volume` is the outer wrapper, while its observation carries a distinct
    /// inner mapped-array identity. Returning any other variant is an
    /// impossible successful-helper combination and is rejected by the host
    /// contract guard after the helper's path-dependent effects have occurred.
    fn prepare_hat_shape(&mut self, part: SkeletonPartIndex) -> PreparedShape;

    fn prepare_root_capsule(&mut self, request: RootCapsuleRequest) -> PreparedShape;

    fn prepare_general_shape(&mut self, request: GeneralShapeRequest) -> PreparedShape;

    /// Clears only [`VOLUMEFLAG_ISENABLED`] (`m_flags & !1`) after native root
    /// capsule construction at `0x82b61de4..0x82b61dec`.
    fn disable_root_capsule_volume(
        &mut self,
        part: SkeletonPartIndex,
        volume: PreparedVolumeIdentity,
    );

    /// Performs the common writes at `0x82b61e30..0x82b61e54` and returns the
    /// stable `PartDefinition` identity used by all later calls for this part.
    ///
    /// The volume is stored in `SkeletonBody::mVolumes[part]` and
    /// `PartDefinition::m_volume`. Native `mMaterialNormal.m_mud` is written
    /// to both `m_mus` and `m_mud`, then `mMaterialNormal.m_res` is copied.
    fn install_volume_for_part(
        &mut self,
        part: SkeletonPartIndex,
        shape: PreparedShape,
    ) -> PreparedPartDefinitionIdentity;

    fn divide_inverse_mass(
        &mut self,
        part: SkeletonPartIndex,
        pass: MassPropertyPass,
        numerator: f32,
        denominator: f32,
    ) -> f32;

    /// Mutates the installed part definition and returns its current inverse
    /// tensor on native boolean success. `None` preserves all backend writes
    /// made by the native false-result path.
    fn compute_mass_properties(&mut self, request: PartMassRequest) -> Option<[f32; 4]>;

    /// Late read of `SkeletonBody::mHatPresent` at `0x82b61f28`.
    fn hat_is_present_after_shape_preparation(&mut self) -> bool;

    /// Sets `m_hasInverseBodyLTM = 1` before writing the four identity rows at
    /// `0x82b61f4c..0x82b61f5c`.
    fn install_hat_inverse_body_identity(
        &mut self,
        part: SkeletonPartIndex,
        part_definition: PreparedPartDefinitionIdentity,
    );

    /// Dispatches the distinct mode-1/mode-2 helpers at `0x82e4b0e0` and
    /// `0x82e4b188`, returning the post-helper tensor copied into the animated
    /// cache. Helper-internal stores are not represented here.
    fn apply_debug_inertia_adjustment(
        &mut self,
        part: SkeletonPartIndex,
        part_definition: PreparedPartDefinitionIdentity,
        adjustment: DebugInertiaAdjustment,
        multiplier: f32,
        current_inverse_inertia: [f32; 4],
    ) -> [f32; 4];
}

/// Observable `SkeletonState` and `SkeletonBody` writes made during setup.
///
/// Writes are intentionally emitted at their native points instead of being
/// committed transactionally at successful return. Callers must discard this
/// partially mutated state after an error and must not retry with the same
/// output object.
pub trait SkeletonDynamicsOutput {
    /// Stores the matching root part index at the root branch beginning
    /// `0x82b61d44`, before its dimension lookups.
    fn write_root_part_index(&mut self, part: SkeletonPartIndex);

    /// Writes `SkeletonState::mPartMass[part]`: the general integral before
    /// `0x82b61cf8`'s scale lookup, or root positive zero after construction.
    fn write_part_unscaled_mass(&mut self, part: SkeletonPartIndex, mass: f32);

    /// Ragdoll and animated cache writes at `0x82b61ea0` and `0x82b61ef8`.
    fn write_inverse_mass(
        &mut self,
        part: SkeletonPartIndex,
        pass: MassPropertyPass,
        inverse_mass: f32,
    );

    /// Ragdoll and animated four-lane writes at
    /// `0x82b61ee4..0x82b61eec` and `0x82b62068..0x82b62074`.
    fn write_inverse_inertia(
        &mut self,
        part: SkeletonPartIndex,
        pass: MassPropertyPass,
        inverse_inertia: [f32; 4],
    );
}

/// Safe typed abort reasons for invalid inputs or native call failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RagdollDynamicsError {
    /// Safe pre-loop abort before any per-part collaborator or output call.
    NonPositiveInverseMassNumerator {
        raw: u32,
    },
    UnsupportedVolumeType {
        part: SkeletonPartIndex,
        raw: u32,
    },
    /// Host-only collaborator-contract violation with no native counterpart.
    PreparedShapeMismatch {
        part: SkeletonPartIndex,
        expected: PreparedShapeClass,
        actual: PreparedShapeClass,
    },
    /// Safe post-install abort; volume, part-definition, and material writes
    /// for this part have already occurred.
    NonPositiveEffectiveMass {
        part: SkeletonPartIndex,
        raw: u32,
    },
    /// Safe post-call abort. Writes performed inside the false-returning
    /// native-equivalent backend call, plus all earlier writes, remain live.
    MassPropertiesFailed {
        part: SkeletonPartIndex,
        pass: MassPropertyPass,
    },
}

/// Builds the four 24-entry inverse-mass/inertia caches in native part order.
///
/// Reference: `SkeletonBody::Init` at `0x82b6195c..0x82b6207c`. Part 1's
/// nonzero hat-type path is tested before the root-bone-ID comparison. Each
/// selected shape then receives the ragdoll pass, whose inertia is cached
/// immediately, followed by the animated pass, optional hat matrix write,
/// optional debug adjustment, and animated-inertia cache write.
/// `capsule_dimension_scale_snapshot` is the caller's one live sample for
/// [`CAPSULE_DIMENSION_SCALE_KEY`]. It is never looked up or resampled here,
/// and the same raw bits are forwarded on every general-shape request.
///
/// # Errors
///
/// A nonpositive numerator aborts before the part loop and before any shape or
/// output callback. A nonpositive effective mass aborts only after the common
/// volume/part-definition/material installation callback, matching the saved
/// native comparison's position. A false mass-property result aborts after the
/// backend has retained all writes made inside that native call. These safe
/// exits deliberately replace debug-assert continuation, are nontransactional,
/// and require the caller to discard rather than retry the collaborators.
///
/// [`RagdollDynamicsError::PreparedShapeMismatch`] is instead a host-only
/// collaborator-contract guard with no native counterpart. It does not undo
/// path-dependent preparation effects; notably, a mismatched hat result is
/// observed only after the complete hat callback has returned.
pub fn prepare_skeleton_dynamics(
    bones: &[SkeletonBoneDynamicsInput; SKELETON_PART_COUNT],
    capsule_dimension_scale_snapshot: f32,
    controls: &mut impl RagdollDynamicsControlSource,
    backend: &mut impl MassPropertiesBackend,
    output: &mut impl SkeletonDynamicsOutput,
) -> Result<[SkeletonBoneDynamics; SKELETON_PART_COUNT], RagdollDynamicsError> {
    let inverse_mass_numerator = controls.read_ragdoll_inverse_mass_numerator();
    if !is_ordered_positive(inverse_mass_numerator) {
        return Err(RagdollDynamicsError::NonPositiveInverseMassNumerator {
            raw: inverse_mass_numerator.to_bits(),
        });
    }

    let root_physics_bone_id = bones[SkeletonPartIndex::SkateboardRoot.index()].physics_bone_id;
    let empty_cache = MassAndInertiaCache {
        inverse_mass: 0.0,
        inverse_inertia: [0.0; 4],
    };
    let mut result = [SkeletonBoneDynamics {
        animated: empty_cache,
        ragdoll: empty_cache,
    }; SKELETON_PART_COUNT];

    for part in SkeletonPartIndex::ALL {
        let bone = bones[part.index()];
        let (shape, effective_mass) = prepare_part_shape(
            part,
            bone,
            root_physics_bone_id,
            capsule_dimension_scale_snapshot,
            controls,
            backend,
            output,
        )?;

        // Native sets CR6 first, performs every common install/material write,
        // and only then branches on the saved ordered comparison.
        let effective_mass_is_positive = is_ordered_positive(effective_mass);
        let part_definition = backend.install_volume_for_part(part, shape);
        if !effective_mass_is_positive {
            return Err(RagdollDynamicsError::NonPositiveEffectiveMass {
                part,
                raw: effective_mass.to_bits(),
            });
        }

        result[part.index()] = prepare_part_mass_properties(
            PartMassContext {
                part,
                bone,
                effective_mass,
                inverse_mass_numerator,
                part_definition,
            },
            controls,
            backend,
            output,
        )?;
    }

    Ok(result)
}

#[derive(Clone, Copy)]
struct PartMassContext {
    part: SkeletonPartIndex,
    bone: SkeletonBoneDynamicsInput,
    effective_mass: f32,
    inverse_mass_numerator: f32,
    part_definition: PreparedPartDefinitionIdentity,
}

fn prepare_part_mass_properties(
    context: PartMassContext,
    controls: &mut impl RagdollDynamicsControlSource,
    backend: &mut impl MassPropertiesBackend,
    output: &mut impl SkeletonDynamicsOutput,
) -> Result<SkeletonBoneDynamics, RagdollDynamicsError> {
    let PartMassContext {
        part,
        bone,
        effective_mass,
        inverse_mass_numerator,
        part_definition,
    } = context;
    let ragdoll_mass = bone.ragdoll_mass_factor * effective_mass;
    let inverse_mass_ragdoll = backend.divide_inverse_mass(
        part,
        MassPropertyPass::Ragdoll,
        inverse_mass_numerator,
        ragdoll_mass,
    );
    output.write_inverse_mass(part, MassPropertyPass::Ragdoll, inverse_mass_ragdoll);
    let ragdoll_request = PartMassRequest {
        part,
        pass: MassPropertyPass::Ragdoll,
        part_definition,
        mass: ragdoll_mass,
        density: MASS_PROPERTIES_DENSITY,
    };
    let Some(ragdoll_inverse_inertia) = backend.compute_mass_properties(ragdoll_request) else {
        return Err(RagdollDynamicsError::MassPropertiesFailed {
            part,
            pass: MassPropertyPass::Ragdoll,
        });
    };
    // Native copy: `0x82b61ee4..0x82b61eec`, before the second call.
    output.write_inverse_inertia(part, MassPropertyPass::Ragdoll, ragdoll_inverse_inertia);

    let animated_mass = bone.mass_factor * effective_mass;
    let inverse_mass_animated = backend.divide_inverse_mass(
        part,
        MassPropertyPass::Animated,
        ANIMATED_INVERSE_MASS_NUMERATOR,
        animated_mass,
    );
    output.write_inverse_mass(part, MassPropertyPass::Animated, inverse_mass_animated);
    let animated_request = PartMassRequest {
        part,
        pass: MassPropertyPass::Animated,
        part_definition,
        mass: animated_mass,
        density: MASS_PROPERTIES_DENSITY,
    };
    let Some(mut animated_inverse_inertia) = backend.compute_mass_properties(animated_request)
    else {
        return Err(RagdollDynamicsError::MassPropertiesFailed {
            part,
            pass: MassPropertyPass::Animated,
        });
    };

    if part == SkeletonPartIndex::Neck1 && backend.hat_is_present_after_shape_preparation() {
        backend.install_hat_inverse_body_identity(part, part_definition);
    }

    let adjustment = match controls.read_debug_inertia_mode(part) {
        1 => Some(DebugInertiaAdjustment::Mode1),
        2 => Some(DebugInertiaAdjustment::Mode2),
        _ => None,
    };
    if let Some(adjustment) = adjustment {
        let multiplier = controls.read_debug_inertia_multiplier(part);
        animated_inverse_inertia = backend.apply_debug_inertia_adjustment(
            part,
            part_definition,
            adjustment,
            multiplier,
            animated_inverse_inertia,
        );
    }
    // Native copy: `0x82b62068..0x82b62074`, after debug adjustment.
    output.write_inverse_inertia(part, MassPropertyPass::Animated, animated_inverse_inertia);
    Ok(SkeletonBoneDynamics {
        animated: MassAndInertiaCache {
            inverse_mass: inverse_mass_animated,
            inverse_inertia: animated_inverse_inertia,
        },
        ragdoll: MassAndInertiaCache {
            inverse_mass: inverse_mass_ragdoll,
            inverse_inertia: ragdoll_inverse_inertia,
        },
    })
}

fn prepare_part_shape(
    part: SkeletonPartIndex,
    bone: SkeletonBoneDynamicsInput,
    root_physics_bone_id: i32,
    capsule_dimension_scale_snapshot: f32,
    controls: &mut impl RagdollDynamicsControlSource,
    backend: &mut impl MassPropertiesBackend,
    output: &mut impl SkeletonDynamicsOutput,
) -> Result<(PreparedShape, f32), RagdollDynamicsError> {
    if part == SkeletonPartIndex::Neck1 && controls.read_hat_type_nonzero() {
        return prepare_hat_shape(part, backend);
    }

    if bone.physics_bone_id == root_physics_bone_id {
        output.write_root_part_index(part);
        // Native lookup order is half-length followed by radius; the
        // CreateCollisionCapsule call receives radius first.
        let half_length = controls.read_root_capsule_half_length();
        let radius = controls.read_root_capsule_radius();
        let shape = backend.prepare_root_capsule(RootCapsuleRequest {
            part,
            radius,
            half_length,
        });
        if shape.class() != PreparedShapeClass::RootCapsule {
            return Err(RagdollDynamicsError::PreparedShapeMismatch {
                part,
                expected: PreparedShapeClass::RootCapsule,
                actual: shape.class(),
            });
        }
        backend.disable_root_capsule_volume(part, shape.volume_identity());
        output.write_part_unscaled_mass(part, 0.0);
        return Ok((shape, ROOT_PART_MASS));
    }

    let Some(expected) = SkeletonVolumeShape::from_raw(bone.volume_type_raw) else {
        return Err(RagdollDynamicsError::UnsupportedVolumeType {
            part,
            raw: bone.volume_type_raw,
        });
    };
    let shape = backend.prepare_general_shape(GeneralShapeRequest {
        part,
        shape: expected,
        physics_bone_size: bone.physics_bone_size,
        volume_scalar: bone.volume_scalar,
        capsule_dimension_scale_snapshot,
    });
    if shape.general_kind() != Some(expected) {
        let expected = match expected {
            SkeletonVolumeShape::Box => PreparedShapeClass::Box,
            SkeletonVolumeShape::Sphere => PreparedShapeClass::Sphere,
            SkeletonVolumeShape::Capsule => PreparedShapeClass::Capsule,
        };
        return Err(RagdollDynamicsError::PreparedShapeMismatch {
            part,
            expected,
            actual: shape.class(),
        });
    }
    let unscaled_mass_integral = shape
        .unscaled_mass_integral()
        .expect("the checked general shape has an integral");
    output.write_part_unscaled_mass(part, unscaled_mass_integral);
    let general_mass_scale = controls.read_general_mass_scale(part);
    Ok((shape, unscaled_mass_integral * general_mass_scale))
}

fn prepare_hat_shape(
    part: SkeletonPartIndex,
    backend: &mut impl MassPropertiesBackend,
) -> Result<(PreparedShape, f32), RagdollDynamicsError> {
    let shape = backend.prepare_hat_shape(part);
    let PreparedShape::HatAggregate { effective_mass, .. } = shape else {
        return Err(RagdollDynamicsError::PreparedShapeMismatch {
            part,
            expected: PreparedShapeClass::HatAggregate,
            actual: shape.class(),
        });
    };
    Ok((shape, effective_mass))
}

fn is_ordered_positive(value: f32) -> bool {
    value.partial_cmp(&0.0) == Some(std::cmp::Ordering::Greater)
}

/// Selects the sphere extent used by the scalar `fsel` at `0x82b61c08..0x82b61c0c`.
///
/// Equal finite candidates retain `first`. An unordered subtraction selects
/// `second`, including same-signed infinity minus itself. The operands are
/// already-prepared scalar candidates; their VMX production remains a backend
/// responsibility.
#[must_use]
pub fn select_sphere_extent(first: f32, second: f32) -> f32 {
    if first - second >= -0.0 {
        first
    } else {
        second
    }
}

/// Applies the capsule nonnegative clamp selected by `fsel` at
/// `0x82b61ae0..0x82b61ae4`.
///
/// Both zero signs select the positive-zero constant. An unordered negation
/// selects and bit-preserves the original remainder.
#[must_use]
pub fn clamp_capsule_half_length_remainder(remainder: f32) -> f32 {
    if -remainder >= -0.0 { 0.0 } else { remainder }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Event {
        Numerator,
        HatType,
        RootIndex(SkeletonPartIndex),
        RootHalfLength,
        RootRadius,
        HatPreparationCompleted(SkeletonPartIndex, HatPreparationObservation),
        PrepareRoot(SkeletonPartIndex, PreparedVolumeIdentity, u32, u32),
        DisableRoot(SkeletonPartIndex, PreparedVolumeIdentity, u32, u32),
        PrepareGeneral(SkeletonPartIndex, SkeletonVolumeShape, [u32; 4], u32, u32),
        PartMass(SkeletonPartIndex, u32),
        GeneralMassScale(SkeletonPartIndex),
        InstallBodyVolume(SkeletonPartIndex, PreparedVolumeIdentity),
        InstallPartVolume(
            SkeletonPartIndex,
            PreparedPartDefinitionIdentity,
            PreparedVolumeIdentity,
        ),
        InstallStaticFriction(SkeletonPartIndex, PreparedPartDefinitionIdentity, u32),
        InstallDynamicFriction(SkeletonPartIndex, PreparedPartDefinitionIdentity, u32),
        InstallRestitution(SkeletonPartIndex, PreparedPartDefinitionIdentity, u32),
        Divide(SkeletonPartIndex, MassPropertyPass, u32, u32),
        InverseMass(SkeletonPartIndex, MassPropertyPass, u32),
        Compute(
            SkeletonPartIndex,
            MassPropertyPass,
            PreparedPartDefinitionIdentity,
            u32,
            u32,
        ),
        ComputeInternalWrite(
            SkeletonPartIndex,
            MassPropertyPass,
            PreparedPartDefinitionIdentity,
        ),
        InverseInertia(SkeletonPartIndex, MassPropertyPass, [u32; 4]),
        ReadHatPresent,
        HatInverseFlag(SkeletonPartIndex, PreparedPartDefinitionIdentity),
        HatInverseMatrix(SkeletonPartIndex, PreparedPartDefinitionIdentity),
        DebugMode(SkeletonPartIndex),
        DebugMultiplier(SkeletonPartIndex),
        DebugAdjust(
            SkeletonPartIndex,
            PreparedPartDefinitionIdentity,
            DebugInertiaAdjustment,
            u32,
        ),
    }

    struct RecordingControls {
        events: Rc<RefCell<Vec<Event>>>,
        numerator: f32,
        general_scale: f32,
        debug_selection: Option<(SkeletonPartIndex, u32)>,
        hat_type_live: Rc<Cell<bool>>,
    }

    impl RagdollDynamicsControlSource for RecordingControls {
        fn read_ragdoll_inverse_mass_numerator(&mut self) -> f32 {
            self.events.borrow_mut().push(Event::Numerator);
            self.numerator
        }

        fn read_hat_type_nonzero(&mut self) -> bool {
            self.events.borrow_mut().push(Event::HatType);
            self.hat_type_live.get()
        }

        fn read_root_capsule_half_length(&mut self) -> f32 {
            self.events.borrow_mut().push(Event::RootHalfLength);
            2.0
        }

        fn read_root_capsule_radius(&mut self) -> f32 {
            self.events.borrow_mut().push(Event::RootRadius);
            3.0
        }

        fn read_general_mass_scale(&mut self, part: SkeletonPartIndex) -> f32 {
            self.events.borrow_mut().push(Event::GeneralMassScale(part));
            self.general_scale
        }

        fn read_debug_inertia_mode(&mut self, part: SkeletonPartIndex) -> u32 {
            self.events.borrow_mut().push(Event::DebugMode(part));
            self.debug_selection
                .filter(|(selected_part, _)| *selected_part == part)
                .map_or(0, |(_, raw)| raw)
        }

        fn read_debug_inertia_multiplier(&mut self, part: SkeletonPartIndex) -> f32 {
            self.events.borrow_mut().push(Event::DebugMultiplier(part));
            4.0
        }
    }

    struct RecordingBackend {
        events: Rc<RefCell<Vec<Event>>>,
        hat_present: bool,
        root_flags: Rc<Cell<u32>>,
        general_integral_override: Option<f32>,
        hat_shape_override: Option<PreparedShape>,
        root_shape_override: Option<PreparedShape>,
        general_shape_override: Option<PreparedShape>,
        fail_at: Option<(SkeletonPartIndex, MassPropertyPass)>,
        material_mud_bits: u32,
        material_res_bits: u32,
        hat_observation: HatPreparationObservation,
    }

    impl MassPropertiesBackend for RecordingBackend {
        fn prepare_hat_shape(&mut self, part: SkeletonPartIndex) -> PreparedShape {
            self.events
                .borrow_mut()
                .push(Event::HatPreparationCompleted(part, self.hat_observation));
            self.hat_shape_override
                .unwrap_or(PreparedShape::HatAggregate {
                    volume: hat_volume_identity(),
                    effective_mass: 11.0,
                    observation: self.hat_observation,
                })
        }

        fn prepare_root_capsule(&mut self, request: RootCapsuleRequest) -> PreparedShape {
            let volume = root_volume_identity(request.part);
            self.events.borrow_mut().push(Event::PrepareRoot(
                request.part,
                volume,
                request.radius.to_bits(),
                request.half_length.to_bits(),
            ));
            self.root_shape_override
                .unwrap_or(PreparedShape::RootCapsule {
                    volume,
                    radius: request.radius,
                    half_length: request.half_length,
                })
        }

        fn prepare_general_shape(&mut self, request: GeneralShapeRequest) -> PreparedShape {
            self.events.borrow_mut().push(Event::PrepareGeneral(
                request.part,
                request.shape,
                request.physics_bone_size.lanes,
                request.volume_scalar.to_bits(),
                request.capsule_dimension_scale_snapshot.to_bits(),
            ));
            if let Some(shape) = self.general_shape_override {
                return shape;
            }
            let volume = general_volume_identity(request.part);
            let integral = self
                .general_integral_override
                .unwrap_or_else(|| part_marker(request.part) + 1.0);
            match request.shape {
                SkeletonVolumeShape::Box => PreparedShape::Box {
                    volume,
                    half_extents: [1.0, 2.0, 3.0],
                    unscaled_mass_integral: integral,
                },
                SkeletonVolumeShape::Sphere => PreparedShape::Sphere {
                    volume,
                    radius: 2.0,
                    unscaled_mass_integral: integral,
                },
                SkeletonVolumeShape::Capsule => PreparedShape::Capsule {
                    volume,
                    radius: 1.0,
                    half_length: 2.0,
                    unscaled_mass_integral: integral,
                },
            }
        }

        fn disable_root_capsule_volume(
            &mut self,
            part: SkeletonPartIndex,
            volume: PreparedVolumeIdentity,
        ) {
            let before = self.root_flags.get();
            let after = before & !VOLUMEFLAG_ISENABLED;
            self.root_flags.set(after);
            self.events
                .borrow_mut()
                .push(Event::DisableRoot(part, volume, before, after));
        }

        fn install_volume_for_part(
            &mut self,
            part: SkeletonPartIndex,
            shape: PreparedShape,
        ) -> PreparedPartDefinitionIdentity {
            let volume = shape.volume_identity();
            let part_definition = part_definition_identity(part);
            let mut events = self.events.borrow_mut();
            events.push(Event::InstallBodyVolume(part, volume));
            events.push(Event::InstallPartVolume(part, part_definition, volume));
            events.push(Event::InstallStaticFriction(
                part,
                part_definition,
                self.material_mud_bits,
            ));
            events.push(Event::InstallDynamicFriction(
                part,
                part_definition,
                self.material_mud_bits,
            ));
            events.push(Event::InstallRestitution(
                part,
                part_definition,
                self.material_res_bits,
            ));
            part_definition
        }

        fn divide_inverse_mass(
            &mut self,
            part: SkeletonPartIndex,
            pass: MassPropertyPass,
            numerator: f32,
            denominator: f32,
        ) -> f32 {
            self.events.borrow_mut().push(Event::Divide(
                part,
                pass,
                numerator.to_bits(),
                denominator.to_bits(),
            ));
            let pass_marker = match pass {
                MassPropertyPass::Ragdoll => 1000.0,
                MassPropertyPass::Animated => 2000.0,
            };
            pass_marker + part_marker(part)
        }

        fn compute_mass_properties(&mut self, request: PartMassRequest) -> Option<[f32; 4]> {
            let mut events = self.events.borrow_mut();
            events.push(Event::Compute(
                request.part,
                request.pass,
                request.part_definition,
                request.mass.to_bits(),
                request.density.to_bits(),
            ));
            events.push(Event::ComputeInternalWrite(
                request.part,
                request.pass,
                request.part_definition,
            ));
            drop(events);
            if self.fail_at == Some((request.part, request.pass)) {
                return None;
            }
            let base = match request.pass {
                MassPropertyPass::Ragdoll => 3000.0,
                MassPropertyPass::Animated => 4000.0,
            } + part_marker(request.part);
            Some([base, base + 0.25, base + 0.5, base + 0.75])
        }

        fn hat_is_present_after_shape_preparation(&mut self) -> bool {
            self.events.borrow_mut().push(Event::ReadHatPresent);
            self.hat_present
        }

        fn install_hat_inverse_body_identity(
            &mut self,
            part: SkeletonPartIndex,
            part_definition: PreparedPartDefinitionIdentity,
        ) {
            let mut events = self.events.borrow_mut();
            events.push(Event::HatInverseFlag(part, part_definition));
            events.push(Event::HatInverseMatrix(part, part_definition));
        }

        fn apply_debug_inertia_adjustment(
            &mut self,
            part: SkeletonPartIndex,
            part_definition: PreparedPartDefinitionIdentity,
            adjustment: DebugInertiaAdjustment,
            multiplier: f32,
            current_inverse_inertia: [f32; 4],
        ) -> [f32; 4] {
            self.events.borrow_mut().push(Event::DebugAdjust(
                part,
                part_definition,
                adjustment,
                multiplier.to_bits(),
            ));
            current_inverse_inertia.map(|lane| lane + 5000.0)
        }
    }

    struct RecordingOutput {
        events: Rc<RefCell<Vec<Event>>>,
        hat_type_live: Rc<Cell<bool>>,
        enable_hat_after_part_zero: bool,
    }

    impl SkeletonDynamicsOutput for RecordingOutput {
        fn write_root_part_index(&mut self, part: SkeletonPartIndex) {
            self.events.borrow_mut().push(Event::RootIndex(part));
        }

        fn write_part_unscaled_mass(&mut self, part: SkeletonPartIndex, mass: f32) {
            self.events
                .borrow_mut()
                .push(Event::PartMass(part, mass.to_bits()));
        }

        fn write_inverse_mass(
            &mut self,
            part: SkeletonPartIndex,
            pass: MassPropertyPass,
            inverse_mass: f32,
        ) {
            self.events
                .borrow_mut()
                .push(Event::InverseMass(part, pass, inverse_mass.to_bits()));
        }

        fn write_inverse_inertia(
            &mut self,
            part: SkeletonPartIndex,
            pass: MassPropertyPass,
            inverse_inertia: [f32; 4],
        ) {
            self.events.borrow_mut().push(Event::InverseInertia(
                part,
                pass,
                inverse_inertia.map(f32::to_bits),
            ));
            if self.enable_hat_after_part_zero
                && part == SkeletonPartIndex::SkateboardRoot
                && pass == MassPropertyPass::Animated
            {
                self.hat_type_live.set(true);
            }
        }
    }

    fn test_bones() -> [SkeletonBoneDynamicsInput; SKELETON_PART_COUNT] {
        std::array::from_fn(|index| {
            let index_u32 = u32::try_from(index).expect("part index fits u32");
            SkeletonBoneDynamicsInput {
                physics_bone_id: i32::try_from(index).expect("part index fits i32"),
                physics_bone_size: PhysicsBoneSizeBits {
                    lanes: [
                        0x3f80_0000 + index_u32,
                        0x4000_0000 + index_u32,
                        0x4040_0000 + index_u32,
                        0x7fa0_0000 + index_u32,
                    ],
                },
                mass_factor: 2.0,
                ragdoll_mass_factor: 3.0,
                volume_type_raw: SkeletonVolumeShape::Box as u32,
                volume_scalar: 0.5,
            }
        })
    }

    struct Harness {
        events: Rc<RefCell<Vec<Event>>>,
        controls: RecordingControls,
        backend: RecordingBackend,
        output: RecordingOutput,
        root_flags: Rc<Cell<u32>>,
    }

    fn harness(
        numerator: f32,
        hat_type: bool,
        hat_present: bool,
        debug_selection: Option<(SkeletonPartIndex, u32)>,
        root_flags: u32,
    ) -> Harness {
        let events = Rc::new(RefCell::new(Vec::new()));
        let hat_type_live = Rc::new(Cell::new(hat_type));
        let root_flags = Rc::new(Cell::new(root_flags));
        Harness {
            events: Rc::clone(&events),
            controls: RecordingControls {
                events: Rc::clone(&events),
                numerator,
                general_scale: 10.0,
                debug_selection,
                hat_type_live: Rc::clone(&hat_type_live),
            },
            backend: RecordingBackend {
                events: Rc::clone(&events),
                hat_present,
                root_flags: Rc::clone(&root_flags),
                general_integral_override: None,
                hat_shape_override: None,
                root_shape_override: None,
                general_shape_override: None,
                fail_at: None,
                material_mud_bits: 0x3e80_0000,
                material_res_bits: 0x3d80_0000,
                hat_observation: test_hat_observation(),
            },
            output: RecordingOutput {
                events,
                hat_type_live,
                enable_hat_after_part_zero: false,
            },
            root_flags,
        }
    }

    fn run(
        bones: &[SkeletonBoneDynamicsInput; SKELETON_PART_COUNT],
        capsule_scale_bits: u32,
        controls: &mut RecordingControls,
        backend: &mut RecordingBackend,
        output: &mut RecordingOutput,
    ) -> Result<[SkeletonBoneDynamics; SKELETON_PART_COUNT], RagdollDynamicsError> {
        prepare_skeleton_dynamics(
            bones,
            f32::from_bits(capsule_scale_bits),
            controls,
            backend,
            output,
        )
    }

    fn event_position(events: &[Event], expected: &Event) -> usize {
        events
            .iter()
            .position(|event| event == expected)
            .expect("expected event")
    }

    fn part_marker(part: SkeletonPartIndex) -> f32 {
        f32::from(u8::try_from(part.index()).expect("24-part index fits u8"))
    }

    fn general_volume_identity(part: SkeletonPartIndex) -> PreparedVolumeIdentity {
        PreparedVolumeIdentity(
            0x1000 + u64::try_from(part.index()).expect("24-part index fits u64"),
        )
    }

    fn root_volume_identity(part: SkeletonPartIndex) -> PreparedVolumeIdentity {
        PreparedVolumeIdentity(
            0x5200 + u64::try_from(part.index()).expect("24-part index fits u64"),
        )
    }

    const fn hat_volume_identity() -> PreparedVolumeIdentity {
        PreparedVolumeIdentity(0x4841_5401)
    }

    const fn hat_inner_identity() -> PreparedHatInnerIdentity {
        PreparedHatInnerIdentity(0x4841_5402)
    }

    const fn test_hat_observation() -> HatPreparationObservation {
        HatPreparationObservation {
            inner_mapped_array: hat_inner_identity(),
            first_enable_collision_sample: HatCollisionState::Disabled,
            cylinder_enable_collision_sample: HatCollisionState::Enabled,
            cylinder_enabled_flag: HatCollisionState::Enabled,
            sphere_enabled_flag: HatCollisionState::Enabled,
            child_order: [HatChildShape::Cylinder, HatChildShape::Sphere],
            unscaled_out_mass_bits: 11.0_f32.to_bits(),
            part_mass_write_bits: 11.0_f32.to_bits(),
            mass_scale_bits: 1.0_f32.to_bits(),
            final_out_mass_bits: 11.0_f32.to_bits(),
        }
    }

    fn assert_test_hat_observation(observation: HatPreparationObservation) {
        assert_ne!(hat_volume_identity().0, observation.inner_mapped_array.0);
        assert_eq!(
            observation.first_enable_collision_sample,
            HatCollisionState::Disabled
        );
        assert_eq!(
            observation.cylinder_enable_collision_sample,
            HatCollisionState::Enabled
        );
        assert_ne!(
            observation.first_enable_collision_sample,
            observation.cylinder_enable_collision_sample
        );
        assert_eq!(
            observation.cylinder_enabled_flag,
            observation.cylinder_enable_collision_sample
        );
        assert_eq!(observation.sphere_enabled_flag, HatCollisionState::Enabled);
        assert_eq!(
            observation.child_order,
            [HatChildShape::Cylinder, HatChildShape::Sphere]
        );
        assert_eq!(
            observation.unscaled_out_mass_bits,
            observation.part_mass_write_bits
        );
        assert_eq!(observation.mass_scale_bits, 1.0_f32.to_bits());
        assert_eq!(observation.final_out_mass_bits, 11.0_f32.to_bits());
    }

    fn part_definition_identity(part: SkeletonPartIndex) -> PreparedPartDefinitionIdentity {
        PreparedPartDefinitionIdentity(
            0xa000 + u64::try_from(part.index()).expect("24-part index fits u64"),
        )
    }

    #[test]
    fn constants_and_raw_lane_storage_are_exact() {
        assert_eq!(ROOT_PART_MASS.to_bits(), 0x3a83_126f);
        assert_eq!(MASS_PROPERTIES_DENSITY.to_bits(), 0x3f80_0000);
        assert_eq!(ANIMATED_INVERSE_MASS_NUMERATOR.to_bits(), 0x3f80_0000);
        assert_eq!(CAPSULE_DIMENSION_SCALE_KEY, 0x0720_4f10_dd93_7907);
        assert_eq!(ROOT_CAPSULE_HALF_LENGTH_KEY, 0x5e43_a9eb_d2be_bbde);
        assert_eq!(ROOT_CAPSULE_RADIUS_KEY, 0x7e90_113b_05ba_cda6);
        assert_eq!(VOLUMEFLAG_ISENABLED, 1);

        let size = PhysicsBoneSizeBits::from_float_lanes([
            -0.0,
            f32::from_bits(0x7fa1_2345),
            f32::NEG_INFINITY,
            f32::from_bits(1),
        ]);
        assert_eq!(size.lanes, [0x8000_0000, 0x7fa1_2345, 0xff80_0000, 1]);
        for (lane, expected) in size.lanes.into_iter().enumerate() {
            assert_eq!(size.float_lane(lane).to_bits(), expected);
        }
    }

    #[test]
    fn full_loop_installs_distinct_identities_and_uses_one_part_definition() {
        let bones = test_bones();
        let target = SkeletonPartIndex::Neck;
        let Harness {
            events,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, false, Some((target, 1)), 1);
        let result = run(
            &bones,
            0x3f99_999a,
            &mut controls,
            &mut backend,
            &mut output,
        )
        .expect("valid dynamics");
        assert_eq!(result.len(), SKELETON_PART_COUNT);

        let volume = general_volume_identity(target);
        let part_definition = part_definition_identity(target);
        assert_ne!(volume.0, part_definition.0);
        let events = events.borrow();
        let body_install = event_position(&events, &Event::InstallBodyVolume(target, volume));
        let part_install = event_position(
            &events,
            &Event::InstallPartVolume(target, part_definition, volume),
        );
        let mus = event_position(
            &events,
            &Event::InstallStaticFriction(target, part_definition, 0x3e80_0000),
        );
        let mud = event_position(
            &events,
            &Event::InstallDynamicFriction(target, part_definition, 0x3e80_0000),
        );
        let res = event_position(
            &events,
            &Event::InstallRestitution(target, part_definition, 0x3d80_0000),
        );
        assert!(body_install < part_install);
        assert!(part_install < mus && mus < mud && mud < res);

        let ragdoll_compute = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::Compute(
                        part,
                        MassPropertyPass::Ragdoll,
                        identity,
                        _,
                        density
                    ) if *part == target
                        && *identity == part_definition
                        && *density == MASS_PROPERTIES_DENSITY.to_bits()
                )
            })
            .expect("ragdoll compute");
        let animated_compute = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::Compute(
                        part,
                        MassPropertyPass::Animated,
                        identity,
                        _,
                        density
                    ) if *part == target
                        && *identity == part_definition
                        && *density == MASS_PROPERTIES_DENSITY.to_bits()
                )
            })
            .expect("animated compute");
        let debug_adjust = event_position(
            &events,
            &Event::DebugAdjust(
                target,
                part_definition,
                DebugInertiaAdjustment::Mode1,
                4.0_f32.to_bits(),
            ),
        );
        assert!(res < ragdoll_compute && ragdoll_compute < animated_compute);
        assert!(animated_compute < debug_adjust);

        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::Compute(..)))
                .count(),
            SKELETON_PART_COUNT * 2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::InstallPartVolume(..)))
                .count(),
            SKELETON_PART_COUNT
        );
    }

    #[test]
    fn root_flag_clear_preserves_every_other_bit_and_native_order() {
        for initial in [0, 1, u32::MAX] {
            let bones = test_bones();
            let Harness {
                events,
                mut controls,
                mut backend,
                mut output,
                root_flags,
            } = harness(1.0, false, false, None, initial);
            run(
                &bones,
                0x3f99_999a,
                &mut controls,
                &mut backend,
                &mut output,
            )
            .expect("valid dynamics");
            let expected = initial & !VOLUMEFLAG_ISENABLED;
            assert_eq!(root_flags.get(), expected);

            let part = SkeletonPartIndex::SkateboardRoot;
            let volume = root_volume_identity(part);
            let events = events.borrow();
            let root_index = event_position(&events, &Event::RootIndex(part));
            let half_length = event_position(&events, &Event::RootHalfLength);
            let radius = event_position(&events, &Event::RootRadius);
            let create = event_position(
                &events,
                &Event::PrepareRoot(part, volume, 3.0_f32.to_bits(), 2.0_f32.to_bits()),
            );
            let disable = event_position(
                &events,
                &Event::DisableRoot(part, volume, initial, expected),
            );
            let zero_mass = event_position(&events, &Event::PartMass(part, 0));
            let install = event_position(&events, &Event::InstallBodyVolume(part, volume));
            assert!(root_index < half_length && half_length < radius);
            assert!(radius < create && create < disable);
            assert!(disable < zero_mass && zero_mass < install);
        }
    }

    #[test]
    fn late_hat_selection_owns_hat_side_effects_and_reuses_part_identity() {
        let bones = test_bones();
        let part = SkeletonPartIndex::Neck1;
        let Harness {
            events,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, true, Some((part, 1)), 1);
        output.enable_hat_after_part_zero = true;
        run(
            &bones,
            0x3f99_999a,
            &mut controls,
            &mut backend,
            &mut output,
        )
        .expect("valid dynamics");

        let part_definition = part_definition_identity(part);
        let events = events.borrow();
        let part_zero_done = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::InverseInertia(
                        SkeletonPartIndex::SkateboardRoot,
                        MassPropertyPass::Animated,
                        _
                    )
                )
            })
            .expect("part zero completion");
        let hat_type = event_position(&events, &Event::HatType);
        let observation = test_hat_observation();
        let hat_boundary =
            event_position(&events, &Event::HatPreparationCompleted(part, observation));
        assert_test_hat_observation(observation);
        let outer_install = event_position(
            &events,
            &Event::InstallBodyVolume(part, hat_volume_identity()),
        );
        let ragdoll_compute = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::Compute(
                        event_part,
                        MassPropertyPass::Ragdoll,
                        identity,
                        _,
                        _
                    ) if *event_part == part && *identity == part_definition
                )
            })
            .expect("hat ragdoll compute");
        let animated_compute = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::Compute(
                        event_part,
                        MassPropertyPass::Animated,
                        identity,
                        _,
                        _
                    ) if *event_part == part && *identity == part_definition
                )
            })
            .expect("hat animated compute");
        let read_hat = event_position(&events, &Event::ReadHatPresent);
        let flag = event_position(&events, &Event::HatInverseFlag(part, part_definition));
        let matrix = event_position(&events, &Event::HatInverseMatrix(part, part_definition));
        let debug_mode = event_position(&events, &Event::DebugMode(part));
        let debug_adjust = event_position(
            &events,
            &Event::DebugAdjust(
                part,
                part_definition,
                DebugInertiaAdjustment::Mode1,
                4.0_f32.to_bits(),
            ),
        );
        assert!(part_zero_done < hat_type && hat_type < hat_boundary);
        assert!(hat_boundary < outer_install && outer_install < ragdoll_compute);
        assert!(ragdoll_compute < animated_compute);
        assert!(animated_compute < read_hat && read_hat < flag && flag < matrix);
        assert!(matrix < debug_mode && debug_mode < debug_adjust);
        assert!(!events.iter().any(|event| {
            matches!(
                event,
                Event::PartMass(event_part, _)
                    | Event::GeneralMassScale(event_part)
                    | Event::PrepareGeneral(event_part, ..)
                    if *event_part == part
            )
        }));
    }

    #[test]
    fn callback_variant_contract_rejects_impossible_successes() {
        let mut bones = test_bones();

        let Harness {
            events,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, true, false, None, 1);
        backend.hat_shape_override = Some(PreparedShape::Sphere {
            volume: PreparedVolumeIdentity(1),
            radius: 1.0,
            unscaled_mass_integral: 1.0,
        });
        assert_eq!(
            run(
                &bones,
                0x3f99_999a,
                &mut controls,
                &mut backend,
                &mut output,
            ),
            Err(RagdollDynamicsError::PreparedShapeMismatch {
                part: SkeletonPartIndex::Neck1,
                expected: PreparedShapeClass::HatAggregate,
                actual: PreparedShapeClass::Sphere,
            })
        );
        let events = events.borrow();
        assert!(events.contains(&Event::HatPreparationCompleted(
            SkeletonPartIndex::Neck1,
            test_hat_observation(),
        )));
        assert!(!events.iter().any(|event| {
            matches!(event, Event::InstallBodyVolume(SkeletonPartIndex::Neck1, _))
        }));
        drop(events);

        let Harness {
            events: _,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, false, None, 1);
        backend.root_shape_override = Some(PreparedShape::HatAggregate {
            volume: PreparedVolumeIdentity(2),
            effective_mass: 1.0,
            observation: test_hat_observation(),
        });
        assert_eq!(
            run(
                &bones,
                0x3f99_999a,
                &mut controls,
                &mut backend,
                &mut output,
            ),
            Err(RagdollDynamicsError::PreparedShapeMismatch {
                part: SkeletonPartIndex::SkateboardRoot,
                expected: PreparedShapeClass::RootCapsule,
                actual: PreparedShapeClass::HatAggregate,
            })
        );

        bones[SkeletonPartIndex::Neck1.index()].volume_type_raw =
            SkeletonVolumeShape::Capsule as u32;
        let Harness {
            events: _,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, false, None, 1);
        backend.general_shape_override = Some(PreparedShape::Box {
            volume: PreparedVolumeIdentity(3),
            half_extents: [1.0; 3],
            unscaled_mass_integral: 1.0,
        });
        assert_eq!(
            run(
                &bones,
                0x3f99_999a,
                &mut controls,
                &mut backend,
                &mut output,
            ),
            Err(RagdollDynamicsError::PreparedShapeMismatch {
                part: SkeletonPartIndex::Neck1,
                expected: PreparedShapeClass::Capsule,
                actual: PreparedShapeClass::Box,
            })
        );
    }

    #[test]
    fn capsule_scale_is_one_raw_snapshot_across_all_callbacks() {
        let mut bones = test_bones();
        for part in SkeletonPartIndex::ALL.into_iter().skip(1) {
            bones[part.index()].volume_type_raw = SkeletonVolumeShape::Capsule as u32;
        }
        let supplied = 0x3f99_999a;
        let Harness {
            events,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, false, None, 1);
        run(&bones, supplied, &mut controls, &mut backend, &mut output).expect("valid dynamics");

        let snapshots: Vec<_> = events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                Event::PrepareGeneral(_, SkeletonVolumeShape::Capsule, _, _, bits) => Some(*bits),
                _ => None,
            })
            .collect();
        assert_eq!(snapshots.len(), SKELETON_PART_COUNT - 1);
        assert!(snapshots.iter().all(|bits| *bits == supplied));
    }

    #[test]
    fn nonpositive_effective_mass_still_performs_common_install() {
        for unscaled_mass in [0.0, -0.0, -2.0, f32::from_bits(0x7fc1_2345)] {
            let bones = test_bones();
            let part = SkeletonPartIndex::Neck1;
            let volume = general_volume_identity(part);
            let part_definition = part_definition_identity(part);
            let Harness {
                events,
                mut controls,
                mut backend,
                mut output,
                root_flags: _,
            } = harness(1.0, false, false, None, 1);
            backend.general_integral_override = Some(unscaled_mass);
            let effective_mass = unscaled_mass * 10.0;
            assert_eq!(
                run(
                    &bones,
                    0x3f99_999a,
                    &mut controls,
                    &mut backend,
                    &mut output,
                ),
                Err(RagdollDynamicsError::NonPositiveEffectiveMass {
                    part,
                    raw: effective_mass.to_bits(),
                })
            );

            let events = events.borrow();
            let mass_write =
                event_position(&events, &Event::PartMass(part, unscaled_mass.to_bits()));
            let scale = event_position(&events, &Event::GeneralMassScale(part));
            let body_install = event_position(&events, &Event::InstallBodyVolume(part, volume));
            let part_install = event_position(
                &events,
                &Event::InstallPartVolume(part, part_definition, volume),
            );
            let mus = event_position(
                &events,
                &Event::InstallStaticFriction(part, part_definition, 0x3e80_0000),
            );
            let mud = event_position(
                &events,
                &Event::InstallDynamicFriction(part, part_definition, 0x3e80_0000),
            );
            let res = event_position(
                &events,
                &Event::InstallRestitution(part, part_definition, 0x3d80_0000),
            );
            assert!(mass_write < scale && scale < body_install);
            assert!(body_install < part_install && part_install < mus);
            assert!(mus < mud && mud < res);
            assert!(!events.iter().any(|event| {
                matches!(event, Event::Divide(event_part, ..) if *event_part == part)
            }));
        }
    }

    #[test]
    fn numerator_abort_precedes_every_part_side_effect() {
        for numerator in [0.0, -0.0, -1.0, f32::from_bits(0x7fc1_2345)] {
            let bones = test_bones();
            let Harness {
                events,
                mut controls,
                mut backend,
                mut output,
                root_flags: _,
            } = harness(numerator, false, false, None, 1);
            assert_eq!(
                run(
                    &bones,
                    0x3f99_999a,
                    &mut controls,
                    &mut backend,
                    &mut output,
                ),
                Err(RagdollDynamicsError::NonPositiveInverseMassNumerator {
                    raw: numerator.to_bits(),
                })
            );
            assert_eq!(&*events.borrow(), &[Event::Numerator]);
        }
    }

    #[test]
    fn ragdoll_mass_property_failure_preserves_internal_and_prior_writes() {
        let bones = test_bones();
        let part = SkeletonPartIndex::Neck1;
        let part_definition = part_definition_identity(part);

        let Harness {
            events,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, false, None, 1);
        backend.fail_at = Some((part, MassPropertyPass::Ragdoll));
        assert_eq!(
            run(
                &bones,
                0x3f99_999a,
                &mut controls,
                &mut backend,
                &mut output,
            ),
            Err(RagdollDynamicsError::MassPropertiesFailed {
                part,
                pass: MassPropertyPass::Ragdoll,
            })
        );
        let events = events.borrow();
        let inverse_mass = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::InverseMass(
                        event_part,
                        MassPropertyPass::Ragdoll,
                        _
                    ) if *event_part == part
                )
            })
            .expect("ragdoll inverse-mass cache");
        let compute = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::Compute(
                        event_part,
                        MassPropertyPass::Ragdoll,
                        identity,
                        _,
                        _
                    ) if *event_part == part && *identity == part_definition
                )
            })
            .expect("failed call");
        let internal = event_position(
            &events,
            &Event::ComputeInternalWrite(part, MassPropertyPass::Ragdoll, part_definition),
        );
        assert!(inverse_mass < compute && compute < internal);
        assert!(!events.iter().any(|event| {
            matches!(
                event,
                Event::InverseInertia(
                    event_part,
                    MassPropertyPass::Ragdoll,
                    _
                ) if *event_part == part
            )
        }));
    }

    #[test]
    fn animated_mass_property_failure_preserves_internal_and_prior_writes() {
        let bones = test_bones();
        let part = SkeletonPartIndex::Neck1;
        let part_definition = part_definition_identity(part);
        let Harness {
            events,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, false, None, 1);
        backend.fail_at = Some((part, MassPropertyPass::Animated));
        assert_eq!(
            run(
                &bones,
                0x3f99_999a,
                &mut controls,
                &mut backend,
                &mut output,
            ),
            Err(RagdollDynamicsError::MassPropertiesFailed {
                part,
                pass: MassPropertyPass::Animated,
            })
        );
        let events = events.borrow();
        let ragdoll_inertia = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::InverseInertia(
                        event_part,
                        MassPropertyPass::Ragdoll,
                        _
                    ) if *event_part == part
                )
            })
            .expect("ragdoll inertia cache");
        let animated_mass = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::InverseMass(
                        event_part,
                        MassPropertyPass::Animated,
                        _
                    ) if *event_part == part
                )
            })
            .expect("animated inverse-mass cache");
        let internal = event_position(
            &events,
            &Event::ComputeInternalWrite(part, MassPropertyPass::Animated, part_definition),
        );
        assert!(ragdoll_inertia < animated_mass && animated_mass < internal);
        assert!(!events.iter().any(|event| {
            matches!(
                event,
                Event::InverseInertia(
                    event_part,
                    MassPropertyPass::Animated,
                    _
                ) | Event::DebugMode(event_part) if *event_part == part
            )
        }));
    }

    #[test]
    fn neutral_debug_modes_select_the_two_distinct_helpers() {
        let bones = test_bones();
        let part = SkeletonPartIndex::LeftArm;
        for (raw, adjustment) in [
            (1, DebugInertiaAdjustment::Mode1),
            (2, DebugInertiaAdjustment::Mode2),
        ] {
            let Harness {
                events,
                mut controls,
                mut backend,
                mut output,
                root_flags: _,
            } = harness(1.0, false, false, Some((part, raw)), 1);
            run(
                &bones,
                0x3f99_999a,
                &mut controls,
                &mut backend,
                &mut output,
            )
            .expect("valid dynamics");
            assert!(events.borrow().contains(&Event::DebugAdjust(
                part,
                part_definition_identity(part),
                adjustment,
                4.0_f32.to_bits(),
            )));
        }

        let Harness {
            events,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, false, Some((part, 3)), 1);
        run(
            &bones,
            0x3f99_999a,
            &mut controls,
            &mut backend,
            &mut output,
        )
        .expect("valid dynamics");
        assert!(
            !events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::DebugMultiplier(_) | Event::DebugAdjust(..)))
        );
    }

    #[test]
    fn unsupported_type_and_host_only_shape_mismatch_stop_before_install() {
        let mut bones = test_bones();
        let part = SkeletonPartIndex::Neck1;
        bones[part.index()].volume_type_raw = 3;
        let Harness {
            events,
            mut controls,
            mut backend,
            mut output,
            root_flags: _,
        } = harness(1.0, false, false, None, 1);
        assert_eq!(
            run(
                &bones,
                0x3f99_999a,
                &mut controls,
                &mut backend,
                &mut output,
            ),
            Err(RagdollDynamicsError::UnsupportedVolumeType { part, raw: 3 })
        );
        assert!(!events.borrow().iter().any(|event| {
            matches!(event, Event::InstallBodyVolume(event_part, _) if *event_part == part)
        }));
    }

    #[test]
    fn scalar_fsel_boundaries_preserve_ties_zeros_infinities_and_nans() {
        assert_eq!(select_sphere_extent(-0.0, 0.0).to_bits(), 0x8000_0000);
        assert_eq!(select_sphere_extent(4.0, 4.0).to_bits(), 4.0_f32.to_bits());
        assert_eq!(
            select_sphere_extent(f32::INFINITY, f32::NEG_INFINITY).to_bits(),
            f32::INFINITY.to_bits()
        );
        assert_eq!(
            select_sphere_extent(f32::NEG_INFINITY, f32::INFINITY).to_bits(),
            f32::INFINITY.to_bits()
        );
        assert_eq!(
            select_sphere_extent(f32::INFINITY, f32::INFINITY).to_bits(),
            f32::INFINITY.to_bits()
        );
        let first_nan = f32::from_bits(0x7fa1_2345);
        assert_eq!(
            select_sphere_extent(first_nan, 7.0).to_bits(),
            7.0_f32.to_bits()
        );
        let second_nan = f32::from_bits(0xffa5_4321);
        assert_eq!(
            select_sphere_extent(7.0, second_nan).to_bits(),
            second_nan.to_bits()
        );

        assert_eq!(
            clamp_capsule_half_length_remainder(2.0).to_bits(),
            2.0_f32.to_bits()
        );
        assert_eq!(clamp_capsule_half_length_remainder(-2.0).to_bits(), 0);
        assert_eq!(clamp_capsule_half_length_remainder(0.0).to_bits(), 0);
        assert_eq!(clamp_capsule_half_length_remainder(-0.0).to_bits(), 0);
        let nan = f32::from_bits(0xffa1_2345);
        assert_eq!(
            clamp_capsule_half_length_remainder(nan).to_bits(),
            nan.to_bits()
        );
    }
}
