//! Original hud2/hudonlinevoting (asset 59, fingerprint f7435871; template entry
//! `OnlineVotingHUD` 0x82fdb8c8, placement 6, g3dHUDRight) and its `hudonlinevoting` class
//! (InitAction character 40, `0x21b4`), driven by `OnlineVotingHUD`
//! (`skate2_core::challenge::turn_hud`): the vote on another player's turn. Its screen reads no
//! data.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 59;

pub(super) struct VotingMovie {
    pub root: Clip,
    /// `hudonlinevoting.mVisible`, undefined until the first `Show`.
    visible: Option<bool>,
}

impl VotingMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)?, visible: None };
        movie.calls(assets)?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls(assets)
    }

    fn calls(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "hudonlinevoting" => self.construct(assets)?,
                _ => return Err(format!("untranslated voting HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action `0x2180`'s `new hudonlinevoting(this)`: the screen stops, `Init` shows the
    /// middle vote and the screen registers. `mVotingHUD._x -= Screen_EdgeOffset` subtracts
    /// zero.
    fn construct(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.playback.stop();
        self.visible = None;
        self.set_vote(assets, "ok")
    }

    /// `SetState`: 1 and 2 `Show(true)` and `Show(false)`; 3 to 6 `SetVote` from the lowest
    /// score to the highest.
    pub fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        match state {
            1 => self.show(assets, true),
            2 => self.show(assets, false),
            3 => self.set_vote(assets, "weak"),
            4 => self.set_vote(assets, "ok"),
            5 => self.set_vote(assets, "sweet"),
            6 => self.set_vote(assets, "gnarly"),
            _ => Ok(()),
        }
    }

    /// `Show(bVisible)`: nothing for the current visibility; showing sets the A and B button
    /// items (`DisableSwap` keeps the PS3 swap off) and plays `hudintro`, hiding `hudoutro`.
    fn show(&mut self, assets: &Assets, visible: bool) -> RenderResult<()> {
        if self.visible == Some(visible) {
            return Ok(());
        }
        if visible {
            let components = self.root.named(&["mVotingHUD", "mHUDComponents"])?;
            for (item, button) in [("mButtonA", "A"), ("mButtonB", "B")] {
                components.named(&[item, "mButtonRender"])?.properties.insert("_test".into(), button.into());
            }
        }
        self.root.goto(assets, if visible { "hudintro" } else { "hudoutro" })?;
        self.visible = Some(visible);
        Ok(())
    }

    /// `SetVote(vote)`: `mHUDComponents` plays the vote's label.
    fn set_vote(&mut self, assets: &Assets, label: &str) -> RenderResult<()> {
        self.root.named(&["mVotingHUD", "mHUDComponents"])?.goto(assets, label)
    }
}

/// The movie's InitAction: the `hudonlinevoting` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 40, 0x21b4))
}

/// Root timeline actions besides `Stop`: frame 0 constructs the class and hides the movie,
/// frame 9 (`hudintro`) shows it and frame 30 hides it and stops.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x2180 => {
            clip.calls.push("hudonlinevoting");
            clip.visible = false;
        }
        0x24e4 => clip.visible = true,
        0x250c => {
            clip.visible = false;
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated voting HUD APT action {offset:#x}").into()),
    }
    Ok(())
}
