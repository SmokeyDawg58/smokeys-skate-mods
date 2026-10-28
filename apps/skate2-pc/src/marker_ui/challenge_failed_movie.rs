//! `FrontEndState_EndChallengeOptions`' screen 44 for a lost challenge with several players,
//! `hudComponents/challengefailed` (fingerprint eb5a8c10; template entry `EndChallengeOptionsFail`
//! `0x82fdb618`, placement 0). Its `InitAction` (character 27, `0x244c`) defines the
//! `challengefailed` class translated here; root action `0x23f8` constructs it. `Initialise`,
//! `Show`, `OnLoaded`, `UpdateInput` and `SetState` are empty. `mResults` is `controls/results_lb`
//! ([`ResultsList`]). What its natives answer comes from [`EndOptionsView`]:
//! `Mission.GetNumEndMissionOptions` and `GetEndMissionOptionText`, and the results list's
//! `HUDComponents` natives.
use super::{
    apt_random::AptRandom,
    assets::Assets,
    clips::Clip,
    end_challenge_movie::option_text,
    map_movie,
    results_list::{ResultsList, ResultsView},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::EndOptionsView;

pub(super) const ASSET: usize = 80;

#[derive(Default)]
pub(super) struct ChallengeFailedMovie {
    pub root: Option<Clip>,
    /// FE state 8 is open on this screen: `OnEnter` opened it and `PopState` has not closed it.
    pub open: bool,
    intro_complete: bool,
    outro_complete: bool,
}

impl ChallengeFailedMovie {
    /// `OnEnter`'s `OpenScreen(44)` for a newly open state: a fresh screen, constructed (its
    /// constructor plays `intro`); `OnLeave`'s `CloseScreen` once the state pops: `outro`.
    pub fn update(&mut self, assets: &Assets, view: Option<&EndOptionsView>, random: &mut AptRandom) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "challengefailed" => construct(&mut root, assets, view, random)?,
                        _ => return Err(format!("untranslated challenge failed callback {call}").into()),
                    }
                }
                self.root = Some(root);
                self.calls()?;
            }
            None if self.open => {
                self.open = false;
                if let Some(root) = self.root.as_mut() {
                    root.goto(assets, "outro")?;
                }
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

    /// Frame 38's `IntroComplete` and frame 48's `OutroComplete`, which unloads it.
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
                _ => return Err(format!("untranslated challenge failed callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new challengefailed(this)`: one of `ID_CHALLENGE_FAILED_COMMENT0` to `19` as the title; the
/// button menu cleared; with players, `mEndOptions` and `failTitle` playing `multi` and
/// `mResults` given the loss (`SetWinState(false)`, `InitData`, which `results_lb` lacks,
/// `UpdateAll`, then the empty `CenterList`), then X, B and A; without, the menu left-justified,
/// `mResults`, `mcHighlight` and `mBackgroundFail` hidden, then X, A and B; each item while there
/// are more options than two, one and none. Then `intro`, one of the first five comments as the
/// title, and `RefreshItems`. The `mStroke` and `mTableBacking` it hides are not in this movie.
fn construct(root: &mut Clip, assets: &Assets, view: &EndOptionsView, random: &mut AptRandom) -> RenderResult<()> {
    let mut results = ResultsList::new(root.named(&["mResults"])?, assets)?;
    set_title(root, assets, random.index(20))?;
    let players = view.leaderboard.num_players > 0;
    if players {
        root.named(&["mEndOptions"])?.goto(assets, "multi")?;
        root.named(&["failTitle"])?.goto(assets, "multi")?;
        let list = root.named(&["mResults"])?;
        results.set_win_state(list, assets, false)?;
        let data = ResultsView { current: &view.leaderboard, second: &view.second, stance_regular: view.stance_regular };
        results.update_all(list, assets, &data)?;
    } else {
        for name in ["mResults", "mcHighlight", "mBackgroundFail"] {
            root.named(&[name])?.visible = false;
        }
    }
    let order = if players { [("X", 2), ("B", 1), ("A", 0)] } else { [("X", 2), ("A", 0), ("B", 1)] };
    let count = view.leaderboard.num_options();
    let items: Vec<(&str, &str)> = order
        .into_iter()
        .zip([2, 1, 0])
        .filter(|&(_, more_than)| count > more_than)
        .map(|((button, slot), _)| (button, option_text(view, slot)))
        .collect();
    root.goto(assets, "intro")?;
    set_title(root, assets, random.index(5))?;
    let menu = root.named(&["mEndOptions", "mButtonMenu"])?;
    if players {
        map_movie::configure_button_items(menu, assets, &items, 0xffff_ffec)
    } else {
        map_movie::configure_left_button_items(menu, assets, &items, 0xffff_ffec)
    }
}

/// `failTitle.title_txt` and `titleGlow_txt` showing `ID_CHALLENGE_FAILED_COMMENT<index>`.
fn set_title(root: &mut Clip, assets: &Assets, index: usize) -> RenderResult<()> {
    let text = assets.initial_text(&format!("ID_CHALLENGE_FAILED_COMMENT{index}"));
    for field in ["title_txt", "titleGlow_txt"] {
        root.named(&["failTitle", field])?.text = Some(text.clone());
    }
    Ok(())
}

/// The movie's `InitAction`: the `challengefailed` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 27, 0x244c))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class, frame 38 calls
/// `IntroComplete` and frame 48 `OutroComplete`, each then stopping; frame 39 only traces.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x23f8 => clip.calls.push("challengefailed"),
        0x287c => {
            clip.calls.push("IntroComplete");
            clip.playback.stop();
        }
        0x28a0 => {}
        0x28b4 => {
            clip.calls.push("OutroComplete");
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated challenge failed APT action {offset:#x}").into()),
    }
    Ok(())
}
