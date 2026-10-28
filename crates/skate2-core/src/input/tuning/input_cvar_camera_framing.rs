//! Live references sampled by `Iris::Rig::DoFramingUpdate82e42ee0`.
use super::{InputCvarError, InputCvarReferences};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CameraFramingBoolean {
    PitchAcceleration,
    PitchSpeed,
    PitchSmoothing,
    YawAcceleration,
    YawSpeed,
    YawSmoothing,
}

impl InputCvarReferences {
    pub(crate) fn try_camera_framing_boolean(
        &self,
        kind: CameraFramingBoolean,
    ) -> Result<bool, InputCvarError> {
        self.try_current_boolean(match kind {
            CameraFramingBoolean::PitchAcceleration => 336,
            CameraFramingBoolean::PitchSpeed => 337,
            CameraFramingBoolean::PitchSmoothing => 338,
            CameraFramingBoolean::YawAcceleration => 339,
            CameraFramingBoolean::YawSpeed => 340,
            CameraFramingBoolean::YawSmoothing => 341,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;

    #[test]
    fn source_defaults_and_live_overrides_keep_six_distinct_targets() {
        use CameraFramingBoolean as B;
        let refs = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            refs.try_camera_framing_boolean(B::PitchSpeed),
            Err(InputCvarError::Unlinked("kbPitchSpeedClamp"))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let fields = [
            B::PitchAcceleration,
            B::PitchSpeed,
            B::PitchSmoothing,
            B::YawAcceleration,
            B::YawSpeed,
            B::YawSmoothing,
        ];
        let defaults = fields.map(|field| refs.try_camera_framing_boolean(field).unwrap());
        assert_eq!(defaults, [true, false, true, true, false, true]);
        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkbPitchSpeedClamp=1\r\nkbYawSmoothing=0\r\n",
            ))
            .unwrap();
        let changed = fields.map(|field| refs.try_camera_framing_boolean(field).unwrap());
        assert_eq!(changed, [true, true, true, true, false, false]);
    }
}
