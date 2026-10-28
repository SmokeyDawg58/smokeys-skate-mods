//! `FrontEndState_Receipt`'s screen 122, cas/receipt (asset 74, template entry `0x82fdbaf8`
//! `Receipt`, placement 0). The InitAction (character 47, `0x1b68`) defines the `receipt` class
//! translated here and root action `0x1b40` constructs it; its `Receipt_*` game calls (`GameCmdTranslator`
//! `0x82620b40..0x82620d78`) answer from [`ReceiptView`]. `mMenu`, `mButtonmenu` and
//! `mScrollbar` are the shared menu picker, button menu and vertical slider.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    localizer, map_movie,
    pause_movie::{bounds, child},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{
    apt_timeline::{DisplayEntry, IDENTITY, PlacementIdentity},
    edit_skater::{RECEIPT_OPTIONS, ReceiptLine, ReceiptView},
};

pub(super) const ASSET: usize = 74;
/// `receipt.MAX_NUM_RECEIPT_ITEMS`.
const MAX_ITEMS: usize = 8;
/// `receipt.mScrollbarLength`.
const SCROLLBAR_LENGTH: f32 = 245.;
/// `UpdateSelectedReceiptItem`: the highlighted line's `textColor` and every other line's.
const SELECTED: u32 = 0xff22_1c0a;
const UNSELECTED: u32 = 0xffd8_c279;
/// The menu picker's `ITEMLABEL_DARK`, its `LIGHT_ON_DARK` default.
const ITEM_LABEL: u32 = 31;

#[derive(Default)]
pub(super) struct ReceiptMovie {
    pub root: Option<Clip>,
    /// The receipt state is pushed: `OnEnter` opened the screen and `OnCompleteImpl`'s pop has
    /// not closed it.
    pub open: bool,
    /// `mWindowIndexMin`.
    window_start: usize,
    previous: Option<ReceiptView>,
}

impl ReceiptMovie {
    /// `OnEnter`'s `OpenScreen(122)` for a newly pushed receipt: a fresh screen, constructed and
    /// loaded (the script's `OnLoaded`, then the state's `SetScreenState` for its mode), then its
    /// `intro`; each later change is the script call that makes it; once the receipt pops,
    /// `CloseScreen` plays the `outro`.
    pub fn update(&mut self, assets: &Assets, view: Option<&ReceiptView>) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "receipt" => {}
                        _ => return Err(format!("untranslated receipt callback {call}").into()),
                    }
                }
                self.window_start = 0;
                construct(&mut root, assets, view)?;
                show(&mut root, assets, view, None, &mut self.window_start)?;
                root.goto(assets, "intro")?;
                self.root = Some(root);
                self.previous = Some(view.clone());
                self.calls()?;
            }
            Some(view) if self.previous.as_ref() != Some(view) => {
                if let Some(root) = self.root.as_mut() {
                    show(root, assets, view, self.previous.as_ref(), &mut self.window_start)?;
                }
                self.previous = Some(view.clone());
            }
            None if self.open => {
                self.open = false;
                self.previous = None;
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

    /// The timeline's calls: frame 14's `IntroComplete` and frame 28's `OutroComplete`, which
    /// unloads the screen.
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "IntroComplete" => {}
                "OutroComplete" => {
                    self.root = None;
                    break;
                }
                _ => return Err(format!("untranslated receipt callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// The constructor's title, labels and `mCash` (`GetPlayerCash` through
/// `GetLocalizedFormatCurrency`), then `OnLoaded`'s `InitMenuPicker`: the three options as
/// the picker's items, one `dummy0` height apart.
fn construct(root: &mut Clip, assets: &Assets, view: &ReceiptView) -> RenderResult<()> {
    let receipt = root.named(&["mReceipt"])?;
    for name in ["titleText", "titleText_glow"] {
        receipt.named(&[name])?.text = Some(assets.initial_text("ID_CAC_RECEIPT_TITLE"));
    }
    for (name, label) in [
        ("cashText", "ID_CAC_RECEIPT_CASH"),
        ("subtotalText", "ID_CAC_RECEIPT_SUBTOTAL"),
        ("balanceText", "ID_CAC_RECEIPT_BALANCE"),
    ] {
        receipt.named(&["mLabels", name])?.text = Some(assets.initial_text(label));
    }
    receipt.named(&["mCash", "labelText"])?.text = Some(currency(assets, view.cash));
    let picker = receipt.named(&["mMenu", "mMenuPicker"])?;
    let height = bounds(picker.named(&["dummy0"])?, assets)?.map_or(0., |r| r.bottom - r.top);
    for i in 0..8 {
        picker.named(&[&format!("dummy{i}")])?.visible = false;
    }
    let group = picker.named(&["mItemGroup"])?;
    for i in 0..RECEIPT_OPTIONS.len() {
        let item = Clip::new(assets, 6, ITEM_LABEL)?;
        let mut matrix = IDENTITY;
        matrix.ty = height * i as f32;
        group.children.insert(
            i as i32,
            Child {
                entry: DisplayEntry {
                    identity: PlacementIdentity { character: ITEM_LABEL, frame: 0 },
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
    Ok(())
}

/// The receipt as its script last drew it: the picker's `Update` and `UpdateText` for a new
/// value, `SetState`'s picker or list and `EnableReceiptList`'s button help for a new state,
/// the list window (`Next` and `Prev` move `mWindowIndexMin` only when the row leaves it),
/// each window line (`UpdateReceiptListItem`), the highlight (`UpdateSelectedReceiptItem`),
/// the slider and `UpdateCash`.
fn show(
    root: &mut Clip,
    assets: &Assets,
    view: &ReceiptView,
    previous: Option<&ReceiptView>,
    window_start: &mut usize,
) -> RenderResult<()> {
    if view.row < *window_start {
        *window_start = view.row;
    } else if view.row >= *window_start + MAX_ITEMS {
        *window_start = view.row + 1 - MAX_ITEMS;
    }
    let receipt = root.named(&["mReceipt"])?;
    receipt.named(&["mMenu"])?.visible = !view.list;
    if previous.is_none_or(|previous| previous.option != view.option) {
        let picker = receipt.named(&["mMenu", "mMenuPicker"])?;
        let group = picker.named(&["mItemGroup"])?;
        for (i, label) in RECEIPT_OPTIONS.iter().enumerate() {
            let item = group.named(&[&format!("item{i}")])?;
            item.goto(assets, if i == view.option { "selected" } else { "deselected" })?;
            let text = assets.initial_text(label);
            item.named(&["itemText"])?.text = Some(text.clone());
            item.named(&["mTextglow", "itemText"])?.text = Some(text);
        }
        let y = child(group, &format!("item{}", view.option))?.entry.matrix.ty;
        child(picker, "mIndicator")?.entry.matrix.ty = y;
    }
    if previous.is_none_or(|previous| previous.list != view.list) {
        let items: &[(&str, &str)] =
            if view.list { &[("B", "ID_COMMON_BACK"), ("X", "ID_CAC_RECEIPT_ITEMTOGGLE")] } else { &[("A", "ID_COMMON_SELECT")] };
        map_movie::configure_button_items(receipt.named(&["mButtonmenu"])?, assets, items, 0xffff_ffec)?;
    }
    for i in 0..MAX_ITEMS {
        let index = *window_start + i;
        let item = receipt.named(&[&format!("mItem{i}")])?;
        item.visible = index < view.lines.len();
        let Some(line) = view.lines.get(index) else { continue };
        let highlighted = view.list && index == view.row;
        item.goto(assets, if highlighted { "on" } else { "off" })?;
        item.named(&["mButton"])?.goto(assets, if line.kept { "on" } else { "off" })?;
        let colour = if highlighted { SELECTED } else { UNSELECTED };
        for (name, text) in [("nameText", line_text(assets, line)?), ("amountText", currency(assets, line.price))] {
            let field = item.named(&[name])?;
            field.text = Some(text);
            field.properties.insert("text_colour".into(), colour.to_string());
        }
    }
    map_movie::update_scrollbar(
        receipt.named(&["mScrollbar"])?,
        assets,
        view.lines.len(),
        MAX_ITEMS,
        *window_start,
        SCROLLBAR_LENGTH,
    )?;
    receipt.named(&["mSubtotal", "labelText"])?.text = Some(currency(assets, view.subtotal));
    receipt.named(&["mBalance", "labelText"])?.text = Some(currency(assets, view.balance));
    Ok(())
}

/// `FormatCurrency`, as `GetItemPrice`, `GetSubtotal` and `GetBalance` (`0x82649cb0`,
/// `0x82649d20`, `0x82649db0`) write it.
fn currency(assets: &Assets, value: i32) -> Vec<u16> {
    localizer::format_currency(assets, value as f32).encode_utf16().collect()
}

/// `GetItemText` (`0x82681198`) for a line: `ID_CAC_RECEIPT_LINEITEM` with the packed component
/// type and part description.
fn line_text(assets: &Assets, line: &ReceiptLine) -> RenderResult<Vec<u16>> {
    let description = line
        .description_candidates
        .iter()
        .find(|key| assets.localize(key).is_ok())
        .or(line.description_candidates.first())
        .map_or("", String::as_str);
    localizer::format_string_labels(assets, "ID_CAC_RECEIPT_LINEITEM", &[&line.component, description])
}

/// receipt.apt's InitAction defines the guarded global class translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 47, 0x1b68))
}

/// Root actions besides `Stop`: frame 0's construction, frame 14's `IntroComplete` and frame
/// 28's `OutroComplete`, each after `stop()`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x1b40 => clip.calls.push("receipt"),
        0x244c => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        0x2478 => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated receipt APT action {offset:#x}").into()),
    }
    Ok(())
}
