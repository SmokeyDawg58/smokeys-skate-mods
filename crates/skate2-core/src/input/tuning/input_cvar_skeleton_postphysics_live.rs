//! Draft cvar-runtime child: live collision/preparation/post-record services.
//! Requires source registrations131..141 documented in the evidence packet.
use super::{InputCvarError, InputCvarReferences, Target, Value};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skeleton_normal::postphysics_preparation::PostPhysicsPreparationCvars,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    skeleton_collision_data::update::{FenceThreshold, SkeletonCollisionCvars},
    skeleton_post_physics_records::SkeletonPostPhysicsRecordServices,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum SkeletonPostPhysicsServiceError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    ReckoningPoisoned,
    ReckoningUnconstructed,
    ProcessedPoisoned,
    MissingProcessedFlags,
}
impl std::fmt::Display for SkeletonPostPhysicsServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => write!(f, "Skeleton post-physics cvar: {error:?}"),
            Self::Overlay(error) => error.fmt(f),
            Self::ReckoningPoisoned => f.write_str("Skeleton post-record Reckoning owner poisoned"),
            Self::ReckoningUnconstructed => {
                f.write_str("Skeleton post-record Reckoning is not constructed")
            }
            Self::ProcessedPoisoned => f.write_str("Skeleton post-record processed owner poisoned"),
            Self::MissingProcessedFlags => {
                f.write_str("Skeleton post-record flags+8c8 are unknown")
            }
        }
    }
}

pub(crate) struct SkeletonPostPhysicsCvars<'a> {
    references: &'a InputCvarReferences,
    overlay: &'a DebugOverlaySlot,
}
impl<'a> SkeletonPostPhysicsCvars<'a> {
    pub(crate) const fn new(
        references: &'a InputCvarReferences,
        overlay: &'a DebugOverlaySlot,
    ) -> Self {
        Self {
            references,
            overlay,
        }
    }
    fn boolean(&self, index: usize) -> Result<u8, SkeletonPostPhysicsServiceError> {
        self.references
            .try_current_boolean(index)
            .map(u8::from)
            .map_err(SkeletonPostPhysicsServiceError::Cvar)
    }
    fn float(&self, index: usize) -> Result<f32, SkeletonPostPhysicsServiceError> {
        self.references
            .try_current_float(index)
            .map_err(SkeletonPostPhysicsServiceError::Cvar)
    }
}
impl PostPhysicsPreparationCvars for SkeletonPostPhysicsCvars<'_> {
    type Error = SkeletonPostPhysicsServiceError;
    fn draw_skateboard_error(&mut self) -> Result<bool, Self::Error> {
        self.boolean(131).map(|value| value != 0)
    }
}
pub(crate) struct FenceCvarTarget(Target);

impl SkeletonCollisionCvars for SkeletonPostPhysicsCvars<'_> {
    type Error = SkeletonPostPhysicsServiceError;
    type FenceTarget = FenceCvarTarget;
    fn capture_fence_threshold(
        &mut self,
        field: FenceThreshold,
    ) -> Result<Self::FenceTarget, Self::Error> {
        let index = match field {
            FenceThreshold::Distance => 286,
            FenceThreshold::OpposingDot => 287,
        };
        let targets = self
            .references
            .targets
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?;
        let target = targets[index]
            .as_ref()
            .ok_or(Self::Error::Cvar(InputCvarError::Unlinked(
                super::DEFINITIONS[index].label,
            )))?;
        Ok(FenceCvarTarget(Arc::clone(target)))
    }
    fn read_fence_threshold(&mut self, target: &Self::FenceTarget) -> Result<f32, Self::Error> {
        match *target
            .0
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?
        {
            Value::Float(value) => Ok(value),
            _ => Err(Self::Error::Cvar(InputCvarError::WrongType)),
        }
    }

    fn draw_body_part_collision(&mut self) -> Result<u8, Self::Error> {
        self.boolean(132)
    }
    fn allow_any_contact(&mut self) -> Result<u8, Self::Error> {
        self.boolean(133)
    }
    fn draw_collision_update(&mut self) -> Result<u8, Self::Error> {
        self.boolean(134)
    }
    fn debug_always_collide(&mut self) -> Result<u8, Self::Error> {
        self.boolean(135)
    }
    fn show_ground_plane(&mut self) -> Result<u8, Self::Error> {
        self.boolean(136)
    }
    fn small_object_force_scalar(&mut self) -> Result<f32, Self::Error> {
        self.float(137)
    }
    fn maximum_ground_distance_offboard(&mut self) -> Result<f32, Self::Error> {
        self.float(138)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(SkeletonPostPhysicsServiceError::Overlay)
    }
}

/// Borrow the SAME physical owner, never a cached normal or fresh Reckoning.
/// Constructors/query methods perform no speculative lock/read. The parent
/// pair scheduler validates this owner against the Skeleton before entry.
pub(crate) struct SkeletonPostRecordServices<'a> {
    cvars: SkeletonPostPhysicsCvars<'a>,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
}
impl<'a> SkeletonPostRecordServices<'a> {
    pub(crate) const fn new(
        references: &'a InputCvarReferences,
        overlay: &'a DebugOverlaySlot,
        owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Self {
        Self {
            cvars: SkeletonPostPhysicsCvars::new(references, overlay),
            owner,
        }
    }
}
impl SkeletonPostPhysicsRecordServices for SkeletonPostRecordServices<'_> {
    type Error = SkeletonPostPhysicsServiceError;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        // Normal hook45940 and proxy hook4d740 capture actual Reckoning, then
        // load its +1f0 ground normal at
        // this callback, after whichever earlier cvar reads the caller reached.
        let retained = self
            .owner
            .reckoning()
            .lock()
            .map_err(|_| Self::Error::ReckoningPoisoned)?;
        Ok(retained
            .reckoning()
            .ok_or(Self::Error::ReckoningUnconstructed)?
            .ground_normal())
    }
    fn use_normals_for_proxy_offset(&mut self) -> Result<u8, Self::Error> {
        self.cvars.boolean(139)
    }
    fn scale_hook_to_hips_y(&mut self) -> Result<f32, Self::Error> {
        self.cvars.float(140)
    }
    fn draw_hook_to_hips(&mut self) -> Result<u8, Self::Error> {
        self.cvars.boolean(141)
    }
}

#[cfg(test)]
#[path = "input_cvar_skeleton_postphysics_live_tests.rs"]
mod tests;
