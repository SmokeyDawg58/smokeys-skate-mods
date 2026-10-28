//! nis/pro_bioinfo_overlay (asset 42, fingerprint b9a276d2):
//! `FrontEndState_NIS`'s pro bio card (template 38 `Pro_Bio_Info`, type 0;
//! overlay states 10 and 11, the left and right cards of a pro intro). Its
//! InitAction (character 64, `0x2989c`) defines the `pro_bioinfo_overlay`
//! class translated here; root action `0x29830` constructs it and hides the
//! root.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 42;

pub(super) struct ProBioMovie {
    pub root: Clip,
    /// The `Audio` calls the timeline made.
    pub sounds: Vec<&'static str>,
}

impl ProBioMovie {
    /// The screen's movie runs its first frame: `new
    /// pro_bioinfo_overlay(this)` (`mScreen.stop()`, `RegisterScreen`), then
    /// `_visible = false`. The screen manager's `PlayClip(id, 'intro')` finds
    /// no `intro` label.
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)?, sounds: Vec::new() };
        movie.calls()?;
        movie.root.goto(assets, "intro")?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls()
    }

    fn calls(&mut self) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "pro_bioinfo_overlay" => self.root.playback.stop(),
                "PlayInterfaceOverlayIn" | "PlayInterfaceOverlayOut" => self.sounds.push(call),
                _ => return Err(format!("untranslated pro bio callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `SetState`: `kScreenState_IntroLeft` (0) and `kScreenState_IntroRight`
    /// (2) fill the card (`GetInfo`) and play their intro,
    /// `kScreenState_OutroLeft` (1) and `kScreenState_OutroRight` (3) their
    /// outro.
    pub fn set_state(&mut self, assets: &Assets, state: i32, pro: &[String; 3]) -> RenderResult<()> {
        let label = match state {
            0 => "introLeft",
            1 => "outroLeft",
            2 => "introRight",
            3 => "outroRight",
            _ => return Ok(()),
        };
        if state % 2 == 0 {
            self.get_info(assets, pro)?;
        }
        self.root.goto(assets, label)?;
        self.calls()
    }

    /// `GetInfo`: `NIS.GetProInfo()`'s name in the header, its hometown and
    /// stance under the `ID_NIS_HOMETOWN` and `ID_NIS_STANCE` titles.
    fn get_info(&mut self, assets: &Assets, pro: &[String; 3]) -> RenderResult<()> {
        let fields: [(&[&str], &str); 5] = [
            (&["mcTextHeader", "labelText"], &pro[0]),
            (&["mcTextBody2", "labelText"], &pro[1]),
            (&["mcTextBody2", "labelTitle"], "ID_NIS_HOMETOWN"),
            (&["mcTextBody3", "labelText"], &pro[2]),
            (&["mcTextBody3", "labelTitle"], "ID_NIS_STANCE"),
        ];
        for (path, text) in fields {
            self.root.named(path)?.text = Some(assets.initial_text(text));
        }
        Ok(())
    }
}

/// pro_bioinfo_overlay.apt's InitAction defines the guarded global class
/// translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 64, 0x2989c))
}

/// Root action `0x29830` (`var screen = new pro_bioinfo_overlay(this);
/// _visible = false;`) and the cards' frames: each intro shows the root
/// and calls `Audio.PlayInterfaceOverlayIn`, each outro
/// `Audio.PlayInterfaceOverlayOut`, each end hides and stops; the holds
/// stop.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x29830 => {
            clip.calls.push("pro_bioinfo_overlay");
            clip.visible = false;
        }
        0x29b44 | 0x29b8c | 0x29bf4 => clip.playback.stop(),
        0x29b48 | 0x29bcc => {
            clip.calls.push("PlayInterfaceOverlayIn");
            clip.visible = true;
        }
        0x29b90 | 0x29bf8 => clip.calls.push("PlayInterfaceOverlayOut"),
        0x29bc0 | 0x29c18 => {
            clip.visible = false;
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated pro bio APT action {offset:#x}").into()),
    }
    Ok(())
}
