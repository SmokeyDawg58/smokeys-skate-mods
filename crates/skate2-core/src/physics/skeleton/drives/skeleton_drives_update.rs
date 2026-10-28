//! `SkeletonDrives::Update` 0x82b54b90..0x82b550f0, on the retained owner.
//! The caller validates actual admission/occurrences and supplies CURRENT target
//! matrices, not constructor physics poses or solver output disguised as animation.
//! Immutable input borrows exclude concurrent native input mutation. Locks cover
//! only immediate payload stores, never service callbacks. Partial writes remain;
//! this is not an atomic batch or native exceptional floating-point parity claim.
#[cfg(test)]
use super::*;
use super::{
    DISPLACEMENT_SCALE, FRESH_VOLUME_TRANSFORM, STRENGTH_SCALE, SimulationDriveDynamicsParams,
    SkeletonBoneDrives, SkeletonDriveNumeric, SkeletonDriveQuaternionSite, SkeletonDriveType,
    SkeletonDrives, SkeletonPartIndex, fmt, skateboard_spawn_compose,
    skateboard_spawn_rigid_inverse, try_simulation_quaternion_candidate,
};
use crate::{
    skeleton_attributes::SkeletonDriveAttributes, skeleton_collision_data::SkeletonCollisionData,
};

#[cfg(test)]
#[path = "skeleton_drives_update_tests.rs"]
mod tests;

pub(crate) const LOCAL_SCALE_KEY: u64 = 0x0a58_9569_f5fa_0e6c;
pub(crate) const ROOT_SCALE_KEY: u64 = 0x13d1_a5a3_e27d_830e;
pub(crate) const NORMAL_FOLLOW_KEY: u64 = 0xca9e_8fa1_ab74_fdd0;
const PARTIAL_MASK: [u8; 24] = [
    1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Ordinary current global attribute lookups and a captured generated-layout selection
/// are distinct source paths. A captured layout remains the same across its four
/// reads; a fresh capture occurs for EACH active normal-follow drive.
pub(crate) trait SkeletonDrivesUpdateAttributeRead {
    type Pointer;
    type Layout;
    type Error;
    fn lookup_global(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn byte(&mut self, pointer: &Self::Pointer) -> Result<u8, Self::Error>;
    fn capture_animation_layout(
        &mut self,
        kind: SkeletonDriveType,
    ) -> Result<Self::Layout, Self::Error>;
    fn animation_float(&mut self, layout: &Self::Layout, offset: usize)
    -> Result<f32, Self::Error>;
}

pub(crate) trait SkeletonDrivesUpdateCvars {
    type Error;
    fn offboard_drive_scalar(&mut self) -> Result<f32, Self::Error>;
    fn soft_damping(&mut self) -> Result<f32, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonDriveUpdatePayload {
    Frames,
    Dynamics,
}

#[derive(Debug)]
pub(crate) enum SkeletonDrivesUpdateError<A, C, N> {
    Attribute(A),
    Cvar(C),
    Numeric(N),
    MissingParent {
        part: usize,
    },
    MissingBonePart {
        part: usize,
    },
    MissingDriveParent {
        part: usize,
        kind: SkeletonDriveType,
    },
    /// Native scratch row0 is unwritten; no default matrix is invented.
    UninitializedInverse {
        part: usize,
        kind: SkeletonDriveType,
        parent: usize,
    },
    /// None identifies Hook0, where kind is Local solely as a diagnostic label.
    Poisoned {
        part: Option<usize>,
        kind: SkeletonDriveType,
        payload: SkeletonDriveUpdatePayload,
    },
}
impl<A: fmt::Display, C: fmt::Display, N: fmt::Display> fmt::Display
    for SkeletonDrivesUpdateError<A, C, N>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "drive update attribute: {e}"),
            Self::Cvar(e) => write!(f, "drive update cvar: {e}"),
            Self::Numeric(e) => write!(f, "drive update quaternion: {e}"),
            Self::MissingParent { part } => {
                write!(f, "drive update Part {part} has no source parent")
            }
            Self::MissingBonePart { part } => {
                write!(f, "drive update row {part} has no retained Part")
            }
            Self::MissingDriveParent { part, kind } => write!(
                f,
                "drive update Part {part} {kind:?} has no retained parent"
            ),
            Self::UninitializedInverse { part, kind, parent } => write!(
                f,
                "drive update Part {part} {kind:?} reads unwritten inverse {parent}"
            ),
            Self::Poisoned {
                part,
                kind,
                payload,
            } => write!(f, "drive update {part:?} {kind:?} {payload:?} is poisoned"),
        }
    }
}
impl<A: std::error::Error + 'static, C: std::error::Error + 'static, N: std::error::Error + 'static>
    std::error::Error for SkeletonDrivesUpdateError<A, C, N>
{
}

type DriveUpdateResult<T, A, C, N> = Result<T, SkeletonDrivesUpdateError<A, C, N>>;

impl SkeletonDrives {
    /// Source flags0/3 only; other `EnableDrive` modes are not reached here.
    /// All parent assertions become checked host stops at their source point,
    /// not preflights that discard already-completed Hook/bone changes.
    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "distinct borrowed source state and lazy services"
    )]
    pub(crate) fn try_update<
        A: SkeletonDrivesUpdateAttributeRead,
        C: SkeletonDrivesUpdateCvars,
        N: SkeletonDriveNumeric,
    >(
        &mut self,
        targets: &[[f32; 16]; 24],
        collision: &SkeletonCollisionData,
        coefficients: &[SkeletonDriveAttributes; 24],
        partial_mask: u8,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
    ) -> DriveUpdateResult<(), A::Error, C::Error, N::Error> {
        use SkeletonDrivesUpdateError as Error;
        let mut inverses = [None; 24];
        for part in 1..24 {
            inverses[part] = Some(skateboard_spawn_rigid_inverse(&targets[part]));
        }
        let pointer = attributes
            .lookup_global(LOCAL_SCALE_KEY)
            .map_err(Error::Attribute)?;
        let local_scale = attributes.float(&pointer).map_err(Error::Attribute)?;
        let pointer = attributes
            .lookup_global(ROOT_SCALE_KEY)
            .map_err(Error::Attribute)?;
        let root_scale = attributes.float(&pointer).map_err(Error::Attribute)?;
        let pointer = attributes
            .lookup_global(NORMAL_FOLLOW_KEY)
            .map_err(Error::Attribute)?;
        let follow = attributes.byte(&pointer).map_err(Error::Attribute)?;
        let selected = if follow != 0 {
            [local_scale, root_scale]
        } else {
            [0.0; 2]
        };
        {
            let mut hook = self.hook_dynamics[0].write().map_err(|_| Error::Poisoned {
                part: None,
                kind: SkeletonDriveType::Local,
                payload: SkeletonDriveUpdatePayload::Dynamics,
            })?;
            hook.linear.damping = 0.0;
            hook.linear.raw_type = 2;
            hook.linear.spring = f32::from_bits(0x4415_ffff);
            hook.linear.strength = f32::from_bits(0x470c_9fff);
        }
        let offboard = if partial_mask != 0 {
            cvars.offboard_drive_scalar().map_err(Error::Cvar)?
        } else {
            1.0
        };
        let strength = collision.drive_strength;
        let soft = strength < 1.0; // Unordered selects normal, matching BO4 !LT.
        let selected_root = selected[1];
        let selected_local = selected[0];
        for part in 1..23 {
            let bone = &mut self.bones[part];
            let (local, root) = if soft && (partial_mask == 0 || PARTIAL_MASK[part] != 0) {
                bone.flags = 3;
                let local_coefficient = coefficients[part].collision_strength;
                let root_coefficient = coefficients[part].collision_strength_root;
                let root = (1.0 - root_coefficient).mul_add(strength, root_coefficient);
                let local = (1.0 - local_coefficient).mul_add(strength, local_coefficient);
                let root = root * offboard;
                let local = local * offboard;
                let root = root * root_scale;
                let local = local * local_scale;
                (local, root)
            } else {
                bone.flags = 0;
                (selected_local, selected_root)
            };
            if self.data.parents()[part].is_none() {
                return Err(Error::MissingParent { part });
            }
            let child = bone.part.ok_or(Error::MissingBonePart { part })?;
            if child.index() == 0 {
                continue;
            }
            let active_local = bone.active[0];
            let child_pose = targets[child.index()];
            let child_inverse = inverses[child.index()].expect("nonzero typed Part has inverse");
            bone.strengths[0] = local;
            bone.strengths[1] = root;
            for kind in [SkeletonDriveType::Local, SkeletonDriveType::LocalRoot] {
                let active = if kind == SkeletonDriveType::Local {
                    active_local
                } else {
                    bone.active[1]
                };
                if !active {
                    continue;
                }
                let parent = bone.parent_parts[kind.index()]
                    .ok_or(Error::MissingDriveParent { part, kind })?;
                let inverse_parent =
                    inverses[parent.index()]
                        .as_ref()
                        .ok_or(Error::UninitializedInverse {
                            part,
                            kind,
                            parent: parent.index(),
                        })?;
                update_frames(
                    child,
                    kind,
                    bone,
                    &child_pose,
                    &child_inverse,
                    inverse_parent,
                    numeric,
                )?;
                update_dynamics(
                    part,
                    kind,
                    bone,
                    if kind == SkeletonDriveType::Local {
                        local
                    } else {
                        root
                    },
                    attributes,
                    cvars,
                )?;
            }
        }
        Ok(())
    }
}

pub(super) fn update_frames<A, C, N: SkeletonDriveNumeric>(
    part: SkeletonPartIndex,
    kind: SkeletonDriveType,
    bone: &SkeletonBoneDrives,
    child: &[f32; 16],
    inverse_child: &[f32; 16],
    inverse_parent: &[f32; 16],
    numeric: &mut N,
) -> Result<(), SkeletonDrivesUpdateError<A, C, N::Error>> {
    use SkeletonDrivesUpdateError as Error;
    let mut anchor = FRESH_VOLUME_TRANSFORM;
    anchor[12..16].copy_from_slice(&child[12..16]);
    let frame_a = skateboard_spawn_compose(inverse_child, &anchor);
    let frame_b = skateboard_spawn_compose(inverse_parent, &anchor);
    let quaternion = try_simulation_quaternion_candidate(frame_a, |values| {
        numeric.quaternion_estimate(
            SkeletonDriveQuaternionSite::BoneChild { part, kind },
            values,
        )
    })
    .map_err(Error::Numeric)?;
    {
        let mut frames = bone.frames[kind.index()]
            .write()
            .map_err(|_| Error::Poisoned {
                part: Some(part.index()),
                kind,
                payload: SkeletonDriveUpdatePayload::Frames,
            })?;
        frames.quaternion_a = quaternion;
        frames.position_a.copy_from_slice(&frame_a[12..16]);
    }
    let quaternion = try_simulation_quaternion_candidate(frame_b, |values| {
        numeric.quaternion_estimate(
            SkeletonDriveQuaternionSite::BoneParent { part, kind },
            values,
        )
    })
    .map_err(Error::Numeric)?;
    {
        let mut frames = bone.frames[kind.index()]
            .write()
            .map_err(|_| Error::Poisoned {
                part: Some(part.index()),
                kind,
                payload: SkeletonDriveUpdatePayload::Frames,
            })?;
        frames.quaternion_b = quaternion;
        frames.position_b.copy_from_slice(&frame_b[12..16]);
    }
    Ok(())
}

pub(super) fn update_dynamics<
    A: SkeletonDrivesUpdateAttributeRead,
    C: SkeletonDrivesUpdateCvars,
    N,
>(
    part: usize,
    kind: SkeletonDriveType,
    bone: &SkeletonBoneDrives,
    strength: f32,
    attributes: &mut A,
    cvars: &mut C,
) -> Result<(), SkeletonDrivesUpdateError<A::Error, C::Error, N>> {
    use SkeletonDrivesUpdateError as Error;
    let (angular, linear) = if bone.flags == 3 {
        let angular_strength = bone.soft[1] * strength;
        let spring = bone.soft[0] * strength;
        let strength = angular_strength * STRENGTH_SCALE;
        let damping = cvars.soft_damping().map_err(Error::Cvar)?;
        (
            SimulationDriveDynamicsParams {
                spring,
                damping,
                strength,
                raw_type: 1,
            },
            SimulationDriveDynamicsParams {
                spring: 0.0,
                damping: 0.0,
                strength: 0.0,
                raw_type: 1,
            },
        )
    } else {
        let layout = attributes
            .capture_animation_layout(kind)
            .map_err(Error::Attribute)?;
        let angular_strength = attributes
            .animation_float(&layout, 8)
            .map_err(Error::Attribute)?;
        let angular_displacement = attributes
            .animation_float(&layout, 12)
            .map_err(Error::Attribute)?;
        let angular_strength = angular_strength * STRENGTH_SCALE;
        let linear_displacement = attributes
            .animation_float(&layout, 4)
            .map_err(Error::Attribute)?;
        let angular_spring = angular_displacement * DISPLACEMENT_SCALE;
        let linear_strength = attributes
            .animation_float(&layout, 0)
            .map_err(Error::Attribute)?;
        (
            SimulationDriveDynamicsParams {
                spring: angular_spring,
                damping: 0.0,
                strength: angular_strength,
                raw_type: 2,
            },
            SimulationDriveDynamicsParams {
                spring: linear_displacement * DISPLACEMENT_SCALE,
                damping: 0.0,
                strength: linear_strength * STRENGTH_SCALE,
                raw_type: 2,
            },
        )
    };
    let mut dynamics = bone.dynamics[kind.index()]
        .write()
        .map_err(|_| Error::Poisoned {
            part: Some(part),
            kind,
            payload: SkeletonDriveUpdatePayload::Dynamics,
        })?;
    dynamics.angular.raw_type = angular.raw_type;
    dynamics.angular.spring = angular.spring;
    dynamics.angular.damping = angular.damping;
    dynamics.angular.strength = angular.strength;
    dynamics.linear.raw_type = linear.raw_type;
    dynamics.linear.spring = linear.spring;
    dynamics.linear.damping = linear.damping;
    dynamics.linear.strength = linear.strength;
    Ok(())
}
