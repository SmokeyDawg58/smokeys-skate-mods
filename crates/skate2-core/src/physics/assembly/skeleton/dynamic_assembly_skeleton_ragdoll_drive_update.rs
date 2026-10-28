//! Concrete mapped-volume consumer for `UpdateRagdoll`0x82b546c0.
use super::{
    Arc, AssemblyPartMaterial, HostSkeletonDriveNumeric, SimulationHookAssemblyPool,
    SkateboardSimulationBackend, SkeletonDriveFeedbackError, SkeletonDriveOwnerError,
    SkeletonDriveOwnerStage, SkeletonDriveUpdateProgress, SkeletonNormalAssembly,
    validate_admitted_payloads,
};
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    skeleton_drives::ragdoll_update::{RagdollDriveCvars, RagdollDriveWeights},
};

impl SkeletonNormalAssembly {
    /// Writes the same frame/dynamics Arcs already admitted to the solver.
    pub(crate) fn try_update_ragdoll_drives_from_retained_volumes<C: RagdollDriveCvars, H>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        hooks: &SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        weights: RagdollDriveWeights,
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
        runtime.update_progress = SkeletonDriveUpdateProgress::UpdatingDrives;
        runtime
            .prepared
            .try_update_ragdoll(
                &targets,
                &self.state.binding.body().attributes.drives,
                weights,
                runtime
                    .update_attributes
                    .as_mut()
                    .expect("retained provider checked"),
                cvars,
                &mut HostSkeletonDriveNumeric,
                &mut HostReckoningNumeric,
            )
            .map_err(E::RagdollUpdate)?;
        runtime.update_progress = SkeletonDriveUpdateProgress::Complete;
        Ok(())
    }
}
