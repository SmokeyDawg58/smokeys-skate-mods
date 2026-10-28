//! `FrontEndState_Sponsor`'s screen 21, unlock/sponsoroverlay (asset 75, fingerprint 139c3d55;
//! template `Sponsor`, placement 4, the `g3dSpinContainer`, drawn through `SetApt3DSpinMatrix`
//! as the unlock overlay is). The InitAction (character 40, `0x1f7c`) defines the
//! `sponsoroverlay` class translated here and root action `0x1f60` constructs it.
//! `mHolder.mSponsorSelector` is controls/sponsor_browser (asset 76, fingerprint 4f6cce94), whose
//! selection the core keeps (`skate2_core::ui::sponsor_browser`); its thumbnails and
//! `mHolder.mMediaThumb9` are image renderers whose logos the owner loads.
use super::{
    assets::Assets,
    clips::Clip,
    hud_leaderboard_movie::{find, find_child},
    localizer::{AptArgument, localized_format_string},
    map_movie::update_scrollbar,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    challenge::sponsor_overlay::{SponsorOverlayHud, state},
    ui::{
        apt_projection::Spin,
        sponsor_browser::{MAX_SPONSORS, SponsorBrowser},
    },
};
use skate2_formats::localization::string_hash;

pub(super) const ASSET: usize = 75;
pub(super) const BROWSER: usize = 76;
/// `sponsor_browser.SliderLength`.
const SLIDER_LENGTH: f32 = 380.0;
/// `mSlider`'s initialize event: `mSkin = "Yellow_Glow"`, which `basic_slider`'s constructor
/// applies (`SetSkin(mSkin)`).
const SLIDER_SKIN: &str = "Yellow_Glow";
/// The image renderers: the selector's nine thumbnails, then `mHolder.mMediaThumb9`.
const IMAGES: usize = 10;
const BOARD_IMAGE: usize = IMAGES - 1;
/// `_alpha = 40` of a thumbnail the selection is not on.
const UNSELECTED_ALPHA: u32 = 102;

#[derive(Default)]
pub(super) struct SponsorOverlayMovie {
    pub root: Option<Clip>,
    /// FE state 11 is on the stack: the state hides the in-game HUD and pauses the game
    /// (`gFEStateInfo[11]`).
    pub open: bool,
    /// `mCurrentState`.
    state: i32,
    /// The selector as last shown.
    shown: Option<SponsorBrowser>,
    /// The logo each image renderer's last `LoadRX2Texture` asked for, and the one it shows.
    wanted: [Option<String>; IMAGES],
    bound: [Option<String>; IMAGES],
    intro_complete: bool,
    outro_complete: bool,
    /// The timelines' `Audio.PlaySound` requests.
    pub sounds: Vec<&'static str>,
}

impl SponsorOverlayMovie {
    /// `OnEnter`'s `OpenScreen(21)` for a newly open state: the screen constructed, then its
    /// `intro`, which starts the spin; `OnLeave`'s `CloseScreen` once `NextSelected` popped the
    /// state: `SetScreenState(mActiveState)`'s `SetState` (the state's outro), then the screen's
    /// `outro` and the spin back. While open, the selector follows the core's.
    pub fn update(&mut self, assets: &Assets, hud: Option<&SponsorOverlayHud>, spin: &mut Spin) -> RenderResult<()> {
        match hud {
            Some(hud) if !self.open => self.open_screen(assets, hud, spin)?,
            Some(hud) => {
                if self.shown.as_ref() != Some(&hud.browser) {
                    if let Some(root) = self.root.as_mut() {
                        let moved = self.shown.as_ref().is_none_or(|shown| shown.min_window_index != hud.browser.min_window_index);
                        if moved {
                            fill_thumbs(root, &hud.browser, &hud.view.swfs, &mut self.wanted);
                        }
                        update_selected_thumb_graphic(root, &hud.browser);
                        update_slider(root, assets, &hud.browser)?;
                    }
                    self.shown = Some(hud.browser.clone());
                }
            }
            None if self.open => {
                self.open = false;
                self.shown = None;
                if let Some(root) = self.root.as_mut() {
                    set_state(root, assets, self.state)?;
                    root.goto(assets, "outro")?;
                }
                spin.intro_exit();
                self.calls()?;
            }
            None => {}
        }
        Ok(())
    }

    /// The loaded movie: its children constructed first (the selector's constructor and its
    /// slider's), then root frame 0's `new sponsoroverlay(this)`: `mCurrentState` -1,
    /// `InitState(Sponsor_GetInitialState())`, `RegisterScreen`.
    fn open_screen(&mut self, assets: &Assets, hud: &SponsorOverlayHud, spin: &mut Spin) -> RenderResult<()> {
        self.open = true;
        self.intro_complete = false;
        self.outro_complete = false;
        self.wanted = Default::default();
        self.bound = Default::default();
        let mut root = Clip::new(assets, ASSET, 0)?;
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        if !calls.contains(&"sponsoroverlay") {
            return Err("sponsor overlay root constructed no screen".into());
        }
        construct_selector(&mut root, assets)?;
        self.state = hud.view.state;
        init_state(&mut root, assets, hud, &mut self.wanted)?;
        root.goto(assets, "intro")?;
        spin.intro_start();
        self.shown = Some(hud.browser.clone());
        self.root = Some(root);
        self.handle(calls)?;
        self.calls()
    }

    /// One front-end update: the movie (`AptIntegration::Update`), then its calls.
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(root) = self.root.as_mut() {
            root.tick(assets)?;
        }
        self.calls()
    }

    /// The screen's `ScreenManager.IntroComplete` and `OutroComplete` since the last call.
    pub fn take_screen(&mut self) -> (bool, bool) {
        (std::mem::take(&mut self.intro_complete), std::mem::take(&mut self.outro_complete))
    }

    /// The logos the image renderers asked for that they do not show yet: each renderer and
    /// the logo's path.
    pub fn image_requests(&self) -> Vec<(usize, String)> {
        self.wanted
            .iter()
            .zip(&self.bound)
            .enumerate()
            .filter(|(_, (wanted, bound))| wanted.is_some() && wanted != bound)
            .filter_map(|(index, (wanted, _))| wanted.clone().map(|path| (index, path)))
            .collect()
    }

    /// The image renderer `index` shows `texture`, its logo once loaded, or nothing when the
    /// archive lacks the logo.
    pub fn set_image(&mut self, index: usize, texture: Option<String>) {
        let (Some(wanted), Some(bound)) = (self.wanted.get(index), self.bound.get_mut(index)) else { return };
        bound.clone_from(wanted);
        let Some(root) = self.root.as_mut() else { return };
        let path = image_path(index);
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        let Some(image) = find(root, &path) else { return };
        match texture {
            Some(key) => {
                image.properties.insert("cac_texture".into(), key);
            }
            None => {
                image.properties.remove("cac_texture");
            }
        }
    }

    /// The frame scripts' calls since the last update.
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        self.handle(calls)
    }

    /// The root's frame 35 `IntroComplete` and frame 55 `OutroComplete`, which unloads it, and
    /// the `Audio.PlaySound` requests.
    fn handle(&mut self, calls: Vec<&'static str>) -> RenderResult<()> {
        for call in calls {
            match call {
                "sponsoroverlay" => {}
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => {
                    self.outro_complete = true;
                    self.root = None;
                    break;
                }
                "FE_SOUND_SPONSOR_OVERLAY_INTRO" | "FE_SOUND_SCRATCH_BOX_FX" => self.sounds.push(call),
                _ => return Err(format!("untranslated sponsor overlay callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// The image renderer `index`: a selector thumbnail, or `mMediaThumb9`.
fn image_path(index: usize) -> Vec<String> {
    if index < BOARD_IMAGE {
        vec!["mHolder".to_owned(), "mSponsorSelector".to_owned(), format!("mMediaThumb{index}")]
    } else {
        vec!["mHolder".to_owned(), "mMediaThumb9".to_owned()]
    }
}

/// `gotoAndPlay(label)`.
fn goto(root: &mut Clip, assets: &Assets, path: &[&str], label: &str) -> RenderResult<()> {
    if let Some(clip) = find(root, path) {
        clip.goto(assets, label)?;
    }
    Ok(())
}

/// A text field's `text = label`, resolved as `SkateAptString` does.
fn set_text(root: &mut Clip, assets: &Assets, path: &[&str], label: &str) {
    if let Some(clip) = find(root, path) {
        clip.text = Some(assets.initial_text(label));
    }
}

/// `sponsor_browser`'s constructor: every thumbnail `Init` and hidden. `mSlider`'s
/// (`basic_slider`) `SetSkin(mSkin)`: each part of its scroll bar `gotoAndStop("Yellow_Glow")`.
fn construct_selector(root: &mut Clip, assets: &Assets) -> RenderResult<()> {
    let Some(selector) = find(root, &["mHolder", "mSponsorSelector"]) else { return Ok(()) };
    for slot in 0..BOARD_IMAGE {
        if let Some(thumb) = find(selector, &[&format!("mMediaThumb{slot}")]) {
            thumb.properties.remove("cac_texture");
            thumb.visible = false;
        }
    }
    let Some(slider) = find(selector, &["mSlider"]) else { return Ok(()) };
    let skin = slider.properties.get("mSkin").cloned().unwrap_or_else(|| SLIDER_SKIN.to_owned());
    for path in [
        &["mcScrollbar", "mArrowTop"][..],
        &["mcScrollbar", "mArrowBottom"],
        &["mcScrollbar", "mcEdge"],
        &["mcScrollbar", "mGak"],
        &["mcScrollbar", "mSliderIndicator", "mIndSolid", "mBarTop"],
        &["mcScrollbar", "mSliderIndicator", "mIndSolid", "mBarMiddle"],
        &["mcScrollbar", "mSliderIndicator", "mIndSolid", "mBarBottom"],
        &["mcScrollbar", "mSliderIndicator", "mIndGlow", "mGlowTop"],
        &["mcScrollbar", "mSliderIndicator", "mIndGlow", "mGlowMiddle"],
        &["mcScrollbar", "mSliderIndicator", "mIndGlow", "mGlowBottom"],
    ] {
        if let Some(part) = find(slider, path) {
            part.goto_and_stop(assets, &skin)?;
        }
    }
    Ok(())
}

/// `InitState(state)`: `mHolder`'s intro for the state, the title (`ID_SPONSOR_TAPE_GIVEN`,
/// else `ID_SPONSOR_SPONSORED`) and the description. In the board state the description formats
/// the board sponsor's name (`GetLocaleText`, a packed string) into
/// `ID_SPONSOR_BOARD_SPONSORED_DESCRIPTION` and `mMediaThumb9` loads the board sponsor's logo;
/// the selectable states, and any state it does not know (as the trucks, after four traces),
/// `Init` the selector with their logos.
fn init_state(root: &mut Clip, assets: &Assets, hud: &SponsorOverlayHud, wanted: &mut [Option<String>; IMAGES]) -> RenderResult<()> {
    let view = &hud.view;
    if view.state == state::TAPE_UNLOCKED {
        goto(root, assets, &["mHolder"], "intro_tape")?;
        set_text(root, assets, &["mHolder", "mTitle", "title_txt"], "ID_SPONSOR_TAPE_GIVEN");
        set_text(root, assets, &["mHolder", "mTitle", "titleGlow_txt"], "ID_SPONSOR_TAPE_GIVEN");
        set_text(root, assets, &["mHolder", "mDescription", "labeltext"], "ID_SPONSOR_TAPE_DESCRIPTION");
        return Ok(());
    }
    set_text(root, assets, &["mHolder", "mTitle", "title_txt"], "ID_SPONSOR_SPONSORED");
    set_text(root, assets, &["mHolder", "mTitle", "titleGlow_txt"], "ID_SPONSOR_SPONSORED");
    if view.state == state::BOARD_SPONSORED {
        goto(root, assets, &["mHolder"], "intro_board")?;
        let name = assets
            .localized
            .get(&string_hash(view.board_sponsor_hal_id.as_bytes()))
            .map_or_else(|| " ".to_owned(), |units| String::from_utf16_lossy(units));
        let description = localized_format_string(assets, "ID_SPONSOR_BOARD_SPONSORED_DESCRIPTION", &[AptArgument::Text(&name)])?;
        if let Some(field) = find(root, &["mHolder", "mDescription", "labeltext"]) {
            field.text = Some(description);
        }
        wanted[BOARD_IMAGE] = Some(view.board_sponsor_icon.clone());
        return Ok(());
    }
    goto(root, assets, &["mHolder"], "intro_picker")?;
    let select = match view.state {
        state::WHEEL_SELECTABLE => "ID_SPONSOR_WHEEL_SELECT",
        state::SHOE_SELECTABLE => "ID_SPONSOR_SHOE_SELECT",
        _ => "ID_SPONSOR_TRUCK_SELECT",
    };
    set_text(root, assets, &["mHolder", "mDescription", "labeltext"], select);
    // Init: SetNumItems (the slider), the window at the start, FillThumbs and
    // UpdateSelectedThumbGraphic.
    update_slider(root, assets, &hud.browser)?;
    fill_thumbs(root, &hud.browser, &view.swfs, wanted);
    update_selected_thumb_graphic(root, &hud.browser);
    Ok(())
}

/// `SetState(state)`: `mHolder`'s outro for the state (`SetIndexPicked` is the core's).
fn set_state(root: &mut Clip, assets: &Assets, state: i32) -> RenderResult<()> {
    let outro = match state {
        state::TAPE_UNLOCKED => "outro_tape",
        state::BOARD_SPONSORED => "outro_board",
        _ => "outro_picker",
    };
    goto(root, assets, &["mHolder"], outro)
}

/// `FillThumbs`: each thumbnail shows the item its window slot holds (`LoadRX2Texture` of its
/// logo), and hides past the last item.
fn fill_thumbs(root: &mut Clip, browser: &SponsorBrowser, swfs: &[String], wanted: &mut [Option<String>; IMAGES]) {
    let Some(selector) = find(root, &["mHolder", "mSponsorSelector"]) else { return };
    for (slot, wanted) in (0..MAX_SPONSORS).zip(wanted.iter_mut()) {
        let index = browser.min_window_index + slot;
        let Some(thumb) = find(selector, &[&format!("mMediaThumb{slot}")]) else { continue };
        let item = usize::try_from(index).ok().filter(|_| index < browser.items).and_then(|index| swfs.get(index));
        thumb.visible = item.is_some();
        if let Some(swf) = item {
            *wanted = Some(swf.clone());
        }
    }
}

/// `UpdateSelectedThumbGraphic`: with items, the selected slot's highlighter shown and its
/// thumbnail at `_alpha` 100, the others hidden and at 40.
fn update_selected_thumb_graphic(root: &mut Clip, browser: &SponsorBrowser) {
    if browser.items <= 0 {
        return;
    }
    let Some(selector) = find(root, &["mHolder", "mSponsorSelector"]) else { return };
    for slot in 0..MAX_SPONSORS {
        let selected = slot == browser.window_index;
        if let Some(highlighter) = find(selector, &[&format!("mHighlighter{slot}")]) {
            highlighter.visible = selected;
        }
        if let Some(thumb) = find_child(selector, &format!("mMediaThumb{slot}")) {
            let alpha = if selected { 0xff } else { UNSELECTED_ALPHA };
            thumb.entry.colour_scale = (thumb.entry.colour_scale & 0x00ff_ffff) | (alpha << 24);
            thumb.clip.script_transform = true;
        }
    }
}

/// `mSlider` as `SetNumItems` leaves it (shown while the items outnumber the window, its length
/// `SliderLength`, `SetNumberOfItems(NumItems, MAX_SPONSORS)`), its indicator where
/// `mWindowedValue` puts it.
fn update_slider(root: &mut Clip, assets: &Assets, browser: &SponsorBrowser) -> RenderResult<()> {
    let Some(slider) = find(root, &["mHolder", "mSponsorSelector", "mSlider"]) else { return Ok(()) };
    slider.visible = browser.items > MAX_SPONSORS;
    let count = usize::try_from(browser.slider.items).unwrap_or(0);
    let rows = usize::try_from(browser.slider.rows).unwrap_or(0);
    let start = usize::try_from(browser.slider.windowed).unwrap_or(0);
    update_scrollbar(slider, assets, count, rows, start, SLIDER_LENGTH)
}

/// sponsoroverlay.apt's InitAction defines the guarded global class translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 40, 0x1f7c))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class, frames 35 and 55 report
/// `IntroComplete` and `OutroComplete` and stop; `mHolder` sounds
/// `FE_SOUND_SPONSOR_OVERLAY_INTRO` on the first frame of each intro; each selector highlighter
/// (sponsor_browser character 36, a single frame) sounds `FE_SOUND_SCRATCH_BOX_FX` as it is made.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (ASSET, 0x1f60) => clip.calls.push("sponsoroverlay"),
        (ASSET, 0x2470) => {
            clip.calls.push("IntroComplete");
            clip.playback.stop();
        }
        (ASSET, 0x249c) => {
            clip.calls.push("OutroComplete");
            clip.playback.stop();
        }
        (ASSET, 0x2558 | 0x25a4 | 0x25e8) => clip.calls.push("FE_SOUND_SPONSOR_OVERLAY_INTRO"),
        (BROWSER, 0x2f6c) => clip.calls.push("FE_SOUND_SCRATCH_BOX_FX"),
        _ => return Err(format!("untranslated sponsor overlay APT action {}:{} {offset:#x}", clip.asset, clip.character).into()),
    }
    Ok(())
}
