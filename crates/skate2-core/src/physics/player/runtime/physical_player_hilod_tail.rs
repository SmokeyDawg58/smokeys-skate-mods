//! Retained `HiLOD` ctor82e03c50 tail fields, before/after initial Ground entry.
//! The enclosing actual-pair caller owns scheduling, collision mutations and
//! final World.Add/JPhy/Actor publication; these records do not certify return.

/// Typed capsule member data from `CapsuleVolume::Initialize`82ad7fa0.
/// Spare union words +48/+4c are not written by this constructor; a serialized
/// Volume with invented zero union words would lose that distinction.
pub(crate) struct IntersectionCapsule {
    pub(crate) transform: [f32; 16],
    pub(crate) radius: f32,
    pub(crate) half_height: f32,
    pub(crate) group_id: u32,
    pub(crate) surface_id: u32,
    pub(crate) flags: u32,
}
impl IntersectionCapsule {
    /// `CreateCollisionCapsule`82e4afa0 resource[0]: size96, alignment16;
    /// other four resource descriptors size0/alignment1. `HiLOD4c60` supplies
    /// radius1 and half-height1 using the `LivingWorld` resource allocator.
    pub(crate) fn initialize() -> Self {
        Self {
            transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            radius: 1.0,
            half_height: 1.0,
            group_id: 0,
            surface_id: 0,
            flags: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonLineResult {
    pub(crate) position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) normal: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) surface_id: i32,
    pub(crate) valid: bool,
}
pub(crate) struct SkeletonLineRecord {
    pub(crate) result: SkeletonLineResult,
    pub(crate) start_position: crate::skeleton_root_transforms::SkeletonRootVector,
    /// No source request has acquired a batch during construction.
    pub(crate) trajectory_batch: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::trajectory::SimulationWorldTrajectoryBatch>,
}

/// Only the members in this source suffix. `mCollideWithWorldOnly` starts from
/// the actual zero-cleared Sim allocation, distinct from `PhysicsSkater` objects.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source fields and host completion guards"
)]
pub(crate) struct PhysicalHiLodTail {
    pre_ground_attempted: bool,
    pub(crate) intersection_capsule: Option<Box<IntersectionCapsule>>,
    pub(crate) line_records: Option<[SkeletonLineRecord; 3]>,
    pub(crate) intended_jump_velocity: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) collide_with_world_only: bool,
    pub(crate) world_added: bool,
    pub(crate) returned: bool,
}
impl PhysicalHiLodTail {
    /// First reset at82e03e40..3e98. Start positions and batch pointers still
    /// contain their actual Sim-allocation zeros until the explicit late reset.
    pub(crate) fn initialize_early_line_results(&mut self) {
        self.line_records = Some(std::array::from_fn(|_| SkeletonLineRecord {
            result: SkeletonLineResult {
                position: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
                normal: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
                surface_id: 0,
                valid: false,
            },
            start_position: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            trajectory_batch: None,
        }));
    }
    /// TeleportPlayerNow 0x82e008c4..938 clears three retained line records
    /// in place. The native loop leaves any trajectory-batch pointer alone.
    pub(crate) fn reset_teleport_line_records(&mut self) -> Result<(), HiLodTailError> {
        let records = self
            .line_records
            .as_mut()
            .ok_or(HiLodTailError::MissingLineRecords)?;
        let zero = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        for record in records {
            record.result.surface_id = 0;
            record.result.valid = false;
            record.result.position = zero;
            record.result.normal = zero;
            record.start_position = zero;
        }
        Ok(())
    }
    pub(crate) fn from_sim_zeroed_allocation() -> Self {
        Self {
            pre_ground_attempted: false,
            intersection_capsule: None,
            line_records: None,
            intended_jump_velocity: None,
            collide_with_world_only: false,
            world_added: false,
            returned: false,
        }
    }
    /// Schedule after all26 state allocation publications through82e04c1c.
    /// Resource null produces null capsule and STILL initializes the records;
    /// it is not a null batch or fabricated successful allocation.
    pub(crate) fn try_initialize_pre_ground(
        &mut self,
        capsule_resource_allocated: bool,
    ) -> Result<(), HiLodTailError> {
        if self.pre_ground_attempted {
            return Err(HiLodTailError::PreGroundAlreadyAttempted);
        }
        self.pre_ground_attempted = true;
        self.intersection_capsule =
            capsule_resource_allocated.then(|| Box::new(IntersectionCapsule::initialize()));
        self.line_records = Some(std::array::from_fn(|_| SkeletonLineRecord {
            result: SkeletonLineResult {
                position: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
                normal: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
                surface_id: 0,
                valid: false,
            },
            start_position: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            trajectory_batch: None,
        }));
        Ok(())
    }
    /// Caller first performs source Wipeout+ac=0.04 store82e04d24 after actual
    /// `SetPhysicsState100` returns. This is the subsequent `HiLOD`+110 store4d28.
    pub(crate) fn initialize_post_ground_jump_velocity(&mut self) {
        self.intended_jump_velocity = Some(
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
        );
    }
    /// Source4d54..4d98 changes its flag BEFORE board and controller calls.
    /// None is the optional NULL query, which selects normal collision exactly
    /// like a present query returning true. Return value identifies needed calls.
    pub(crate) fn select_network_collision(
        &mut self,
        network_query: Option<bool>,
    ) -> Option<HiLodCollisionChange> {
        let world_only = network_query == Some(false);
        if self.collide_with_world_only == world_only {
            return None;
        }
        self.collide_with_world_only = world_only;
        Some(if world_only {
            HiLodCollisionChange::WorldOnly
        } else {
            HiLodCollisionChange::Normal
        })
    }
    /// UpdateCollisionStatus82e01b9c..1bf0 makes the same flag write before
    /// its board and SkeletonController calls, and only for a changed flag.
    pub(crate) fn select_collision_status(
        &mut self,
        world_only: bool,
    ) -> Option<HiLodCollisionChange> {
        if self.collide_with_world_only == world_only {
            return None;
        }
        self.collide_with_world_only = world_only;
        Some(if world_only {
            HiLodCollisionChange::WorldOnly
        } else {
            HiLodCollisionChange::Normal
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HiLodCollisionChange {
    WorldOnly,
    Normal,
}
#[derive(Debug)]
pub(crate) enum HiLodTailError {
    PreGroundAlreadyAttempted,
    MissingLineRecords,
}
impl std::fmt::Display for HiLodTailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HiLOD constructor tail: {self:?}")
    }
}
