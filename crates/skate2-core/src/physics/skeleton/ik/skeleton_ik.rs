//! Retained IK constructor/reset ownership, 0x82b550f0 / 0x82b51370.
//! The native heap object belongs to the same Skeleton lifetime. Existing
//! target mappings and transforms remain solely in `SkeletonIkTargets`.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "active IK stages await integration")
)]

use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    skeleton_ik_attributes::SkeletonIkVaultAttributes,
    skeleton_ik_geometry::{SkeletonIkGeometry, SkeletonIkGeometryProgress},
    skeleton_ik_state::{SkeletonIkBlendError, SkeletonIkState},
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

#[path = "../../player/wipeout/physical_player_post_wipeout_foot_ik_owner.rs"]
mod post_wipeout_foot_owner;

#[cfg(test)]
#[path = "skeleton_ik_suffix_tests.rs"]
mod suffix_tests;

/// Late Update gate at 0x82b59c98 and the services of its two remaining calls.
/// The shared error retains the failure at the individual source read/query.
pub(crate) trait SkeletonIkSuffixServices:
    crate::skeleton_ik_state::contact::SkeletonIkContactServices
    + crate::skeleton_ik_state::drive_frames::SkeletonIkDriveFramesServices
    + crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsServices<
        Error = <Self as crate::skeleton_ik_state::contact::SkeletonIkContactServices>::Error,
    >
{
    fn enable_contact_offset(
        &mut self,
    ) -> Result<bool, <Self as crate::skeleton_ik_state::contact::SkeletonIkContactServices>::Error>;
}

#[derive(Debug)]
pub(crate) enum SkeletonIkSuffixError<P, A, C, N, R> {
    Unconstructed,
    EnableContact(C),
    Contact(crate::skeleton_ik_state::contact::SkeletonIkContactError<C, N>),
    DriveFrames(crate::skeleton_ik_state::drive_frames::SkeletonIkDriveFramesError<P, A, C, N, R>),
}
impl<
    P: std::fmt::Debug,
    A: std::fmt::Debug,
    C: std::fmt::Debug,
    N: std::fmt::Debug,
    R: std::fmt::Debug,
> std::fmt::Display for SkeletonIkSuffixError<P, A, C, N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unconstructed => f.write_str("IK suffix owner is absent"),
            Self::EnableContact(error) => write!(f, "IK contact enable: {error:?}"),
            Self::Contact(error) => write!(f, "{error}"),
            Self::DriveFrames(error) => write!(f, "IK drive frames: {error:?}"),
        }
    }
}

/// Numerical input fields at IK+20. Source owner/row pointers are borrowed
/// from the actual assembly when updating; no second pose buffer is allocated.
/// Constructor memset55168 clears every lane, including matrix diagonals.
pub(crate) struct SkeletonIkDataIn {
    pub(crate) animation_to_world: SkeletonRootMatrix,
    pub(crate) world_to_animation: SkeletonRootMatrix,
    pub(crate) physics_skateboard: SkeletonRootMatrix,
    pub(crate) inverse_skateboard: SkeletonRootMatrix,
    pub(crate) hips: SkeletonRootMatrix,
    pub(crate) skateboard: SkeletonRootMatrix,
    pub(crate) toe_contact_normal: SkeletonRootVector,
    pub(crate) hips_offset: SkeletonRootVector,
}
impl SkeletonIkDataIn {
    fn cleared() -> Self {
        let zero = SkeletonRootMatrix::known([0.0; 16]);
        Self {
            animation_to_world: zero,
            world_to_animation: zero,
            physics_skateboard: zero,
            inverse_skateboard: zero,
            hips: zero,
            skateboard: zero,
            toe_contact_normal: SkeletonRootVector::known([0.0; 4]),
            hips_offset: SkeletonRootVector::known([0.0; 4]),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonIkUpdateError {
    Input(crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError),
    Unconstructed,
    IncompleteConstruction(SkeletonIkGeometryProgress),
    Blend(SkeletonIkBlendError<AttributeRuntimeError>),
}
impl std::fmt::Display for SkeletonIkUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(error) => write!(f, "IK processed input: {error}"),
            Self::Unconstructed => f.write_str("IK owner is absent"),
            Self::IncompleteConstruction(progress) => {
                write!(f, "IK construction incomplete: {progress:?}")
            }
            Self::Blend(error) => write!(f, "IK blend: {error:?}"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonIkStagesError<N, R, C> {
    Prefix(SkeletonIkUpdateError),
    Initial(
        crate::skeleton_ik_state::initial::SkeletonIkInitialError<AttributeRuntimeError, C, N, R>,
    ),
    Blend(crate::skeleton_ik_state::blend_transforms::SkeletonIkTransformBlendError<N, R>),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Display
    for SkeletonIkStagesError<N, R, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prefix(error) => write!(f, "IK data/status/blends: {error:?}"),
            Self::Initial(error) => write!(f, "IK initial transforms: {error:?}"),
            Self::Blend(error) => write!(f, "IK world transform blend: {error:?}"),
        }
    }
}

pub(crate) struct SkeletonIk {
    attributes: SkeletonIkVaultAttributes,
    data_in: SkeletonIkDataIn,
    state: SkeletonIkState,
    geometry: SkeletonIkGeometry,
}
impl SkeletonIk {
    /// `RequestPushExit82b3bb70` on the retained IK state.
    pub(crate) fn request_push_exit(&mut self) {
        self.state.request_push_exit();
    }

    /// 0x82b59c94..0x82b59cb0, after the completed transform stages. Disabled
    /// contact preserves its histories; every failure preserves the completed
    /// prefix. `DriveFrames` borrows the actual rows and global animation attrs.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_suffix<P, V, A, S, N, R>(
        &mut self,
        targets: &SkeletonIkTargets,
        processed: &P,
        rows: &mut V,
        attributes: &mut A,
        services: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<
        (),
        SkeletonIkSuffixError<
            V::Error,
            A::Error,
            <S as crate::skeleton_ik_state::contact::SkeletonIkContactServices>::Error,
            N::Error,
            R::Error,
        >,
    >
    where
        P: crate::skeleton_ik_state::contact::SkeletonIkContactRead,
        V: crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsRows,
        A: crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsAttributes,
        S: SkeletonIkSuffixServices,
        N: crate::reckoning_numeric::ReckoningNumeric,
        R: crate::reckoning_sin_cos::ReckoningSinCosNumeric,
    {
        if services
            .enable_contact_offset()
            .map_err(SkeletonIkSuffixError::EnableContact)?
        {
            self.state
                .try_update_contact(
                    crate::skeleton_ik_state::contact::SkeletonIkContactInput {
                        data_in: &self.data_in,
                        targets,
                        processed,
                    },
                    services,
                    numeric,
                )
                .map_err(SkeletonIkSuffixError::Contact)?;
        }
        self.state
            .try_update_drive_frames(
                &self.data_in,
                targets,
                rows,
                attributes,
                services,
                numeric,
                rounding,
            )
            .map_err(SkeletonIkSuffixError::DriveFrames)
    }

    /// Instance binding, cleared input, then full Reset all precede the first
    /// global deck read551c0. The caller retains this prefix before proceeding.
    pub(crate) fn new(vault: &Arc<AttributeRuntimeVault>, targets: &mut SkeletonIkTargets) -> Self {
        let attributes = SkeletonIkVaultAttributes::bind(vault);
        let data_in = SkeletonIkDataIn::cleared();
        let state = SkeletonIkState::new(targets);
        Self {
            attributes,
            data_in,
            state,
            geometry: SkeletonIkGeometry::new(),
        }
    }
    pub(crate) fn geometry(&self) -> &SkeletonIkGeometry {
        &self.geometry
    }
    pub(crate) fn geometry_mut(&mut self) -> &mut SkeletonIkGeometry {
        &mut self.geometry
    }
    pub(crate) fn reset_push_abort(&mut self) {
        self.state.reset_push_abort();
    }

    pub(crate) fn state(&self) -> &SkeletonIkState {
        &self.state
    }
    pub(crate) fn set_world_foot_target(
        &mut self,
        left: bool,
        position: SkeletonRootVector,
        blend: f32,
    ) {
        self.state.set_world_foot_target(left, position, blend);
    }

    pub(crate) fn set_world_hand_target(
        &mut self,
        left: bool,
        position: SkeletonRootVector,
        blend: f32,
    ) {
        self.state.set_world_hand_target(left, position, blend);
    }

    pub(crate) fn set_local_foot_target(
        &mut self,
        left: bool,
        position: SkeletonRootVector,
        blend: f32,
    ) {
        self.state.set_local_foot_target(left, position, blend);
    }
    pub(crate) fn try_set_foot_target_normal(
        &mut self,
        left: bool,
        normal: SkeletonRootVector,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), crate::skeleton_ik_state::set_target_normal::SetTargetNormalError> {
        self.state.try_set_foot_target_normal(left, normal, cvars)
    }
    pub(crate) fn finish_skeleton_init(&mut self) {
        self.state.finish_skeleton_init();
    }

    /// Source `Skeleton::EnableIK` 0x82b462e0; histories and targets survive.
    pub(crate) fn enable(&mut self) {
        self.state.enable();
    }
    /// Source `Skeleton::DisableIK` 0x82b462f0; targets and contact histories survive.
    pub(crate) fn disable(&mut self) {
        self.state.disable();
    }
    pub(crate) fn data_in(&self) -> &SkeletonIkDataIn {
        &self.data_in
    }

    /// Reset51370 preserves the actual Instance, input, geometry and IDs.
    pub(crate) fn reset(&mut self, targets: &mut SkeletonIkTargets) {
        self.state.reset(targets);
    }

    /// Input assignment417bc then status59c74 and blends59c78. All
    /// numerical source copies finish before replacing the previous input.
    /// The same enclosing owner retains/borrows the actual two row buffers,
    /// Assembly and selected Island; no line query occurs at this stage.
    pub(crate) fn try_update_prefix(
        &mut self,
        data: SkeletonIkDataIn,
        status_flags: u32,
        blend_flags: u32,
    ) -> Result<(), SkeletonIkUpdateError> {
        self.try_update_prefix_read(
            data,
            || Ok(status_flags),
            || Ok(blend_flags & 0x1000_0000 != 0),
        )
    }

    pub(crate) fn try_update_prefix_read(
        &mut self,
        data: SkeletonIkDataIn,
        read_status: impl FnOnce()->Result<u32,crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
        read_blend: impl FnMut()->Result<bool,crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
    ) -> Result<(), SkeletonIkUpdateError> {
        self.try_store_data_only(data)?;
        self.state
            .update_status(read_status().map_err(SkeletonIkUpdateError::Input)?);
        self.state
            .try_update_blend_values_read(&self.geometry, read_blend, &mut self.attributes)
            .map_err(SkeletonIkUpdateError::Blend)
    }

    /// Initial59c80 then Blend59c88 on the SAME retained state and targets.
    #[expect(
        clippy::type_complexity,
        reason = "distinct numerical and cvar failures"
    )]
    pub(crate) fn try_update_transforms<N, R, C>(
        &mut self,
        targets: &SkeletonIkTargets,
        previous: &[crate::animation::AnimationMatrix],
        cvars: &mut C,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), SkeletonIkStagesError<N::Error, R::Error, C::Error>>
    where
        N: crate::reckoning_numeric::ReckoningNumeric,
        R: crate::reckoning_sin_cos::ReckoningSinCosNumeric,
        C: crate::skeleton_ik_state::initial::SkeletonIkInitialCvars,
    {
        self.state
            .try_calculate_initial_transforms(
                &self.data_in,
                targets,
                previous,
                &mut self.attributes,
                cvars,
                numeric,
                rounding,
            )
            .map_err(SkeletonIkStagesError::Initial)?;
        self.state
            .try_blend_transforms(&self.data_in, targets, previous, numeric, rounding)
            .map_err(SkeletonIkStagesError::Blend)
    }
}

#[path = "skeleton_ik_fill_runtime.rs"]
pub(crate) mod fill_runtime;

impl SkeletonIk {
    pub(crate) fn try_store_data_only(
        &mut self,
        data: SkeletonIkDataIn,
    ) -> Result<(), SkeletonIkUpdateError> {
        if self.geometry.progress() != SkeletonIkGeometryProgress::Complete {
            return Err(SkeletonIkUpdateError::IncompleteConstruction(
                self.geometry.progress(),
            ));
        }
        self.data_in = data;
        Ok(())
    }
    pub(crate) fn try_begin_wipeout_adjust(
        &mut self,
        targets: &mut SkeletonIkTargets,
        read_status:impl FnOnce()->Result<u32,crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
        read_blend:impl FnMut()->Result<bool,crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
    ) -> Result<(), SkeletonIkUpdateError> {
        use crate::skeleton_ik_attributes::SkeletonIkAttributeRead;
        let offset = self
            .attributes
            .scalar(0x54c3_5670_4afa_0a9a)
            .map_err(|e| SkeletonIkUpdateError::Blend(SkeletonIkBlendError::Attribute(e)))?;
        self.state.begin_wipeout_adjust();
        targets.add_wipeout_height(offset);
        self.state.seed_wipeout_foot_blends();
        self.state
            .update_status(read_status().map_err(SkeletonIkUpdateError::Input)?);
        self.state
            .try_update_blend_values_read(&self.geometry, read_blend, &mut self.attributes)
            .map_err(SkeletonIkUpdateError::Blend)
    }
    pub(crate) fn finish_wipeout_adjust(&mut self) {
        self.state.finish_wipeout_adjust();
    }
}
