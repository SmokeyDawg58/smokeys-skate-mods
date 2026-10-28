//! Live references sampled by the actual Rig dynamics prefix.
use super::{InputCvarError, InputCvarReferences};

#[derive(Clone, Copy)]
pub(crate) enum CameraDynamicsBoolean {
    ShowTrajectory,
    HeadingAcceleration,
    HeadingSpeed,
    HeadingSmoothing,
    ElevationAcceleration,
    ElevationSpeed,
    ElevationSmoothing,
}
impl InputCvarReferences {
    pub(crate) fn try_camera_dynamics_boolean(
        &self,
        field: CameraDynamicsBoolean,
    ) -> Result<bool, InputCvarError> {
        self.try_current_boolean(match field {
            CameraDynamicsBoolean::ShowTrajectory => 322,
            CameraDynamicsBoolean::HeadingAcceleration => 323,
            CameraDynamicsBoolean::HeadingSpeed => 324,
            CameraDynamicsBoolean::HeadingSmoothing => 325,
            CameraDynamicsBoolean::ElevationAcceleration => 326,
            CameraDynamicsBoolean::ElevationSpeed => 327,
            CameraDynamicsBoolean::ElevationSmoothing => 328,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn actual_defaults_and_live_value_are_sampled() {
        let refs = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            refs.try_camera_dynamics_boolean(CameraDynamicsBoolean::ShowTrajectory),
            Err(InputCvarError::Unlinked("kbShowTrajectoryInfo"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert!(
            !refs
                .try_camera_dynamics_boolean(CameraDynamicsBoolean::ShowTrajectory)
                .unwrap()
        );
        for field in [
            CameraDynamicsBoolean::HeadingAcceleration,
            CameraDynamicsBoolean::HeadingSpeed,
            CameraDynamicsBoolean::HeadingSmoothing,
            CameraDynamicsBoolean::ElevationAcceleration,
            CameraDynamicsBoolean::ElevationSpeed,
            CameraDynamicsBoolean::ElevationSmoothing,
        ] {
            assert!(refs.try_camera_dynamics_boolean(field).unwrap());
        }
        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkbHeadingSmoothing=0\r\nkbShowTrajectoryInfo=1\r\n",
            ))
            .unwrap();
        assert!(
            !refs
                .try_camera_dynamics_boolean(CameraDynamicsBoolean::HeadingSmoothing)
                .unwrap()
        );
        assert!(
            refs.try_camera_dynamics_boolean(CameraDynamicsBoolean::ShowTrajectory)
                .unwrap()
        );
    }
}
