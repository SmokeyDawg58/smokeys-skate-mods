//! FE28's movieplayer and FMV HUD templates 138..142. Original APT classes
//! drive their overlays at every other main.swf tick, starting on SetState(1)
//! from PlayMovie8265bf98. The child movies' authored frame rates do not own
//! the main animation clock (AptCIH::tick82a1b580).
use super::{
    assets::Assets,
    clips::Clip,
    localizer,
    objective_movie::{set_x, text_field_width},
    pause_movie::child,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::movie_player::MovieScreen;
use skate2_formats::apt::Matrix;

pub(super) const PLAYER: usize = 87;
pub(super) const BASIC: usize = 88;
pub(super) const MAP: usize = 89;
pub(super) const CREATE_SPOT: usize = 90;
pub(super) const ADVANCED: usize = 91;
pub(super) const HOM: usize = 92;
/// Each tutorial constructor's Screen_EdgeOffset == 0 branch.
pub(super) const PLACEMENT: Matrix = Matrix {
    a: 0.85,
    b: 0.,
    c: 0.,
    d: 0.85,
    tx: 98.,
    ty: 0.,
};

#[derive(Default)]
pub(super) struct FmvTutorialMovie {
    player: Option<Clip>,
    pub tutorial: Option<Clip>,
    generation: Option<u64>,
    started: bool,
    frame: usize,
    delay: u32,
}

impl FmvTutorialMovie {
    /// OnLoaded acknowledges real clip construction, not IntroComplete: the
    /// native state may start its movie while the empty screen's intro runs.
    pub fn update(
        &mut self,
        assets: &Assets,
        state: Option<MovieScreen>,
    ) -> RenderResult<Option<u64>> {
        let Some(state) = state else {
            if self.generation.take().is_some() {
                if let Some(player) = self.player.as_mut() {
                    player.goto(assets, "outro")?;
                }
                // Tutorial assets have no outro; CloseScreen destroys them.
                self.tutorial = None;
                self.started = false;
            }
            return Ok(None);
        };
        if self.generation != Some(state.generation) {
            let mut player = Clip::new(assets, PLAYER, 0)?;
            player.goto(assets, "intro")?;
            let tutorial = match state.tutorial {
                Some(template @ 138..=142) => {
                    let asset = BASIC + (template - 138) as usize;
                    let mut root = Clip::new(assets, asset, 0)?;
                    match asset {
                        CREATE_SPOT => {
                            // Constructor00afc3: localize OVER/RAIL, autoSize
                            // left, then place the second word five pixels on.
                            let over = root.named(&["mOver"])?;
                            over.named(&["txtOver"])?.text =
                                Some(assets.initial_text("ID_TRICKDISPLAY_CONTEXTTYPE_OVER"));
                            over.named(&["txtRail"])?.text =
                                Some(assets.initial_text("ID_CONTEXTENTITYTYPE_RAIL"));
                            let label = child(over, "txtOver")?;
                            let right =
                                label.entry.matrix.tx + text_field_width(&label.clip, assets)?;
                            set_x(child(over, "txtRail")?, right + 5.);
                        }
                        HOM => {
                            root.named(&["mDollartext", "labelText"])?.text = Some(
                                localizer::format_currency(assets, 1500.)
                                    .encode_utf16()
                                    .collect(),
                            )
                        }
                        _ => {}
                    }
                    // Initialise differs from the static defaults in Create
                    // a Spot: 14 startup ticks; Advanced Replay uses six.
                    self.frame = if matches!(asset, MAP | HOM) { 10 } else { 2 };
                    self.delay = match asset {
                        CREATE_SPOT => 14,
                        ADVANCED => 6,
                        _ => 0,
                    };
                    Some(root)
                }
                Some(template) => return Err(format!("unknown FMV HUD template {template}").into()),
                None => None,
            };
            self.player = Some(player);
            self.tutorial = tutorial;
            self.generation = Some(state.generation);
            self.started = false;
        }
        // SetState(1) only sets mStartedPlaying; it does not reset the frame
        // on a loop or on every presentation update.
        self.started |= state.playing;
        Ok(Some(state.generation))
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(player) = self.player.as_mut() {
            player.tick(assets)?;
            if player.calls.contains(&"OutroComplete") {
                self.player = None;
            }
        }
        if let Some(root) = self.tutorial.as_mut() {
            // PlayAnimation runs before the timeline's frame actions. All
            // five set kFRAMEDELAY=1; numeric gotoAndStop is one-based.
            if self.started {
                if self.delay == 0 {
                    self.delay = 1;
                    let count = root
                        .definition(assets)?
                        .movie()
                        .ok_or("FMV HUD root is not a movie")?
                        .frames
                        .len();
                    // jumpToFrame82a1b410 ignores targets beyond nFrames.
                    if self.frame <= count {
                        root.goto_number_and_stop(assets, self.frame)?;
                    } else {
                        root.playback.stop();
                    }
                    self.frame += 1;
                } else {
                    self.delay -= 1;
                }
            }
            root.tick(assets)?;
        }
        Ok(())
    }
}

/// Authored class registrations in the six fingerprinted movie assets.
/// These InitActions define the classes used by the root constructors;
/// FmvTutorialMovie owns their OnLoaded, SetState and animation behavior.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!(
        (asset, character, offset),
        (PLAYER, 1, 0x260)
            | (BASIC, 73, 0xfc08)
            | (MAP, 59, 0x5e0c)
            | (CREATE_SPOT, 150, 0xaf24)
            | (ADVANCED, 149, 0x113f0)
            | (HOM, 48, 0x64c0)
    )
}

/// Only the fingerprinted root constructors/classes, trace statements and
/// empty MoviePlayer screen's lifecycle callbacks. Plain Stop/End actions
/// are handled by Clip, and authored text/geometry remain in the assets.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, clip.character, offset) {
        (PLAYER, 0, 0x24c | 0x260) => clip.playback.stop(),
        (PLAYER, 0, 0x360) => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        (PLAYER, 0, 0x38c) => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        (BASIC, 0, 0xfbf4 | 0xfc08 | 0xfea0)
        | (MAP, 0, 0x5de4 | 0x5e0c | 0x60a4)
        | (CREATE_SPOT, 0, 0xaf08 | 0xaf24 | 0xb214)
        | (ADVANCED, 0, 0x113d4 | 0x113f0 | 0x11688)
        | (HOM, 0, 0x6498 | 0x64c0 | 0x6774) => {}
        _ => {
            return Err(format!(
                "untranslated FMV HUD action {}:{} {offset:#x}",
                clip.asset, clip.character
            )
            .into());
        }
    }
    Ok(())
}
