//! Connected `Skeleton::UpdateWipeout` 0x82b5ea28 on the same retained owners.
use super::{
    AssemblyPartMaterial, InitialCvars, LiveHooks, SimulationHookAssemblyPool,
    SkateboardSimulationBackend, SkeletonGeneralCvars, SkeletonGroundedUpdateError,
    SkeletonNormalAssembly,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::{
        drive_owner::update_owner::SkeletonDriveFeedbackError,
        general_owner::live_input::{GroundedInputError, GroundedInputRead},
    },
    input_cvar_runtime::{
        InputCvarError, InputCvarReferences, ragdoll_drives::RagdollDriveSettings,
    },
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_drives::ragdoll_update::RagdollDriveWeights,
    skeleton_processed_contact::grounded_runtime::GroundedProcessed,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector, wipeout::WipeoutRootInput},
};
#[derive(Debug)]
pub(crate) enum WipeoutRootReadError {
    Cvar(InputCvarError),
    Input(GroundedInputError),
}
#[derive(Debug)]
pub(crate) enum WipeoutSkeletonUpdateError<N, S, C> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pair(crate::dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError),
    Pool(crate::dynamic_assembly_session::skater_pool::SimulationSkaterPoolError),
    Root(WipeoutRootReadError),
    General(SkeletonGroundedUpdateError<N, S, C>),
    Ragdoll(SkeletonDriveFeedbackError<InputCvarError>),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Display
    for WipeoutSkeletonUpdateError<N, S, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(e) => write!(f, "wipeout skeleton missing: {e}"),
            Self::Poisoned => f.write_str("wipeout skeleton poisoned"),
            Self::ForeignOwner => f.write_str("wipeout skeleton foreign owner"),
            Self::Pair(e) => write!(f, "wipeout skeleton pair: {e}"),
            Self::Pool(e) => write!(f, "wipeout skeleton pool: {e}"),
            Self::Root(WipeoutRootReadError::Cvar(e)) => write!(f, "wipeout roots cvar: {e:?}"),
            Self::Root(WipeoutRootReadError::Input(e)) => write!(f, "wipeout roots input: {e}"),
            Self::General(e) => write!(f, "wipeout skeleton: {e}"),
            Self::Ragdoll(e) => write!(f, "wipeout ragdoll: {e}"),
        }
    }
}
struct RootInput<'a, 'p> {
    refs: &'a InputCvarReferences,
    processed: &'a GroundedProcessed<'p>,
}
impl WipeoutRootInput for RootInput<'_, '_> {
    type Error = WipeoutRootReadError;
    fn blend_camera_root(&mut self) -> Result<bool, Self::Error> {
        self.refs
            .try_wipeout_camera_root_blend()
            .map_err(WipeoutRootReadError::Cvar)
    }
    fn state_timer(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .wipeout_state_timer()
            .map_err(WipeoutRootReadError::Input)
    }
    fn height_offset(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_wipeout_height_offset()
            .map_err(WipeoutRootReadError::Cvar)
    }
    fn centre_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.processed
            .wipeout_centre_velocity()
            .map_err(WipeoutRootReadError::Input)
    }
}
impl SkeletonNormalAssembly {
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same owners and distinct numerical services"
    )]
    pub(crate) fn try_update_wipeout_complete<N, S, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        processed: &GroundedProcessed<'_>,
        weights: RagdollDriveWeights,
        refs: &InputCvarReferences,
        numeric: &mut N,
        rounding: &mut S,
        cvars: &mut C,
    ) -> Result<
        (),
        WipeoutSkeletonUpdateError<
            N::Error,
            S::Error,
            <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
        >,
    >
    where
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        C: SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        use SkeletonGroundedUpdateError as G;
        use WipeoutSkeletonUpdateError as E;
        self.root_transforms
            .try_update_wipeout(
                self.post_physics_records.centre_of_mass_typed(),
                &mut RootInput { refs, processed },
            )
            .map_err(E::Root)?;
        let mut actual_hooks = LiveHooks {
            pool: hooks,
            backend,
            handle: self.drives.as_ref().and_then(|v| v.hook().cloned()),
            stage: self
                .drives
                .as_ref()
                .map(super::super::drive_owner::SkeletonDriveRuntime::stage),
        };
        // This native caller ignores UpdateHookPositions' boolean reset request.
        self.general_update
            .try_update_hooks(
                &self.root_transforms,
                self.current_animation.hips(),
                self.current_animation.skateboard(),
                &mut self.state.proxy_volume_target,
                &mut actual_hooks,
                numeric,
            )
            .map_err(|e| E::General(G::Hooks(e)))?;
        self.general_update
            .set_animated_skate_root(*self.root_transforms.animation_to_world());
        if processed
            .wipeout_state_timer()
            .map_err(|e| E::Root(WipeoutRootReadError::Input(e)))?
            == 0.0
        {
            let data = self.try_build_ik_data(processed).map_err(|e| {
                E::General(G::Ik(crate::skeleton_ik::SkeletonIkStagesError::Prefix(e)))
            })?;
            self.current_animation
                .try_update_ik_data_only(data)
                .map_err(|e| {
                    E::General(G::Ik(crate::skeleton_ik::SkeletonIkStagesError::Prefix(e)))
                })?;
            self.current_animation
                .try_begin_wipeout_ik_adjust(
                    || processed.flags_8c8(0x600),
                    || processed.flags_8d0(0x1000_0000).map(|v| v != 0),
                )
                .map_err(|e| {
                    E::General(G::Ik(crate::skeleton_ik::SkeletonIkStagesError::Prefix(e)))
                })?;
            self.current_animation
                .try_update_ik_transforms(&mut InitialCvars(cvars), numeric, rounding)
                .map_err(|e| E::General(G::Ik(e)))?;
            self.current_animation
                .try_update_ik_suffix(
                    processed,
                    &mut self.animation_targets,
                    self.current_animation_attributes
                        .as_mut()
                        .ok_or(E::Missing("animation attributes"))?,
                    cvars,
                    numeric,
                    rounding,
                )
                .map_err(|e| E::General(G::IkSuffix(e)))?;
            self.current_animation
                .finish_wipeout_ik_adjust()
                .map_err(|e| {
                    E::General(G::Ik(crate::skeleton_ik::SkeletonIkStagesError::Prefix(e)))
                })?;
        }
        self.try_update_ragdoll_drives_from_retained_volumes(
            backend,
            hooks,
            weights,
            &mut RagdollDriveSettings(refs),
        )
        .map_err(E::Ragdoll)?;
        *self.general_update.next_trajectory_transform_mut() = SkeletonRootMatrix::known([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ]);
        Ok(())
    }
}
