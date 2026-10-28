//! Live cvar targets sampled by the Actor breadcrumb respawn selector.
use super::{InputCvarError, InputCvarReferences};

impl InputCvarReferences {
    /// `BreadCrumbManager::Implementation::Update` tests this after the hips copy.
    pub(crate) fn try_breadcrumb_min_ticks_after_reset(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(367)
    }

    /// `Actor::CalcRespawnTransform` samples this linked float after its deck-velocity dot.
    pub(crate) fn try_breadcrumb_insignificant_square_velocity(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(364)
    }

    /// `_TestForNavMeshConstraints` reads `kbEnableNavMeshConstraints.mTarget` first.
    pub(crate) fn try_breadcrumb_navmesh_constraints(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(357)
    }

    /// `GetWipeoutSpotBreadCrumb` reads `kEnableWipeoutSpotBreadCrumb.mTarget` first.
    pub(crate) fn try_wipeout_spot_breadcrumb(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(358)
    }
}

#[cfg(test)]
mod tests {
    use super::InputCvarReferences;
    use crate::input_cvar_runtime::InputCvarManager;

    #[test]
    fn breadcrumb_gates_read_live_targets_after_startup_override() {
        let references = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&references).unwrap();
        assert!(references.try_breadcrumb_navmesh_constraints().unwrap());
        assert!(references.try_wipeout_spot_breadcrumb().unwrap());
        assert_eq!(
            references
                .try_breadcrumb_insignificant_square_velocity()
                .unwrap(),
            0.25
        );
        assert_eq!(
            references.try_breadcrumb_min_ticks_after_reset().unwrap(),
            120
        );

        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkbEnableNavMeshConstraints=0\r\nkEnableWipeoutSpotBreadCrumb=0\r\nkfInsignificantSquareVelocity=0.5\r\nkMinTicksAfterResetForBreadCrumb=240\r\n",
            ))
            .unwrap();
        assert!(!references.try_breadcrumb_navmesh_constraints().unwrap());
        assert!(!references.try_wipeout_spot_breadcrumb().unwrap());
        assert_eq!(
            references
                .try_breadcrumb_insignificant_square_velocity()
                .unwrap(),
            0.5
        );
        assert_eq!(
            references.try_breadcrumb_min_ticks_after_reset().unwrap(),
            240
        );
    }
}
