//! `FrontEndState_Skate101`'s screen 135, tutorials/skate101 (asset 35,
//! fingerprint fbc3f712; template entry `0x82fdbbc8` `Skate101`, placement 0,
//! drawn flat). The InitAction (character 32, `0x12d4`) defines the `skate101`
//! class translated here and root action `0x1290` constructs it. What its
//! `Mission` natives answer comes from
//! `skate2_core::challenge::system::Skate101View`. FE sounds are not played.
use super::{assets::Assets, clips::Clip, localizer};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::system::Skate101View;

pub(super) const ASSET: usize = 35;

#[derive(Default)]
pub(super) struct Skate101Movie {
    pub root: Option<Clip>,
    /// The Skate 101 state is open: `OnEnter` opened the screen and
    /// `PopState` has not closed it. The state pauses the game and leaves the
    /// HUD as it is (`gFEStateInfo[79]`: `mbPause`, `eHUDState_Ignore`).
    pub open: bool,
    intro_complete: bool,
}

impl Skate101Movie {
    /// `OnEnter`'s `OpenScreen(135)` for a newly open state: a fresh screen,
    /// constructed, then its `intro`; `OnLeave`'s `CloseScreen` once the state
    /// pops: its `outro`.
    pub fn update(&mut self, assets: &Assets, view: Option<&Skate101View>) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                self.intro_complete = false;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "skate101" => construct(&mut root, assets, view)?,
                        _ => return Err(format!("untranslated Skate 101 callback {call}").into()),
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

    /// The frame scripts' calls: frame 1's `Audio.Play101OverlayIn` and frame
    /// 15's `Audio.Play101OverlayOut` (FE sounds are not played), frame 14's
    /// `IntroComplete` and frame 29's `OutroComplete`, which unloads it.
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "Play101OverlayIn" | "Play101OverlayOut" => {}
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => {
                    self.root = None;
                    break;
                }
                _ => return Err(format!("untranslated Skate 101 callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new skate101(this)`: `mScreen.stop()`; the title
/// `FELanguage.GetLocalizedFormatStringCustom(Mission.GetSkate101TitleString(),
/// "x")` (`0x826250e8`); the description text
/// `Mission.GetSkate101DescriptionString()` (`0x82625118`); the description's
/// `maxscroll` and `ScreenManager.CalculateFontHeight(18)` give a height
/// nothing reads; `mButton.SetButtonType("A")` (`Button Item.SetButtonType`:
/// the PS3 circle swap never applies on the 360, so the render target takes
/// the name); `RegisterScreen` hands the screen to the `ScreenManager`.
fn construct(root: &mut Clip, assets: &Assets, view: &Skate101View) -> RenderResult<()> {
    root.playback.stop();
    root.named(&["mcTitle", "mcHeader", "labelText"])?.text = Some(localizer::format_custom(assets, &view.title, "x")?);
    root.named(&["mcBodyText", "mcDescription", "labelText"])?.text = Some(assets.initial_text(&view.text));
    root.named(&["mButton", "mButtonRender"])?.properties.insert("_test".into(), "A".into());
    Ok(())
}

/// skate101.apt's InitAction defines the guarded global class translated
/// above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 32, 0x12d4))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class; frame
/// 1 calls `StartIntro` on `mcBodyText.mcMenuPanel.mcMenu.mMenuPicker`, which
/// the movie does not have, and plays the intro sound; frame 14 stops on
/// `IntroComplete`; frame 15 plays the outro sound; frame 29 stops on
/// `OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x1290 => clip.calls.push("skate101"),
        0x150c => clip.calls.push("Play101OverlayIn"),
        0x1550 => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        0x1574 => clip.calls.push("Play101OverlayOut"),
        0x1590 => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated Skate 101 APT action {offset:#x}").into()),
    }
    Ok(())
}
