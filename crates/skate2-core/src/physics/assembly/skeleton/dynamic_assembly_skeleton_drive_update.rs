//! Current controlled animation targets -> SAME admitted drive payloads.
//! `SkeletonDrives::Update` (0x82b54b90) is only a part of `GeneralUpdate`:
//! target adjustments/IK and hook motion are not implemented by this projection.
use super::{
    Arc, AssemblyPartMaterial, SimulationHookAssemblyPool, SimulationSkaterPoolError,
    SkateboardSimulationBackend, SkeletonDriveOwnerError, SkeletonDriveOwnerStage,
    SkeletonDriveRuntime, SkeletonNormalAssembly, fmt,
};
use crate::{
    animation::AnimationMatrix,
    simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame,
    skeleton_animation_targets::{SkeletonAnimationTargetError, SkeletonAnimationTargets},
    skeleton_drive_attributes::{SkeletonDriveAttributeError, SkeletonDriveAttributes},
    skeleton_drives::{
        HostSkeletonDriveNumeric,
        update::{SkeletonDrivesUpdateCvars, SkeletonDrivesUpdateError},
    },
};
use std::convert::Infallible;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonDriveUpdateProgress {
    NotStarted,
    UpdatingTargets,
    UpdatingDrives,
    Complete,
}

#[derive(Debug)]
pub(crate) enum SkeletonDriveFeedbackError<C> {
    Pool(SimulationSkaterPoolError),
    Owner(SkeletonDriveOwnerError),
    MissingAttributes,
    ForeignData,
    CollisionNotCurrent,
    AlreadyAttempted,
    MappedVolumesUnavailable,
    Targets(SkeletonAnimationTargetError),
    Update(SkeletonDrivesUpdateError<SkeletonDriveAttributeError, C, Infallible>),
    RagdollUpdate(
        crate::skeleton_drives::ragdoll_update::RagdollDriveUpdateError<
            SkeletonDriveAttributeError,
            C,
            Infallible,
            crate::reckoning_numeric::ReckoningHostNumericError,
        >,
    ),
}
impl<C: fmt::Debug> fmt::Display for SkeletonDriveFeedbackError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "drive feedback pool: {e}"),
            Self::Owner(e) => write!(f, "drive feedback owner: {e}"),
            Self::MissingAttributes => {
                f.write_str("original retained drive attribute provider is absent")
            }
            Self::ForeignData => f.write_str("drive and body SkeletonData owners differ"),
            Self::CollisionNotCurrent => {
                f.write_str("drive feedback requires current completed collision response")
            }
            Self::AlreadyAttempted => {
                f.write_str("drive feedback already attempted for this collision frame")
            }
            Self::MappedVolumesUnavailable => {
                f.write_str("drive update requires completed mapped volumes")
            }
            Self::Targets(e) => write!(
                f,
                "current animation Part {} needs bone {}, pose has {} rows",
                e.part, e.bone, e.pose_count
            ),
            Self::RagdollUpdate(e) => write!(f, "ragdoll drive update: {e:?}"),
            Self::Update(e) => write!(f, "drive feedback source update: {e:?}"),
        }
    }
}
impl<C: fmt::Debug> std::error::Error for SkeletonDriveFeedbackError<C> {}

impl SkeletonDriveRuntime {
    pub(crate) const fn update_progress(&self) -> SkeletonDriveUpdateProgress {
        self.update_progress
    }
}

impl SkeletonNormalAssembly {
    /// `GeneralUpdate` 0x82b5bb70..0x82b5bb7c consumes existing post-IK rows.
    /// This entry neither remaps the raw pose nor uses contact-frame identity
    /// as an execution guard. Its caller owns native `GeneralUpdate` progression.
    pub(crate) fn try_update_drives_from_retained_volumes<C: SkeletonDrivesUpdateCvars, H>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        hooks: &SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        cvars: &mut C,
    ) -> Result<(), SkeletonDriveFeedbackError<C::Error>> {
        use SkeletonDriveFeedbackError as E;
        self.check_drive_backend(backend).map_err(E::Owner)?;
        let runtime = self
            .drives
            .as_mut()
            .ok_or(E::Owner(SkeletonDriveOwnerError::Missing))?;
        if runtime.stage != SkeletonDriveOwnerStage::Complete {
            return Err(E::Owner(SkeletonDriveOwnerError::Incomplete));
        }
        if !Arc::ptr_eq(runtime.prepared.data(), &self.state.binding.body().data) {
            return Err(E::ForeignData);
        }
        if runtime.update_attributes.is_none() {
            return Err(E::MissingAttributes);
        }
        let targets = self
            .animation_targets
            .completed_volumes()
            .ok_or(E::MappedVolumesUnavailable)?;
        validate_admitted_payloads(runtime, &self.session, backend, hooks)?;
        if std::env::var_os("SKATE2_TRACE_TOE_TARGETS").is_some() {
            let at = |part: usize| [targets[part][12], targets[part][13], targets[part][14]];
            let roots = &self.root_transforms;
            let world = roots.animation_to_world().columns();
            let to_world = |p: [f32; 3]| -> [f32; 3] {
                std::array::from_fn(|axis| {
                    world[0].xyz[axis] * p[0]
                        + world[1].xyz[axis] * p[1]
                        + world[2].xyz[axis] * p[2]
                        + world[3].xyz[axis]
                })
            };
            let board = roots.board().columns();
            eprintln!(
                "toe targets: volume0={:?} volume0_z={:?} part15={:?} part16={:?} part19={:?} part20={:?} world15={:?} world19={:?} board_p={:?} board_z={:?} predicted={:?} anim_to_board_p={:?} anim_to_world_p={:?}",
                at(0),
                [targets[0][8], targets[0][9], targets[0][10]],
                at(15),
                at(16),
                at(19),
                at(20),
                to_world(at(15)),
                to_world(at(19)),
                board[3].xyz,
                board[2].xyz,
                roots.predicted_board_position().xyz,
                roots.animation_to_board().columns()[3].xyz,
                world[3].xyz
            );
        }
        let partial_mask = u8::from(self.post_physics_records.proxy_enabled());
        runtime.update_progress = SkeletonDriveUpdateProgress::UpdatingDrives;
        runtime
            .prepared
            .try_update(
                &targets,
                &self.state.collision,
                &self.state.binding.body().attributes.drives,
                partial_mask,
                runtime
                    .update_attributes
                    .as_mut()
                    .expect("retained provider checked"),
                cvars,
                &mut HostSkeletonDriveNumeric,
            )
            .map_err(E::Update)?;
        runtime.update_progress = SkeletonDriveUpdateProgress::Complete;
        Ok(())
    }

    pub(crate) fn animation_targets(&self) -> &SkeletonAnimationTargets {
        &self.animation_targets
    }

    pub(crate) fn try_retain_drive_update_attributes(
        &mut self,
        backend: &SkateboardSimulationBackend,
        attributes: &mut Option<SkeletonDriveAttributes>,
    ) -> Result<(), SkeletonDriveOwnerError> {
        self.check_drive_backend(backend)?;
        let runtime = self
            .drives
            .as_mut()
            .ok_or(SkeletonDriveOwnerError::Missing)?;
        if runtime.stage != SkeletonDriveOwnerStage::Complete || attributes.is_none() {
            return Err(SkeletonDriveOwnerError::Incomplete);
        }
        if runtime.update_attributes.is_some() {
            return Err(SkeletonDriveOwnerError::AlreadyCreated);
        }
        // Move the actual provider AFTER constructor selection, never rebind it.
        runtime.update_attributes = attributes.take();
        Ok(())
    }

    pub(crate) fn try_update_drives_from_controlled_pose<C: SkeletonDrivesUpdateCvars, H>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        hooks: &SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        frame: &Arc<SimulationContactSpyFrame>,
        pose: &[AnimationMatrix],
        partial_mask: u8,
        cvars: &mut C,
    ) -> Result<(), SkeletonDriveFeedbackError<C::Error>> {
        use SkeletonDriveFeedbackError as Error;
        self.check_drive_backend(backend).map_err(Error::Owner)?;
        let runtime = self
            .drives
            .as_mut()
            .ok_or(Error::Owner(SkeletonDriveOwnerError::Missing))?;
        if runtime.stage != SkeletonDriveOwnerStage::Complete {
            return Err(Error::Owner(SkeletonDriveOwnerError::Incomplete));
        }
        if !Arc::ptr_eq(runtime.prepared.data(), &self.state.binding.body().data) {
            return Err(Error::ForeignData);
        }
        if runtime.update_attributes.is_none() {
            return Err(Error::MissingAttributes);
        }
        if self.collision_frame.progress()
            != super::super::collision_owner::SkeletonCollisionFrameProgress::Complete
            || self
                .collision_frame
                .frame()
                .is_none_or(|actual| !Arc::ptr_eq(actual, frame))
        {
            return Err(Error::CollisionNotCurrent);
        }
        if runtime
            .update_frame
            .as_ref()
            .is_some_and(|actual| Arc::ptr_eq(actual, frame))
        {
            return Err(Error::AlreadyAttempted);
        }
        validate_admitted_payloads(runtime, &self.session, backend, hooks)?;
        runtime.update_frame = Some(Arc::clone(frame));
        runtime.update_progress = SkeletonDriveUpdateProgress::UpdatingTargets;
        self.current_animation.invalidate_mapped_pose();
        self.animation_targets
            .try_map_current_pose(&self.state.binding.body().data, pose)
            .map_err(Error::Targets)?;
        let targets = self
            .animation_targets
            .completed_volumes()
            .expect("successful source mapping");
        runtime.update_progress = SkeletonDriveUpdateProgress::UpdatingDrives;
        runtime
            .prepared
            .try_update(
                &targets,
                &self.state.collision,
                &self.state.binding.body().attributes.drives,
                partial_mask,
                runtime
                    .update_attributes
                    .as_mut()
                    .expect("retained provider checked"),
                cvars,
                &mut HostSkeletonDriveNumeric,
            )
            .map_err(Error::Update)?;
        runtime.update_progress = SkeletonDriveUpdateProgress::Complete;
        Ok(())
    }
}

fn validate_admitted_payloads<C, H>(
    runtime: &SkeletonDriveRuntime,
    session: &crate::skeleton_assembly::SkeletonPreNormalAssemblySession,
    backend: &SkateboardSimulationBackend,
    hooks: &SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
) -> Result<(), SkeletonDriveFeedbackError<C>> {
    use SkeletonDriveFeedbackError as Error;
    let hook = runtime
        .hook
        .as_ref()
        .ok_or(Error::Owner(SkeletonDriveOwnerError::Incomplete))?;
    let hook_session = hooks
        .skeleton_hook_session(hook)
        .map_err(|e| Error::Owner(SkeletonDriveOwnerError::Hook(e)))?;
    for (index, child) in [23, 0].into_iter().enumerate() {
        // Native hook Add may be null; validate a live occurrence when one
        // exists, while still updating the original retained null payload.
        hook_session
            .try_validate_cross_attachment_drive(
                backend,
                index,
                index,
                session,
                child,
                runtime.prepared.hook_frames(index),
                runtime.prepared.hook_dynamics(index),
            )
            .map_err(|e| Error::Owner(SkeletonDriveOwnerError::Drive(e)))?;
    }
    for emission in runtime.prepared.eligible_bone_drives() {
        let ordinal = runtime.bone_entries[emission.part.index()][emission.kind.index()]
            .ok_or(Error::Owner(SkeletonDriveOwnerError::Incomplete))?;
        if !session
            .try_validate_post_attachment_drive(
                backend,
                ordinal,
                emission.parent.index(),
                emission.part.index(),
                emission.frames,
                emission.dynamics,
            )
            .map_err(|e| Error::Owner(SkeletonDriveOwnerError::Drive(e)))?
        {
            return Err(Error::Owner(SkeletonDriveOwnerError::Incomplete));
        }
    }
    Ok(())
}

#[path = "dynamic_assembly_skeleton_ragdoll_drive_update.rs"]
pub(crate) mod ragdoll_update;
