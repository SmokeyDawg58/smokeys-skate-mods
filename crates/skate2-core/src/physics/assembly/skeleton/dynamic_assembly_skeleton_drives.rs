//! Actual `SkeletonDrives` admission, full Set and ordered release.
//! Sources: 0x82b59b08, 0x82b50868, 0x82b4c940, 0x82b47ce0/0x82b50720.
//! Host construction stages preserve partial ownership; no implicit Drop cleanup.
use super::{SkeletonNormalAssembly, SkeletonPlacementError};
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyPostDriveError,
        hook_pool::{
            SimulationHookAssemblyCandidate, SimulationHookAssemblyHandle,
            SimulationHookAssemblyPool, SimulationHookPoolError,
        },
        skater_pool::SimulationSkaterPoolError,
    },
    ragdoll::SkeletonPartIndex,
    simulation_contact_jacobian::network_preparation::SimulationManagerContactDomain,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_wheels::{FRESH_VOLUME_TRANSFORM, SkateboardWheelMassNumeric},
    skeleton_drives::{
        SkeletonDriveAttributeRead, SkeletonDriveCvars, SkeletonDriveNumeric, SkeletonDriveType,
        SkeletonDrives, SkeletonDrivesError, SkeletonHookDefinition,
    },
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{fmt, sync::Arc};

#[cfg(test)]
#[path = "dynamic_assembly_skeleton_drives_tests.rs"]
pub(crate) mod tests;

#[path = "dynamic_assembly_skeleton_drive_update.rs"]
pub(crate) mod update_owner;

#[path = "dynamic_assembly_skeleton_hook_soft.rs"]
pub(crate) mod hook_soft;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonDriveOwnerStage {
    HookAdmission,
    HookDrive(usize),
    BoneDrive {
        part: usize,
        kind: SkeletonDriveType,
    },
    Complete,
    ReleaseHooks,
    ReleaseBones,
}

pub(crate) struct SkeletonDriveRuntime {
    prepared: SkeletonDrives,
    pending_hook:
        Option<SimulationHookAssemblyCandidate<AssemblyPartMaterial, SkeletonHookDefinition>>,
    hook: Option<SimulationHookAssemblyHandle>,
    // Stable host ledger ordinals reference entries actually retained by this
    // session, not fabricated native Drive or Body identities.
    bone_entries: [[Option<usize>; 2]; 24],
    stage: SkeletonDriveOwnerStage,
    update_attributes: Option<crate::skeleton_drive_attributes::SkeletonDriveAttributes>,
    update_frame:
        Option<Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>>,
    update_progress: update_owner::SkeletonDriveUpdateProgress,
}
impl SkeletonDriveRuntime {
    pub(crate) fn global_animation_instance(
        &self,
    ) -> Option<&crate::attribute_runtime::AttributeRuntimeInstance> {
        self.update_attributes.as_ref().map(
            crate::skeleton_drive_attributes::SkeletonDriveAttributes::global_animation_instance,
        )
    }
    pub(crate) fn prepared(&self) -> &SkeletonDrives {
        &self.prepared
    }
    pub(crate) fn hook(&self) -> Option<&SimulationHookAssemblyHandle> {
        self.hook.as_ref()
    }
    pub(crate) const fn stage(&self) -> SkeletonDriveOwnerStage {
        self.stage
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonDriveOwnerError {
    Detached,
    ForeignBackend,
    AlreadyCreated,
    Missing,
    Incomplete,
    Hook(SimulationHookPoolError),
    Drive(DynamicAssemblyPostDriveError),
    Placement(SkeletonPlacementError),
}
impl fmt::Display for SkeletonDriveOwnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Detached => f.write_str("skeleton drives require an attached owner"),
            Self::ForeignBackend => f.write_str("skeleton drive backend mismatch"),
            Self::AlreadyCreated => {
                f.write_str("skeleton drive construction already attempted; release first")
            }
            Self::Missing => f.write_str("skeleton drive owner is absent"),
            Self::Incomplete => f.write_str("skeleton drive admission is incomplete"),
            Self::Hook(e) => write!(f, "skeleton hook: {e}"),
            Self::Drive(e) => write!(f, "skeleton bone drive: {e}"),
            Self::Placement(e) => write!(f, "skeleton placement: {e}"),
        }
    }
}
impl std::error::Error for SkeletonDriveOwnerError {}

#[derive(Debug)]
pub(crate) enum SkeletonDriveConstructionError<A, N, V> {
    Pool(SimulationSkaterPoolError),
    Owner(SkeletonDriveOwnerError),
    Preparation(SkeletonDrivesError<A, N, V>),
}
impl<A: fmt::Debug, N: fmt::Debug, V: fmt::Debug> fmt::Display
    for SkeletonDriveConstructionError<A, N, V>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "skeleton drive pool: {e}"),
            Self::Owner(e) => write!(f, "skeleton drive owner: {e}"),
            Self::Preparation(e) => write!(f, "skeleton drive preparation: {e:?}"),
        }
    }
}
impl<A: fmt::Debug, N: fmt::Debug, V: fmt::Debug> std::error::Error
    for SkeletonDriveConstructionError<A, N, V>
{
}

impl SkeletonNormalAssembly {
    pub(crate) fn drive_owner(&self) -> Option<&SkeletonDriveRuntime> {
        self.drives.as_ref()
    }

    fn check_drive_backend(
        &self,
        backend: &SkateboardSimulationBackend,
    ) -> Result<(), SkeletonDriveOwnerError> {
        match self.session.attached_authority {
            None => Err(SkeletonDriveOwnerError::Detached),
            Some(authority) if authority != backend.authority() => {
                Err(SkeletonDriveOwnerError::ForeignBackend)
            }
            Some(_) => Ok(()),
        }
    }

    /// Fresh payload preparation then actual source hook/bone admission. Host
    /// errors before actual admission publish no runtime; after it starts the
    /// whole partial owner remains installed and requires explicit release.
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "explicit owner authorities and independent source-service error types"
    )]
    pub(crate) fn try_initialize_drives<
        A: SkeletonDriveAttributeRead,
        N: SkeletonDriveNumeric,
        V: SkeletonDriveCvars,
        H,
    >(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        manager: &mut SimulationManagerContactDomain,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        attributes: &mut A,
        numeric: &mut N,
        mass: &mut impl SkateboardWheelMassNumeric,
        cvars: &mut V,
    ) -> Result<(), SkeletonDriveConstructionError<A::Error, N::Error, V::Error>> {
        use SkeletonDriveConstructionError as Error;
        self.check_drive_backend(backend).map_err(Error::Owner)?;
        hooks
            .check_authorities(manager.assembly_pool_authority(), backend.authority())
            .map_err(|e| Error::Owner(SkeletonDriveOwnerError::Hook(e)))?;
        if self.drives.is_some() {
            return Err(Error::Owner(SkeletonDriveOwnerError::AlreadyCreated));
        }
        let mut prepared = SkeletonDrives::try_create(
            Arc::clone(&self.state.binding.body().data),
            attributes,
            numeric,
            mass,
            cvars,
        )
        .map_err(Error::Preparation)?;
        let candidate = SimulationHookAssemblyCandidate::try_new(
            prepared
                .take_hook_session()
                .expect("fresh source hook session"),
        )
        .map_err(|e| Error::Owner(SkeletonDriveOwnerError::Hook(e.error)))?;
        self.drives = Some(SkeletonDriveRuntime {
            prepared,
            pending_hook: Some(candidate),
            hook: None,
            bone_entries: [[None; 2]; 24],
            stage: SkeletonDriveOwnerStage::HookAdmission,
            update_attributes: None,
            update_frame: None,
            update_progress: update_owner::SkeletonDriveUpdateProgress::NotStarted,
        });
        let runtime = self
            .drives
            .as_mut()
            .expect("runtime installed before admission");
        let candidate = runtime
            .pending_hook
            .take()
            .expect("fresh candidate retained");
        let hook = match hooks.try_insert_skeleton_hook(backend, manager, candidate) {
            Ok(hook) => hook,
            Err(failure) => {
                runtime.pending_hook = Some(failure.candidate);
                return Err(Error::Owner(SkeletonDriveOwnerError::Hook(failure.error)));
            }
        };
        runtime.hook = Some(hook.clone());
        for (index, child) in [23, 0].into_iter().enumerate() {
            runtime.stage = SkeletonDriveOwnerStage::HookDrive(index);
            // Source stores nullable hook results and proceeds, with spy0.
            hooks
                .try_connect_skeleton(
                    backend,
                    &hook,
                    index,
                    &mut self.session,
                    child,
                    runtime.prepared.hook_frames(index),
                    runtime.prepared.hook_dynamics(index),
                )
                .map_err(|e| Error::Owner(SkeletonDriveOwnerError::Hook(e)))?;
        }
        for emission in runtime.prepared.eligible_bone_drives() {
            let part = emission.part.index();
            runtime.stage = SkeletonDriveOwnerStage::BoneDrive {
                part,
                kind: emission.kind,
            };
            let ordinal = self.session.post_attachment_drives.len();
            self.session
                .try_add_post_attachment_drive(
                    backend,
                    emission.parent.index(),
                    part,
                    emission.frames,
                    emission.dynamics,
                )
                .map_err(|e| Error::Owner(SkeletonDriveOwnerError::Drive(e)))?;
            // Retain BEFORE the unconditional native spy write: even a recorded
            // null or later checked failure leaves a recoverable actual prefix.
            runtime.bone_entries[part][emission.kind.index()] = Some(ordinal);
            self.session
                .try_set_post_attachment_drive_spy(backend, ordinal, 1)
                .map_err(|e| Error::Owner(SkeletonDriveOwnerError::Drive(e)))?;
        }
        runtime.stage = SkeletonDriveOwnerStage::Complete;
        Ok(())
    }

    /// Complete retained-constructor-pose `Skeleton::Set`: Parts0..23, Hips/Root
    /// volume matrices, root-relative hook0 then1, then full4 proxy target.
    pub(crate) fn try_set_retained_pose_with_hooks<H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
    ) -> Result<(), SkeletonDriveOwnerError> {
        self.check_drive_backend(backend)?;
        let runtime = self
            .drives
            .as_ref()
            .ok_or(SkeletonDriveOwnerError::Missing)?;
        if runtime.stage != SkeletonDriveOwnerStage::Complete {
            return Err(SkeletonDriveOwnerError::Incomplete);
        }
        let hook = runtime
            .hook
            .clone()
            .ok_or(SkeletonDriveOwnerError::Incomplete)?;
        // Checked typed owner preflight, not a substitute for the source writes.
        hooks
            .skeleton_hook_session(&hook)
            .map_err(SkeletonDriveOwnerError::Hook)?;
        self.try_place_parts_from_retained_physics_pose(backend)
            .map_err(SkeletonDriveOwnerError::Placement)?;
        let data = &self.state.binding.body().data;
        let volume_matrix = |part: usize| {
            crate::world::orthonormalize_collision_affine(
                crate::skateboard_spawn_math::skateboard_spawn_compose(
                    &data.physics_pose()[part],
                    &data.bone_to_volume()[part],
                ),
            )
        };
        let hips = volume_matrix(23);
        let root = volume_matrix(0);
        let mut outer = FRESH_VOLUME_TRANSFORM;
        for lane in 0..4 {
            outer[12 + lane] = 0.0 - root[12 + lane];
        }
        let hips_hook = crate::skateboard_spawn_math::skateboard_spawn_compose(&outer, &hips);
        let root_hook = crate::skateboard_spawn_math::skateboard_spawn_compose(&outer, &root);
        hooks
            .try_set_skeleton_hook_part(backend, &hook, 0, hips_hook)
            .map_err(SkeletonDriveOwnerError::Hook)?;
        hooks
            .try_set_skeleton_hook_part(backend, &hook, 1, root_hook)
            .map_err(SkeletonDriveOwnerError::Hook)?;
        self.state.proxy_volume_target =
            Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
                root_hook[12..16]
                    .try_into()
                    .expect("four translation lanes"),
            ));
        self.retained_pose_set = true;
        Ok(())
    }

    /// Source Release followed by `BoneDrives` destructors23..0/local thenroot.
    /// Explicit cleanup also recovers partial host construction; never on Drop.
    pub(crate) fn try_release_drives<H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
    ) -> Result<(), SkeletonDriveOwnerError> {
        self.check_drive_backend(backend)?;
        let runtime = self
            .drives
            .as_mut()
            .ok_or(SkeletonDriveOwnerError::Missing)?;
        if let Some(hook) = &runtime.hook {
            hooks
                .skeleton_hook_session(hook)
                .map_err(SkeletonDriveOwnerError::Hook)?;
        }
        runtime.stage = SkeletonDriveOwnerStage::ReleaseHooks;
        self.retained_pose_set = false;
        if let Some(hook) = &runtime.hook {
            hooks
                .try_remove_skeleton_hook_drives(backend, hook)
                .map_err(SkeletonDriveOwnerError::Hook)?;
            let removed = hooks
                .try_remove_skeleton_hook(backend, hook)
                .map_err(SkeletonDriveOwnerError::Hook)?;
            runtime.hook = None;
            drop(removed);
        }
        // Pending candidates are detached by the existing failed Add transaction.
        // Drop their actual session before releasing the retained hook definition.
        drop(runtime.pending_hook.take());
        runtime.prepared.release_hook_definition();
        runtime.stage = SkeletonDriveOwnerStage::ReleaseBones;
        for part in (0..24).rev() {
            for kind in [SkeletonDriveType::Local, SkeletonDriveType::LocalRoot] {
                if let Some(ordinal) = runtime.bone_entries[part][kind.index()] {
                    self.session
                        .try_remove_post_attachment_drive_at(backend, ordinal)
                        .map_err(SkeletonDriveOwnerError::Drive)?;
                    runtime.bone_entries[part][kind.index()] = None;
                }
                runtime
                    .prepared
                    .mark_bone_released(SkeletonPartIndex::ALL[part], kind);
            }
        }
        self.session
            .try_finish_post_attachment_drive_cleanup(backend)
            .map_err(SkeletonDriveOwnerError::Drive)?;
        self.drives = None;
        Ok(())
    }
}

#[path = "dynamic_assembly_skeleton_hook_offset.rs"]
pub(crate) mod hook_offset;
