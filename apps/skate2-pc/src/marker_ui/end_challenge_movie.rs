//! `FrontEndState_EndChallengeOptions`'s screen 41 for a won challenge with several players,
//! `hudComponents/endchallengeoptions` (asset 60, fingerprint 3fe11302; template entry
//! `EndChallengeOptions` `0x82fdb5e8`, placement 4, the `g3dSpinContainer`). Its `InitAction`
//! (character 47, `0x1eb0`) defines the `endchallengeoptions` class translated here; root action
//! `0x1e54` constructs it. `mResults` is `controls/results_lb`'s `RESULTS_LB` (asset 61, the
//! `results_lb` class), whose `mcList` is `controls/dynamic_selectable_list`'s `Selectable_List`
//! (asset 62, the `dynamic_selectable_list` class) filled with the `LIST_TEXTBOX` and
//! `LIST_NUMBERBOX` cells of `controls/list_item_textbox` and `list_item_numberbox` (assets 63
//! and 64); the list's hidden demo cells come from `list_item_awardbox`, `rank_badges` and
//! `list_item_bonusbox` (assets 65 to 67). Of the members the class writes, the movie authors
//! only `glow_mc`, `mBackLarge`, `mResults`, `mcMiddleInfo` and `mHighlightLarge`;
//! `ActionScript` ignores the rest. What its natives answer comes from [`EndOptionsView`]:
//! `Mission.GetWinState`, `GetTitle`, `GetNumEndMissionOptions` and `GetEndMissionOptionText`,
//! and `HUDComponents.Results_GetNumPlayers`, `Results_GetLayoutType`,
//! `Results_GetLeaderboardDataBlock`, `Results_GetLeaderboardNameBlock` and
//! `Results_GetLeaderboardColTitle`. `OnEnter`'s `cMsgResultScreenEffect` to the presentation is
//! not ported.
use super::{
    assets::Assets,
    clips::Clip,
    localizer, map_movie,
    objective_movie::{set_x, set_y},
    pause_movie::child,
    results_list::{ResultsList, ResultsView, layout},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    EndOptionsView,
    challenge::{
        results::{OPTION_TEXT, option},
        system::MissionTitle,
    },
    ui::apt_projection::Spin,
};

pub(super) const ASSET: usize = 60;
/// Screen43, template82fdb608, endchallenge_gvr fingerprint3bce2a31.
pub(super) const GVR_ASSET: usize = 84;
pub(super) const RESULTS_LB: usize = 61;
pub(super) const SELECTABLE_LIST: usize = 62;
pub(super) const LIST_TEXTBOX: usize = 63;
pub(super) const LIST_NUMBERBOX: usize = 64;
pub(super) const LIST_AWARDBOX: usize = 65;
pub(super) const RANK_BADGES: usize = 66;
pub(super) const LIST_BONUSBOX: usize = 67;

#[derive(Default)]
pub(super) struct EndChallengeMovie {
    pub root: Option<Clip>,
    /// FE state 8 is open on this screen: `OnEnter` opened it and `PopState` has not closed it.
    pub open: bool,
    intro_complete: bool,
    outro_complete: bool,
    results: Option<ResultsList>,
    previous_view: Option<EndOptionsView>,
}

impl EndChallengeMovie {
    /// `OnEnter`'s `OpenScreen(41)` for a newly open state: a fresh screen, constructed (its
    /// constructor plays `intro`, which starts the spin); `OnLeave`'s `CloseScreen` once the
    /// state pops: `outro` and the spin back.
    pub fn update(&mut self, assets: &Assets, view: Option<&EndOptionsView>, title: &MissionTitle, spin: &mut Spin) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                let mut root = Clip::new(assets, if view.options.screen == skate2_core::challenge::results::screen::RESULTS_GVR { GVR_ASSET } else { ASSET }, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "endchallengeoptions" | "endchallenge_gvr" => self.results = Some(construct(&mut root, assets, view, title)?),
                        _ => return Err(format!("untranslated end challenge options callback {call}").into()),
                    }
                }
                spin.intro_start();
                self.root = Some(root);
                self.previous_view = Some(view.clone());
                self.calls()?;
            }
            Some(view) if self.previous_view.as_ref() != Some(view) => {
                if let (Some(root), Some(results)) = (self.root.as_mut(), self.results.as_mut()) {
                    // OptionSelect8267707c/826770cc: SetScreenState(6), preserving
                    // the constructed list and screen timeline (no second intro).
                    set_state(root, assets, view, results, 6)?;
                }
                self.previous_view = Some(view.clone());
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
                _ => return Err(format!("untranslated end challenge options callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new endchallengeoptions(this)`: with players, `mcMiddleInfo` and `glow_mc` play `multi`
/// and the challenge name fields take `Mission.GetTitle()`; the Goofy vs Regular integers move
/// `mResults` to (`GOOFY_LEADERBOARD_X`, `GOOFY_LEADERBOARD_Y`) (380, 370); the screen plays
/// `intro`; then `SetEndOptions` and `SetState(5)`. `mResults` is constructed first.
fn construct(root: &mut Clip, assets: &Assets, view: &EndOptionsView, title: &MissionTitle) -> RenderResult<ResultsList> {
    let mut results = ResultsList::new(root.named(&["mResults"])?, assets)?;
    if view.leaderboard.num_players <= 0 {
        return Err("end challenge options without players is not translated".into());
    }
    root.named(&["mcMiddleInfo"])?.goto(assets, "multi")?;
    root.named(&["glow_mc"])?.goto(assets, "multi")?;
    let name = localizer::mission_title(assets, title)?;
    for field in ["mcTextChallenegeName", "mcTextChallenegeNameGlow"] {
        root.named(&["mcMiddleInfo", field])?.text = Some(name.clone());
    }
    if root.asset == GVR_ASSET && view.leaderboard.results_type == layout::GOOFY_VS_REGULAR_INT {
        // endchallenge_gvr constructor0x230b..2394: final totals, goofy then regular.
        for (names, value) in [(["mLeft", "mLeftGlow"], view.team_totals[0]), (["mRight", "mRightGlow"], view.team_totals[1])] {
            for name in names {
                root.named(&["mBackLarge", "mTotalScores", name])?.text = Some(value.to_string().encode_utf16().collect());
            }
        }
    }
    if root.asset == ASSET && view.leaderboard.results_type == layout::GOOFY_VS_REGULAR_INT {
        let piece = child(root, "mResults")?;
        set_x(piece, 380.);
        set_y(piece, 370.);
    }
    root.goto(assets, "intro")?;
    set_end_options(root, assets, view)?;
    set_state(root, assets, view, &mut results, 5)?;
    Ok(results)
}

/// `SetEndOptions`: `mBackLarge.mButtonMenu` cleared and given the options X, B and A while
/// there are more than two, one and none (the button menu's constructor justification and
/// `SetInfo("Marker_streak", 24, 0xffffec)`).
fn set_end_options(root: &mut Clip, assets: &Assets, view: &EndOptionsView) -> RenderResult<()> {
    let count = view.leaderboard.num_options();
    let mut items = Vec::new();
    for (slot, button) in [(2, "X"), (1, "B"), (0, "A")] {
        if count > slot {
            items.push((button, option_text(view, slot)));
        }
    }
    map_movie::configure_button_items(root.named(&["mBackLarge", "mButtonMenu"])?, assets, &items, 0xffff_ffec)
}

/// `Mission.GetEndMissionOptionText(slot)`: the current leaderboard's option (the
/// Continue_Retry and Retry_Quit pairs by the win state) as its `gEndMissionObjectives` text,
/// `#` for Unknown.
pub(super) fn option_text(view: &EndOptionsView, slot: usize) -> &'static str {
    let option = view.leaderboard.option(slot, view.options.win);
    if option == option::UNKNOWN { "#" } else { usize::try_from(option).ok().and_then(|o| OPTION_TEXT.get(o)).copied().unwrap_or("#") }
}

/// `SetState`: 5 gives `mResults` the win state, then `InitData` (which `results_lb` lacks),
/// `UpdateAll`, `CenterList` (empty) and `SetEndOptions`.
fn set_state(root: &mut Clip, assets: &Assets, view: &EndOptionsView, results: &mut ResultsList, state: i32) -> RenderResult<()> {
    if !matches!(state, 5 | 6) {
        return Err(format!("end challenge options state {state} is not translated").into());
    }
    let list = root.named(&["mResults"])?;
    if state == 5 {
        results.set_win_state(list, assets, view.options.win)?;
    }
    let data = ResultsView { current: &view.leaderboard, second: &view.second, stance_regular: view.stance_regular };
    results.update_all(list, assets, &data)?;
    set_end_options(root, assets, view)
}

/// The movie's `InitAction`: the `endchallengeoptions` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 47, 0x1eb0) | (GVR_ASSET, 53, 0x21e0))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class, frame 9 calls
/// `IntroComplete` and frame 18 `OutroComplete`, each then stopping.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x1e54 => clip.calls.push("endchallengeoptions"),
        0x2184 => clip.calls.push("endchallenge_gvr"),
        0x2444 | 0x2758 => {
            clip.calls.push("IntroComplete");
            clip.playback.stop();
        }
        0x2474 | 0x2788 => {
            clip.calls.push("OutroComplete");
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated end challenge options APT action {offset:#x}").into()),
    }
    Ok(())
}
