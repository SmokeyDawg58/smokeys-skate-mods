//! `controls/results_lb` (asset 61, the `results_lb` class registered as `RESULTS_LB`) and the
//! `controls/dynamic_selectable_list` it drives (`mcList`, asset 62, `Selectable_List`) with the
//! `LIST_TEXTBOX` and `LIST_NUMBERBOX` cells of `controls/list_item_textbox` and
//! `list_item_numberbox` (assets 63 and 64), as the end-of-challenge screens use them: the
//! constructor, `SetWinState`, `UpdateAll` (`SetListData`) and `CreateTitles`. The list's hidden
//! demo cells come from `list_item_awardbox`, `rank_badges` and `list_item_bonusbox` (assets 65 to
//! 67). What the natives answer comes from [`ResultsView`]: `HUDComponents.Results_GetNumPlayers`,
//! `Results_GetLayoutType`, `Results_GetLeaderboardDataBlock` (`0x826b9608`),
//! `Results_GetLeaderboardNameBlock` (`0x826b9ba0`), `Results_GetLeaderboardColTitle`
//! (`0x826b9e20`), their `ResultsSecond_` counterparts (`0x826b9f58`, `0x826ba1a8`, `0x826ba2f8`),
//! `PlayerStance_IsRegular` (`0x826b4bd8`) and `NumberToDistance` (`0x826b2798`).
//!
//! The scripts build their arrays with `new Array(...)`, whose element 0 is the argument pushed
//! last (`AptActionInterpreter::_createObject` `0x82a136b0` reads the stack from its top).
use std::collections::BTreeMap;

use super::{
    assets::Assets,
    clips::{Child, Clip},
    end_challenge_movie::{RESULTS_LB, SELECTABLE_LIST},
    hud_leaderboard_movie::split_time,
    localizer,
    objective_movie::{set_x, set_xscale, set_y, text_field_width},
    pause_movie::{bounds, child},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    challenge::results::LeaderboardData,
    ui::apt_timeline::{DisplayEntry, IDENTITY, PlacementIdentity},
};
use skate2_formats::{apt::Character, localization::string_hash};

/// `results_lb`'s `TITLES` export.
const TITLES: u32 = 3;
/// `results_lb`'s `TITLES_HOLDER`, an empty one-frame sprite: what `createEmptyMovieClip` makes.
const EMPTY_CLIP: u32 = 4;
/// Flash keeps timeline instances 16384 depths below the ones scripts create.
const SCRIPT_DEPTH: i32 = 0x4000;
/// `dynamic_selectable_list.MAX_VISIBLE_ROWS`.
const MAX_VISIBLE_ROWS: i32 = 16;

/// `results_lb.eResultsLBType_*`.
pub(super) mod layout {
    pub const PLACE_NAME_INT: i32 = 0;
    pub const BEST_TRICK_FLOAT: i32 = 1;
    pub const BEST_TRICK_INT: i32 = 2;
    pub const DEATH_RACE: i32 = 3;
    pub const BEST_TRICK_NON_ELIM_FLOAT: i32 = 4;
    pub const BEST_TRICK_NON_ELIM_INT: i32 = 5;
    pub const PLACE_NAME_SCORE_TIME: i32 = 6;
    pub const PLACE_NAME_STRING: i32 = 7;
    pub const GOOFY_VS_REGULAR_TEAM_RESULTS: i32 = 8;
    pub const TEAM_LEADER_BOARDS_START: i32 = 9;
    pub const GOOFY_VS_REGULAR_INT: i32 = 10;
    pub const GOOFY_VS_REGULAR_FLOAT: i32 = 11;
}

/// What the results natives read: FE state 27's current and second current leaderboards
/// (`mCurrentLeaderboard`, `mSecondCurrentLeaderboard`) and the local player's stance.
pub(super) struct ResultsView<'a> {
    pub current: &'a LeaderboardData,
    pub second: &'a LeaderboardData,
    pub stance_regular: bool,
}

/// A `mList2dDataArray` value as its `AptValue`.
#[derive(Clone, Debug)]
pub(super) enum Value {
    Int(i32),
    Float(f32),
    /// A string as a text field takes it: a `#` literal or a HAL id.
    Text(String),
}

/// One player of a data block: the position, score and float score as they are, the status
/// string and the formatted time.
struct Block {
    position: i32,
    score: i32,
    float_score: f32,
    status: String,
    time: String,
}

/// `Results_GetLeaderboardDataBlock` (`0x826b9608`) and `ResultsSecond_GetLeaderboardDataBlock`
/// (`0x826b9f58`): `mPosition`, `mScore`, `mFloatScore`, `mPlayerStatus` and the time through
/// `FormatTime(..., true, 0)` (split as `split_time` does, `0x826b97ac`), `ID_RACE_DNF` below
/// zero; with the names (`mPlayerName`) of the name blocks.
fn blocks(assets: &Assets, board: &LeaderboardData) -> Vec<(Block, String)> {
    let count = usize::try_from(board.num_players).unwrap_or(0).min(board.players.len());
    board.players[..count]
        .iter()
        .map(|player| {
            let time = if player.time < 0. {
                "ID_RACE_DNF".to_owned()
            } else {
                let (hours, minutes, seconds, hundredths) = split_time(player.time);
                format!("#{}", localizer::format_time(assets, hours, minutes, seconds, hundredths, true, 0))
            };
            let block = Block {
                position: player.position,
                score: player.score,
                float_score: player.float_score,
                status: status_text(assets, &player.status),
                time,
            };
            (block, player.name.clone())
        })
        .collect()
}

/// The team layouts' player `index` of a board.
fn team(blocks: &[(Block, String)], index: usize) -> Result<&(Block, String), String> {
    blocks.get(index).ok_or_else(|| "results team block past its players".to_owned())
}

/// `mPlayerStatus` as `SetPlayerStatusHAL` (`0x82641b80`) wrote it: a set status is the
/// prefixed `FormatString` of its HAL id (every argument a string) cut to 19 bytes; the reset
/// `#NOSTATUS` stays.
fn status_text(assets: &Assets, status: &str) -> String {
    if status.starts_with('#') {
        return status.to_owned();
    }
    let formatted = localizer::format_hal(assets, string_hash(status.as_bytes())).map(|units| String::from_utf16_lossy(&units)).unwrap_or_default();
    let mut text = format!("#{formatted}");
    let mut end = text.len().min(19);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}

/// `NumberToDistance` (`0x826b2798`): a float above zero (else zero) to one decimal through
/// `ID_COMMON_DISTANCE`.
pub(super) fn number_to_distance(assets: &Assets, value: f32) -> RenderResult<String> {
    let value = if value > 0. { value } else { 0. };
    let text = localizer::format_custom(assets, "ID_COMMON_DISTANCE", &format!("{:.1}", f64::from(value)))?;
    Ok(format!("#{}", String::from_utf16_lossy(&text)))
}

/// `results_lb` on its screen's `mResults`.
pub(super) struct ResultsList {
    list: SelectableList,
    num_titles: i32,
}

impl ResultsList {
    /// The `results_lb` constructor, after `mcList`'s own: no titles, then
    /// `mcList.SetUseColWidths(true)`.
    pub fn new(results: &mut Clip, assets: &Assets) -> RenderResult<Self> {
        let mut list = SelectableList::new(results.named(&["mcList"])?, assets)?;
        list.use_fixed_col_widths = true;
        Ok(Self { list, num_titles: 0 })
    }

    /// `SetWinState(winState)`: a win gives the list the `DARK_ON_LIGHT` bar.
    pub fn set_win_state(&mut self, results: &mut Clip, assets: &Assets, win: bool) -> RenderResult<()> {
        if win {
            self.list.set_selection_bar_skin(results.named(&["mcList"])?, assets, "DARK_ON_LIGHT")?;
        }
        Ok(())
    }

    /// `UpdateAll`: `SetListData`. The list's offsets, sizing, skin and font; without players it
    /// hides. Otherwise each player's row at its position (`r6[position - 1]`) by the layout: the
    /// team layouts above `TeamLeaderBoardsStart` pair the first three players with the second
    /// board's, ranked by score, under a totals row; then the definition, the rows shown, the data
    /// and the titles.
    pub fn update_all(&mut self, results: &mut Clip, assets: &Assets, view: &ResultsView) -> RenderResult<()> {
        let clip = results.named(&["mcList"])?;
        self.list.y_item_offset = 37.;
        self.list.dynamic_size_selector = false;
        self.list.selector_sizing = 734.;
        self.list.set_selection_bar_skin(clip, assets, "DARK_ON_LIGHT")?;
        self.list.font_size = "normal";
        let players = view.current.num_players;
        if players < 1 {
            clip.visible = false;
            return Ok(());
        }
        let kind = view.current.results_type;
        let first = blocks(assets, view.current);
        let definition = self.list.definition_array(results, kind)?;
        let mut rows: BTreeMap<i32, Vec<Value>> = BTreeMap::new();
        let mut titles = 0;
        if kind > layout::TEAM_LEADER_BOARDS_START {
            titles = 4;
            let second = blocks(assets, view.second);
            let mut ranks = [-1_i32; 3];
            for rank in 1..4 {
                let (mut best, mut best_index) = (0, 0);
                for (index, slot) in ranks.iter().enumerate() {
                    let score = team(&second, index)?.0.score;
                    if *slot == -1 && score >= best {
                        best = score;
                        best_index = index;
                    }
                }
                ranks[best_index] = rank;
            }
            let (mut first_total, mut first_float_total, mut second_total, mut second_float_total) = (0, 0.0_f32, 0, 0.0_f32);
            for index in 0..3 {
                let (block, name) = team(&first, index)?;
                let pair = ranks.iter().position(|&rank| rank == block.position).ok_or("results team rank unmatched")?;
                let (other, other_name) = team(&second, pair)?;
                first_total += block.score;
                first_float_total += block.float_score;
                second_total += other.score;
                second_float_total += other.float_score;
                let row = match kind {
                    layout::GOOFY_VS_REGULAR_INT if view.stance_regular => {
                        vec![Value::Text(other_name.clone()), Value::Int(other.score), Value::Text(name.clone()), Value::Int(block.score)]
                    }
                    layout::GOOFY_VS_REGULAR_INT => {
                        vec![Value::Text(name.clone()), Value::Int(block.score), Value::Text(other_name.clone()), Value::Int(other.score)]
                    }
                    layout::GOOFY_VS_REGULAR_FLOAT => {
                        vec![Value::Text(name.clone()), Value::Int(block.score), Value::Text(other_name.clone()), Value::Float(other.float_score)]
                    }
                    _ => continue,
                };
                rows.insert(block.position - 1, row);
            }
            let total = |value: Value, other: Value| vec![Value::Text("ID_GVR_TOTAL".into()), value, Value::Text("ID_GVR_TOTAL".into()), other];
            match kind {
                layout::GOOFY_VS_REGULAR_INT if view.stance_regular => {
                    rows.insert(players - 1, total(Value::Int(second_total), Value::Int(first_total)));
                }
                layout::GOOFY_VS_REGULAR_INT => {
                    rows.insert(players - 1, total(Value::Int(first_total), Value::Int(second_total)));
                }
                layout::GOOFY_VS_REGULAR_FLOAT => {
                    rows.insert(players - 1, total(Value::Float(second_float_total), Value::Float(first_float_total)));
                }
                _ => {}
            }
        } else {
            for (block, name) in &first {
                let (position, name) = (Value::Int(block.position), Value::Text(name.clone()));
                let (row, count) = match kind {
                    layout::PLACE_NAME_INT => (vec![position, name, Value::Int(block.score)], 3),
                    layout::BEST_TRICK_FLOAT => {
                        let distance = Value::Text(number_to_distance(assets, block.float_score)?);
                        (vec![position, name, distance, Value::Text(block.status.clone())], 4)
                    }
                    layout::BEST_TRICK_INT => (vec![position, name, Value::Int(block.score), Value::Text(block.status.clone())], 4),
                    layout::DEATH_RACE => (vec![position, name, Value::Text(block.time.clone())], 3),
                    layout::BEST_TRICK_NON_ELIM_FLOAT => (vec![position, name, Value::Text(number_to_distance(assets, block.float_score)?)], 3),
                    layout::BEST_TRICK_NON_ELIM_INT | layout::GOOFY_VS_REGULAR_TEAM_RESULTS => {
                        (vec![position, name, Value::Int(block.score)], 3)
                    }
                    layout::PLACE_NAME_SCORE_TIME => (vec![position, name, Value::Int(block.score), Value::Text(block.time.clone())], 3),
                    layout::PLACE_NAME_STRING => (vec![position, name, Value::Text(block.status.clone())], 3),
                    _ => return Err(format!("results_lb layout {kind} traces its type error").into()),
                };
                rows.insert(block.position - 1, row);
                titles = count;
            }
            if kind == layout::GOOFY_VS_REGULAR_TEAM_RESULTS {
                let position = |row: Option<&Vec<Value>>| match row.and_then(|row| row.first()) {
                    Some(Value::Int(position)) => Ok(*position),
                    _ => Err("results team rows absent"),
                };
                let goofy_first = position(rows.get(&0))? < position(rows.get(&1))?;
                let (first_team, second_team) = if goofy_first {
                    ("ID_GVR_TEAM_GOOFY", "ID_GVR_TEAM_REGULAR")
                } else {
                    ("ID_GVR_TEAM_REGULAR", "ID_GVR_TEAM_GOOFY")
                };
                for (index, team) in [(0, first_team), (1, second_team)] {
                    if let Some(cell) = rows.get_mut(&index).and_then(|row| row.get_mut(1)) {
                        *cell = Value::Text(team.into());
                    }
                }
            }
        }
        self.list.definition = definition;
        self.list.set_num_visible_rows(players)?;
        let length = rows.range(0..).next_back().map_or(0, |(&last, _)| last + 1);
        let data = (0..length).map(|row| rows.remove(&row).ok_or_else(|| "results rows leave a hole".into())).collect::<RenderResult<Vec<_>>>()?;
        self.list.set_list_data_array(results.named(&["mcList"])?, assets, data)?;
        self.create_titles(results, assets, view, titles)
    }

    /// `CreateTitles(num)`: the old titles removed, then each column's `TITLES` in
    /// `mTitleHolder`, its `mTitle` showing the column title (the team layouts' second half from
    /// the second board's columns), auto-sized as the column's cells align and at the column's x;
    /// the first is centred 13 further left, and the team layouts place all four at fixed x.
    fn create_titles(&mut self, results: &mut Clip, assets: &Assets, view: &ResultsView, num: i32) -> RenderResult<()> {
        self.num_titles = num;
        let kind = view.current.results_type;
        let col_x = SelectableList::col_x(results.named(&["mcList"])?, num);
        let holder = results.named(&["mTitleHolder"])?;
        holder.children.retain(|_, c| !c.entry.name.as_deref().is_some_and(|n| n.starts_with("TITLE")));
        let mut second_column = 0;
        for i in 0..num {
            let index = usize::try_from(i).unwrap_or(0);
            let heading = if kind > layout::TEAM_LEADER_BOARDS_START && i > num / 2 - 1 {
                second_column += 1;
                view.second.col_titles.get(second_column - 1)
            } else {
                view.current.col_titles.get(index)
            };
            let heading = heading.map(String::as_str).unwrap_or_default();
            let mut title = Clip::new(assets, RESULTS_LB, TITLES)?;
            let field = title.named(&["mTitle"])?;
            field.text = Some(if heading.starts_with('#') {
                assets.initial_text(heading)
            } else {
                localizer::format_hal(assets, string_hash(heading.as_bytes()))?
            });
            // Auto-size is lazy in Apt: these _x writes precede string allocation.
            // The measured width below is needed only for the first (left) title.
            let width = text_field_width(title.named(&["mTitle"])?, assets)?;
            let x = col_x.get(index).copied().ok_or("results title past the columns")?;
            set_text_x(child(&mut title, "mTitle")?, if i == 0 { x - width / 2. - 13. } else { x });
            attach(holder, i, format!("TITLE{i}"), title);
            if matches!(kind, layout::GOOFY_VS_REGULAR_INT | layout::GOOFY_VS_REGULAR_FLOAT) {
                for (name, x) in [("TITLE0", -23.), ("TITLE1", 228.), ("TITLE2", 275.), ("TITLE3", 522.)] {
                    if let Some(title) = holder.children.values_mut().find(|c| c.entry.name.as_deref() == Some(name)) {
                        set_text_x(child(&mut title.clip, "mTitle")?, x);
                    }
                }
            }
        }
        // EnsureStringAllocated82a12540 adjusts the requested edge only after
        // CreateTitles1217..12c2 has finished positioning all title fields.
        for i in 0..num {
            let title = child(holder, &format!("TITLE{i}"))?;
            auto_size(child(&mut title.clip, "mTitle")?, assets, self.list.col_alignment(i as usize))?;
        }
        Ok(())
    }
}

/// `LIST_TEXTBOX` (`list_item_textbox`) or `LIST_NUMBERBOX` (`list_item_numberbox`) with its
/// font size, skin, alignment and fixed width.
#[derive(Clone, Debug)]
struct Cell {
    number: bool,
    font_size: &'static str,
    skin: &'static str,
    alignment: &'static str,
    fixed_width: f32,
    /// `SetFontColor` after the value: the online bolding's colour of `textbox` and `textGlow`.
    colour: Option<u32>,
}

impl Cell {
    /// `SetValue(val)`: `small` for the small font, `mTextBox` stopped on the skin, then
    /// `textbox` (and `textGlow` in a selected skin) at `_x` 0 showing the value, auto-sized by
    /// the alignment. The text box shows the value as it is; the number box shows `#` and
    /// `FELanguage.GetLocalizedFormatNumberString(val)` (`0x82604198`: `FormatNumber` of an
    /// integer, of a float with no decimals), a negative as `ID_CHALLENGE_CUT_PLAYER_SCORE`
    /// (`mNegativeShowsAsHal`).
    fn set_value(&self, cell: &mut Clip, assets: &Assets, value: &Value) -> RenderResult<()> {
        if self.font_size == "small" {
            cell.goto_and_stop(assets, "small")?;
        }
        let text = match (self.number, value) {
            (true, Value::Int(n)) if *n < 0 => assets.initial_text("ID_CHALLENGE_CUT_PLAYER_SCORE"),
            (true, Value::Float(n)) if *n < 0. => assets.initial_text("ID_CHALLENGE_CUT_PLAYER_SCORE"),
            (true, Value::Int(n)) => localizer::format_int(assets, *n).encode_utf16().collect(),
            (true, Value::Float(n)) => localizer::format_float(assets, *n, 0).encode_utf16().collect(),
            (false, Value::Text(text)) => assets.initial_text(text),
            _ => return Err("results list cell value differs from its column".into()),
        };
        let text_box = cell.named(&["mTextBox"])?;
        text_box.goto_and_stop(assets, self.skin)?;
        let fields: &[&str] = if self.skin == "unselected" { &["textbox"] } else { &["textbox", "textGlow"] };
        for name in fields {
            let field = child(text_box, name)?;
            set_text_x(field, 0.);
            field.clip.text = Some(text.clone());
            if self.alignment != "left" {
                // numberbox0xdc0/textbox0xd5c: _width = fixedWidth after
                // autoSize. TextMembersIndex width20 -> objectMemberSet82a16b5c
                // changes the box before lazy string allocation; no glyph scaling.
                field.clip.properties.insert("text_width".into(), self.fixed_width.to_string());
            }
            auto_size(field, assets, self.alignment)?;
            match self.colour {
                Some(colour) => {
                    field.clip.properties.insert("text_colour".into(), (0xff00_0000 | colour).to_string());
                }
                None => {
                    field.clip.properties.remove("text_colour");
                }
            }
        }
        Ok(())
    }
}

/// `dynamic_selectable_list` as `results_lb` drives it.
struct SelectableList {
    dynamic_size_selector: bool,
    selector_sizing: f32,
    selection_bar_skin: &'static str,
    /// `mSelected_Skin`: the window row's cell skin.
    selected_skin: &'static str,
    picker_offset: f32,
    font_size: &'static str,
    y_item_offset: f32,
    use_fixed_col_widths: bool,
    row_length: f32,
    column_gap: f32,
    /// `mListOptionDefinition`: each column's cell linkage.
    definition: Vec<&'static str>,
    data: Vec<Vec<Value>>,
    col_widths: Vec<f32>,
    /// `mColAlignArray`.
    col_align: Vec<&'static str>,
    num_visible_rows: i32,
    min_window_index: i32,
    window_index: i32,
    cells: BTreeMap<(i32, usize), Cell>,
    /// `mUseOnlineSpecialBolding` with `mOnline_SelectionBoldHighlight` and `mBoldingColor`.
    online_bolding: Option<(i32, u32)>,
    /// `mUseOnlineSpecialThumbsUp` with `mOnline_ThumbsUpHighlight`.
    online_thumb: Option<i32>,
    /// `mUseOnlineSpecialSelectionIndicator`: `mOnlineSelectionBar` marks the window row.
    online_bar: bool,
    /// `CurrentValue`.
    current_value: i32,
}

impl SelectableList {
    /// The constructor: the defaults, `SetSelectionBarSkin("DARK_ON_LIGHT")`, and the thumb,
    /// online bar and `demo0`..`demo4` hidden.
    fn new(clip: &mut Clip, assets: &Assets) -> RenderResult<Self> {
        let mut list = Self {
            dynamic_size_selector: true,
            selector_sizing: 0.,
            selection_bar_skin: "",
            selected_skin: "",
            picker_offset: 20.,
            font_size: "normal",
            y_item_offset: 30.,
            use_fixed_col_widths: false,
            row_length: 0.,
            column_gap: 20.,
            definition: Vec::new(),
            data: Vec::new(),
            col_widths: Vec::new(),
            col_align: Vec::new(),
            num_visible_rows: 0,
            min_window_index: 0,
            window_index: 0,
            cells: BTreeMap::new(),
            online_bolding: None,
            online_thumb: None,
            online_bar: false,
            current_value: 0,
        };
        list.set_selection_bar_skin(clip, assets, "DARK_ON_LIGHT")?;
        for name in ["mThumbIndicator", "mOnlineSelectionBar", "demo0", "demo1", "demo2", "demo3", "demo4"] {
            clip.named(&[name])?.visible = false;
        }
        Ok(list)
    }

    /// `results_lb.GetDefinitionArray(type)`: each layout's `SetColWidthsArray` (the Goofy vs
    /// Regular integers also `SetColAlignmentArray` and move the list to (-23, 62)) and its cell
    /// linkages; an unknown layout traces its type error and answers an empty array.
    fn definition_array(&mut self, results: &mut Clip, kind: i32) -> RenderResult<Vec<&'static str>> {
        const NUMBER: &str = "LIST_NUMBERBOX";
        const TEXT: &str = "LIST_TEXTBOX";
        let (widths, definition): (&[f32], Vec<&'static str>) = match kind {
            layout::PLACE_NAME_INT | layout::BEST_TRICK_NON_ELIM_INT | layout::GOOFY_VS_REGULAR_TEAM_RESULTS => {
                (&[50., 200., 100.], vec![NUMBER, TEXT, NUMBER])
            }
            layout::BEST_TRICK_FLOAT => (&[50., 200., 100., 200.], vec![NUMBER, TEXT, TEXT, TEXT]),
            layout::BEST_TRICK_INT => (&[50., 200., 100., 200.], vec![NUMBER, TEXT, NUMBER, TEXT]),
            layout::DEATH_RACE | layout::PLACE_NAME_STRING => (&[50., 200., 200.], vec![NUMBER, TEXT, TEXT]),
            layout::BEST_TRICK_NON_ELIM_FLOAT => (&[50., 200., 100.], vec![NUMBER, TEXT, TEXT]),
            layout::PLACE_NAME_SCORE_TIME => (&[50., 200., 50., 50.], vec![NUMBER, TEXT, NUMBER, TEXT]),
            layout::GOOFY_VS_REGULAR_INT => {
                let list = child(results, "mcList")?;
                set_x(list, -23.);
                set_y(list, 62.);
                self.col_align = vec!["left", "right", "left", "right"];
                (&[185., 70., 185., 67.], vec![TEXT, NUMBER, TEXT, NUMBER])
            }
            layout::GOOFY_VS_REGULAR_FLOAT => (&[185., 70., 150., 50.], vec![TEXT, NUMBER, TEXT, NUMBER]),
            _ => return Err(format!("results_lb layout {kind} traces its type error").into()),
        };
        self.col_widths = widths.to_vec();
        Ok(definition)
    }

    /// `SetSelectionBarSkin(newSkin)` while `mcTestPicker` shows: `DARK_ON_LIGHT` plays on
    /// the picker and the window row's cells take `LIGHT_ON_DARK`. `results_lb` and
    /// `dynamic_selectable_list` each carry a copy of the class, which agree on this skin only.
    fn set_selection_bar_skin(&mut self, clip: &mut Clip, assets: &Assets, skin: &'static str) -> RenderResult<()> {
        if skin != "DARK_ON_LIGHT" {
            return Err(format!("selection bar skin {skin} is not translated").into());
        }
        let picker = clip.named(&["mcTestPicker"])?;
        if picker.visible {
            self.selection_bar_skin = skin;
            self.selected_skin = "LIGHT_ON_DARK";
            picker.goto(assets, skin)?;
        }
        Ok(())
    }

    /// `SetNumVisibleRows(num)` then `SetInitialWindow(0)`; past `MAX_VISIBLE_ROWS` it asserts.
    fn set_num_visible_rows(&mut self, num: i32) -> RenderResult<()> {
        if num > MAX_VISIBLE_ROWS {
            return Err("results list rows past MAX_VISIBLE_ROWS".into());
        }
        self.num_visible_rows = num;
        self.min_window_index = 0;
        self.window_index = 0;
        Ok(())
    }

    /// `SetListDataArray(newArray)`: the data, then `DestroyListWindow`, `CreateList`
    /// (`CreateRows`, `CreateCols`, `AlignRows`), `UpdateWindow` and `ResizeSelectionBar`. The
    /// current row it also sets serves only `UpdateInput`, which a results list that
    /// `Results_GetIsNavigatable` refuses never reaches.
    fn set_list_data_array(&mut self, clip: &mut Clip, assets: &Assets, data: Vec<Vec<Value>>) -> RenderResult<()> {
        self.data = data;
        clip.children.retain(|_, c| !c.entry.name.as_deref().is_some_and(|n| n.starts_with("ROW")));
        self.cells.clear();
        let columns = i32::try_from(self.definition.len()).unwrap_or(0);
        for row in 0..self.num_visible_rows {
            let empty = Clip::new(assets, RESULTS_LB, EMPTY_CLIP)?;
            attach(clip, self.num_visible_rows + columns + row, format!("ROW{row}"), empty);
        }
        self.create_cols(clip, assets)?;
        for row in 0..self.num_visible_rows {
            let piece = child(clip, &format!("ROW{row}"))?;
            #[expect(clippy::cast_precision_loss, reason = "ActionScript numbers")]
            let offset = row as f32 * self.y_item_offset;
            set_y(piece, piece.entry.matrix.ty + offset);
        }
        self.update_window(clip, assets)?;
        self.resize_selection_bar(clip)
    }

    /// `CreateCols`: in each row, each column's cell at its depth, given the font size, its
    /// column's width (fixed widths) and alignment (when the alignment array covers it), then its
    /// value, after the previous column by that width and the gap; the row length gains each width
    /// and gap, the last without.
    fn create_cols(&mut self, clip: &mut Clip, assets: &Assets) -> RenderResult<()> {
        if !self.use_fixed_col_widths {
            return Err("results list measured column widths are not translated".into());
        }
        self.row_length = 0.;
        for (column, linkage) in self.definition.clone().into_iter().enumerate() {
            let character = linkage_character(assets, SELECTABLE_LIST, linkage)?;
            for row in 0..self.num_visible_rows {
                let row_clip = &mut child(clip, &format!("ROW{row}"))?.clip;
                let mut cell_clip = Clip::new(assets, SELECTABLE_LIST, character)?;
                let cell = Cell {
                    number: linkage == "LIST_NUMBERBOX", font_size: self.font_size,
                    skin: "unselected", alignment: self.col_alignment(column),
                    fixed_width: self.col_widths.get(column).copied().unwrap_or(f32::NAN),
                    colour: None,
                };
                let value = self.value(row, column)?;
                cell.set_value(&mut cell_clip, assets, &value)?;
                let depth = i32::try_from(column).unwrap_or(0);
                attach(row_clip, depth, format!("COL{column}"), cell_clip);
                if column > 0 {
                    let previous = child(row_clip, &format!("COL{}", column - 1))?;
                    let (x, y) = (previous.entry.matrix.tx, previous.entry.matrix.ty);
                    let width = self.col_widths.get(column - 1).copied().unwrap_or(f32::NAN);
                    let piece = child(row_clip, &format!("COL{column}"))?;
                    set_x(piece, x + width + self.column_gap);
                    set_y(piece, y);
                }
                self.cells.insert((row, column), cell);
            }
            let gap = if column + 1 == self.definition.len() { 0. } else { self.column_gap };
            self.row_length += self.col_widths.get(column).copied().unwrap_or(f32::NAN) + gap;
        }
        Ok(())
    }

    /// A cell's `textAlignment`: the alignment array's entry for its column, else the cells'
    /// default `left`; `GetColAlignment(col)` answers it.
    fn col_alignment(&self, column: usize) -> &'static str {
        self.col_align.get(column).copied().unwrap_or("left")
    }

    /// `mList2dDataArray[row][column]`.
    fn value(&self, row: i32, column: usize) -> RenderResult<Value> {
        usize::try_from(row)
            .ok()
            .and_then(|r| self.data.get(r))
            .and_then(|r| r.get(column))
            .cloned()
            .ok_or_else(|| "results list cell past its data".into())
    }

    /// `UpdateWindow`: each visible cell skinned (the window row's in `mSelected_Skin`) and set
    /// from its windowed row, then `UpdateIndicator`.
    fn update_window(&mut self, clip: &mut Clip, assets: &Assets) -> RenderResult<()> {
        for row in 0..self.num_visible_rows {
            for column in 0..self.definition.len() {
                let selected = row == self.window_index;
                let skin = if selected { self.selected_skin } else { "unselected" };
                let value = self.value(row + self.min_window_index, column)?;
                // The online bolding colours its row: dark under the window, else its colour.
                let colour = self.online_bolding.filter(|&(bold, _)| bold == row).map(|(_, colour)| if selected { 0x003a_524f } else { colour });
                let cell = self.cells.get_mut(&(row, column)).ok_or("results list cell absent")?;
                cell.skin = skin;
                cell.colour = colour;
                let cell_clip = clip.named(&[&format!("ROW{row}"), &format!("COL{column}")])?;
                cell.set_value(cell_clip, assets, &value)?;
            }
        }
        self.update_indicator(clip, assets)?;
        self.update_thumbs_up(clip, assets)
    }

    /// The `_y` that centres a bar on a row: the row's, less half of `ROW0`'s height beyond the
    /// row offset. A list without the row (one of no rows) has no such clip: the script reads
    /// `undefined`, the sum is `NaN` and the bar keeps its `_y`.
    fn bar_y(&self, clip: &mut Clip, assets: &Assets, row: i32) -> RenderResult<Option<f32>> {
        let Some(row0) = find_child(clip, "ROW0") else { return Ok(None) };
        let r = bounds(&row0.clip, assets)?.ok_or("results list row has no bounds")?;
        let half = (((r.bottom - r.top) * row0.entry.matrix.d).abs() - self.y_item_offset) / 2.;
        Ok(find_child(clip, &format!("ROW{row}")).map(|target| target.entry.matrix.ty - half))
    }

    /// `UpdateThumbsUp`: `mThumbIndicator` at the players' choice row.
    fn update_thumbs_up(&self, clip: &mut Clip, assets: &Assets) -> RenderResult<()> {
        let Some(row) = self.online_thumb else { return Ok(()) };
        let y = self.bar_y(clip, assets, row)?;
        let thumb = child(clip, "mThumbIndicator")?;
        thumb.clip.visible = true;
        if let Some(y) = y {
            set_y(thumb, y);
        }
        Ok(())
    }

    /// `UpdateIndicator`: `mcTestPicker` shown, centred on the window row and playing the
    /// bar's skin.
    fn update_indicator(&self, clip: &mut Clip, assets: &Assets) -> RenderResult<()> {
        let y = self.bar_y(clip, assets, self.window_index)?;
        let picker = child(clip, if self.online_bar { "mOnlineSelectionBar" } else { "mcTestPicker" })?;
        picker.clip.visible = true;
        if let Some(y) = y {
            set_y(picker, y);
        }
        picker.clip.goto(assets, if self.online_bar { "LIGHT_ON_DARK" } else { self.selection_bar_skin })
    }

    /// `NextValue` and `PrevValue`: the current row a step on within the data, the window row
    /// following it (scrolling past the window's ends), then `UpdateWindow`.
    fn step(&mut self, clip: &mut Clip, assets: &Assets, forward: bool) -> RenderResult<()> {
        let last = i32::try_from(self.data.len()).unwrap_or(0) - 1;
        let max_window = self.min_window_index + self.num_visible_rows - 1;
        if forward {
            self.current_value += 1;
            if self.current_value > last {
                self.current_value = last;
                return Ok(());
            }
            if self.current_value > max_window {
                self.min_window_index += 1;
            } else {
                self.window_index += 1;
            }
        } else {
            self.current_value -= 1;
            if self.current_value < 0 {
                self.current_value = 0;
                return Ok(());
            }
            if self.current_value < self.min_window_index {
                self.min_window_index -= 1;
            } else {
                self.window_index -= 1;
            }
        }
        self.update_window(clip, assets)
    }

    /// `ResizeSelectionBar`: `mcTestPicker`'s light selector stretched to the manual size (or
    /// the row length) less the picker offset, and its height scaled to the row offset.
    fn resize_selection_bar(&self, clip: &mut Clip) -> RenderResult<()> {
        let width = if self.dynamic_size_selector { self.row_length } else { self.selector_sizing };
        let picker = child(clip, if self.online_bar { "mOnlineSelectionBar" } else { "mcTestPicker" })?;
        set_yscale(picker, 100. * self.y_item_offset / 40.);
        let selector = &mut child(&mut picker.clip, "mLightSelector")?.clip;
        set_x(child(selector, "mSelectorLeft")?, -self.picker_offset);
        let center = child(selector, "mSelectorCenter")?;
        set_x(center, 64. - self.picker_offset);
        set_xscale(center, 100. * (width - 128.) / 64.);
        set_x(child(selector, "mSelectorRight")?, width - 64. - self.picker_offset);
        set_xscale(child(selector, "mBacklight")?, 100. * width / 64.);
        Ok(())
    }

    /// `GetColX(col)` of the first `count` columns: `ROW0`'s cells' x, `undefined` past them or
    /// in a list of no rows.
    fn col_x(clip: &mut Clip, count: i32) -> Vec<f32> {
        let count = usize::try_from(count).unwrap_or(0);
        let Some(row0) = find_child(clip, "ROW0") else { return vec![f32::NAN; count] };
        let row0 = &mut row0.clip;
        (0..count).map(|column| find_child(row0, &format!("COL{column}")).map_or(f32::NAN, |c| c.entry.matrix.tx)).collect()
    }
}

/// `dynamic_selectable_list` as an online results screen drives it directly: fixed column
/// widths, the gap and row offset, its own widths, alignments and cells, the online bolding,
/// thumbs-up and selection bar, the window from its first row, and its input.
pub(super) struct OnlineList {
    list: SelectableList,
}

/// How an online results screen sets its list up (`InitDataArray`).
pub(super) struct OnlineListLayout {
    pub column_gap: f32,
    pub row_offset: f32,
    pub widths: Vec<f32>,
    pub alignments: Vec<&'static str>,
    pub definition: Vec<&'static str>,
    /// `SetUpOnlineBolding(true, row, colour)`.
    pub bolding: (i32, u32),
    /// `SetUpOnlineThumbnail(true, row)`, none for `(false, 0)`.
    pub thumb: Option<i32>,
    /// `SetSelectorSizing(false, size)`.
    pub selector: f32,
}

impl OnlineList {
    /// The list's constructor.
    pub fn new(clip: &mut Clip, assets: &Assets) -> RenderResult<Self> {
        Ok(Self { list: SelectableList::new(clip, assets)? })
    }

    /// `InitDataArray`'s calls on the list, then `SetListDataArray` and `UpdateWindow`.
    pub fn set_data(&mut self, clip: &mut Clip, assets: &Assets, layout: OnlineListLayout, data: Vec<Vec<Value>>) -> RenderResult<()> {
        let list = &mut self.list;
        list.use_fixed_col_widths = true;
        list.column_gap = layout.column_gap;
        list.y_item_offset = layout.row_offset;
        list.col_widths = layout.widths;
        list.col_align = layout.alignments;
        list.definition = layout.definition;
        list.online_bolding = Some(layout.bolding);
        list.online_thumb = layout.thumb;
        // SetUseOnlineSelectionBar(true): the window row's cells take DARK_ON_LIGHT.
        list.online_bar = true;
        list.selected_skin = "DARK_ON_LIGHT";
        list.dynamic_size_selector = false;
        list.selector_sizing = layout.selector;
        list.set_num_visible_rows(i32::try_from(data.len()).unwrap_or(0))?;
        list.current_value = 0;
        list.set_list_data_array(clip, assets, data)?;
        list.update_window(clip, assets)
    }

    /// `UpdateInput`: up and down step the current row; the sound of each step.
    pub fn update_input(&mut self, clip: &mut Clip, assets: &Assets, forward: bool) -> RenderResult<()> {
        self.list.step(clip, assets, forward)
    }

    /// `GetCurrentIndex`.
    pub fn current(&self) -> usize {
        usize::try_from(self.list.current_value).unwrap_or(0)
    }
}

/// `dynamic_selectable_list` as skate.Profile's offline tabs drive it (`mDisplay_sp.mList`):
/// `SetState`'s row offset, font size, fixed column widths and selection bar, then each
/// `SetupOfflineDataTabListControl`'s cells, alignments, widths, rows and data, and its input.
pub(super) struct ProfileList {
    list: SelectableList,
}

/// What `SetupOfflineDataTabListControl` gives the list besides its data.
pub(super) struct ProfileListLayout {
    pub definition: Vec<&'static str>,
    pub alignments: Vec<&'static str>,
    pub widths: Vec<f32>,
    pub rows: i32,
}

impl ProfileList {
    /// The list's constructor.
    pub fn new(clip: &mut Clip, assets: &Assets) -> RenderResult<Self> {
        Ok(Self { list: SelectableList::new(clip, assets)? })
    }

    /// `SetYOffset(offset)`, `SetFontSize(size)`, `SetUseColWidths(true)` and
    /// `SetSelectionBarSkin("DARK_ON_LIGHT")`.
    pub fn set_style(&mut self, clip: &mut Clip, assets: &Assets, offset: f32, size: &'static str) -> RenderResult<()> {
        self.list.y_item_offset = offset;
        self.list.font_size = size;
        self.list.use_fixed_col_widths = true;
        self.list.set_selection_bar_skin(clip, assets, "DARK_ON_LIGHT")
    }

    /// `SetListOptionDefinition`, `SetColAlignmentArray`, `SetColWidthsArray`,
    /// `SetNumVisibleRows(rows)` and `SetInitialWindow(0)`, then `SetListDataArray(data)`, whose
    /// current row is then the first.
    pub fn set_data(&mut self, clip: &mut Clip, assets: &Assets, layout: ProfileListLayout, data: Vec<Vec<Value>>) -> RenderResult<()> {
        let list = &mut self.list;
        list.definition = layout.definition;
        list.col_align = layout.alignments;
        list.col_widths = layout.widths;
        list.set_num_visible_rows(layout.rows)?;
        list.current_value = 0;
        list.set_list_data_array(clip, assets, data)
    }

    /// `UpdateInput`'s `NextValue` or `PrevValue`.
    pub fn step(&mut self, clip: &mut Clip, assets: &Assets, forward: bool) -> RenderResult<()> {
        self.list.step(clip, assets, forward)
    }

    /// `CurrentValue`.
    pub fn current(&self) -> usize {
        usize::try_from(self.list.current_value).unwrap_or(0)
    }
}

/// A member, `None` (`undefined`) when absent.
fn find_child<'a>(clip: &'a mut Clip, name: &str) -> Option<&'a mut Child> {
    clip.children.values_mut().find(|c| c.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

/// `attachMovie`'s linkage: the movie's export or import of that name.
fn linkage_character(assets: &Assets, asset: usize, name: &str) -> RenderResult<u32> {
    let movie = &assets.movies[asset].movie;
    movie
        .exports
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.character_id)
        .or_else(|| movie.imports.iter().find(|i| i.name == name).map(|i| i.character_id))
        .ok_or_else(|| format!("APT linkage {name} absent").into())
}

/// `attachMovie` and `createEmptyMovieClip`: the clip named, at a script depth, at the origin.
fn attach(parent: &mut Clip, depth: i32, name: String, clip: Clip) {
    let entry = DisplayEntry {
        identity: PlacementIdentity { character: clip.character, frame: 0 },
        matrix: IDENTITY,
        colour_scale: u32::MAX,
        colour_bias: 0,
        ratio: 0.,
        name: Some(name),
        clip_depth: -1,
        events: Vec::new(),
        blend_mode: None,
    };
    parent.children.insert(SCRIPT_DEPTH + depth, Child { entry, clip });
}

/// `_x` set on a text field: `AptCIH::objectMemberSet` (`0x82a16bb4`) adds 2 before
/// `SetProceduralProperty(_x)`, so 0 keeps the authored placement at 2.
pub(super) fn set_text_x(piece: &mut Child, x: f32) {
    set_x(piece, x + 2.);
}

/// AptCIH::EnsureStringAllocated82a12540..1260c preserves the requested edge when
/// SkateAptString82a71020 replaces a single line's bounds width with its text extent.
pub(super) fn auto_size(piece: &mut Child, assets: &Assets, alignment: &str) -> RenderResult<()> {
    let field = &mut piece.clip;
    let Character::Text(text) = field.definition(assets)? else {
        return Err("results auto-size target is not text".into());
    };
    if text.multiline || text.alignment != 0 {
        return Err("results auto-size field differs from the verified layout".into());
    }
    let fraction = match alignment {
        "left" => 0., "right" => 1., "center" => 0.5,
        _ => return Err(format!("invalid results auto-size {alignment}").into()),
    };
    let old_width = field.properties.get("text_width").map(|v| v.parse::<f32>()).transpose()?
        .unwrap_or(text.bounds.right - text.bounds.left);
    let extent = text_field_width(field, assets)? - 4.;
    field.properties.insert("text_autosize".into(), alignment.into());
    field.properties.insert("text_width".into(), extent.to_string());
    set_x(piece, piece.entry.matrix.tx + (old_width - extent) * fraction);
    Ok(())
}

/// `SetProceduralProperty(_yscale)` (`0x829ea608`): `value * 0.01` (zero becomes 0.0001)
/// replaces the unrotated matrix's y scale.
fn set_yscale(piece: &mut Child, value: f32) {
    piece.clip.script_transform = true;
    let mut scale = value * f32::from_bits(0x3c23_d70a);
    if scale == 0. {
        scale = f32::from_bits(0x38d1_b717);
    }
    let m = &mut piece.entry.matrix;
    if m.b.abs() >= f32::from_bits(0x38d1_b717) || m.c.abs() >= f32::from_bits(0x38d1_b717) {
        let inverse = 1. / m.c.mul_add(m.c, m.d * m.d).sqrt();
        m.d = inverse * m.d * scale;
        m.c = inverse * m.c * scale;
    } else {
        m.d = scale;
    }
}
