//! Original hud2/hudcountdown (asset 58, fingerprint b09d0ef1; template entry `CountdownHUD`
//! 0x82fdb828, placement 0, drawn flat) and its `hudcountdown` class (InitAction character 20,
//! `0x1acc`), driven by `CountdownHUD` (`skate2_core::challenge::turn_hud`): the count before a
//! turn, then GO.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::turn_hud::TurnHudView;

pub(super) const ASSET: usize = 58;
/// `FrontEndManager::IsInStadiumZone` (`0x82606528`): `mZoneKey` is the `stadium` location
/// trigger, `StringHash64("stadium")` compared in full (`cmpld`).
const STADIUM_ZONE: u64 = 0xe1dd_e899_e21b_999b;
/// `LanguageManager::GetCurrentLanguageName` of the English build, a label of `mcNumber`.
const LANGUAGE: &str = "English";

pub(super) struct CountdownMovie {
    pub root: Clip,
    pub sounds: Vec<&'static str>,
}

impl CountdownMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)?, sounds: Vec::new() };
        movie.calls()?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls()
    }

    /// Root action `0x1a9c`'s `new hudcountdown(this)`, then `Stop`: the constructor stops the
    /// screen and registers it.
    fn calls(&mut self) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "hudcountdown" => {}
                _ => return Err(format!("untranslated countdown HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `SetState`: 1 `HideCountdown` plays `mcCountdownText` to `nothing`; 2 `ShowCountdown`.
    /// `zone` is the front end's district zone key (`mZoneKey`).
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &TurnHudView, zone: u64) -> RenderResult<()> {
        match state {
            1 => self.root.named(&["mcCountdownText"])?.goto(assets, "nothing"),
            2 => {
                let TurnHudView::Countdown { value, has_audio } = *view else {
                    return Err("countdown HUD state 2 without countdown data".into());
                };
                let in_stadium = zone == STADIUM_ZONE;
                self.show_countdown(assets, value, has_audio, in_stadium)
            }
            _ => Ok(()),
        }
    }

    /// `ShowCountdown`: `Countdown_GetCountdownTime` (`0x826b8bd8`: `#n`, or GO formatted
    /// from `ID_MISSION_COUNTDOWN_GO` at zero) picks `mcNumber`'s label (3, 2 and 1 by number,
    /// anything else the language's GO), then the count's intro; with audio
    /// (`Countdown_ChallengeHasAudio`) the count sounds, or in the stadium each number and GO
    /// its own. The `debugNum` it fills is not authored.
    fn show_countdown(&mut self, assets: &Assets, value: i32, has_audio: bool, in_stadium: bool) -> RenderResult<()> {
        let number = (1..=3).contains(&value).then(|| format!("#{value}"));
        let text = self.root.named(&["mcCountdownText"])?;
        text.named(&["mcNumber"])?.goto_and_stop(assets, number.as_deref().unwrap_or(LANGUAGE))?;
        text.goto(assets, "intro")?;
        if has_audio {
            self.sounds.push(if !in_stadium {
                "FE_SOUND_CHALLENGE_COUNT"
            } else {
                match value {
                    3 => "FE_SOUND_CHALLENGE_STAD_3",
                    2 => "FE_SOUND_CHALLENGE_STAD_2",
                    1 => "FE_SOUND_CHALLENGE_STAD_1",
                    _ => "FE_SOUND_CHALLENGE_STAD_GO",
                }
            });
        }
        Ok(())
    }
}

/// The movie's InitAction: the `hudcountdown` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 20, 0x1acc))
}

/// Timeline actions besides `Stop`: the root constructs the class; `mcCountdownText` hides on
/// its first frame and at `invisible`, and shows at `intro`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x1a9c => {
            clip.calls.push("hudcountdown");
            clip.playback.stop();
        }
        0x1dd4 | 0x1e0c => {
            clip.visible = false;
            clip.playback.stop();
        }
        0x1df8 => clip.visible = true,
        _ => return Err(format!("untranslated countdown HUD APT action {offset:#x}").into()),
    }
    Ok(())
}
