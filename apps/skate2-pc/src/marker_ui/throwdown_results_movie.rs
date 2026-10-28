//! `FrontEndState_EndChallengeOptions`'s screen 47 for a throwdown, challenge/throwdownresults
//! (asset 53, fingerprint a1423ba4; template entry `ThrowDownEndChallengeOptions` `0x82fdb648`,
//! placement 4, the `g3dSpinContainer`). Its InitAction (character 80, `0x28fc`) defines the `throwdownresults` class
//! translated here; root action `0x28d4` constructs it. What its `Mission` and `HUDComponents`
//! natives answer comes from [`EndOptionsView`]: `GetEndMissionThrowDownType`, `GetWinState`,
//! `GetNumEndMissionOptions`, `GetEndMissionOptionText` (`gEndMissionObjectives`),
//! `Results_GetNumPlayers`, `Results_GetLeaderboardDataBlock`, `Results_GetLeaderboardNameBlock`
//! and `Results_GetSelfRank`.
use super::{
    assets::Assets,
    clips::Clip,
    end_challenge_movie::option_text,
    hud_leaderboard_movie::{find, find_child},
    localizer, map_movie,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    EndOptionsView,
    ui::apt_projection::Spin,
    challenge::{
        results::throwdown,
        system::MissionTitle,
    },
};

pub(super) const ASSET: usize = 53;
/// The rows the screen authors.
const ROWS: i32 = 8;

#[derive(Default)]
pub(super) struct ThrowdownResultsMovie {
    pub root: Option<Clip>,
    /// FE state 8 is open: `OnEnter` opened the screen and `PopState` has not closed it.
    pub open: bool,
    intro_complete: bool,
    outro_complete: bool,
}

impl ThrowdownResultsMovie {
    /// `OnEnter`'s `OpenScreen(47)` for a newly open state: a fresh screen, constructed (its
    /// constructor plays `intro`, which starts the spin); `OnLeave`'s `CloseScreen` once the
    /// state pops: `outro` and the spin back.
    pub fn update(&mut self, assets: &Assets, view: Option<&EndOptionsView>, title: &MissionTitle, spin: &mut Spin) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "throwdownresults" => construct(&mut root, assets, view, title)?,
                        _ => return Err(format!("untranslated throwdown results callback {call}").into()),
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

    /// Frame 8's `IntroComplete` and frame 29's `OutroComplete`, which unloads it.
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
                _ => return Err(format!("untranslated throwdown results callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new throwdownresults(this)`: `RegisterScreen`; `mThrowDown` plays its `1up` or `skate`
/// section for the throwdown type; the screen plays `intro`; then `SetEndOptions` and
/// `SetState(5)`.
fn construct(root: &mut Clip, assets: &Assets, view: &EndOptionsView, title: &MissionTitle) -> RenderResult<()> {
    match view.options.throwdown {
        throwdown::ONE_UP => root.named(&["mThrowDown"])?.goto(assets, "1up")?,
        throwdown::SKATE => root.named(&["mThrowDown"])?.goto(assets, "skate")?,
        _ => {}
    }
    root.goto(assets, "intro")?;
    set_end_options(root, assets, view, title)?;
    set_state(root, assets, view, title, 5)
}

/// `SetEndOptions`: the title in `mTitleHolder`, then `mEndOptions.mButtonMenu` cleared,
/// right-justified, `SetInfo("Marker_streak", 24, 0)` (black) and the options X, B and A
/// added while there are more than two, one and none.
fn set_end_options(root: &mut Clip, assets: &Assets, view: &EndOptionsView, title: &MissionTitle) -> RenderResult<()> {
    let title = localizer::mission_title(assets, title)?;
    for field in ["mcTitle", "mcTitleGlow"] {
        root.named(&["mThrowDown", "mTitleHolder", field])?.text = Some(title.clone());
    }
    let count = view.leaderboard.num_options();
    let mut items = Vec::new();
    for (slot, button) in [(2, "X"), (1, "B"), (0, "A")] {
        if count > slot {
            items.push((button, option_text(view, slot)));
        }
    }
    map_movie::configure_button_items(root.named(&["mThrowDown", "mEndOptions", "mButtonMenu"])?, assets, &items, 0xff00_0000)
}

/// `SetState`: states 5 and 6 hide the rows past the players (`mcUp` is not authored, and
/// the `skate` section authors rows 0 to 3 only, so writes to the others do nothing), empty
/// their `mcSkate` and fade their strips to half, then `FillData` and `SetEndOptions`.
fn set_state(root: &mut Clip, assets: &Assets, view: &EndOptionsView, title: &MissionTitle, state: i32) -> RenderResult<()> {
    if state != 5 && state != 6 {
        return Ok(());
    }
    let throw_down = root.named(&["mThrowDown"])?;
    for row in view.leaderboard.num_players.max(0)..ROWS {
        if let Some(name) = find(throw_down, &[&format!("playerName{row}")]) {
            name.visible = false;
        }
        if let Some(skate) = find(throw_down, &[&format!("mcSkate{row}")]) {
            skate.goto(assets, "empty")?;
        }
        if let Some(strip) = find_child(throw_down, &format!("mcStrip{row}")) {
            strip.entry.colour_scale = (strip.entry.colour_scale & 0x00ff_ffff) | 0x7f00_0000;
        }
    }
    fill_data(root, assets, view)?;
    set_end_options(root, assets, view, title)
}

/// `FillData`: `UpdateHighlight` of the self rank, then each player's name in its row and,
/// for a score below 6, `mcSkate` playing the score's label. The background's
/// `ScaleVertically` names `mBackground`, which the movie authors as `mcBackground`, so it
/// does nothing.
fn fill_data(root: &mut Clip, assets: &Assets, view: &EndOptionsView) -> RenderResult<()> {
    update_highlight(root, assets, view.leaderboard.self_rank)?;
    let throw_down = root.named(&["mThrowDown"])?;
    let rows = usize::try_from(view.leaderboard.num_players.max(0)).unwrap_or(0);
    for (row, player) in view.leaderboard.players.iter().enumerate().take(rows) {
        let name = assets.initial_text(&player.name);
        let label = throw_down.named(&[&format!("playerName{row}")])?;
        for field in ["name_label", "name_labelGlow"] {
            label.named(&[field])?.text = Some(name.clone());
        }
        if player.score < 6 {
            throw_down.named(&[&format!("mcSkate{row}")])?.goto(assets, &player.score.to_string())?;
        }
    }
    Ok(())
}

/// `UpdateHighlight(rank)`: the rank's row strip and name stop on `highlight`, the others on
/// `none`.
fn update_highlight(root: &mut Clip, assets: &Assets, rank: i32) -> RenderResult<()> {
    let throw_down = root.named(&["mThrowDown"])?;
    for row in 0..ROWS {
        let state = if row == rank - 1 { "highlight" } else { "none" };
        for name in [format!("mcStrip{row}"), format!("playerName{row}")] {
            if let Some(clip) = find(throw_down, &[&name]) {
                clip.goto_and_stop(assets, state)?;
            }
        }
    }
    Ok(())
}

/// The movie's InitAction: the `throwdownresults` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 80, 0x28fc))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class, frame 8 calls
/// `IntroComplete` and frame 29 `OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x28d4 => clip.calls.push("throwdownresults"),
        0x2f80 => clip.calls.push("IntroComplete"),
        0x2fbc => clip.calls.push("OutroComplete"),
        _ => return Err(format!("untranslated throwdown results APT action {offset:#x}").into()),
    }
    Ok(())
}
