//! `FrontEndState_PartyPlayChangeGame`'s screen 65, `challenge/partyplaychangegame` (asset 68,
//! fingerprint 1f90c718; template entry `PartyPlayChangeGame` `0x82fdb768`, placement 0). Its
//! `InitAction` (character 31, `0x395c`) defines the `partyplaychangegame` class translated here;
//! root action `0x3924` constructs it and the `ScreenManager` plays its `intro`. `mMenu`'s `item0`
//! (the game) and `item1` (the location) are `highlight_selector`s, `mImage` the image renderer
//! and `mcBacker.mButtons` the button menu. What `FEPartyPlay` answers comes from
//! [`ChangeSnapshot`].
use super::{
    assets::Assets,
    clips::Clip,
    map_movie,
    objective_movie::{set_x, text_field_width},
    pause_movie::child,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::party_play::{ChangeSnapshot, Game, change_state};
use skate2_formats::apt::Character;

pub(super) const ASSET: usize = 68;

/// `highlight_selector`'s colours: `mTitleOnColor`, `mTitleOffColor`, `mSubtitleOnColor` and
/// `mSubtitleOffColor`.
const TITLE_ON: u32 = 0xff_ff_ff_ec;
const TITLE_OFF: u32 = 0xff_99_8b_63;
const SUBTITLE_ON: u32 = 0xff_ff_e9_a9;
const SUBTITLE_OFF: u32 = 0xff_5f_4d_34;
/// `TextField.autoSize`'s left alignment.
const LEFT: u32 = 0;

#[derive(Default)]
pub(super) struct PartyPlayChangeMovie {
    pub root: Option<Clip>,
    /// FE state 39 is open: `OnEnter` opened the screen and `OnLeave` has not closed it.
    pub open: bool,
    intro_complete: bool,
}

impl PartyPlayChangeMovie {
    /// `OnEnter`'s `OpenScreen(65)` for a newly open state: a fresh screen, constructed, then its
    /// `intro`; `OnLeave`'s `CloseScreen`: its `outro`. Then each `SetState` the state sent, with
    /// the location image `LoadRX2Texture` names.
    pub fn update(&mut self, assets: &Assets, view: Option<&ChangeSnapshot>, states: &[u32], image: Option<&str>) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "partyplaychangegame" => construct(&mut root, assets, view, image)?,
                        _ => return Err(format!("untranslated party play change game callback {call}").into()),
                    }
                }
                root.goto(assets, "intro")?;
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
        if let (Some(view), Some(root)) = (view.filter(|_| self.open), self.root.as_mut()) {
            for &state in states {
                set_state(root, assets, view, image, state)?;
            }
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

    /// Frame 64's `IntroComplete`, and frame 78's `OutroComplete`, which unloads it.
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => {
                    self.root = None;
                    break;
                }
                _ => return Err(format!("untranslated party play change game callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new partyplaychangegame(movie)`: the screen stopped, the item titles, `mIcon` on `spot`, B
/// Back and A Continue, then `SetState(kState_SelectEvent)`, `RefreshEvent`, `RefreshLocation`
/// and `SetState(kState_RefreshLocationImage)`.
fn construct(root: &mut Clip, assets: &Assets, view: &ChangeSnapshot, image: Option<&str>) -> RenderResult<()> {
    root.playback.stop();
    selector_title(root.named(&["mMenu", "item0"])?, assets, "ID_PARTYPLAY_EVENT")?;
    selector_title(root.named(&["mMenu", "item1"])?, assets, "ID_PARTYPLAY_LOCATION")?;
    root.named(&["mIcon"])?.goto(assets, "spot")?;
    let buttons = root.named(&["mcBacker", "mButtons"])?;
    map_movie::configure_button_items(buttons, assets, &[("B", "ID_COMMON_BACK"), ("A", "ID_COMMON_CONTINUE")], 0xffff_ffec)?;
    set_state(root, assets, view, image, change_state::SELECT_EVENT)?;
    refresh_event(root, assets, view)?;
    refresh_location(root, assets, view)?;
    set_state(root, assets, view, image, change_state::REFRESH_LOCATION_IMAGE)
}

/// `SetState(state)`: the selected item, a refresh, or the location image.
fn set_state(root: &mut Clip, assets: &Assets, view: &ChangeSnapshot, image: Option<&str>, state: u32) -> RenderResult<()> {
    match state {
        change_state::SELECT_EVENT | change_state::SELECT_LOCATION => {
            let event = state == change_state::SELECT_EVENT;
            selector_state(root.named(&["mMenu", "item0"])?, assets, event)?;
            selector_state(root.named(&["mMenu", "item1"])?, assets, !event)
        }
        change_state::REFRESH_EVENT => refresh_event(root, assets, view),
        change_state::REFRESH_LOCATION => refresh_location(root, assets, view),
        change_state::REFRESH_LOCATION_IMAGE => {
            let control = root.named(&["mImage"])?;
            match image {
                Some(texture) => control.properties.insert("cac_texture".into(), texture.to_owned()),
                None => control.properties.remove("cac_texture"),
            };
            control.visible = true;
            Ok(())
        }
        _ => Err(format!("party play change game state {state} is not translated").into()),
    }
}

/// `RefreshEvent`: the game's `mIcon` section and title.
fn refresh_event(root: &mut Clip, assets: &Assets, view: &ChangeSnapshot) -> RenderResult<()> {
    let (icon, title) = match view.game {
        Game::SpotBattle => ("spot", "ID_PARTYPLAY_SPOT_BATTLE_TITLE"),
        Game::Skate => ("skate", "ID_PARTYPLAY_SKATE_TITLE"),
        Game::Hom => ("hom", "ID_PARTYPLAY_HOM_TITLE"),
    };
    root.named(&["mIcon"])?.goto(assets, icon)?;
    selector_subtitle(root.named(&["mMenu", "item0"])?, assets, title)
}

/// `RefreshLocation`: `mImage` hidden until its image loads, and the location's title.
fn refresh_location(root: &mut Clip, assets: &Assets, view: &ChangeSnapshot) -> RenderResult<()> {
    root.named(&["mImage"])?.visible = false;
    selector_subtitle(root.named(&["mMenu", "item1"])?, assets, &view.location)
}

/// `highlight_selector.SetTitle(str)`: both title labels.
fn selector_title(item: &mut Clip, assets: &Assets, label: &str) -> RenderResult<()> {
    let text = assets.initial_text(label);
    for name in ["mTitle", "mTitleglow"] {
        item.named(&[name, "TEXTLABEL"])?.text = Some(text.clone());
    }
    Ok(())
}

/// `highlight_selector.SetSubtitle(str)`: the label auto-sized left, then the right arrow after
/// it.
fn selector_subtitle(item: &mut Clip, assets: &Assets, label: &str) -> RenderResult<()> {
    let field = item.named(&["mSubtitle", "TEXTLABEL"])?;
    field.text = Some(assets.initial_text(label));
    let Character::Text(text) = field.definition(assets)? else {
        return Err("highlight selector subtitle is not text".into());
    };
    if text.multiline || text.alignment != LEFT {
        return Err("highlight selector subtitle differs from the verified layout".into());
    }
    field.properties.insert("text_autosize".into(), "left".into());
    place_arrow(item, assets)
}

/// `mArrowright._x = mSubtitle._x + mSubtitle.TEXTLABEL._width + mArrowMargin` (0).
fn place_arrow(item: &mut Clip, assets: &Assets) -> RenderResult<()> {
    let width = text_field_width(item.named(&["mSubtitle", "TEXTLABEL"])?, assets)?;
    let x = child(item, "mSubtitle")?.entry.matrix.tx;
    set_x(child(item, "mArrowright")?, x + width);
    Ok(())
}

/// `highlight_selector.SetState(script)`: `selected` or `unselected` played, the title's and
/// subtitle's on or off colours, both arrows shown or hidden, and the right one placed.
fn selector_state(item: &mut Clip, assets: &Assets, selected: bool) -> RenderResult<()> {
    item.goto(assets, if selected { "selected" } else { "unselected" })?;
    let (title, subtitle) = if selected { (TITLE_ON, SUBTITLE_ON) } else { (TITLE_OFF, SUBTITLE_OFF) };
    item.named(&["mTitle", "TEXTLABEL"])?.properties.insert("text_colour".into(), title.to_string());
    item.named(&["mSubtitle", "TEXTLABEL"])?.properties.insert("text_colour".into(), subtitle.to_string());
    for arrow in ["mArrowright", "mArrowleft"] {
        item.named(&[arrow])?.visible = selected;
    }
    place_arrow(item, assets)
}

/// The movie's `InitAction`: the `partyplaychangegame` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 31, 0x395c))
}

/// Root timeline actions: frame 0 constructs the class, frame 64 `IntroComplete` and frame 78
/// `OutroComplete`, each then stopping. Frames 1 and 78 also call `ScreenManager.EnableDimmer`, a
/// method screenmanager.apt never defines; FE state 39's `mbDimmer` drives the dimmer.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x3924 => clip.calls.push("partyplaychangegame"),
        0x3d0c => {}
        0x3d28 => {
            clip.calls.push("IntroComplete");
            clip.playback.stop();
        }
        0x3d54 => {
            clip.calls.push("OutroComplete");
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated party play change game APT action {offset:#x}").into()),
    }
    Ok(())
}
