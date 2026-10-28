//! Child of `post_wipeout_foot_live`: reuse the SAME concrete Part/record source.
use super::{FootIkLiveDebug, FootIkLiveError, FootIkLiveInput, SkeletonNormalAssembly, Source};
use crate::{
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_ik_state::{post_wipeout_foot::FootIkError, post_wipeout_update::IkPostUpdateSource},
};

pub(crate) trait IkPostUpdateLiveDebug: FootIkLiveDebug {
    /// Actual cvar213, source defaulttrue.
    fn do_new_post_physics(&mut self) -> Result<bool, Self::Error>;
    /// Actual cvar214, source defaultfalse.
    fn draw_new_post_physics(&mut self) -> Result<bool, Self::Error>;
}
impl<D: IkPostUpdateLiveDebug> IkPostUpdateSource for Source<'_, D> {
    fn do_new_post_physics_ik(&mut self) -> Result<bool, Self::Error> {
        self.input
            .debug
            .do_new_post_physics()
            .map_err(FootIkLiveError::Debug)
    }
    fn draw_new_post_physics_ik(&mut self) -> Result<bool, Self::Error> {
        self.input
            .debug
            .draw_new_post_physics()
            .map_err(FootIkLiveError::Debug)
    }
}
impl SkeletonNormalAssembly {
    /// Actual mandatory57bfc call AFTER selected foot/hand helpers. This returns
    /// before `HeadTracking` and pose publication and does not complete Update4.
    #[expect(
        clippy::type_complexity,
        reason = "preserve separate source and numeric failures"
    )]
    pub(crate) fn try_post_wipeout_ik_update_live<
        D: IkPostUpdateLiveDebug,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        input: FootIkLiveInput<'_, D>,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), FootIkError<FootIkLiveError<D::Error>, N::Error, R::Error>> {
        let owners = self
            .current_animation
            .post_wipeout_foot_parts_mut()
            .ok_or(FootIkError::Missing("retained IK runtime"))?;
        let mut source = Source {
            global_animation: self.drives.as_ref().and_then(
                super::super::drive_owner::SkeletonDriveRuntime::global_animation_instance,
            ),
            session: &mut self.session,
            history: &mut self.physics_history,
            own_attributes: owners.attributes,
            input,
        };
        owners
            .state
            .try_post_wipeout_ik_update(owners.targets, &mut source, numeric, rounding)
    }
}

impl SkeletonNormalAssembly {
    /// Reference: 0x82b57bf4. Actual selected hand helper, before mandatory IK update.
    #[expect(
        clippy::type_complexity,
        reason = "distinct source and numeric failures"
    )]
    pub(crate) fn try_post_wipeout_hand_live<
        D: FootIkLiveDebug,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        input: FootIkLiveInput<'_, D>,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), FootIkError<FootIkLiveError<D::Error>, N::Error, R::Error>> {
        let owners = self
            .current_animation
            .post_wipeout_foot_parts_mut()
            .ok_or(FootIkError::Missing("retained IK runtime"))?;
        let deck = *input.deck;
        let mut source = Source {
            global_animation: self.drives.as_ref().and_then(
                super::super::drive_owner::SkeletonDriveRuntime::global_animation_instance,
            ),
            session: &mut self.session,
            history: &mut self.physics_history,
            own_attributes: owners.attributes,
            input,
        };
        owners
            .state
            .try_post_wipeout_hand_ik(owners.targets, &deck, &mut source, numeric, rounding)
    }
}
