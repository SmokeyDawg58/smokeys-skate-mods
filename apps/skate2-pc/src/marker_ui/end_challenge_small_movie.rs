//! `FrontEndState_EndChallengeOptions`' screen 42 for a won challenge without several players,
//! `hudComponents/endchallenge_small` (asset 71, fingerprint 916b324b; template entry
//! `EndChallengeOptions_Small` `0x82fdb5f8`, placement 4, the `g3dSpinContainer`). Its
//! `InitAction` (character 69, `0x2480`) defines the `endchallenge_small` class translated here;
//! root action `0x2440` constructs it. Of the clips the class writes, the movie authors only
//! `glow_mc`, `mEndOptions` and `mcMiddleInfo`'s challenge name fields; the cash lines,
//! highlights, backs, `mcTitle`, `mResults`, `mTableBacking`, `mcTextChallengeTitle` and
//! `description_mc` are absent, so the natives read for them (`Profile.GetTotalCash`,
//! `Results_GetLeaderboardMainTitle`, `Mission.GetChallengeTypeName` and `GetDescription`, the
//! Goofy vs Regular scores) change nothing. What its natives answer comes from
//! [`EndOptionsView`]: `Mission.GetTitle`, `GetNumEndMissionOptions` and
//! `GetEndMissionOptionText`, and `HUDComponents.Results_GetNumPlayers`.
use super::{apt_random::AptRandom, assets::Assets, clips::Clip, end_challenge_movie::option_text, localizer, map_movie};
use crate::gpu_renderer::RenderResult;
use skate2_core::{EndOptionsView, challenge::system::MissionTitle, ui::apt_projection::Spin};

pub(super) const ASSET: usize = 71;

#[derive(Default)]
pub(super) struct EndChallengeSmallMovie {
    pub root: Option<Clip>,
    /// FE state 8 is open on this screen: `OnEnter` opened it and `PopState` has not closed it.
    pub open: bool,
    intro_complete: bool,
    outro_complete: bool,
}

impl EndChallengeSmallMovie {
    /// `OnEnter`'s `OpenScreen(42)` for a newly open state: a fresh screen, constructed (its
    /// constructor plays `intro`, which starts the spin); `OnLeave`'s `CloseScreen` once the
    /// state pops: `outro` and the spin back.
    pub fn update(
        &mut self,
        assets: &Assets,
        view: Option<&EndOptionsView>,
        title: &MissionTitle,
        spin: &mut Spin,
        random: &mut AptRandom,
    ) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "endchallenge_small" => construct(&mut root, assets, view, title, random)?,
                        _ => return Err(format!("untranslated end challenge small callback {call}").into()),
                    }
                }
                spin.intro_start();
                self.root = Some(root);
                self.calls()?;
            }
            None if self.open => {
                self.open = false;
                if let Some(root) = self.root.as_mut() {
                    root.goto(assets, "outro")?;
                }
                spin.intro_exit();
                self.calls()?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(root) = self.root.as_mut() {
            root.tick(assets)?;
        }
        self.calls()
    }

    /// The screen's `ScreenManager.IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// The screen's `ScreenManager.OutroComplete` since the last call.
    pub fn take_outro_complete(&mut self) -> bool {
        std::mem::take(&mut self.outro_complete)
    }

    /// Frame 9's `IntroComplete` and frame 18's `OutroComplete`, which unloads it.
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => {
                    self.outro_complete = true;
                    self.root = None;
                    break;
                }
                _ => return Err(format!("untranslated end challenge small callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new endchallenge_small(this)`: with players, `mcMiddleInfo`, `mEndOptions` and `glow_mc`
/// play `multi` and the challenge name fields take `Mission.GetTitle()`; without, they take one
/// of `ID_CHALLENGE_COMPLETE_COMMENT0` to `18`. Then `intro`, `SetEndOptions` and `SetState(5)`.
fn construct(root: &mut Clip, assets: &Assets, view: &EndOptionsView, title: &MissionTitle, random: &mut AptRandom) -> RenderResult<()> {
    let name = if view.leaderboard.num_players > 0 {
        for clip in ["mcMiddleInfo", "mEndOptions", "glow_mc"] {
            root.named(&[clip])?.goto(assets, "multi")?;
        }
        localizer::mission_title(assets, title)?
    } else {
        assets.initial_text(&format!("ID_CHALLENGE_COMPLETE_COMMENT{}", random.index(19)))
    };
    for field in ["mcTextChallenegeName", "mcTextChallenegeNameGlow"] {
        root.named(&["mcMiddleInfo", field])?.text = Some(name.clone());
    }
    root.goto(assets, "intro")?;
    set_end_options(root, assets, view)?;
    set_state(root, assets, view, 5)
}

/// `SetEndOptions`: `mEndOptions.mButtonMenu` cleared and given the options X, B and A while
/// there are more than two, one and none.
fn set_end_options(root: &mut Clip, assets: &Assets, view: &EndOptionsView) -> RenderResult<()> {
    let count = view.leaderboard.num_options();
    let mut items = Vec::new();
    for (slot, button) in [(2, "X"), (1, "B"), (0, "A")] {
        if count > slot {
            items.push((button, option_text(view, slot)));
        }
    }
    map_movie::configure_button_items(root.named(&["mEndOptions", "mButtonMenu"])?, assets, &items, 0xffff_ffec)
}

/// `SetState`: 5 and 6 end in `SetEndOptions`; the results and table backing they also reach
/// are absent.
fn set_state(root: &mut Clip, assets: &Assets, view: &EndOptionsView, state: i32) -> RenderResult<()> {
    if !matches!(state, 5 | 6) {
        return Err(format!("end challenge small state {state} is not translated").into());
    }
    set_end_options(root, assets, view)
}

/// The movie's `InitAction`: the `endchallenge_small` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 69, 0x2480))
}

/// Timeline actions besides `Stop`: root frame 0 traces and constructs the class, frame 9
/// calls `IntroComplete` and frame 18 `OutroComplete`, each then stopping.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x2440 => clip.calls.push("endchallenge_small"),
        0x2a14 => {
            clip.calls.push("IntroComplete");
            clip.playback.stop();
        }
        0x2a44 => {
            clip.calls.push("OutroComplete");
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated end challenge small APT action {offset:#x}").into()),
    }
    Ok(())
}
