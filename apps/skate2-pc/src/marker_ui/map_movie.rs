//! Retained original ChallengeMap, slider and LetterBox presentation.
//! Narrow FELocation branch: career/news/challenge data are not fabricated.
use super::{
    assets::Assets,
    clips::{Child, Clip},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    challenge::stuff_menu::MapPopup,
    ui::apt_timeline::{DisplayEntry, IDENTITY, PlacementIdentity},
};
use skate2_formats::apt::{Character, Rect};

/// The info component as `challenge_info.SetMode` fills it: the header, the picker's one option
/// (a HAL id) and, in the purchase mode, the property's texts.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct InfoPanel {
    pub title: String,
    pub option: &'static str,
    pub property: Option<PropertyPanel>,
}

/// `SetMode(4)`'s purchase texts, as the screen shows them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PropertyPanel {
    pub description: String,
    /// The texture `mRenderImage0.LoadRX2Texture` loaded, if the image exists.
    pub image: Option<String>,
    /// `GetPropertyPriceOrPaid`: the formatted price, or `ID_CIC_PURCHASED`'s text.
    pub price: String,
    /// `Cash_GetFormattedCashString` of the wallet, while the property is for sale.
    pub cash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HomPathPanel {
    pub reached: usize,
    pub total: usize,
    pub objective: String,
}

pub(super) struct MapMovie {
    pub root: Clip,
    pub letterbox: Clip,
    pub intro_complete: bool,
    pub outro_complete: bool,
    pub sounds: Vec<&'static str>,
    visible: bool,
    window_start: usize,
    previous_rows: Vec<(String, bool)>,
    previous_selected: Option<usize>,
    previous_info: Option<InfoPanel>,
    previous_hom: Option<[HomPathPanel; 4]>,
    letter_info: bool,
    /// The bottom menu's buttons as `UpdateLetterBox` last added them.
    buttons: Vec<(&'static str, &'static str)>,
    /// `FrontEndState_Popup`'s sk8popup (asset 25) over the map, and the template and option it
    /// shows.
    pub popup: Option<Clip>,
    popup_shown: Option<(MapPopup, usize)>,
}

impl MapMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, 9, 0)?;
        root.playback.stop();
        root.visible = false;
        let mut letterbox = Clip::new(assets, 11, 0)?;
        letterbox.playback.stop();
        letterbox.visible = false;
        Ok(Self {
            root,
            letterbox,
            intro_complete: false,
            outro_complete: true,
            sounds: Vec::new(),
            visible: false,
            window_start: 0,
            previous_rows: Vec::new(),
            previous_selected: None,
            previous_info: None,
            previous_hom: None,
            letter_info: false,
            buttons: Vec::new(),
            popup: None,
            popup_shown: None,
        })
    }

    /// Rows and titles are already localized, including native category counts.
    pub fn update(
        &mut self,
        assets: &Assets,
        visible: bool,
        rows: &[(&str, bool)],
        selected: usize,
        title: &str,
        info: Option<&InfoPanel>,
    ) -> RenderResult<()> {
        self.sounds.clear();
        if self.visible != visible {
            self.visible = visible;
            self.intro_complete = false;
            self.outro_complete = false;
            if visible {
                self.root = Clip::new(assets, 9, 0)?;
                self.root.playback.stop();
                self.letterbox = Clip::new(assets, 11, 0)?;
                self.letterbox.playback.stop();
                self.configure(assets)?;
                self.previous_rows.clear();
                self.previous_selected = None;
                self.previous_info = None;
                self.previous_hom = None;
                self.window_start = 0;
                self.letter_info = false;
                self.root.goto(assets, "intro")?;
                self.letterbox
                    .named(&["mcLetterbox"])?
                    .goto(assets, "niceintro")?;
            } else {
                self.root.goto(assets, "outro")?;
                if self.letter_info {
                    self.letter_info = false;
                    self.letterbox
                        .named(&["mcLetterbox", "mInfo"])?
                        .goto(assets, "infooutro")?;
                }
                self.letterbox
                    .named(&["mcLetterbox"])?
                    .goto(assets, "niceoutro")?;
            }
        }
        if visible {
            if info.is_none() {
                let changed = self.previous_rows.len() != rows.len()
                    || self
                        .previous_rows
                        .iter()
                        .zip(rows)
                        .any(|(old, new)| old.0 != new.0 || old.1 != new.1);
                if changed {
                    self.window_start = 0;
                }
                if selected < self.window_start {
                    self.window_start = selected;
                } else if selected >= self.window_start + 9 {
                    self.window_start = selected - 8;
                }
                if changed || self.previous_selected != Some(selected) {
                    let picker =
                        self.root
                            .named(&["mcStuffPanel", "mMenuPicker", "mMenuPicker"])?;
                    // PlaySelectHighlightRegion(Stuff), APT 5cb8..5cc2,
                    // clears the constructor's SetCursorInvisible(true).
                    update_picker(picker, assets, rows, selected, self.window_start, 9, false)?;
                    update_slider(
                        self.root.named(&["mcStuffPanel", "mScroller"])?,
                        assets,
                        rows.len(),
                        self.window_start,
                    )?;
                    self.previous_rows = rows.iter().map(|(s, b)| ((*s).into(), *b)).collect();
                    self.previous_selected = Some(selected);
                }
            }
            self.root
                .named(&["mcStuffPanel", "mcTextStuff", "stuffTitleText"])?
                .text = Some(title.encode_utf16().collect());
            if self.previous_info.as_ref() != info {
                self.root.named(&["mcStuffPanel"])?.visible = info.is_none();
                self.root.named(&["mcInfoPanel"])?.visible = info.is_none();
                self.root
                    .named(&["mTallHighlight"])?
                    .goto(assets, if info.is_some() { "on" } else { "off" })?;
                let clip = self.root.named(&["mcChallengeInfo"])?;
                clip.visible = info.is_some();
                // SetState 20 is SetMode(3), 22 (a property) SetMode(4).
                match info {
                    Some(InfoPanel { title, option, property: None }) => configure_info(clip, assets, title, option)?,
                    Some(InfoPanel { title, option, property: Some(property) }) => {
                        configure_property(clip, assets, title, option, property)?;
                    }
                    None => {}
                }
                self.previous_info = info.cloned();
            }
        }
        self.callbacks(assets)
    }

    /// Original SetState40/41 and SetHomPanelData66e4..67bc. All four path
    /// sprites, headers, completed stamps and placement come from the APT.
    pub fn update_hom(&mut self, hom: Option<&[HomPathPanel; 4]>) -> RenderResult<()> {
        if self.previous_hom.as_ref() == hom {
            return Ok(());
        }
        self.root.named(&["mcStuffPanel"])?.visible = hom.is_none();
        self.root.named(&["mcInfoPanel"])?.visible = hom.is_none();
        let panel = self.root.named(&["mHallofMeat"])?;
        panel.visible = hom.is_some();
        if let Some(paths) = hom {
            for (index, path) in paths.iter().enumerate() {
                let row = panel.named(&[&format!("mChallenge{index}")])?;
                row.named(&["mCompleted", "valueText"])?.text = Some(
                    format!("#{}/{}", path.reached, path.total).encode_utf16().collect(),
                );
                row.named(&["labelText"])?.text = Some(path.objective.encode_utf16().collect());
                // mChallenge0..3 (characters 38, 42, 46, 50) author no completeText, and APT
                // drops SetHomPanelData's _alpha and _visible writes to a missing child.
                if let Some(complete) = authored_child(row, "completeText") {
                    complete.clip.visible = path.reached == path.total;
                    if complete.clip.visible {
                        // mHomPathCompleteTextAlpha=50. Native counts the active
                        // challenge too; retain that comparison exactly.
                        complete.entry.colour_scale = (complete.entry.colour_scale & 0x00ff_ffff) | 0x7f00_0000;
                        complete.clip.script_transform = true;
                    }
                }
            }
        }
        self.previous_hom = hom.cloned();
        Ok(())
    }

    /// `UpdateLetterBox` (`challengemap.apt` `0x5e64`): `ClearBottomMenu`, each `AddMenuItem`,
    /// `RefreshBottomMenu`, when the configuration's buttons change.
    pub fn set_buttons(
        &mut self,
        assets: &Assets,
        items: &[(&'static str, &'static str)],
    ) -> RenderResult<()> {
        if self.buttons != items {
            configure_button_items(
                self.letterbox
                    .named(&["mcLetterbox", "mInfo", "mcBottomMenu", "mButtonMenu"])?,
                assets,
                items,
                0xffff_ffec,
            )?;
            self.buttons = items.to_vec();
        }
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if self.root.visible {
            self.root.tick(assets)?;
        }
        if self.letterbox.visible {
            self.letterbox.tick(assets)?;
        }
        if let Some(popup) = self.popup.as_mut() {
            popup.tick(assets)?;
            let mut calls = Vec::new();
            popup.take_calls(&mut calls);
            for call in calls {
                match call {
                    "CacGenderPopupIntroComplete" => {}
                    "CacGenderPopupOutroComplete" => self.popup = None,
                    _ => return Err(format!("untranslated map popup callback {call}").into()),
                }
            }
        }
        self.callbacks(assets)
    }

    /// The popup over the map (`FrontEndManager::OpenPopup`'s sk8popup): a new popup state's
    /// intro, its template's title, description and options with the picker's value, or its
    /// outro once it closes.
    pub fn update_popup(&mut self, assets: &Assets, popup: Option<(MapPopup, usize)>) -> RenderResult<()> {
        if popup == self.popup_shown {
            return Ok(());
        }
        if let Some((template, selected)) = popup {
            let clip = match self.popup.as_mut() {
                // ConfirmPurchaseCallback can open the dismissal warning as
                // its purchase popup closes. A different template is a new
                // popup state, with its own authored intro, not a cursor edit.
                Some(clip) if self.popup_shown.is_some_and(|(old, _)| old == template) => clip,
                _ => {
                    let mut clip = Clip::new(assets, 25, 0)?;
                    clip.goto(assets, "intro")?;
                    self.popup.insert(clip)
                }
            };
            super::edit_skater_movie::configure_popup(
                clip,
                assets,
                selected,
                template.title(),
                template.description(),
                template.options(),
                false,
            )?;
        } else if let Some(clip) = self.popup.as_mut() {
            clip.goto(assets, "outro")?;
        }
        self.popup_shown = popup;
        Ok(())
    }

    fn callbacks(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        self.letterbox.take_calls(&mut calls);
        for call in calls {
            match call {
                // Frames 1 and 35: ScreenManager.EnableDimmer, a method screenmanager.apt never
                // defines; FE state 21's mbDimmer drives the dimmer (MarkerUi::sync_dimmer).
                "MapDimmerOn" | "MapDimmerOff" => {}
                "MapIntroComplete" => {
                    self.intro_complete = true;
                    self.enable_letter_info(assets)?;
                }
                "MapOutroComplete" => {
                    self.outro_complete = true;
                    self.root.visible = false;
                }
                "LetterIntroComplete" => self.enable_letter_info(assets)?,
                "LetterOutroComplete" => self.letterbox.visible = false,
                "FE_SOUND_PAGETITLE_FLICKER"
                | "FE_SOUND_LETTERBOX_IN"
                | "FE_SOUND_LETTERBOX_OUT" => self.sounds.push(call),
                _ => return Err(format!("untranslated map callback {call}").into()),
            }
        }
        Ok(())
    }

    fn enable_letter_info(&mut self, assets: &Assets) -> RenderResult<()> {
        if !self.letter_info {
            self.letter_info = true;
            self.letterbox
                .named(&["mcLetterbox", "mInfo"])?
                .goto(assets, "infointro")?;
        }
        Ok(())
    }

    fn configure(&mut self, assets: &Assets) -> RenderResult<()> {
        // challengemap constructor59a4 / SetState21,30; no career news owner.
        self.root.named(&["mHallofMeat"])?.visible = false;
        self.root.named(&["mcChallengeInfo"])?.visible = false;
        self.root.named(&["mTallHighlight"])?.goto(assets, "off")?;
        self.root
            .named(&["mcStuffPanel", "mcTextStuff"])?
            .goto(assets, "off")?;
        let news = self.root.named(&["mcInfoPanel"])?;
        news.named(&["mNewsHeader"])?.goto(assets, "normal")?;
        for i in 0..3 {
            let item = news.named(&[&format!("mNewsItem{i}")])?;
            item.goto(assets, if i == 0 { "on" } else { "off" })?;
            if i == 0 {
                item.named(&["mText", "labelText"])?.text = Some(
                    assets
                        .localize("ID_SCREEN_MAP_NEWS_DEFAULT")?
                        .encode_utf16()
                        .collect(),
                );
                item.named(&["mIcon"])?.goto(assets, "news")?;
            }
        }
        self.letterbox
            .named(&["mcLetterbox", "mInfo", "mTitle", "mTitleText", "labeltext"])?
            .text = Some(
            assets
                .localize("ID_SCREEN_MAP_TITLE")?
                .encode_utf16()
                .collect(),
        );
        // UpdateLetterBox Basic: B/Back added first, A/Select second; the
        // imported button menu's default justification is right.
        configure_buttons(
            self.letterbox
                .named(&["mcLetterbox", "mInfo", "mcBottomMenu", "mButtonMenu"])?,
            assets,
        )?;
        self.buttons = vec![("B", "ID_COMMON_BACK"), ("A", "ID_COMMON_SELECT")];
        Ok(())
    }
}

fn configure_info(info: &mut Clip, assets: &Assets, title: &str, option: &str) -> RenderResult<()> {
    // TriggerInfoComponent_FELocation8265f180 emits72,20 -> SetMode3.
    // Mission IsDescriptionEnabled82625b40=false for FELocation(substate3);
    // IsLocationImageEnabled826256d8=false without a Challenge owner.
    // A career challenge's (substate 0) description, objectives and location image follow its
    // challenge_local_data flags (IsDescriptionEnabled, IsObjectivesEnabled82625850,
    // IsLocationImageEnabled); this panel draws none of them.
    for child in info.children.values_mut() {
        child.clip.visible = matches!(child.entry.name.as_deref(), Some("mHeader" | "mMenuPicker"));
    }
    info.named(&["mHeader", "labelText"])?.text = Some(title.encode_utf16().collect());
    let header = child(info, "mHeader")?;
    let y = header.entry.matrix.ty + height(&header.clip, assets)? + 10.;
    // challenge_info.UpdatePositions3ee4 computes the body offset first;
    // 3f6d..40db then selects smalltext/smalltextcenter for wrapped titles.
    set_header(&mut header.clip, assets, title)?;
    child(info, "mMenuPicker")?.entry.matrix.ty = y;
    let label = assets.localize(option)?;
    update_picker(
        info.named(&["mMenuPicker", "mMenuPicker"])?,
        assets,
        &[(label.as_str(), true)],
        0,
        0,
        1,
        false,
    )
}

/// `challenge_info.SetMode(4)` (`challenge_info.apt` `0x482e`, the PurchaseProperty branch at
/// `0x4fb4`) and `UpdatePositions` (`0x3ec4`): every element off, then the header
/// (`GetChallengeTitle`), the description, the location image, the price or `ID_CIC_PURCHASED`,
/// the picker's one option and, for a property for sale, the wallet (`mCash`, its value right of
/// the auto-sized label); laid out from the header down.
fn configure_property(info: &mut Clip, assets: &Assets, title: &str, option: &str, property: &PropertyPanel) -> RenderResult<()> {
    for child in info.children.values_mut() {
        child.clip.visible = match child.entry.name.as_deref() {
            Some("mHeader" | "mRenderImage0" | "mDescription" | "mPriceOrPurchased" | "mMenuPicker") => true,
            Some("mCash") => property.cash.is_some(),
            _ => false,
        };
    }
    let header = child(info, "mHeader")?;
    let mut y = header.entry.matrix.ty + height(&header.clip, assets)? + 10.;
    set_header(&mut header.clip, assets, title)?;
    let image = info.named(&["mRenderImage0"])?;
    match &property.image {
        Some(texture) => {
            image.properties.insert("cac_texture".into(), texture.clone());
        }
        None => {
            image.properties.remove("cac_texture");
        }
    }
    let image = child(info, "mRenderImage0")?;
    image.entry.matrix.ty = y;
    y += height(&image.clip, assets)? * image.entry.matrix.d.abs() + 10.;
    info.named(&["mDescription", "labelText"])?.text = Some(property.description.encode_utf16().collect());
    y += 5.;
    let description = child(info, "mDescription")?;
    description.entry.matrix.ty = y;
    y += (16. * lines(description.clip.named(&["labelText"])?, assets)?).floor() + 15.;
    info.named(&["mPriceOrPurchased", "labelText"])?.text = Some(property.price.encode_utf16().collect());
    let price = child(info, "mPriceOrPurchased")?;
    price.entry.matrix.ty = y;
    y += (16. * lines(price.clip.named(&["labelText"])?, assets)?).floor() + 5.;
    let label = assets.localize(option)?;
    let picker = info.named(&["mMenuPicker", "mMenuPicker"])?;
    update_picker(picker, assets, &[(label.as_str(), true)], 0, 0, 1, false)?;
    // GetHeight: mHeight, the one item's mItemOffset (dummy0._height) in the picker's own space.
    let item_height = height(picker.named(&["dummy0"])?, assets)?;
    child(info, "mMenuPicker")?.entry.matrix.ty = y;
    y += item_height + 10.;
    if let Some(cash) = &property.cash {
        let wallet = child(info, "mCash")?;
        wallet.entry.matrix.ty = y;
        let label = child(&mut wallet.clip, "totalbetHALText")?;
        label.clip.properties.insert("text_autosize".into(), "left".into());
        let right = label.entry.matrix.tx + super::objective_movie::text_field_width(&label.clip, assets)?;
        let value = child(&mut wallet.clip, "totalbetText")?;
        value.clip.text = Some(cash.encode_utf16().collect());
        value.entry.matrix.tx = right;
    }
    Ok(())
}

/// The header's title; a title its field wraps (`maxscroll > 1`) goes to `smalltext`, and one
/// that then fits a line to `smalltextcenter` (`UpdatePositions` `0x3f6d`), each frame's field
/// given the title again.
fn set_header(header: &mut Clip, assets: &Assets, title: &str) -> RenderResult<()> {
    let text: Vec<u16> = title.encode_utf16().collect();
    header.named(&["labelText"])?.text = Some(text.clone());
    if lines(header.named(&["labelText"])?, assets)? > 1. {
        header.goto(assets, "smalltext")?;
        header.named(&["labelText"])?.text = Some(text.clone());
        if lines(header.named(&["labelText"])?, assets)? == 1. {
            header.goto(assets, "smalltextcenter")?;
            header.named(&["labelText"])?.text = Some(text);
        }
    }
    Ok(())
}

/// A text field's `maxscroll`: a multiline field's line count, 1 for a single line.
fn lines(field: &Clip, assets: &Assets) -> RenderResult<f32> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("map maxscroll target is not text".into());
    };
    if !text.multiline {
        return Ok(1.);
    }
    Ok(super::tutorial_movie::max_scroll(field, assets)? as f32)
}

pub(super) fn update_picker(
    picker: &mut Clip,
    assets: &Assets,
    rows: &[(&str, bool)],
    selected: usize,
    start: usize,
    visible_rows: usize,
    cursor_invisible: bool,
) -> RenderResult<()> {
    if !rows.is_empty() && selected >= rows.len() {
        return Err("map row selection outside menu".into());
    }
    let item_asset=picker.properties.get("picker_item_asset").map_or(Ok(6),|s|s.parse::<usize>())?;
    let character=if let Some(name)=picker.properties.get("picker_item_export") {
        assets.movies[item_asset].movie.exports.iter().find(|e|e.name==*name)
            .ok_or_else(||format!("missing picker export {name}"))?.character_id
    } else {31};
    let item_height=if let Some(offset)=picker.properties.get("picker_item_offset") {offset.parse::<f32>()?}
        else {height(picker.named(&["dummy0"])?, assets)?};
    for child in picker.children.values_mut() {
        if child
            .entry
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("dummy"))
        {
            child.clip.visible = false;
        }
    }
    let group = picker.named(&["mItemGroup"])?;
    // menu_picker SetValue2b00 retains the scrolling window, then UpdateText.
    for i in 0..visible_rows {
        if !group.children.contains_key(&(i as i32)) {
            let item = Clip::new(assets, item_asset, character)?;
            let mut matrix = IDENTITY;
            matrix.ty = item_height * i as f32;
            group.children.insert(
                i as i32,
                Child {
                    entry: DisplayEntry {
                        identity: PlacementIdentity {
                            character,
                            frame: 0,
                        },
                        matrix,
                        colour_scale: u32::MAX,
                        colour_bias: 0,
                        ratio: 0.,
                        name: Some(format!("item{i}")),
                        clip_depth: -1,
                        events: Vec::new(),
                        blend_mode: None,
                    },
                    clip: item,
                },
            );
        }
        let item = group.named(&[&format!("item{i}")])?;
        item.visible = start + i < rows.len();
        if let Some((label, enabled)) = rows.get(start + i) {
            item.goto(
                assets,
                if !enabled {
                    "disabled"
                } else if start + i == selected {
                    "selected"
                } else {
                    "deselected"
                },
            )?;
            let text: Vec<u16> = label.encode_utf16().collect();
            item.named(&["itemText"])?.text = Some(text.clone());
            if item_asset==6 {item.named(&["mTextglow", "itemText"])?.text = Some(text);}
        }
    }
    if item_asset!=6 {return Ok(());}
    let indicator = child(picker, "mIndicator")?;
    indicator.clip.visible = !cursor_invisible && !rows.is_empty();
    indicator.entry.matrix.ty = item_height * selected.saturating_sub(start) as f32;
    Ok(())
}

fn update_slider(
    slider: &mut Clip,
    assets: &Assets,
    count: usize,
    start: usize,
) -> RenderResult<()> {
    update_scrollbar(slider, assets, count, 9, start, 275.)
}

/// Shared basic_slider geometry; the caller supplies the native window start.
pub(super) fn update_scrollbar(
    slider: &mut Clip, assets: &Assets, count: usize, visible_rows: usize,
    start: usize, slider_length: f32,
) -> RenderResult<()> {
    let bar = slider.named(&["mcScrollbar"])?;
    bar.visible = count > visible_rows;
    if !bar.visible {
        return Ok(());
    }
    // basic_slider SetSliderLength275, SetNumberOfItems(count,9), SetValue.
    let top_y = child(bar, "mArrowTop")?.entry.matrix.ty;
    child(bar, "mArrowBottom")?.entry.matrix.ty = top_y + slider_length;
    for name in ["mGak", "mcEdge"] {
        let h = height(&child(bar, name)?.clip, assets)?;
        child(bar, name)?.entry.matrix.ty = top_y + slider_length - h;
    }
    let begin = top_y + height(&child(bar, "mArrowTop")?.clip, assets)?;
    let end = top_y + slider_length - height(&child(bar, "mArrowBottom")?.clip, assets)?;
    let length = end - begin;
    let indicator_length = length * visible_rows as f32 / count as f32;
    let indicator = bar.named(&["mSliderIndicator"])?;
    for (group, top, mid, bottom) in [
        ("mIndSolid", "mBarTop", "mBarMiddle", "mBarBottom"),
        ("mIndGlow", "mGlowTop", "mGlowMiddle", "mGlowBottom"),
    ] {
        let part = indicator.named(&[group])?;
        let mid_height = height(&child(part, mid)?.clip, assets)?;
        let mid_y = child(part, top)?.entry.matrix.ty + height(&child(part, top)?.clip, assets)?;
        let middle = child(part, mid)?;
        middle.entry.matrix.d = indicator_length / mid_height;
        middle.entry.matrix.ty = mid_y;
        child(part, bottom)?.entry.matrix.ty = mid_y + indicator_length;
    }
    let cap = height(indicator.named(&["mIndSolid", "mBarTop"])?, assets)?;
    child(bar, "mSliderIndicator")?.entry.matrix.ty =
        begin - cap + (length - indicator_length) / (count - visible_rows) as f32 * start as f32;
    Ok(())
}

fn configure_buttons(view: &mut Clip, assets: &Assets) -> RenderResult<()> {
    configure_button_items(view,assets,&[("B","ID_COMMON_BACK"),("A","ID_COMMON_SELECT")],0xffff_ffec)
}
pub(super) fn configure_button_items(view:&mut Clip,assets:&Assets,items:&[(&str,&str)],colour:u32)->RenderResult<()>{
    justified_button_items(view,assets,items,colour,false)
}
/// `SetJustify("left")` before `RefreshItems`: from the original anchor rightwards, each button,
/// its text 10 inside the button's width, then the 25 offset.
pub(super) fn configure_left_button_items(view:&mut Clip,assets:&Assets,items:&[(&str,&str)],colour:u32)->RenderResult<()>{
    justified_button_items(view,assets,items,colour,true)
}
fn justified_button_items(view:&mut Clip,assets:&Assets,items:&[(&str,&str)],colour:u32,left:bool)->RenderResult<()>{
    styled_button_items(view,assets,items,colour,left,"Marker_streak",24.)
}

/// The same button_menu.SetInfo/RefreshItems with the owning screen's font.
pub(super) fn styled_button_items(view:&mut Clip,assets:&Assets,items:&[(&str,&str)],colour:u32,left:bool,font_name:&str,font_height:f32)->RenderResult<()> {
    // GetFontByAptName82851790 resolves Flash names without case and uses the
    // first non-debug font for an unmapped name (Contacts asks for HelveticaBold).
    // Measure with the same resolver used when drawing the text below.
    let font = assets.apt_font(font_name)?;
    // RefreshItems always starts at the original anchor, including when
    // native Options rebuilds its button help while changing subpages.
    let mut x = if let Some(x) = view.properties.get("button_original_start_x") {
        x.parse::<f32>()?
    } else {
        let x = child(view, "mText0")?.entry.matrix.tx;
        view.properties.insert("button_original_start_x".into(), x.to_string());
        x
    };
    for i in 0..4 {
        child(view, &format!("mText{i}"))?.clip.visible = i < items.len();
        child(view, &format!("mButton{i}"))?.clip.visible = i < items.len();
    }
    for (i, (button, label)) in items.iter()
        .enumerate()
    {
        let units: Vec<u16> = assets.localize(label)?.encode_utf16().collect();
        let width = units
            .iter()
            .filter_map(|&c| font.metrics.mapped_glyph(c))
            .map(|g| g.advance * font.scale[0] * font_height)
            .sum::<f32>();
        let button_clip = child(view, &format!("mButton{i}"))?;
        let r = bounds(&button_clip.clip, assets)?.ok_or("map button bounds absent")?;
        let button_width = r.right - r.left;
        let (text_x, button_x);
        if left {
            button_x = x;
            text_x = x + button_width - 10.;
            x = text_x + width + 25.;
        } else {
            text_x = x - width;
            button_x = text_x + 10. - button_width;
            x = button_x - 25.;
        }
        let text = child(view, &format!("mText{i}"))?;
        text.entry.matrix.tx = text_x;
        text.entry.matrix.ty = 0.5 - font.metrics.ascent * font.scale[1] * font_height / 2. - 4.;
        text.clip.text = Some(units);
        text.clip
            .properties
            .insert("text_font".into(), font_name.into());
        text.clip
            .properties
            .insert("text_height".into(), font_height.to_string());
        text.clip
            .properties
            .insert("text_colour".into(), colour.to_string());
        let button_clip = child(view, &format!("mButton{i}"))?;
        button_clip.entry.matrix.tx = button_x;
        button_clip
            .clip
            .named(&["mButtonRender"])?
            .properties
            .insert("_test".into(), (*button).into());
    }
    Ok(())
}

fn child<'a>(clip: &'a mut Clip, name: &str) -> RenderResult<&'a mut Child> {
    authored_child(clip, name).ok_or_else(|| format!("missing authored map child {name}").into())
}
fn authored_child<'a>(clip: &'a mut Clip, name: &str) -> Option<&'a mut Child> {
    clip.children.values_mut().find(|c| c.entry.name.as_deref() == Some(name))
}
fn height(clip: &Clip, assets: &Assets) -> RenderResult<f32> {
    Ok(bounds(clip, assets)?.map_or(0., |r| r.bottom - r.top))
}
fn bounds(clip: &Clip, assets: &Assets) -> RenderResult<Option<Rect>> {
    let mut result = match clip.definition(assets)? {
        Character::Shape { bounds } => Some(*bounds),
        Character::Text(t) => Some(t.bounds),
        _ => None,
    };
    for child in clip.children.values() {
        if let Some(r) = bounds(&child.clip, assets)? {
            let m = child.entry.matrix;
            let xs = [r.left * m.a + m.tx, r.right * m.a + m.tx];
            let ys = [r.top * m.d + m.ty, r.bottom * m.d + m.ty];
            let r = Rect {
                left: xs[0].min(xs[1]),
                right: xs[0].max(xs[1]),
                top: ys[0].min(ys[1]),
                bottom: ys[0].max(ys[1]),
            };
            result = Some(match result {
                None => r,
                Some(o) => Rect {
                    left: o.left.min(r.left),
                    right: o.right.max(r.right),
                    top: o.top.min(r.top),
                    bottom: o.bottom.max(r.bottom),
                },
            });
        }
    }
    Ok(result)
}

pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!(
        (asset, character, offset),
        (9, 143, 0x59e4)
            | (10, 55, 0x1690)
            | (10, 54, 0x1fd8)
            | (10, 53, 0x1ffc)
            | (11, 25, 0xbd4c)
    )
}
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (9, 0x5970 | 0x59e4) | (10, 0x1690 | 0x1fd8 | 0x1ffc) | (11, 0xbd2c | 0xbd4c) => {}
        (9, 0x6868) => clip.calls.push("MapDimmerOn"),
        (9, 0x6884) => {
            clip.playback.stop();
            clip.calls.push("MapIntroComplete");
        }
        (9, 0x68a4) => clip.calls.push("MapDimmerOff"),
        (9, 0x68d0) => {
            clip.playback.stop();
            clip.calls.push("MapOutroComplete");
        }
        (11, 0xc1ac) => clip.calls.push("FE_SOUND_PAGETITLE_FLICKER"),
        (11, 0xc1f8) => clip.calls.push("FE_SOUND_LETTERBOX_IN"),
        (11, 0xc238) => clip.calls.push("FE_SOUND_LETTERBOX_OUT"),
        (11, 0xc21c) => {
            clip.playback.stop();
            clip.calls.push("LetterIntroComplete");
        }
        (11, 0xc25c) => {
            clip.playback.stop();
            clip.calls.push("LetterOutroComplete");
        }
        (12, 0x1348) => {
            clip.visible = true;
            clip.calls.push("FE_SOUND_CORE_FADE");
        }
        (12, 0x1370) => clip.calls.push("FE_SOUND_CORE_FADE"),
        (12, 0x1390) => {
            clip.playback.stop();
            clip.visible = false;
        }
        _ => return Err(format!("untranslated map APT action {}:{offset:#x}", clip.asset).into()),
    }
    Ok(())
}
