//! `FrontEndState_EndChallengeOptions`'s online display (`mIsOnlineDisplay`): screen 117,
//! `ranked/ranked_results_overall` (asset 112, fingerprint 203d2cc7; template entry
//! `RankedOnlineEndChallengeOptions` `0x82fdbaa8`, placement 0; class `ranked_results_overall`,
//! InitAction character 132 `0x6a98`). Its `mList` is `controls/dynamic_selectable_list` driven
//! directly (`InitDataArray`), its `mChallengeIcon` an `image_renderer` that loads the challenge's
//! icon, and its rank-up panel's `mRankIcon` a `controls/challengesignup_icon` (asset 113, fingerprint ed9556a4). What
//! its natives answer comes from [`EndOptionsView`]'s final results and
//! [`OnlineResults`]: `Mission.GetOnlineChallengeInstanceData`, `GetOnlineChallengeTypeName`,
//! `Results_GetOnlineQuitButtonEnabled`, `EndMission_Online_GetFinalResultsTimer`, and
//! `HUDComponents.Results_GetNumPlayers`, `Results_GetLayoutType`,
//! `Results_GetLeaderboardDataBlock`, `Results_GetLeaderboardNameBlock` and
//! `Results_GetOnlineLeaderboardCashAndEXPDataBlock`. The unranked layouts are the ones the port
//! plays; the ranked ones, the rank-up and badge panels and Xbox LIVE's bumper buttons have no
//! service behind them.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    hud_leaderboard_movie::split_time,
    localizer, map_movie,
    objective_movie::{set_x, text_field_width},
    online_hub_movie,
    pause_movie::child,
    results_list::{OnlineList, OnlineListLayout, Value, layout, number_to_distance},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{EndOptionsView, challenge::results::OnlineResults};
use skate2_formats::apt::Character;

pub(super) const ASSET: usize = 112;
pub(super) const ICON: usize = 113;
/// `playerBoldColor`, `selectorSize` and `listRowHeight`.
const BOLD_COLOUR: u32 = 0x00f8_f27e;
const SELECTOR_SIZE: f32 = 762.;
const ROW_HEIGHT: f32 = 38.;
/// The rows `mSkateCol` authors (`mcSkate0` to `mcSkate5`, `ROW_HEIGHT` apart), the rows of
/// the list `normal` has room for.
const AUTHORED_ROWS: usize = 6;
/// The S.K.A.T.E. rows a game's players fill.
const SKATE_ROWS: usize = skate2_net::protocol::MAX_PLAYERS;

/// `mScreenMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Final,
    Round,
    FreeskateWin,
    FreeskateFail,
    Skate,
}

#[derive(Default)]
pub(super) struct OnlineResultsMovie {
    pub root: Option<Clip>,
    pub open: bool,
    intro_complete: bool,
    outro_complete: bool,
    list: Option<OnlineList>,
    mode: Option<Mode>,
    shown: Option<OnlineResults>,
    pub sounds: Vec<&'static str>,
}

impl OnlineResultsMovie {
    /// `OnEnter`'s `OpenScreen(117)`: a fresh screen whose constructor sets it up, each time the
    /// display opens anew; its `SetScreenState`s as the timers, the quit button and the list's
    /// row change; `CloseScreen`'s outro once the state pops.
    pub fn update(&mut self, assets: &Assets, view: Option<&EndOptionsView>) -> RenderResult<()> {
        let online = view.and_then(|view| view.online.as_ref().map(|online| (view, online)));
        match online {
            Some((view, online)) if !self.open || self.shown.as_ref().is_some_and(|shown| shown.instance != online.instance) => {
                self.open = true;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "ranked_results_overall" => {
                            let (list, mode) = construct(&mut root, assets, view, online)?;
                            self.list = Some(list);
                            self.mode = Some(mode);
                        }
                        _ => return Err(format!("untranslated online results callback {call}").into()),
                    }
                }
                self.root = Some(root);
                // OnEnter clears mOnlineResultsTimer and mOnlineRoundResultsTimer; OnUpdate shows them.
                self.shown = Some(OnlineResults { timer: 0, round_timer: 0, ..online.clone() });
                self.calls()?;
            }
            Some((_, online)) => {
                let previous = self.shown.replace(online.clone());
                let (Some(root), Some(list), Some(mode)) = (self.root.as_mut(), self.list.as_mut(), self.mode) else { return Ok(()) };
                // OnUpdate's SetScreenState(100): UpdateTimerValue, a round's timer or the board's.
                let timer = |online: &OnlineResults| if mode == Mode::Round { online.round_timer } else { online.timer };
                if previous.as_ref().is_none_or(|previous| timer(previous) != timer(online)) {
                    if matches!(mode, Mode::Final | Mode::Round | Mode::Skate) {
                        let text = format!("#{}", timer(online));
                        root.named(&["mWindow", "mContent", "txtNextTimer"])?.text = Some(assets.initial_text(&text));
                    }
                }
                // SetEnableOnlineQuitButton's SetScreenState(4): SetButtonOptions.
                if previous.as_ref().is_none_or(|previous| previous.quit_enabled != online.quit_enabled) {
                    set_button_options(root, assets, online)?;
                }
                // The list's UpdateInput: NextValue and PrevValue to the row.
                let list_clip = root.named(&["mWindow", "mContent", "mListHolder", "mList"])?;
                while list.current() < online.selected {
                    let before = list.current();
                    list.update_input(list_clip, assets, true)?;
                    if list.current() == before {
                        break;
                    }
                }
                while list.current() > online.selected {
                    let before = list.current();
                    list.update_input(list_clip, assets, false)?;
                    if list.current() == before {
                        break;
                    }
                }
            }
            None if self.open => {
                self.open = false;
                if let Some(root) = self.root.as_mut() {
                    root.goto(assets, "outro")?;
                }
                self.calls()?;
            }
            None => {}
        }
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(root) = self.root.as_mut() {
            root.tick(assets)?;
        }
        self.calls()
    }

    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    pub fn take_outro_complete(&mut self) -> bool {
        std::mem::take(&mut self.outro_complete)
    }

    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "FadeSound" => self.sounds.push("FE_SOUND_CORE_FADE"),
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => {
                    self.outro_complete = true;
                    self.root = None;
                    self.list = None;
                    break;
                }
                _ => return Err(format!("untranslated online results callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new ranked_results_overall(this)`: the S.K.A.T.E. letters hidden, the mode by the instance
/// data (`SetupSkate`, `SetupActivityWin`/`Fail` or `SetupNormal`, the ranked columns shown only
/// ranked), then `SetInstanceNameData`, `SetState(5)` (`InitDataArray`), `SetTitle` and
/// `SetButtonOptions`; no rank or badge is earned without the profile service; the timer starts
/// empty and the list takes input.
fn construct(root: &mut Clip, assets: &Assets, view: &EndOptionsView, online: &OnlineResults) -> RenderResult<(OnlineList, Mode)> {
    let window = root.named(&["mWindow"])?;
    let column = window.named(&["mSkateCol"])?;
    for index in 0..SKATE_ROWS {
        skate_row(column, assets, index)?.clip.visible = false;
    }
    let mode = if online.skate {
        Mode::Skate
    } else if online.free_skate {
        if online.activity_won { Mode::FreeskateWin } else { Mode::FreeskateFail }
    } else if online.round {
        Mode::Round
    } else {
        Mode::Final
    };
    if online.ranked {
        return Err("ranked online results are not translated".into());
    }
    // More players than `normal`'s rows take `full`, the rank-up's layout (`SetupNewRank`), as
    // it does before the screen's text is set: the list 40 higher, the next label, timer and
    // buttons 80 lower; its rank-up panel stays hidden.
    let players = usize::try_from(view.leaderboard.num_players).unwrap_or(0).min(view.leaderboard.players.len());
    let full = players > AUTHORED_ROWS && matches!(mode, Mode::Skate | Mode::Final | Mode::Round);
    window.goto(
        assets,
        match mode {
            Mode::Skate | Mode::Final | Mode::Round if full => "full",
            Mode::Skate | Mode::Final | Mode::Round => "normal",
            Mode::FreeskateWin => "win",
            Mode::FreeskateFail => "fail",
        },
    )?;
    if full {
        window.named(&["mContent"])?.goto(assets, "full")?;
        window.named(&["mRankUp"])?.visible = false;
    }
    let content = window.named(&["mContent"])?;
    content.named(&["mEXPColHeader"])?.visible = false;
    content.named(&["mCashColHeader"])?.visible = false;
    set_instance_name_data(root, assets, online, mode)?;
    let list = init_data_array(root, assets, view, online, mode)?;
    set_title(root, assets, online, mode)?;
    set_button_options(root, assets, online)?;
    let content = root.named(&["mWindow", "mContent"])?;
    content.named(&["txtNextTimer"])?.text = Some(Vec::new());
    Ok((list, mode))
}

/// `SetInstanceNameData`: the points column's title (hidden when empty), the challenge's title
/// and icon, and what comes next: a round's next round (nothing after the last), a ranked
/// game's next challenge, an unranked game's lobby, or freeskate.
fn set_instance_name_data(root: &mut Clip, assets: &Assets, online: &OnlineResults, mode: Mode) -> RenderResult<()> {
    let content = root.named(&["mWindow", "mContent"])?;
    let header = content.named(&["mPointsColHeader"])?;
    header.text = Some(assets.initial_text(&online.column_header));
    header.visible = !online.column_header.is_empty();
    let info = content.named(&["mChallengeInfo"])?;
    info.named(&["txtChallengeTitle"])?.text = Some(assets.initial_text(&online.title));
    // image_renderer LoadAssetByName: FEImageRenderer::AddTextureByAssetName of the icon.
    if let Some(key) = asset_texture(assets, &online.icon) {
        info.named(&["mChallengeIcon"])?.properties.insert("cac_texture".into(), key);
    }
    let next = match mode {
        Mode::Round if online.last_round => "#",
        Mode::Round => "ID_ONLINE_CHALLENGE_NEXT_ROUND",
        Mode::Final | Mode::Skate if online.ranked => "ID_ONLINE_CHALLENGE_LOADS_IN",
        Mode::Final | Mode::Skate => "ID_ONLINEPOSTRESULTS_RETURNINGTOLOBBY",
        Mode::FreeskateWin | Mode::FreeskateFail => "ID_ONLINE_BACK_TO_FREESKATE",
    };
    content.named(&["txtNextLabel"])?.text = Some(assets.initial_text(next));
    Ok(())
}

/// The loaded texture an asset name names: `FETextureAsset(assetName)` (0x82607838) hashes the
/// name with ".Texture" (the challenge icons are `minimap.rx2`'s).
fn asset_texture(assets: &Assets, name: &str) -> Option<String> {
    assets.texture(&format!("{name}.Texture")).ok().map(|texture| texture.name.clone())
}

/// `InitDataArray`: the list's columns for the mode and layout, each player's row by rank
/// (`mPlayerIndexByRank`), S.K.A.T.E.'s letters beside their rows, the first row bold, the
/// players' choice's thumbs-up, and the online selection bar.
fn init_data_array(root: &mut Clip, assets: &Assets, view: &EndOptionsView, online: &OnlineResults, mode: Mode) -> RenderResult<OnlineList> {
    const NUMBER: &str = "LIST_NUMBERBOX";
    const TEXT: &str = "LIST_TEXTBOX";
    let board = &view.leaderboard;
    let kind = board.results_type;
    let (widths, alignments, definition): (Vec<f32>, Vec<&'static str>, Vec<&'static str>) = if mode == Mode::Skate {
        (vec![72., 458.], vec!["center", "left"], vec![NUMBER, TEXT])
    } else if online.column_header.is_empty() {
        (vec![72., 324.], vec!["center", "left"], vec![NUMBER, TEXT])
    } else if matches!(kind, layout::DEATH_RACE | layout::PLACE_NAME_STRING) {
        (vec![72., 510., 130.], vec!["center", "left", "center"], vec![NUMBER, TEXT, TEXT])
    } else if kind == layout::PLACE_NAME_SCORE_TIME {
        (vec![72., 396., 130., 110.], vec!["center", "left", "right", "right"], vec![NUMBER, TEXT, NUMBER, TEXT])
    } else if matches!(kind, layout::BEST_TRICK_FLOAT | layout::BEST_TRICK_NON_ELIM_FLOAT) {
        (vec![72., 510., 130.], vec!["center", "left", "center"], vec![NUMBER, TEXT, TEXT])
    } else {
        (vec![72., 510., 130.], vec!["center", "left", "center"], vec![NUMBER, TEXT, NUMBER])
    };
    let window = root.named(&["mWindow"])?;
    if mode == Mode::Skate || !online.column_header.is_empty() && kind != layout::PLACE_NAME_SCORE_TIME {
        let header = child(window.named(&["mContent"])?, "mPointsColHeader")?;
        set_x(header, 610. + 2.);
        if mode == Mode::Skate {
            auto_size_center(header, assets)?;
        }
    }
    if mode == Mode::Skate {
        let column_x = child(window, "mSkateCol")?.entry.matrix.tx;
        let column = window.named(&["mSkateCol"])?;
        for index in 0..SKATE_ROWS {
            set_x(skate_row(column, assets, index)?, 624. - column_x);
        }
    }
    let players = usize::try_from(board.num_players).unwrap_or(0).min(board.players.len());
    if mode == Mode::Skate {
        let column = window.named(&["mSkateCol"])?;
        for index in 0..players.min(SKATE_ROWS) {
            skate_row(column, assets, index)?.clip.visible = true;
        }
    }
    let mut rows = Vec::new();
    let mut thumb = None;
    for rank in 1..=i32::try_from(players).unwrap_or(0) {
        for (index, player) in board.players[..players].iter().enumerate() {
            if player.position != rank {
                continue;
            }
            let row = usize::try_from(rank - 1).unwrap_or(0);
            if usize::try_from(online.players_choice).ok() == Some(index) {
                thumb = Some(rank - 1);
            }
            let name = Value::Text(player.name.clone());
            let position = Value::Int(player.position);
            if mode == Mode::Skate {
                // The letters the score spells, by frame label.
                let letters = skate_row(window.named(&["mSkateCol"])?, assets, row)?;
                letters.clip.goto(assets, &player.score.to_string())?;
                rows.push(vec![position, name]);
                continue;
            }
            let time = if player.time < 0. {
                "ID_RACE_DNF".to_owned()
            } else {
                let (hours, minutes, seconds, hundredths) = split_time(player.time);
                format!("#{}", localizer::format_time(assets, hours, minutes, seconds, hundredths, true, 0))
            };
            rows.push(if online.column_header.is_empty() {
                vec![position, name]
            } else {
                match kind {
                    layout::DEATH_RACE => vec![position, name, Value::Text(time)],
                    layout::PLACE_NAME_STRING => vec![position, name, Value::Text(player.status.clone())],
                    layout::PLACE_NAME_SCORE_TIME => vec![position, name, Value::Int(player.score), Value::Text(time)],
                    layout::BEST_TRICK_FLOAT | layout::BEST_TRICK_NON_ELIM_FLOAT => {
                        vec![position, name, Value::Text(number_to_distance(assets, player.float_score)?)]
                    }
                    _ => vec![position, name, Value::Int(player.score)],
                }
            });
        }
    }
    let list_clip = root.named(&["mWindow", "mContent", "mListHolder", "mList"])?;
    let mut list = OnlineList::new(list_clip, assets)?;
    let layout = OnlineListLayout {
        column_gap: 4.,
        row_offset: ROW_HEIGHT,
        widths,
        alignments,
        definition,
        bolding: (0, BOLD_COLOUR),
        thumb,
        selector: SELECTOR_SIZE,
    };
    list.set_data(list_clip, assets, layout, rows)?;
    Ok(list)
}

/// `mSkateCol`'s row: an authored `mcSkate` row, or past them the same letters again,
/// `ROW_HEIGHT` further down each, for a seventh and eighth player.
fn skate_row<'a>(column: &'a mut Clip, assets: &Assets, index: usize) -> RenderResult<&'a mut Child> {
    let name = format!("mcSkate{index}");
    if !column.children.values().any(|row| row.entry.name.as_deref() == Some(name.as_str())) {
        let last = child(column, &format!("mcSkate{}", AUTHORED_ROWS - 1))?;
        let mut matrix = last.entry.matrix;
        matrix.ty += ROW_HEIGHT * index.saturating_sub(AUTHORED_ROWS - 1) as f32;
        let character = last.clip.character;
        let depth = column.children.keys().next_back().map_or(1, |depth| depth + 2);
        column.children.insert(depth, online_hub_movie::placed(Clip::new(assets, ASSET, character)?, character, &name, matrix));
    }
    child(column, &name)
}

/// `SetTitle`: the type (Unranked, or the freeskate activity's result) and the challenge type's
/// name after it, 12 further right.
fn set_title(root: &mut Clip, assets: &Assets, online: &OnlineResults, mode: Mode) -> RenderResult<()> {
    let (kind, title) = match mode {
        Mode::FreeskateWin => ("ID_COMMON_FREESKATEACTIVITY_WON", "#"),
        Mode::FreeskateFail => ("ID_COMMON_FREESKATEACTIVITY_LOST", "#"),
        Mode::Final | Mode::Round | Mode::Skate => ("ID_COMMON_UNRANKED", online.type_name.as_str()),
    };
    let heading = root.named(&["mWindow", "mContent", "mTitle"])?;
    for (type_field, title_field) in [("txtType", "txtTitle"), ("txtTypeGlow", "txtTitleGlow")] {
        heading.named(&[type_field])?.text = Some(assets.initial_text(kind));
        heading.named(&[title_field])?.text = Some(assets.initial_text(title));
        if mode == Mode::FreeskateFail {
            heading.named(&[title_field])?.properties.insert("text_colour".into(), (0xff00_0000_u32 | 1_580_322).to_string());
        }
        // AutoAlign(txtType, txtTitle): the type auto-sized left, the title after it.
        let type_piece = child(heading, type_field)?;
        type_piece.clip.properties.insert("text_autosize".into(), "left".into());
        let after = type_piece.entry.matrix.tx + text_field_width(&type_piece.clip, assets)?;
        set_x(child(heading, title_field)?, after + 12.);
    }
    Ok(())
}

/// `SetButtonOptions`: B to leave online once the quit is enabled.
fn set_button_options(root: &mut Clip, assets: &Assets, online: &OnlineResults) -> RenderResult<()> {
    let items: &[(&str, &str)] = if online.quit_enabled { &[("B", "ID_ONLINE_HUB_EXIT_ONLINE")] } else { &[] };
    map_movie::styled_button_items(root.named(&["mWindow", "mContent", "mButtonMenu"])?, assets, items, 0xffff_feec, false, "Marker_streak", 24.)
}

/// `autoSize = "center"` after an `_x` write: the field keeps its centre as it takes its text's
/// width.
fn auto_size_center(piece: &mut Child, assets: &Assets) -> RenderResult<()> {
    let Character::Text(text) = piece.clip.definition(assets)? else {
        return Err("online results auto-size target is not text".into());
    };
    let old_width = text.bounds.right - text.bounds.left;
    let extent = text_field_width(&piece.clip, assets)? - 4.;
    piece.clip.properties.insert("text_autosize".into(), "center".into());
    piece.clip.properties.insert("text_width".into(), extent.to_string());
    set_x(piece, piece.entry.matrix.tx + (old_width - extent) * 0.5);
    Ok(())
}

/// The InitActions: the screen's class and challengesignup_icon's class and registration.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 132, 0x6a98) | (ICON, 81, 0x1630) | (ICON, 80, 0x1738))
}

/// The root's actions: the constructor (and `ScreenManager.EnableDimmer`, which the screen
/// manager never defines), the fade sounds and `IntroComplete`/`OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (ASSET, 0x6a68) => clip.calls.push("ranked_results_overall"),
        (ASSET, 0x85d0 | 0x8614) => clip.calls.push("FadeSound"),
        (ASSET, 0x85f0) => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        (ASSET, 0x8634) => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated online results APT action {}:{offset:#x}", clip.asset).into()),
    }
    Ok(())
}
