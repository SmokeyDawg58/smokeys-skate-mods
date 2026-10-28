//! Original mainmenu APT4860d06f; shared menu_picker, fonts and letterbox.
use super::{assets::Assets, clips::Clip, letterbox::LetterBox, map_movie};
use crate::gpu_renderer::RenderResult;
/// PC port change: the online mode is named "Online" wherever the original
/// shows ID_SCREEN_GAME_MODE_ONLINE ("Xbox LIVE").
pub(super) const ONLINE_TITLE: &str = "Online";
pub(super) struct MainMovie {
    pub root: Clip,
    /// The front end's global LetterBox; Options drives it while open.
    pub letterbox: LetterBox,
    pub intro_complete: bool,
    visible: bool,
    /// OnEnter's screen 8 request, until the screen manager links it.
    pending: bool,
    /// OutroComplete queues destruction for the next APT tick.
    destroy: bool,
    outro_complete: bool,
    selected: usize,
    new_profile: bool,
}
impl MainMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, 27, 0)?;
        root.playback.stop();
        root.visible = false;
        Ok(Self {
            root,
            letterbox: LetterBox::new(assets)?,
            intro_complete: false,
            visible: false,
            pending: false,
            destroy: false,
            outro_complete: false,
            selected: 0,
            new_profile: false,
        })
    }
    /// OnGainFocus8264a018 after Options pops: SetState4 retitles the
    /// shared letterbox without animating it.
    pub fn gain_focus(&mut self, assets: &Assets) -> RenderResult<()> {
        update_letterbox_info(&mut self.letterbox, assets)
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn take_outro_complete(&mut self) -> bool {
        std::mem::take(&mut self.outro_complete)
    }
    pub fn update(&mut self, assets: &Assets, visible: bool, selected: usize, new_profile: bool) -> RenderResult<()> {
        let changed = self.selected != selected || self.new_profile != new_profile;
        self.selected = selected;
        self.new_profile = new_profile;
        if visible && !self.visible {
            // OnEnter82659fd8 requests screen8. Its constructor runs on link,
            // after ProcessOpenScreens and the original async loading stages.
            self.pending = true;
            self.visible = true;
            self.intro_complete = false;
        } else if !visible && self.visible {
            // OnLeave8265a0b8's CloseScreen plays the outro.
            self.visible = false;
            self.intro_complete = false;
            if !std::mem::take(&mut self.pending) {
                self.root.goto(assets, "outro")?;
            }
        }
        if visible && !self.pending && changed {
            self.update_picker(assets)?;
        }
        self.callbacks(assets)
    }
    /// ScreenManager.OnLoaded calls InitialStandardScreenReady (APT b30).
    pub fn link(&mut self, assets: &Assets) -> RenderResult<()> {
        if !std::mem::take(&mut self.pending) {
            return Ok(());
        }
        self.root = Clip::new(assets, 27, 0)?;
        self.destroy = false;
        self.outro_complete = false;
        self.update_picker(assets)?;
        // Constructor0641..064c: EnableLetterBox(true), UpdateLetterBoxInfo.
        self.letterbox.enable(assets, true)?;
        update_letterbox_info(&mut self.letterbox, assets)?;
        self.root.goto(assets, "intro")?;
        self.callbacks(assets)
    }
    fn update_picker(&mut self, assets: &Assets) -> RenderResult<()> {
        let text = skate2_core::ui::main_menu::labels(self.new_profile)
            .map(|key| if key == "ID_SCREEN_GAME_MODE_ONLINE" { Ok(ONLINE_TITLE.to_string()) }
                else { assets.localize(key) })
            .into_iter()
            .collect::<RenderResult<Vec<_>>>()?;
        let rows = text.iter().map(|s| (s.as_str(), true)).collect::<Vec<_>>();
        map_movie::update_picker(
            self.root.named(&["mMenu", "mMenuPicker"])?,
            assets,
            &rows,
            self.selected,
            0,
            skate2_core::ui::main_menu::LABELS.len(),
            false,
        )?;
        Ok(())
    }
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if std::mem::take(&mut self.destroy) {
            self.root.visible = false;
        }
        if self.root.visible {
            self.root.tick(assets)?;
        }
        self.letterbox.tick(assets)?;
        self.callbacks(assets)
    }
    fn callbacks(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                // Frame 13: ScreenManager.IntroComplete, EnableLetterBoxInfo(true).
                "MainIntroComplete" => {
                    self.intro_complete = true;
                    self.letterbox.enable_info(assets, true)?;
                }
                // Frame 14: EnableLetterBoxInfo(false), CycleLetterBox.
                "MainOutroBegin" => {
                    self.letterbox.enable_info(assets, false)?;
                    self.letterbox.cycle(assets)?;
                }
                // Frame 22: ScreenManager.OutroComplete.
                "MainOutroComplete" => {
                    self.destroy = true;
                    self.outro_complete = true;
                }
                _ => return Err(format!("untranslated main-menu callback {call}").into()),
            }
        }
        Ok(())
    }
}
/// UpdateLetterBoxInfo069e: the title and the A/Select button help.
fn update_letterbox_info(letterbox: &mut LetterBox, assets: &Assets) -> RenderResult<()> {
    letterbox.set_title(&assets.localize("ID_SCREEN_MAIN_MENU_TITLE")?)?;
    map_movie::configure_button_items(
        letterbox.clip.named(&["mcLetterbox", "mInfo", "mcBottomMenu", "mButtonMenu"])?,
        assets,
        &[("A", "ID_COMMON_SELECT"), ("Y", "#Mod Info")],
        0xffffffec,
    )
}
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x538 | 0x554 => clip.playback.stop(),
        0x82c => {
            clip.playback.stop();
            clip.calls.push("MainIntroComplete");
        }
        0x84c => clip.calls.push("MainOutroBegin"),
        0x870 => {
            clip.playback.stop();
            clip.calls.push("MainOutroComplete");
        }
        _ => return Err(format!("untranslated main-menu APT action {offset:#x}").into()),
    }
    Ok(())
}
