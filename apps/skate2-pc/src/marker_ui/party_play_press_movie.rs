//! `FrontEndState_PressA`'s screen 64, challenge/partyplaypresscontinue (asset 52,
//! fingerprint 6b5d9b87; template entry `PartyPlayPressA`, drawn flat). Its InitActions
//! define `iscreenmodule` (character 50, `0x3884`) and the `partyplaypresscontinue` class
//! (character 51, `0x38bc`) translated here; root action `0x3838` constructs it from
//! `Game.PartyPlay_GetDataBlock` (`0x82622768`, `skate2_core::PressADialog`).
use super::{assets::Assets, clips::Clip, localizer, map_movie};
use crate::gpu_renderer::RenderResult;
use skate2_core::PressADialog;

pub(super) const ASSET: usize = 52;

#[derive(Default)]
pub(super) struct PartyPlayPressMovie {
    pub root: Option<Clip>,
    /// The press-A state is open: `OnEnter` opened the screen and `PopState` has not
    /// closed it.
    pub open: bool,
}

impl PartyPlayPressMovie {
    /// `OnEnter`'s `OpenScreen(64)` for a newly open state: a fresh screen, constructed,
    /// then its `intro`; the state's pop closes it with its `outro`.
    pub fn update(&mut self, assets: &Assets, view: Option<&PressADialog>) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "partyplaypresscontinue" => construct(&mut root, assets, view)?,
                        _ => return Err(format!("untranslated press-continue callback {call}").into()),
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

    /// Frame 50's `IntroComplete` (`ReceiveInput` does not wait for it) and frame 62's
    /// `OutroComplete`, which unloads it.
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
                _ => return Err(format!("untranslated press-continue callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new partyplaypresscontinue(this)`: `RegisterScreen`; `mPanel.mContinue` lists A Accept;
/// the data block `[turn, round, controller, name]` gives `mPlayerinfo.mPlayer`'s
/// `ID_PARTYPLAY_PLAYERNUM`, the Xbox ports at the controller (the PS3 ports hidden),
/// `mPanel` at the turn, `mRoundtext`'s `ID_PARTYPLAY_ROUND` and `mplayer_text`'s `#name`.
fn construct(root: &mut Clip, assets: &Assets, view: &PressADialog) -> RenderResult<()> {
    map_movie::configure_button_items(root.named(&["mPanel", "mContinue"])?, assets, &[("A", "ID_COMMON_ACCEPT")], 0xffff_ffec)?;
    let turn = view.player_turn.to_string();
    let player = localizer::localized_format_string(assets, "ID_PARTYPLAY_PLAYERNUM", &[localizer::AptArgument::Text(&turn)])?;
    for field in ["playerText", "playerTextGlow"] {
        root.named(&["mPlayerinfo", "mPlayer", field])?.text = Some(player.clone());
    }
    root.named(&["mPlayerinfo", "mPort", "mPS3ports"])?.visible = false;
    root.named(&["mPlayerinfo", "mPort", "mXboxports"])?.goto(assets, &view.controller.to_string())?;
    root.named(&["mPanel"])?.goto(assets, &view.player_turn.to_string())?;
    let round = view.round.to_string();
    let round = localizer::localized_format_string(assets, "ID_PARTYPLAY_ROUND", &[localizer::AptArgument::Text(&round)])?;
    for field in ["labelText", "labelText_glow"] {
        root.named(&["mRoundtext", field])?.text = Some(round.clone());
    }
    let name = assets.initial_text(&format!("#{}", view.name));
    for field in ["labelText_glow", "labelText"] {
        root.named(&["mplayer_text", "mcText", field])?.text = Some(name.clone());
    }
    Ok(())
}

/// The movie's InitActions: `iscreenmodule` and the `partyplaypresscontinue` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 50, 0x3884) | (ASSET, 51, 0x38bc))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class, frame 50 stops on
/// `IntroComplete`, frame 62 on `OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x3838 => clip.calls.push("partyplaypresscontinue"),
        0x3bb0 => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        0x3bdc => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated press-continue APT action {offset:#x}").into()),
    }
    Ok(())
}
