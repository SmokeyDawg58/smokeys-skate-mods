//! Source construction of `SkeletonDrives` (0x82b59b08), its two real hook
//! Parts, and the bone payloads consumed by post-admission Drive creation.
//!
//! This owner has no invented Body/Island/Entity pointers. The actual skeleton
//! owner performs admission, spy writes, placement and ordered removal. All
//! payload preparation here precedes those side effects. A failed construction
//! publishes no owner; it does not promise rollback of the native constructor.
//! Retained immutable `SkeletonData` excludes concurrent pose/attribute alias
//! mutation. Numeric estimates keep the existing explicit host boundary.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "skeleton drives await live player construction")
)]

use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyPartAttachmentMetadata, DynamicAssemblyPartAttachmentTemplate,
        DynamicAssemblyPartAttachmentTemplateError, DynamicAssemblyPartPayload,
        DynamicAssemblyRuntimeTemplate, DynamicAssemblySession, DynamicAssemblySessionCreateError,
        DynamicAssemblyTemplateError,
    },
    ragdoll::{SKELETON_PART_COUNT, SkeletonPartIndex},
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
    },
    simulation_rigid_body_pool::SimulationInertia,
    simulation_volume_integral::fat_box_unit_mass,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_spawn_math::{skateboard_spawn_compose, skateboard_spawn_rigid_inverse},
    skateboard_wheels::{
        FRESH_VOLUME_TRANSFORM, SkateboardWheelMassNumeric, apply_unit_density_mass_to_inertia,
    },
    skeleton_data::SkeletonData,
    world::try_simulation_quaternion_candidate,
};
use skate2_formats::collision::{AssemblyPartMaterial, Volume, VolumeType};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

#[cfg(test)]
#[path = "skeleton_drives_tests.rs"]
mod tests;

#[path = "skeleton_drives_ragdoll_update.rs"]
pub(crate) mod ragdoll_update;
#[path = "skeleton_drives_update.rs"]
pub(crate) mod update;

const DISPLACEMENT_SCALE: f32 = f32::from_bits(0x426f_ffff);
const STRENGTH_SCALE: f32 = f32::from_bits(0x4560_fffe);
const SOFT_KEYS: [u64; 4] = [
    0xe1a7_f2dc_fab0_b8d0,
    0xf300_ffd5_99e1_709e,
    0xf508_d452_b962_f68d,
    0x15a2_6b61_43e7_df97,
];
const ZERO_PARAMS: SimulationDriveDynamicsParams = SimulationDriveDynamicsParams {
    spring: 0.0,
    damping: 0.0,
    strength: 0.0,
    raw_type: 0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonDriveType {
    Local,
    LocalRoot,
}

impl SkeletonDriveType {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Local => 0,
            Self::LocalRoot => 1,
        }
    }
}

/// Two shared generated Instances are resolved/changed for EVERY `BoneDrives`
/// constructor (0x82b48890); the four ordinary global lookups follow immediately.
/// Cached layout reads are separate scalar operations, not a guessed profile.
pub(crate) trait SkeletonDriveAttributeRead {
    type Pointer;
    type Error;
    fn find_and_change_animation_drive(
        &mut self,
        kind: SkeletonDriveType,
    ) -> Result<(), Self::Error>;
    fn lookup_global(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn animation_float(
        &mut self,
        kind: SkeletonDriveType,
        byte_offset: usize,
    ) -> Result<f32, Self::Error>;
}

/// Current cvar target loads at 0x82b50a38/3c/70/74. Defaults1/10 are NOT
/// substituted here: callers supply the actual current service.
pub(crate) trait SkeletonDriveCvars {
    type Error;
    fn hard_drive_displacement(&mut self) -> Result<f32, Self::Error>;
    fn hard_drive_strength(&mut self) -> Result<f32, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonDriveQuaternionSite {
    BoneChild {
        part: SkeletonPartIndex,
        kind: SkeletonDriveType,
    },
    BoneParent {
        part: SkeletonPartIndex,
        kind: SkeletonDriveType,
    },
    HookParent {
        hook: usize,
    },
    HookChild {
        hook: usize,
    },
}

pub(crate) trait SkeletonDriveNumeric {
    type Error;
    fn quaternion_estimate(
        &mut self,
        site: SkeletonDriveQuaternionSite,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error>;
}

pub(crate) struct HostSkeletonDriveNumeric;
impl SkeletonDriveNumeric for HostSkeletonDriveNumeric {
    type Error = std::convert::Infallible;
    fn quaternion_estimate(
        &mut self,
        _site: SkeletonDriveQuaternionSite,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        Ok(magnitudes.map(|value| value.sqrt().recip()))
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonDrivesError<A, N, C> {
    Attribute(A),
    Numeric(N),
    Cvar(C),
    Allocation,
    PartTemplate(DynamicAssemblyPartAttachmentTemplateError),
    Template(DynamicAssemblyTemplateError),
    Session(DynamicAssemblySessionCreateError),
}
impl<A: fmt::Display, N: fmt::Display, C: fmt::Display> fmt::Display
    for SkeletonDrivesError<A, N, C>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "skeleton drive attribute: {e}"),
            Self::Numeric(e) => write!(f, "skeleton drive quaternion: {e}"),
            Self::Cvar(e) => write!(f, "skeleton hook cvar: {e}"),
            Self::Allocation => f.write_str("skeleton hook host storage allocation failed"),
            Self::Template(e) => write!(f, "skeleton hook template: {e}"),
            Self::PartTemplate(e) => write!(f, "skeleton hook Part template: {e}"),
            Self::Session(e) => write!(f, "skeleton hook session: {e}"),
        }
    }
}
impl<A: std::error::Error + 'static, N: std::error::Error + 'static, C: std::error::Error + 'static>
    std::error::Error for SkeletonDrivesError<A, N, C>
{
}

#[derive(Debug)]
pub(crate) struct SkeletonBoneDrives {
    frames: [Arc<RwLock<SimulationDriveFrames>>; 2],
    dynamics: [Arc<RwLock<SimulationDriveDynamics>>; 2],
    parent_parts: [Option<SkeletonPartIndex>; 2],
    active: [bool; 2],
    flags: u32,
    strengths: [f32; 2],
    soft: [f32; 4],
    part: Option<SkeletonPartIndex>,
}

impl SkeletonBoneDrives {
    fn fresh() -> Self {
        Self {
            frames: std::array::from_fn(|_| Arc::new(RwLock::new(fresh_frames()))),
            dynamics: std::array::from_fn(|_| Arc::new(RwLock::new(fresh_dynamics()))),
            // Source parent IDs/other references are unwritten until SetUpDrive.
            parent_parts: [None; 2],
            active: [false; 2],
            flags: 0,
            strengths: [0.0; 2],
            soft: [0.0; 4],
            part: None,
        }
    }
    pub(crate) fn frames(&self, kind: SkeletonDriveType) -> &Arc<RwLock<SimulationDriveFrames>> {
        &self.frames[kind.index()]
    }
    pub(crate) fn dynamics(
        &self,
        kind: SkeletonDriveType,
    ) -> &Arc<RwLock<SimulationDriveDynamics>> {
        &self.dynamics[kind.index()]
    }
    pub(crate) const fn parent_parts(&self) -> &[Option<SkeletonPartIndex>; 2] {
        &self.parent_parts
    }
    pub(crate) const fn active(&self) -> &[bool; 2] {
        &self.active
    }
    pub(crate) const fn flags(&self) -> u32 {
        self.flags
    }
    pub(crate) const fn strengths(&self) -> &[f32; 2] {
        &self.strengths
    }
    /// Collision displacement/strength, ragdoll displacement/strength, in order.
    pub(crate) const fn soft_parameters(&self) -> &[f32; 4] {
        &self.soft
    }
    pub(crate) const fn part(&self) -> Option<SkeletonPartIndex> {
        self.part
    }
}

#[derive(Debug)]
pub(crate) struct SkeletonHookPartDefinition {
    volume: Volume,
    inertia: Arc<SimulationInertia>,
    material: AssemblyPartMaterial,
    inverse_body_transform: Option<[f32; 16]>,
}
impl SkeletonHookPartDefinition {
    pub(crate) const fn volume(&self) -> &Volume {
        &self.volume
    }
    pub(crate) fn inertia(&self) -> &Arc<SimulationInertia> {
        &self.inertia
    }
    pub(crate) const fn material(&self) -> &AssemblyPartMaterial {
        &self.material
    }
    pub(crate) const fn inverse_body_transform(&self) -> Option<&[f32; 16]> {
        self.inverse_body_transform.as_ref()
    }
    /// The inherited `PartDefinition` word is0; later `InsertHookAssembly` requests1.
    #[expect(
        clippy::unused_self,
        reason = "read-only projection of the source definition's default static word"
    )]
    pub(crate) const fn is_static_body(&self) -> bool {
        false
    }
    pub(crate) fn inverse_tensor_w(&self) -> f32 {
        self.inertia.inverse_tensor[0]
    }
}

/// The two ORIGINAL Box instances, with their own mass and default material.
/// This is neither the board hook factory nor a fake collision-address image.
#[derive(Debug)]
pub(crate) struct SkeletonHookDefinition {
    parts: [SkeletonHookPartDefinition; 2],
}
impl SkeletonHookDefinition {
    pub(crate) const fn parts(&self) -> &[SkeletonHookPartDefinition; 2] {
        &self.parts
    }
}

pub(crate) type SkeletonHookAssemblySession =
    DynamicAssemblySession<SkateboardSimulationBackend, SkeletonHookDefinition>;

/// An eligible source `CreateDrive` request. Both endpoints derive from the
/// retained `SkeletonData`, not from caller-provided Body IDs or arena addresses.
pub(crate) struct SkeletonBoneDriveEmission<'a> {
    pub(crate) part: SkeletonPartIndex,
    pub(crate) parent: SkeletonPartIndex,
    pub(crate) kind: SkeletonDriveType,
    pub(crate) frames: &'a Arc<RwLock<SimulationDriveFrames>>,
    pub(crate) dynamics: &'a Arc<RwLock<SimulationDriveDynamics>>,
}

pub(crate) struct SkeletonDrives {
    data: Arc<SkeletonData>,
    bones: [SkeletonBoneDrives; SKELETON_PART_COUNT],
    hook_frames: [Arc<RwLock<SimulationDriveFrames>>; 2],
    hook_dynamics: [Arc<RwLock<SimulationDriveDynamics>>; 2],
    hook_definition: Option<Arc<SkeletonHookDefinition>>,
    hook_session: Option<SkeletonHookAssemblySession>,
    ragdoll_strength: f32,
    ik_active: [bool; SKELETON_PART_COUNT],
}

impl SkeletonDrives {
    /// Constructor -> `InitDrives` -> the definition/payload prefix of `InitHooks`.
    /// Actual `HookPool` insert and `AddDrive` calls follow in the owning session.
    /// Local Arcs are unpublished during callbacks, so no lock can be poisoned
    /// or aliased here; allocation of Arcs follows normal Rust allocator policy.
    #[expect(
        clippy::type_complexity,
        reason = "preserve the three independent source-service error types"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "preserve constructor service-call and payload-write chronology in one place"
    )]
    pub(crate) fn try_create<
        A: SkeletonDriveAttributeRead,
        N: SkeletonDriveNumeric,
        C: SkeletonDriveCvars,
    >(
        data: Arc<SkeletonData>,
        attributes: &mut A,
        numeric: &mut N,
        mass: &mut impl SkateboardWheelMassNumeric,
        cvars: &mut C,
    ) -> Result<Self, SkeletonDrivesError<A::Error, N::Error, C::Error>> {
        use SkeletonDrivesError::{Attribute, Cvar, Numeric};
        let hook_frames = std::array::from_fn(|_| Arc::new(RwLock::new(fresh_frames())));
        let hook_dynamics = std::array::from_fn(|_| Arc::new(RwLock::new(fresh_dynamics())));
        let mut bones: [SkeletonBoneDrives; SKELETON_PART_COUNT] =
            std::array::from_fn(|_| SkeletonBoneDrives::fresh());
        for bone in &mut bones {
            attributes
                .find_and_change_animation_drive(SkeletonDriveType::Local)
                .map_err(Attribute)?;
            attributes
                .find_and_change_animation_drive(SkeletonDriveType::LocalRoot)
                .map_err(Attribute)?;
            for (destination, key) in bone.soft.iter_mut().zip(SOFT_KEYS) {
                let pointer = attributes.lookup_global(key).map_err(Attribute)?;
                *destination = attributes.float(&pointer).map_err(Attribute)?;
            }
        }
        let ik_active = [false; SKELETON_PART_COUNT];
        // InitDrives samples the physics-pose buffer once, then traverses0..23.
        let poses = data.physics_pose();
        for (part, bone) in bones.iter_mut().enumerate() {
            let Some(parent) = data.parents()[part] else {
                continue;
            };
            let part = SkeletonPartIndex::from_index(part).expect("fixed24 source loop");
            for (kind, selected_parent, flags) in [
                (SkeletonDriveType::Local, parent, 2),
                (
                    SkeletonDriveType::LocalRoot,
                    SkeletonPartIndex::from_index(23).expect("Hips23"),
                    0,
                ),
            ] {
                bone.part = Some(part);
                bone.parent_parts[kind.index()] = Some(selected_parent);
                bone.active[kind.index()] = true;
                bone.flags = flags;
                let frames = try_bone_frames(
                    part,
                    kind,
                    &poses[selected_parent.index()],
                    &poses[part.index()],
                    numeric,
                )
                .map_err(Numeric)?;
                *bone.frames[kind.index()]
                    .write()
                    .expect("unpublished payload") = frames;
                let dynamics = try_initial_dynamics(attributes, kind).map_err(Attribute)?;
                *bone.dynamics[kind.index()]
                    .write()
                    .expect("unpublished payload") = dynamics;
            }
        }

        // InitHooks: Part0 mass -> ParentFrame -> ChildFrame, then Part1.
        let mut parts = Vec::new();
        parts
            .try_reserve_exact(2)
            .map_err(|_| SkeletonDrivesError::Allocation)?;
        for (hook, frames) in hook_frames.iter().enumerate() {
            parts.push(create_hook_part(mass));
            let quaternion_b =
                try_simulation_quaternion_candidate(FRESH_VOLUME_TRANSFORM, |values| {
                    numeric.quaternion_estimate(
                        SkeletonDriveQuaternionSite::HookParent { hook },
                        values,
                    )
                })
                .map_err(Numeric)?;
            let quaternion_a =
                try_simulation_quaternion_candidate(FRESH_VOLUME_TRANSFORM, |values| {
                    numeric.quaternion_estimate(
                        SkeletonDriveQuaternionSite::HookChild { hook },
                        values,
                    )
                })
                .map_err(Numeric)?;
            *frames.write().expect("unpublished payload") = SimulationDriveFrames {
                quaternion_a,
                position_a: [0.0; 4],
                quaternion_b,
                position_b: [0.0; 4],
            };
        }
        let hard = SimulationDriveDynamicsParams {
            spring: f32::from_bits(0x4415_ffff),
            damping: 0.0,
            strength: f32::from_bits(0x470c_9fff),
            raw_type: 2,
        };
        *hook_dynamics[0].write().expect("unpublished payload") = SimulationDriveDynamics {
            linear: hard,
            angular: hard,
        };
        let linear_displacement = cvars.hard_drive_displacement().map_err(Cvar)?;
        let linear_strength = cvars.hard_drive_strength().map_err(Cvar)?;
        let linear_spring = linear_displacement * DISPLACEMENT_SCALE;
        let linear_strength = linear_strength * STRENGTH_SCALE;
        let angular_displacement = cvars.hard_drive_displacement().map_err(Cvar)?;
        let angular_strength = cvars.hard_drive_strength().map_err(Cvar)?;
        // Native angular strength multiplication precedes angular spring.
        let angular_strength = angular_strength * STRENGTH_SCALE;
        let angular_spring = angular_displacement * DISPLACEMENT_SCALE;
        *hook_dynamics[1].write().expect("unpublished payload") = SimulationDriveDynamics {
            linear: SimulationDriveDynamicsParams {
                spring: linear_spring,
                damping: 0.0,
                strength: linear_strength,
                raw_type: 2,
            },
            angular: SimulationDriveDynamicsParams {
                spring: angular_spring,
                damping: 0.0,
                strength: angular_strength,
                raw_type: 2,
            },
        };
        let parts: [SkeletonHookPartDefinition; 2] =
            parts.try_into().expect("two source hook Parts");
        let hook_definition = Arc::new(SkeletonHookDefinition { parts });
        let hook_session = create_hook_session(Arc::clone(&hook_definition))?;
        Ok(Self {
            data,
            bones,
            hook_frames,
            hook_dynamics,
            hook_definition: Some(hook_definition),
            hook_session: Some(hook_session),
            ragdoll_strength: 0.0,
            ik_active,
        })
    }

    pub(crate) fn data(&self) -> &Arc<SkeletonData> {
        &self.data
    }
    pub(crate) fn bone(&self, part: SkeletonPartIndex) -> &SkeletonBoneDrives {
        &self.bones[part.index()]
    }
    pub(crate) fn hook_frames(&self, hook: usize) -> &Arc<RwLock<SimulationDriveFrames>> {
        &self.hook_frames[hook]
    }
    pub(crate) fn hook_dynamics(&self, hook: usize) -> &Arc<RwLock<SimulationDriveDynamics>> {
        &self.hook_dynamics[hook]
    }
    pub(crate) fn hook_definition(&self) -> Option<&Arc<SkeletonHookDefinition>> {
        self.hook_definition.as_ref()
    }
    /// Moves the one real detached session/template. No reconstructed wrapper or
    /// duplicate body is generated when the shared `HookPool` consumes it.
    pub(crate) fn take_hook_session(&mut self) -> Option<SkeletonHookAssemblySession> {
        self.hook_session.take()
    }
    /// Release0x82b47ce0 drops hook geometry before the `BoneDrives` destructors.
    /// Caller must first remove hook drives, remove/drop the actual hook session
    /// (including a retained failure candidate), then release this final owner.
    /// Frames/dynamics remain retained for the subsequent bone cleanup phase.
    pub(crate) fn release_hook_definition(&mut self) {
        self.hook_definition = None;
    }
    /// `BoneDrives` destructor0x82b48878, after removal of this actual drive.
    /// Original parent Part IDs persist; live Body/Drive references belong to
    /// the checked runtime owner and are cleared there, never fabricated here.
    pub(crate) fn mark_bone_released(&mut self, part: SkeletonPartIndex, kind: SkeletonDriveType) {
        self.bones[part.index()].active[kind.index()] = false;
    }
    pub(crate) const fn ragdoll_strength(&self) -> f32 {
        self.ragdoll_strength
    }
    pub(crate) const fn ik_active(&self) -> &[bool; SKELETON_PART_COUNT] {
        &self.ik_active
    }

    /// `InitHooks0x82b50b90`..ce4 excludes endpoints0/23 even if they have parents.
    /// Source order is Part1..22, LOCAL then `LOCAL_ROOT`, only eligible Parts.
    pub(crate) fn eligible_bone_drives(
        &self,
    ) -> impl Iterator<Item = SkeletonBoneDriveEmission<'_>> {
        (1..23).flat_map(move |index| {
            let bone = &self.bones[index];
            [SkeletonDriveType::Local, SkeletonDriveType::LocalRoot]
                .into_iter()
                .filter_map(move |kind| {
                    let parent = bone.parent_parts[kind.index()]?;
                    Some(SkeletonBoneDriveEmission {
                        part: SkeletonPartIndex::from_index(index).expect("source1..22"),
                        parent,
                        kind,
                        frames: bone.frames(kind),
                        dynamics: bone.dynamics(kind),
                    })
                })
        })
    }
}

fn fresh_frames() -> SimulationDriveFrames {
    SimulationDriveFrames {
        quaternion_a: [0.0, 0.0, 0.0, 1.0],
        position_a: [0.0; 4],
        quaternion_b: [0.0, 0.0, 0.0, 1.0],
        position_b: [0.0; 4],
    }
}
fn fresh_dynamics() -> SimulationDriveDynamics {
    SimulationDriveDynamics {
        linear: ZERO_PARAMS,
        angular: ZERO_PARAMS,
    }
}

/// `SetUpDrive0x82b48ad8` -> `SetDriveFrames0x82b3fa90`. World-axis identity at
/// child position is mapped through EACH rigid inverse; child rotation is not
/// reused as the joint frame. Explicit identity products retain raw W effects.
fn try_bone_frames<N: SkeletonDriveNumeric>(
    part: SkeletonPartIndex,
    kind: SkeletonDriveType,
    parent: &[f32; 16],
    child: &[f32; 16],
    numeric: &mut N,
) -> Result<SimulationDriveFrames, N::Error> {
    let inverse_parent = skateboard_spawn_rigid_inverse(parent);
    let inverse_child = skateboard_spawn_rigid_inverse(child);
    let mut anchor = FRESH_VOLUME_TRANSFORM;
    anchor[12..16].copy_from_slice(&child[12..16]);
    let child_frame = skateboard_spawn_compose(&inverse_child, &anchor);
    let parent_frame = skateboard_spawn_compose(&inverse_parent, &anchor);
    let quaternion_a = try_simulation_quaternion_candidate(child_frame, |values| {
        numeric.quaternion_estimate(
            SkeletonDriveQuaternionSite::BoneChild { part, kind },
            values,
        )
    })?;
    let quaternion_b = try_simulation_quaternion_candidate(parent_frame, |values| {
        numeric.quaternion_estimate(
            SkeletonDriveQuaternionSite::BoneParent { part, kind },
            values,
        )
    })?;
    Ok(SimulationDriveFrames {
        quaternion_a,
        position_a: child_frame[12..16].try_into().expect("four lanes"),
        quaternion_b,
        position_b: parent_frame[12..16].try_into().expect("four lanes"),
    })
}

fn try_initial_dynamics<A: SkeletonDriveAttributeRead>(
    attributes: &mut A,
    kind: SkeletonDriveType,
) -> Result<SimulationDriveDynamics, A::Error> {
    let angular_strength = attributes.animation_float(kind, 8)?;
    let angular_displacement = attributes.animation_float(kind, 12)?;
    let angular_strength = angular_strength * STRENGTH_SCALE;
    let (angular_spring, linear_spring, linear_strength) = if kind == SkeletonDriveType::Local {
        (angular_displacement * DISPLACEMENT_SCALE, 0.0, 0.0)
    } else {
        let linear_displacement = attributes.animation_float(kind, 4)?;
        let angular_spring = angular_displacement * DISPLACEMENT_SCALE;
        let linear_strength = attributes.animation_float(kind, 0)?;
        let linear_spring = linear_displacement * DISPLACEMENT_SCALE;
        let linear_strength = linear_strength * STRENGTH_SCALE;
        (angular_spring, linear_spring, linear_strength)
    };
    Ok(SimulationDriveDynamics {
        linear: SimulationDriveDynamicsParams {
            spring: linear_spring,
            damping: 0.0,
            strength: linear_strength,
            raw_type: 2,
        },
        angular: SimulationDriveDynamicsParams {
            spring: angular_spring,
            damping: 0.0,
            strength: angular_strength,
            raw_type: 2,
        },
    })
}

fn create_hook_part(numeric: &mut impl SkateboardWheelMassNumeric) -> SkeletonHookPartDefinition {
    let volume = Volume {
        transform: FRESH_VOLUME_TRANSFORM,
        vtable_offset: VolumeType::Box as u32,
        shape_words: [0x3dcc_cccd; 3],
        radius: 0.0,
        group_id: 0,
        surface_id: 0,
        flags: 0,
    };
    let (diagonal, amount) =
        fat_box_unit_mass(volume.shape_words.map(f32::from_bits), volume.radius);
    let threshold = ((diagonal[1] + diagonal[2]) + diagonal[0]) * f32::from_bits(0x3586_37be);
    // Proven fresh fixed Box: the COM/axis branch uses the existing source
    // threshold, not a guessed default mass or admission-state inference.
    let inverse_body_transform = if 0.0 < threshold {
        None
    } else {
        Some(FRESH_VOLUME_TRANSFORM)
    };
    let mut inertia = SimulationInertia::native_default();
    apply_unit_density_mass_to_inertia(
        diagonal,
        amount,
        f32::from_bits(0x42c8_0000),
        &mut inertia,
        numeric,
    );
    SkeletonHookPartDefinition {
        volume,
        inertia: Arc::new(inertia),
        inverse_body_transform,
        material: AssemblyPartMaterial {
            static_friction: f32::from_bits(0x3f00_0000),
            dynamic_friction: f32::from_bits(0x3e99_999a),
            restitution: f32::from_bits(0x3ecc_cccd),
        },
    }
}

fn create_hook_session<A, N, C>(
    definition: Arc<SkeletonHookDefinition>,
) -> Result<SkeletonHookAssemblySession, SkeletonDrivesError<A, N, C>> {
    let mut metadata = Vec::new();
    let mut payloads = Vec::new();
    metadata
        .try_reserve_exact(2)
        .map_err(|_| SkeletonDrivesError::Allocation)?;
    payloads
        .try_reserve_exact(2)
        .map_err(|_| SkeletonDrivesError::Allocation)?;
    for part in definition.parts() {
        metadata.push(DynamicAssemblyPartAttachmentMetadata {
            inverse_body_transform: part
                .inverse_body_transform
                .unwrap_or(FRESH_VOLUME_TRANSFORM),
            has_inverse_body_transform: u32::from(part.inverse_body_transform.is_some()),
            is_static_body: 0,
        });
        payloads.push(DynamicAssemblyPartPayload {
            inertia: Arc::clone(&part.inertia).into(),
            material: part.material,
        });
    }
    let attachment = DynamicAssemblyPartAttachmentTemplate::try_from_parts(metadata, payloads)
        .map_err(SkeletonDrivesError::PartTemplate)?;
    let template = DynamicAssemblyRuntimeTemplate::try_from_owned_parts(
        definition,
        Arc::new(attachment),
        Vec::new(),
        Vec::new(),
    )
    .map_err(SkeletonDrivesError::Template)?;
    // InitHooks CreateAssembly0x82b50a94 reaches Part::Initialize0x82ae8fc8:
    // 9020 stores I at+00; 9024 stores four-zero P at+30; 9028 stores J at+10;
    // 902c stores K at+20. This is NOT an all-zero Part matrix and is proven
    // independently of the collision Box's own identity transform. The caller
    // changes only group/userData before Static1 insertion0x82b50aec.
    DynamicAssemblySession::try_new(Arc::new(template), vec![FRESH_VOLUME_TRANSFORM; 2])
        .map_err(SkeletonDrivesError::Session)
}
