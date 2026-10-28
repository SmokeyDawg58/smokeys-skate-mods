//! One live state projection retained by the actual Skeleton assembly owner.
//!
//! Geometry remains the real constructor-owned allocation. Its immutable
//! flags/group are INITIAL snapshots; `volume_headers` is the sole mutable
//! source for live top-level Volume flags/group. In particular, a hat's entry
//! denotes its outer Aggregate, never its two child Volumes. There are no
//! fabricated native Body/State references or second mutable mass snapshots.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "skeleton live state is consumed by normal session orchestration"
    )
)]

use crate::{
    ragdoll::SKELETON_PART_COUNT, skeleton_assembly::SkeletonAssemblyBinding,
    skeleton_body_definition::SkeletonPartGeometry, skeleton_collision_data::SkeletonCollisionData,
    skeleton_culling::SkeletonCullingResource,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;

#[cfg(test)]
#[path = "skeleton_runtime_state_tests.rs"]
pub(crate) mod tests;

/// Logical Part material reference, not an address or a material overwrite.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonPartMaterialSelection {
    Definition,
    Zero,
    Normal,
    RagdollBody,
    RagdollHead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SkeletonVolumeHeader {
    pub(crate) flags: u32,
    pub(crate) group_id: u32,
}

/// Not Clone: the owning session keeps ONE authoritative mutable projection.
/// Construction receives its exact binding and moves already-created collision
/// data. The orchestrator must retain that identity and exclusive mutation.
#[derive(Debug)]
pub(crate) struct SkeletonRuntimeState {
    pub(crate) binding: Arc<SkeletonAssemblyBinding>,
    pub(crate) collision: SkeletonCollisionData,
    pub(crate) assembly_group: u32,
    pub(crate) part_user_data: [u32; SKELETON_PART_COUNT],
    pub(crate) volume_headers: [SkeletonVolumeHeader; SKELETON_PART_COUNT],
    pub(crate) material_selection: [SkeletonPartMaterialSelection; SKELETON_PART_COUNT],
    pub(crate) current_culling: Arc<SkeletonCullingResource>,
    /// PDB Body+1e00 int[24]. Initial storage is not written by the recovered
    /// ctor; None is unknown, not a zero/negative counter or a guessed branch.
    pub(crate) disable_counts: [Option<i32>; SKELETON_PART_COUNT],
    pub(crate) volume_disable_count_active: Option<bool>,
    pub(crate) is_ragdoll: bool,
    /// `SkeletonState` target written by full `Skeleton::Set` at 0x82b4cbf4.
    /// None means this projection has not observed that store, not native zero.
    pub(crate) proxy_volume_target: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    zero_material: AssemblyPartMaterial,
    normal_material: AssemblyPartMaterial,
    pub(crate) ragdoll_material: AssemblyPartMaterial,
    pub(crate) ragdoll_head_material: AssemblyPartMaterial,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonRuntimeStateError {
    UnknownDisableCountActive,
    UnknownDisableCount { part: usize },
}

impl std::fmt::Display for SkeletonRuntimeStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownDisableCountActive => {
                f.write_str("Skeleton volume-disable activity has not been established")
            }
            Self::UnknownDisableCount { part } => write!(
                f,
                "Skeleton Part {part} has an unknown disable counter with divergent collision outcomes"
            ),
        }
    }
}
impl std::error::Error for SkeletonRuntimeStateError {}

impl SkeletonRuntimeState {
    pub(crate) fn new(
        binding: Arc<SkeletonAssemblyBinding>,
        collision: SkeletonCollisionData,
    ) -> Self {
        let volume_headers =
            std::array::from_fn(|part| match &binding.body().parts[part].geometry {
                SkeletonPartGeometry::Primitive(volume) => SkeletonVolumeHeader {
                    flags: volume.flags,
                    group_id: volume.group_id,
                },
                SkeletonPartGeometry::Hat(hat) => SkeletonVolumeHeader {
                    flags: hat.outer_metadata().flags,
                    group_id: hat.outer_metadata().group_id,
                },
            });
        let normal = binding.body().attributes.normal;
        let ragdoll = binding.body().attributes.ragdoll;
        let ragdoll_head = binding.body().attributes.ragdoll_head;
        let normal_material = AssemblyPartMaterial {
            static_friction: normal.static_friction,
            dynamic_friction: normal.dynamic_friction,
            restitution: normal.restitution,
        };
        Self {
            current_culling: Arc::clone(binding.initial_culling()),
            proxy_volume_target: None,
            binding,
            collision,
            // Assembly::Initialize82ae1370 and Part::Initialize82ae903c/9050.
            assembly_group: 0,
            part_user_data: [0; SKELETON_PART_COUNT],
            volume_headers,
            material_selection: [SkeletonPartMaterialSelection::Definition; SKELETON_PART_COUNT],
            disable_counts: [None; SKELETON_PART_COUNT],
            volume_disable_count_active: None,
            // Ctor82b62590 stores zero, before Init82b625c4.
            is_ragdoll: false,
            // Source writes all three positive zeros before selecting root
            // material at82b621e4. This unused storage is not exposed as an
            // initialized native material before that selection occurs.
            zero_material: AssemblyPartMaterial {
                static_friction: 0.0,
                dynamic_friction: 0.0,
                restitution: 0.0,
            },
            normal_material,
            ragdoll_material: AssemblyPartMaterial {
                static_friction: ragdoll.static_friction,
                dynamic_friction: ragdoll.dynamic_friction,
                restitution: ragdoll.restitution,
            },
            ragdoll_head_material: AssemblyPartMaterial {
                static_friction: ragdoll_head.static_friction,
                dynamic_friction: ragdoll_head.dynamic_friction,
                restitution: ragdoll_head.restitution,
            },
        }
    }

    /// Source group then ascending Part-userdata stores. Constructor chooses1;
    /// `SetUpNormal` and `NormalCollision` each independently choose6.
    #[expect(
        clippy::manual_slice_fill,
        reason = "retain explicit ascending source Part stores"
    )]
    pub(crate) fn set_group_and_userdata(&mut self, group: u32) {
        self.assembly_group = group;
        for value in &mut self.part_user_data {
            *value = group;
        }
    }

    pub(crate) fn select_root_zero_material(&mut self) {
        self.zero_material.static_friction = 0.0;
        self.zero_material.dynamic_friction = 0.0;
        self.zero_material.restitution = 0.0;
        self.material_selection[0] = SkeletonPartMaterialSelection::Zero;
    }

    pub(crate) fn select_normal_material(&mut self, part: usize) {
        self.material_selection[part] = SkeletonPartMaterialSelection::Normal;
    }

    /// Read the actual selected source allocation, not the initial template
    /// material copy. Part indices are validated by the owning session.
    pub(crate) fn material(&self, part: usize) -> &AssemblyPartMaterial {
        match self.material_selection[part] {
            SkeletonPartMaterialSelection::Definition => &self.binding.body().parts[part].material,
            SkeletonPartMaterialSelection::Zero => &self.zero_material,
            SkeletonPartMaterialSelection::Normal => &self.normal_material,
            SkeletonPartMaterialSelection::RagdollBody => &self.ragdoll_material,
            SkeletonPartMaterialSelection::RagdollHead => &self.ragdoll_head_material,
        }
    }

    pub(crate) fn select_normal_culling(&mut self) {
        self.current_culling = Arc::clone(self.binding.initial_culling());
    }

    /// `NormalCollision` (0x82b50400). Completed prefix survives a host refusal.
    /// Unknown counters may join only if BOTH native branches have the same
    /// final header (already enabled/group0). That preserves None and does not
    /// claim a native counter value, read trace, or conditional-store trace.
    /// The source branch-only folded `OnEnd` is a verified BLR with no effects.
    pub(crate) fn try_set_up_normal_collision(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        self.set_group_and_userdata(6);
        self.volume_headers[0].group_id = 4;
        self.volume_headers[0].flags &= !1;
        self.disable_counts[0] = Some(0);
        for part in 1..SKELETON_PART_COUNT {
            self.try_enable_collision_part(part)?;
            // Repeated unconditionally after the optional Volume changes.
            self.part_user_data[part] = 6;
        }
        Ok(())
    }

    /// `SetUpFullCollision`82b47780: root header BEFORE group/Part userdata,
    /// then enable Parts1..23. There is no repeated per-Part userdata suffix.
    pub(crate) fn try_set_up_full_collision(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        self.volume_headers[0].group_id = 4;
        self.volume_headers[0].flags &= !1;
        self.disable_counts[0] = Some(0);
        self.set_group_and_userdata(6);
        for part in 1..SKELETON_PART_COUNT {
            self.try_enable_collision_part(part)?;
        }
        Ok(())
    }

    /// `SetUpRagdollCollision`82b47870: `SetUpFullCollision` without its root
    /// header prefix; group/Part userdata7, then enable Parts1..23.
    pub(crate) fn try_set_up_ragdoll_collision(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        self.set_group_and_userdata(7);
        for part in 1..SKELETON_PART_COUNT {
            self.try_enable_collision_part(part)?;
        }
        Ok(())
    }

    /// `PartCollideWithLightDMOOnly`82b46328 (0x11), `SetUpNormalCollision(Part)`
    /// 82b47940 (6) and `PartCollideWithLWOnly`82b463b8 (0x12): the shared
    /// conditional enable, then only that Part's userdata; no group store.
    pub(crate) fn try_part_collide_with(
        &mut self,
        part: usize,
        user_data: u32,
    ) -> Result<(), SkeletonRuntimeStateError> {
        self.try_enable_collision_part(part)?;
        self.part_user_data[part] = user_data;
        Ok(())
    }

    /// Body `CollideWithWorldOnly` 0x82b4fa80 first executes all of
    /// `EnableSkeletonCollision` 0x82b476f0, then publishes group/userdata3.
    /// Root volume/count and proxy state belong to the later outer suffix.
    pub(crate) fn try_collide_with_world_only(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        for part in 1..SKELETON_PART_COUNT {
            self.try_enable_collision_part(part)?;
        }
        self.set_group_and_userdata(3);
        Ok(())
    }

    /// Outer Skeleton ground/world-only suffix, 0x82b54410/14 and543b0/b4.
    /// The Volume's group and every non-root field remain unchanged.
    pub(crate) fn disable_root_collision(&mut self) {
        self.volume_headers[0].flags &= !1;
        self.disable_counts[0] = Some(0);
    }

    /// Shared source conditional operation. Unknown-count joins retain the
    /// existing bounded semantics: no claim of a known counter or read trace.
    fn try_enable_collision_part(&mut self, part: usize) -> Result<(), SkeletonRuntimeStateError> {
        let header = &mut self.volume_headers[part];
        match self.disable_counts[part] {
            Some(0) => {
                header.flags |= 1;
                header.group_id = 0;
            }
            Some(_) => {}
            None if header.flags & 1 != 0 && header.group_id == 0 => {}
            None => return Err(SkeletonRuntimeStateError::UnknownDisableCount { part }),
        }
        Ok(())
    }

    /// `NoCollision` (0x82b47640). Does not change Part userdata, Assembly group,
    /// other Volume groups, collision data, or the disable-count-active byte.
    pub(crate) fn set_up_no_collision(&mut self, reset_collision_timers: bool) {
        self.volume_headers[0].group_id = 4;
        self.volume_headers[0].flags &= !1;
        self.disable_counts[0] = Some(0);
        for part in 1..SKELETON_PART_COUNT {
            self.volume_headers[part].flags &= !1;
            if reset_collision_timers {
                self.disable_counts[part] = Some(0);
            }
        }
    }

    /// `CollideWithWorldOnlyOffboard` 82b54324..350. Its preceding
    /// SetUpNoCollision(true) always leaves root count zero, but keep the
    /// actual conditional when this primitive is used on the retained Body.
    pub(crate) fn enable_root_collision_if_ready(&mut self) {
        if self.disable_counts[0] == Some(0) {
            self.volume_headers[0].flags |= 1;
            self.volume_headers[0].group_id = 0;
        }
    }

    pub(crate) fn set_root_user_data(&mut self, value: u32) {
        self.part_user_data[0] = value;
    }

    /// `SkeletonBody::UpdateVolumesStates` 0x82b479d0..0x82b47aa4. Called
    /// after collision update. Positive signed counters decrement ascending,
    /// then update only their actual Volume enabled bit; final activity is
    /// committed after all24 rows. Unknown host storage refuses at its read.
    pub(crate) fn try_update_volume_states(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        match self.volume_disable_count_active {
            Some(false) => return Ok(()),
            None => return Err(SkeletonRuntimeStateError::UnknownDisableCountActive),
            Some(true) => {}
        }
        let mut remains_active = false;
        for part in 0..SKELETON_PART_COUNT {
            let count = self.disable_counts[part]
                .ok_or(SkeletonRuntimeStateError::UnknownDisableCount { part })?;
            if count > 0 {
                let next = count - 1;
                self.disable_counts[part] = Some(next);
                // Both native branch calls resolve to an empty folded OnEnd.
                if next == 0 {
                    self.volume_headers[part].flags |= 1;
                } else {
                    remains_active = true;
                    self.volume_headers[part].flags &= !1;
                }
            }
        }
        self.volume_disable_count_active = Some(remains_active);
        Ok(())
    }

    /// `Skeleton::DisableLeftFootCollisions` 0x82b4f298 (Parts 15..17) and
    /// `DisableRightFootCollisions` 0x82b4f328 (Parts 19..21). Per Part: the
    /// Body active byte, its counter, the folded empty `OnEnd`, then only that
    /// Volume's enabled bit is cleared.
    pub(crate) fn disable_foot_collisions(&mut self, right: bool, frames: i32) {
        let first = if right { 19 } else { 15 };
        for part in first..first + 3 {
            self.disable_part_collision_for_frames(part, frames);
        }
    }

    /// DisableHandCollisions82b4ef00. The reference includes forearms and head,
    /// and preserves every group and flag except the Volume enabled bit.
    pub(crate) fn disable_hand_collisions(&mut self, frames: i32) {
        for part in [7, 3, 8, 4, 1] {
            self.disable_part_collision_for_frames(part, frames);
        }
    }

    /// DisableLowerLegCollisions82b4f3b8, through controller82dd4770.
    pub(crate) fn disable_lower_leg_collisions(&mut self, frames: i32) {
        for part in [19, 15, 20, 16, 17, 21] {
            self.disable_part_collision_for_frames(part, frames);
        }
    }

    fn disable_part_collision_for_frames(&mut self, part: usize, frames: i32) {
        self.volume_disable_count_active = Some(true);
        self.disable_counts[part] = Some(frames);
        // OnEnd at each native site is an empty folded function.
        self.volume_headers[part].flags &= !1;
    }

    /// `Skeleton::DisableFeetCollisions` 0x82b46220, reached through the
    /// SkeletonController thunk 0x82dd4778. Parts 19, 15, 20, 16 in that order;
    /// per Part the folded empty `OnEnd`, Volume group 4, its enabled bit
    /// cleared, then a zero counter. The Body disable-count-active byte is not
    /// written.
    pub(crate) fn disable_feet_collisions(&mut self) {
        for part in [19, 15, 20, 16] {
            self.volume_headers[part].group_id = 4;
            self.volume_headers[part].flags &= !1;
            self.disable_counts[part] = Some(0);
        }
    }

    /// `Skeleton::EnableFeetCollisions` 0x82b4f4b0, reached through the
    /// SkeletonController thunk 0x82dd4780. Parts 19, 15, 20, 16 in that order:
    /// a bone whose `HasCollision` byte is set takes `SkeletonBody::EnableCollision`
    /// 0x82b3c2d8 (only with a zero disable counter: enabled bit set, group
    /// zero), any other takes `DisableCollisionWithWorld` 0x82b3c348.
    pub(crate) fn try_enable_feet_collisions(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        for part in [19, 15, 20, 16] {
            if self.binding.body().attributes.bones[part].has_collision != 0 {
                let count = self.disable_counts[part]
                    .ok_or(SkeletonRuntimeStateError::UnknownDisableCount { part })?;
                if count == 0 {
                    self.volume_headers[part].flags |= 1;
                    self.volume_headers[part].group_id = 0;
                }
            } else {
                self.volume_headers[part].group_id = 4;
                self.volume_headers[part].flags &= !1;
                self.disable_counts[part] = Some(0);
            }
        }
        Ok(())
    }

    /// `TeleportReset` (0x82b3bf88): ragdoll flag, selective `CollisionReset`, then
    /// ascending24 counters, finally disable-count-active. Never enables a
    /// Volume or changes material selection/culling/group/compliance.
    #[expect(
        clippy::manual_slice_fill,
        reason = "retain explicit ascending source counter stores"
    )]
    pub(crate) fn teleport_reset(&mut self) {
        self.is_ragdoll = false;
        self.collision.reset();
        for count in &mut self.disable_counts {
            *count = Some(0);
        }
        self.volume_disable_count_active = Some(false);
    }
}
