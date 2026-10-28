//! The Lobby Browser (PC port addition), driven by `skate2_core::ui::lobby_browser`, on
//! unranked/unranked_custom_match (asset 105) with that screen's timeline, intro, outro, panel,
//! glare, title, button help and `mMatch` card. The panel is drawn without its header band,
//! preview frame and help band, its content stretched under its title strip for the list, and a
//! card for each game in view replaces the selectors: a `mcShading` panel with the challenge's
//! title over its location's small image (the screen's `image_renderer`) on the left and the
//! challenge type over the game's phase and players on the right, `menu_picker`'s indicator on
//! the selected card and `basic_slider` beside the cards when the list is longer than the view.
//! `mMatch` shows while the list loads and while a join waits; a failed join's notice is
//! sk8popup over the screen.
use super::{
    assets::Assets,
    clips::Clip,
    map_movie,
    online_hub_movie::{self, SCALER},
    online_match_movie::CUSTOM,
    pause_movie::{bounds, child},
    progression_popup::ProgressionPopup,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{
    apt_timeline::IDENTITY,
    lobby_browser::{Lobby, LobbyPhase, Notice, Snapshot, State, VISIBLE},
};
use skate2_formats::apt::Character;
use std::collections::BTreeMap;

/// background_scaler's `mcShading` export: a card's frame.
const SHADING: u32 = 51;
/// menu_picker's `mIndicator` and basic_slider's vertical slider.
const PICKER: usize = 6;
const INDICATOR: u32 = 53;
const SLIDER_ASSET: usize = 10;
const SLIDER: u32 = 53;
/// unranked_custom_match's text fields: the title in HelveticaPresto and its HelveticaGlow,
/// and `txtHelp` in Helvetica LT Std; its `mPreviewImage`, an `image_renderer`.
const TEXT: u32 = 29;
const GLOW: u32 = 28;
const BODY: u32 = 7;
const IMAGE_RENDERER: u32 = 8;
/// The screen's panel (shape 4 of its sprite at depth 1, 512 high) in `shape_bands`: its title
/// strip, the line under its header band, rows of its content clear of the preview's frame
/// stretched to the top of its bottom edge, and that edge.
const PANEL_BANDS: [[f32; 4]; 4] =
    [[0.0, 64.0, 0.0, 64.0], [146.0, 152.0, 64.0, 70.0], [158.0, 162.0, 70.0, 469.0], [469.0, 512.0, 469.0, 512.0]];
/// The list's top and bottom on the stage: the panel's, placed at 106, from under its line (67)
/// to its bottom edge (469).
const LIST_TOP: f32 = 173.0;
const LIST_BOTTOM: f32 = 575.0;
/// A card's size, the space around each card as `VISIBLE` of them fill the list, the first
/// card's top left on the stage and the distance between cards.
const CARD: [f32; 2] = [604.0, 130.0];
const CARD_SPACE: f32 = (LIST_BOTTOM - LIST_TOP - CARD[1] * VISIBLE as f32) / (VISIBLE + 1) as f32;
const LIST: [f32; 2] = [318.0, LIST_TOP + CARD_SPACE];
const CARD_PITCH: f32 = CARD[1] + CARD_SPACE;
/// The cards' fades at this share of `mcShading`'s height, so that their text and image sit on
/// the solid middle.
const FADE: f32 = 0.5;
/// The slider's left edge on the stage; its bar starts at the slider's `mcScrollbar` (35).
const SLIDER_X: f32 = 926.0;
const SLIDER_TOP: f32 = 35.0;
/// A card's layout: its text and image left of the indicator's column, its right margin, the
/// lines' tops and heights, the image's top and size (the status line 22 above its bottom),
/// and the widest title and type.
const TEXT_LEFT: f32 = 44.0;
const TEXT_RIGHT: f32 = 588.0;
const LINE_TOP: f32 = 12.0;
const TITLE_HEIGHT: f32 = 22.0;
const STATUS_HEIGHT: f32 = 18.0;
const IMAGE: [f32; 4] = [44.0, 42.0, 128.0, 72.0];
const STATUS_TOP: f32 = IMAGE[1] + IMAGE[3] - 22.0;
const TITLE_WIDTH: f32 = 330.0;
const MODE_WIDTH: f32 = 230.0;
/// The indicator's offset from a label, as `menu_picker` places it beside an item's text.
const INDICATOR_OFFSET: [f32; 2] = [-49.0, -8.0];
/// `InitializeScreen`'s widest title before it is scaled down.
const SCREEN_TITLE_WIDTH: f32 = 616.0;
/// menu_picker's selected and deselected item colours and glow, and `txtHelp`'s colour.
const SELECTED: u32 = 0xffff_feec;
const DESELECTED: u32 = 0xffda_e8e1;
const SELECTED_GLOW: u32 = 0xffff_ff99;
const STATUS: u32 = 0xffb4_b19b;
/// `mOptions` depths past its timeline's: the message of an empty list, the slider and the
/// cards.
const MESSAGE_DEPTH: i32 = 98;
const SLIDER_DEPTH: i32 = 99;
const CARD_DEPTH: i32 = 100;

pub(super) struct LobbyBrowserMovie {
    pub root: Clip,
    pub popup: ProgressionPopup,
    open: bool,
    /// The state the screen shows, and the texture of each card's image.
    shown: Option<(Snapshot, Vec<Option<String>>)>,
    /// `mOptions`' placement, which the cards are laid out in.
    options: [f32; 2],
    /// `ScreenManager.IntroComplete` since the last call.
    intro_complete: bool,
}

impl LobbyBrowserMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, CUSTOM, 0)?;
        root.playback.stop();
        root.visible = false;
        Ok(Self { root, popup: ProgressionPopup::default(), open: false, shown: None, options: [0.0; 2], intro_complete: false })
    }

    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// Opens the screen with its first state, plays the outro once it closes, and refreshes it
    /// as the state changes; `images` holds the texture loaded for each card's image.
    pub fn update(&mut self, assets: &Assets, snapshot: Option<&Snapshot>, images: &BTreeMap<String, String>) -> RenderResult<()> {
        match snapshot {
            Some(_) if !self.open => {
                self.open = true;
                self.root = Clip::new(assets, CUSTOM, 0)?;
                self.root.visible = true;
                self.shown = None;
                let mut calls = Vec::new();
                self.root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "OnlineMatchScreen" => self.initialize(assets)?,
                        _ => return Err(format!("untranslated lobby browser callback {call}").into()),
                    }
                }
            }
            None if self.open => {
                self.open = false;
                self.shown = None;
                self.root.goto(assets, "outro")?;
                return self.popup.show(assets, None);
            }
            _ => {}
        }
        let Some(snapshot) = snapshot else { return Ok(()) };
        let textures = snapshot.lobbies.iter().map(|lobby| images.get(&lobby.image).cloned()).collect::<Vec<_>>();
        if self.shown.as_ref().is_some_and(|(shown, shown_textures)| shown == snapshot && *shown_textures == textures) {
            return Ok(());
        }
        self.apply(assets, snapshot, &textures)?;
        self.shown = Some((snapshot.clone(), textures));
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.popup.tick(assets)?;
        if !self.root.visible {
            return Ok(());
        }
        self.root.tick(assets)?;
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => self.root.visible = false,
                "WheelLoop" => self.root.named(&["mOptions", "mMatch", "mLoadingGraphic", "mcWheel"])?.goto_number(assets, 1)?,
                _ => return Err(format!("untranslated lobby browser callback {call}").into()),
            }
        }
        Ok(())
    }

    /// The screen's constructor as `InitializeScreen` runs it, then the browser's layout: the
    /// panel's bands, the title, the cards' place in `mOptions` and `mMatch` over their middle.
    fn initialize(&mut self, assets: &Assets) -> RenderResult<()> {
        let panel = [1, 3].into_iter().try_fold(&mut self.root, |clip, depth| clip.children.get_mut(&depth).map(|c| &mut c.clip))
            .ok_or("custom match panel absent")?;
        panel.properties.insert("shape_bands".into(), super::shape_bands(&PANEL_BANDS));
        let title = self.root.named(&["mTitle"])?;
        let text = assets.localize("#Lobby Browser")?;
        let mut width = 0.;
        for field in ["txtTitle", "txtGlow"] {
            let label = title.named(&[field])?;
            label.text = Some(text.encode_utf16().collect());
            width = super::edit_skater_movie::autosize_text(label, assets, &text)?;
        }
        if width > SCREEN_TITLE_WIDTH {
            for field in ["txtTitle", "txtGlow"] {
                let label = child(title, field)?;
                label.entry.matrix.a *= SCREEN_TITLE_WIDTH / width;
                label.entry.matrix.d *= SCREEN_TITLE_WIDTH / width;
                label.clip.script_transform = true;
            }
        }
        self.root.named(&["mGamerTag"])?.visible = false;
        let content = self.root.named(&["mContent"])?;
        content.visible = true;
        content.named(&["txtHelp"])?.visible = false;
        content.named(&["mPreviewImage"])?.visible = false;
        let placement = child(&mut self.root, "mOptions")?;
        self.options = [placement.entry.matrix.tx, placement.entry.matrix.ty];
        let options = &mut placement.clip;
        options.visible = true;
        for selector in 0..3 {
            options.named(&[&format!("mSelector{selector}")])?.visible = false;
        }
        // The card centres on the list's middle; the screen's fades of it are the screen's own.
        let card = child(options, "mMatch")?;
        let extent = bounds(&card.clip, assets)?.ok_or("custom match card has no bounds")?;
        let middle = [LIST[0] + CARD[0] / 2.0, LIST[1] + (CARD_PITCH * (VISIBLE - 1) as f32 + CARD[1]) / 2.0];
        card.entry.matrix.tx = middle[0] - self.options[0] - (extent.left + extent.right) / 2.0;
        card.entry.matrix.ty = middle[1] - self.options[1] - (extent.top + extent.bottom) / 2.0;
        card.clip.script_transform = true;
        card.clip.visible = false;
        Ok(())
    }

    /// The cards in view, or `mMatch` while the list loads and a join waits, the slider, the
    /// button help and the notice.
    fn apply(&mut self, assets: &Assets, snapshot: &Snapshot, textures: &[Option<String>]) -> RenderResult<()> {
        let [x, y] = [LIST[0] - self.options[0], LIST[1] - self.options[1]];
        let options = self.root.named(&["mOptions"])?;
        options.children.retain(|&depth, _| depth < MESSAGE_DEPTH);
        let listing = snapshot.state == State::List;
        if listing {
            for (index, (lobby, texture)) in snapshot.lobbies.iter().zip(textures).enumerate() {
                let selected = snapshot.first + index == snapshot.selected;
                let card = card(assets, lobby, selected, texture.as_deref())?;
                let top = y + CARD_PITCH * index as f32;
                options.children.insert(CARD_DEPTH + i32::try_from(index)?, at(card, "mLobby", x, top, 1.0));
            }
            if snapshot.count == 0 {
                let message = label(assets, TEXT, &assets.localize("#No online games right now.")?, TITLE_HEIGHT, SELECTED, CARD[0], 2)?;
                let top = y + (CARD_PITCH * (VISIBLE - 1) as f32 + CARD[1] - TITLE_HEIGHT) / 2.0;
                options.children.insert(MESSAGE_DEPTH, at(message, "mMessage", x, top, 1.0));
            }
            let mut slider = Clip::new(assets, SLIDER_ASSET, SLIDER)?;
            let length = CARD_PITCH * (VISIBLE - 1) as f32 + CARD[1];
            map_movie::update_scrollbar(&mut slider, assets, snapshot.count, VISIBLE, snapshot.first, length)?;
            let top = y - SLIDER_TOP;
            options.children.insert(SLIDER_DEPTH, at(slider, "mScrollbar", SLIDER_X - self.options[0], top, 1.0));
        }
        let card = options.named(&["mMatch"])?;
        card.visible = !listing;
        if !listing {
            let waiting = if snapshot.state == State::Joining { "ID_ONLINE_MATCHING_SKATERS" } else { "ID_RANKED_MATCHMAKING" };
            card.named(&["txtTitle"])?.text = Some(assets.localize(waiting)?.encode_utf16().collect());
        }
        let help = self.root.named(&["mContent", "mButtonHelp"])?;
        help.visible = snapshot.state != State::Joining;
        let mut items = vec![("B", "ID_COMMON_BACK")];
        if listing {
            if snapshot.count > 0 {
                items.push(("A", "#Join"));
            }
            items.push(("X", "#Refresh"));
        }
        map_movie::styled_button_items(help, assets, &items, 0xffff_feec, false, "Marker_streak", 20.0)?;
        self.popup.show(assets, snapshot.notice.map(notice))
    }
}

/// The notice's popup: its title and message, OK, not cancellable.
fn notice(notice: Notice) -> (String, String, Vec<String>, usize, bool) {
    let (title, message) = notice.text();
    (title.to_owned(), message.to_owned(), vec!["ID_COMMON_OK".to_owned()], 0, false)
}

/// One game's card: its frame, the indicator when selected, the challenge's title and image on
/// the left, its type and the game's phase and players on the right.
fn card(assets: &Assets, lobby: &Lobby, selected: bool, texture: Option<&str>) -> RenderResult<Clip> {
    let mut card = Clip::new(assets, SCALER, SHADING)?;
    frame(&mut card, assets, CARD[0], CARD[1])?;
    let colour = if selected { SELECTED } else { DESELECTED };
    let title = assets.localize(&lobby.title)?;
    let mut children = Vec::new();
    if selected {
        let indicator = Clip::new(assets, PICKER, INDICATOR)?;
        children.push(at(indicator, "mIndicator", TEXT_LEFT + INDICATOR_OFFSET[0], LINE_TOP + INDICATOR_OFFSET[1], 1.0));
        let mut glow = label(assets, GLOW, &title, TITLE_HEIGHT, SELECTED_GLOW, TITLE_WIDTH, 0)?;
        let scale = fit(assets, &mut glow, TITLE_HEIGHT, TITLE_WIDTH)?;
        children.push(at(glow, "txtGlow", TEXT_LEFT, LINE_TOP, scale));
    }
    let mut name = label(assets, TEXT, &title, TITLE_HEIGHT, colour, TITLE_WIDTH, 0)?;
    let scale = fit(assets, &mut name, TITLE_HEIGHT, TITLE_WIDTH)?;
    children.push(at(name, "txtTitle", TEXT_LEFT, LINE_TOP, scale));
    if let Some(texture) = texture {
        let mut image = Clip::new(assets, CUSTOM, IMAGE_RENDERER)?;
        image.properties.insert("cac_texture".into(), texture.to_owned());
        let extent = bounds(&image, assets)?.ok_or("lobby card image has no bounds")?;
        let scale = [IMAGE[2] / (extent.right - extent.left), IMAGE[3] / (extent.bottom - extent.top)];
        let mut placement = at(image, "mImage", IMAGE[0] - extent.left * scale[0], IMAGE[1] - extent.top * scale[1], scale[0]);
        placement.entry.matrix.d = scale[1];
        children.push(placement);
    }
    let mut mode = label(assets, TEXT, &assets.localize(lobby.mode)?, TITLE_HEIGHT, colour, MODE_WIDTH, 1)?;
    let scale = fit(assets, &mut mode, TITLE_HEIGHT, MODE_WIDTH)?;
    children.push(at(mode, "txtMode", TEXT_RIGHT - MODE_WIDTH, LINE_TOP, scale));
    let players = format!("({}/{})", lobby.players, lobby.max_players);
    let status = match lobby.phase {
        Some(phase) => {
            let label = assets.localize(match phase {
                LobbyPhase::PreGame => "#Pre-Game",
                LobbyPhase::InGame => "#In Game",
            })?;
            format!("{label} {players}")
        }
        None => players,
    };
    let status = label(assets, BODY, &status, STATUS_HEIGHT, STATUS, MODE_WIDTH, 1)?;
    children.push(at(status, "txtStatus", TEXT_RIGHT - MODE_WIDTH, STATUS_TOP, 1.0));
    for (depth, placement) in (CARD_DEPTH..).zip(children) {
        card.children.insert(depth, placement);
    }
    Ok(card)
}

/// One line of text in a field of the screen's: its height, colour, width and alignment (0
/// left, 1 right, 2 centred), its line's top at the clip's origin.
fn label(assets: &Assets, character: u32, text: &str, height: f32, colour: u32, width: f32, alignment: u32) -> RenderResult<Clip> {
    let mut clip = Clip::new(assets, CUSTOM, character)?;
    clip.text = Some(text.encode_utf16().collect());
    for (key, value) in [
        ("text_height", height.to_string()),
        ("text_colour", colour.to_string()),
        ("text_width", width.to_string()),
        ("text_alignment", alignment.to_string()),
        ("text_left", "0".to_owned()),
        ("text_top", "0".to_owned()),
        ("text_multiline", "false".to_owned()),
    ] {
        clip.properties.insert(key.into(), value);
    }
    Ok(clip)
}

/// The horizontal scale that narrows a label to `width` when its text is wider, as
/// `InitializeScreen` fits the title: the field then spans the whole text, so the narrowed text
/// fills `width` whatever its alignment.
fn fit(assets: &Assets, clip: &mut Clip, height: f32, width: f32) -> RenderResult<f32> {
    let Character::Text(text) = clip.definition(assets)? else { return Err("lobby card label is not text".into()) };
    let Some(Character::Font { name, .. }) = assets.movies[clip.asset].movie.characters.get(text.font_id as usize).and_then(Option::as_ref) else {
        return Err("lobby card font absent".into());
    };
    let font = assets.apt_font(name)?;
    let extent: f32 = clip.text.as_deref().unwrap_or_default().iter().filter_map(|&c| font.metrics.mapped_glyph(c)).map(|glyph| glyph.advance * font.scale[0] * height).sum();
    if extent <= width {
        return Ok(1.0);
    }
    clip.properties.insert("text_width".into(), extent.to_string());
    Ok(width / extent)
}

/// A clip placed at `x`, `y`, scaled horizontally by `scale`.
fn at(clip: Clip, name: &str, x: f32, y: f32, scale: f32) -> super::clips::Child {
    let mut matrix = IDENTITY;
    matrix.a = scale;
    matrix.tx = x;
    matrix.ty = y;
    let character = clip.character;
    online_hub_movie::placed(clip, character, name, matrix)
}

/// background_scaler's `ScaleDimensions` for an outer size, the corners' sizes off it, with the
/// pieces then at `FADE` of their height.
fn frame(slice: &mut Clip, assets: &Assets, width: f32, height: f32) -> RenderResult<()> {
    let mut size = |name: &str| -> RenderResult<(f32, f32)> {
        let piece = bounds(&child(slice, name)?.clip, assets)?.ok_or("lobby browser frame piece has no bounds")?;
        Ok((piece.right - piece.left, piece.bottom - piece.top))
    };
    let (left, top) = size("mTopleft")?;
    let (right, bottom) = size("mBottomright")?;
    online_hub_movie::scale_dimensions(slice, assets, width - left - right, height / FADE - top - bottom)?;
    for piece in slice.children.values_mut() {
        piece.entry.matrix.d *= FADE;
        piece.entry.matrix.ty *= FADE;
        piece.clip.script_transform = true;
    }
    Ok(())
}
