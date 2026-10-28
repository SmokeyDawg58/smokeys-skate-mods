//! PC Skate 3 animation choices, independent of the native Skate 2 CAC style.
//! GetCACSettings82590D20 maps style; posture profile mapping is82590B50.
use super::{ProfileError, Result, SkaterProfile};

const SAVABLE: &str = "SK82_PC";
const LEAF: &str = "SK82_PC::Skate3Style";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PcSkate3Style {
    pub style: u32,
    pub posture: u32,
}

impl PcSkate3Style {
    pub const STYLES: [&'static str; 4] = ["Default", "Loose", "Gonzo", "Aggressive"];
    pub const POSTURES: [&'static str; 4] = ["Default", "Stiff", "Slouch", "Buff"];

    pub fn validate(self) -> Result<()> {
        if self.style >= Self::STYLES.len() as u32 || self.posture >= Self::POSTURES.len() as u32 {
            return Err(ProfileError("invalid Skate 3 style or posture selection"));
        }
        Ok(())
    }
}

impl SkaterProfile {
    pub fn skate3_style(&self) -> Result<PcSkate3Style> {
        let Some(leaf) = self.leaf(SAVABLE, LEAF)? else {
            return Ok(PcSkate3Style::default());
        };
        if leaf.version != 1 {
            return Err(ProfileError("unsupported PC Skate 3 style version"));
        }
        let [style, posture] = leaf.tuples.as_slice() else {
            return Err(ProfileError("invalid PC Skate 3 style tuples"));
        };
        let settings = PcSkate3Style {
            style: u32::from_be_bytes(
                style
                    .as_slice()
                    .try_into()
                    .map_err(|_| ProfileError("invalid Skate 3 style storage"))?,
            ),
            posture: u32::from_be_bytes(
                posture
                    .as_slice()
                    .try_into()
                    .map_err(|_| ProfileError("invalid Skate 3 posture storage"))?,
            ),
        };
        settings.validate()?;
        Ok(settings)
    }

    pub fn set_skate3_style(&mut self, settings: PcSkate3Style) -> Result<()> {
        settings.validate()?;
        // A malformed or future leaf must not be overwritten with defaults.
        self.skate3_style()?;
        self.set_leaf(
            SAVABLE,
            &["CAC"],
            LEAF,
            1,
            &[
                settings.style.to_be_bytes().to_vec(),
                settings.posture.to_be_bytes().to_vec(),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_and_postures_survive_other_profile_writers_without_changing_native_choices() {
        let mut profile = SkaterProfile::new_empty();
        profile
            .set_leaf("FUTURE", &["CAC"], "opaque", 23, &[vec![9, 8, 7, 6]])
            .unwrap();
        let unknown = profile.leaf("FUTURE", "opaque").unwrap();
        let cac = profile.cac_record().unwrap();
        assert_eq!(profile.skate3_style().unwrap(), PcSkate3Style::default());
        let gestures = super::super::PcGestureSettings {
            selected: super::super::GestureSet::Skate3,
            skate3: [36, 17, 5, 29],
        };
        for style in 0..4 {
            for posture in 0..4 {
                let expected = PcSkate3Style { style, posture };
                profile.set_skate3_style(expected).unwrap();
                profile.set_gesture_settings(gestures).unwrap();
                profile.set_display_settings(Default::default()).unwrap();
                profile = SkaterProfile::parse(&profile.encode().unwrap()).unwrap();
                assert_eq!(profile.skate3_style().unwrap(), expected);
                assert_eq!(profile.gesture_settings().unwrap(), gestures);
                assert_eq!(profile.cac_record().unwrap(), cac);
                assert_eq!(profile.leaf("FUTURE", "opaque").unwrap(), unknown);
            }
        }
    }

    #[test]
    fn malformed_and_future_style_extensions_are_not_replaced() {
        let mut profile = SkaterProfile::new_empty();
        let before = profile.encode().unwrap();
        for settings in [
            PcSkate3Style {
                style: 4,
                posture: 0,
            },
            PcSkate3Style {
                style: 0,
                posture: 4,
            },
        ] {
            assert!(profile.set_skate3_style(settings).is_err());
            assert_eq!(profile.encode().unwrap(), before);
        }
        for (version, tuples) in [
            (2, vec![vec![0; 4], vec![0; 4]]),
            (1, vec![vec![0; 3], vec![0; 4]]),
            (1, vec![vec![0; 4]]),
            (1, vec![4_u32.to_be_bytes().to_vec(), vec![0; 4]]),
        ] {
            let mut profile = SkaterProfile::new_empty();
            profile
                .set_leaf(SAVABLE, &["CAC"], LEAF, version, &tuples)
                .unwrap();
            let before = profile.encode().unwrap();
            assert!(profile.skate3_style().is_err());
            assert!(profile.set_skate3_style(Default::default()).is_err());
            assert_eq!(profile.encode().unwrap(), before);
        }
    }
}
