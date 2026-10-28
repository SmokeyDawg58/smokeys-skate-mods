//! Original hud2/actionoverlay2 (asset 32, fingerprint 9bd9bc9c) and its
//! `actionoverlay2` class (InitAction 48 `0x366c`), the `SignUpHUD` screen
//! (`skate2_core::challenge::signup_hud`; template entry `0x82fdb8f8`,
//! placement 0). `SetScreenState` runs the class's `SetState` at once, so
//! each state is applied with the title the component holds then.
use super::{
    assets::Assets,
    clips::Clip,
    localizer,
    objective_movie::{set_x, text_field_width},
    pause_movie::child,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::signup_hud::SignUpHudView;
use skate2_formats::apt::Character;

/// `actionoverlay2.kActionOverlayState_*`.
mod state {
    pub const OFF: i32 = 0;
    pub const CHALLENGE: i32 = 1;
    pub const TELEPORT: i32 = 2;
    pub const PRESS_START: i32 = 3;
    pub const DMO_GRAB: i32 = 4;
    pub const OTS: i32 = 5;
}

/// The pieces the constructor moves by `Screen_EdgeOffset` (zero): the
/// ActionScript `_x` still takes them off their timelines' moves.
const EDGE_PIECES: [[&str; 2]; 9] = [
    ["mDmo", "DMO0"],
    ["mDmo", "DMO1"],
    ["mDmo", "DMO2"],
    ["ots_mc", "otsBacker_mc"],
    ["ots_mc", "mButtonItem"],
    ["ots_mc", "mOtsTitle"],
    ["ots_mc", "mChallengeInstanceName"],
    ["otsIcon_mc", "icon_mc"],
    ["mDmo", "mBackleft"],
];

pub(super) struct ActionOverlayMovie {
    pub root: Clip,
    /// The instance's `mVisible` (the prototype's false until set).
    visible: bool,
    /// `mDMOHUD` and `mOTSHUD`: which outro `Show(false)` plays.
    dmo_hud: bool,
    ots_hud: bool,
}

impl ActionOverlayMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, 32, 0)?, visible: false, dmo_hud: false, ots_hud: false };
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
                "actionoverlay2" => self.construct(assets)?,
                _ => return Err(format!("untranslated action overlay callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action 3634's `new actionoverlay2(this)`: the edge-offset moves,
    /// `mDMOHUD`/`mOTSHUD` false, `Sign up` centre-auto-sized in both symbol
    /// fields, the button placed by `SetSymbol`'s formula and the symbol 16
    /// right. `RegisterScreen`'s `Show(false)` finds it hidden already.
    fn construct(&mut self, assets: &Assets) -> RenderResult<()> {
        for [holder, name] in EDGE_PIECES {
            let piece = child(self.root.named(&[holder])?, name)?;
            set_x(piece, piece.entry.matrix.tx);
        }
        self.visible = false;
        self.dmo_hud = false;
        self.ots_hud = false;
        let sign_up = assets.initial_text("ID_COMMON_SIGNUP");
        for name in ["signup", "signuplabel"] {
            let field = self.root.named(&["mSignup", "mOverlaySymbol", name])?;
            auto_size_center(field, assets)?;
            field.text = Some(sign_up.clone());
        }
        self.set_symbol(assets)?;
        let symbol = child(self.root.named(&["mSignup"])?, "mOverlaySymbol")?;
        set_x(symbol, symbol.entry.matrix.tx + 16.);
        Ok(())
    }

    /// `Show(showScreen)`: hiding a shown overlay plays its outro (DMO, OTS
    /// or button).
    fn show(&mut self, assets: &Assets, show_screen: bool) -> RenderResult<()> {
        if self.visible != show_screen {
            if !show_screen {
                let outro = if self.dmo_hud {
                    "dmo_outro"
                } else if self.ots_hud {
                    "ots_outro"
                } else {
                    "button_outro"
                };
                self.root.goto(assets, outro)?;
            }
            self.visible = show_screen;
        }
        Ok(())
    }

    /// actionoverlay2.InitDMOHUD (APT 0x3bf8): authored rows and button labels.
    fn init_dmo(&mut self, assets: &Assets, params: [bool; 5]) -> RenderResult<()> {
        let [upright_disabled, reset_disabled, marker, member, full] = params;
        let panel = self.root.named(&["mDmo"])?;
        for (name, dim) in [("DMO0", upright_disabled), ("DMO1", reset_disabled),
            ("DMO2", !marker || (!member && full))] {
            let row = child(panel, name)?;
            // APT stores its colour multiplier as an eight-bit channel.
            row.entry.colour_scale = (row.entry.colour_scale & 0x00ff_ffff)
                | (if dim { 76 } else { 255 } << 24);
            row.clip.script_transform = true;
        }
        let (button, label) = if marker && member {
            ("DPad_Left", "ID_ACTIONOVERLAY_REMOVE")
        } else if marker && full {
            ("DPad_Right", "ID_ACTIONOVERLAY_MAX_OBJS")
        } else {
            ("DPad_Right", "ID_ACTIONOVERLAY_ADD")
        };
        set_button(panel.named(&["DMO2", "mButton"])?, button)?;
        for field in ["labelText", "labelTextGlow"] {
            panel.named(&["DMO2", "mText", field])?.text = Some(assets.initial_text(label));
        }
        if marker {
            let field = panel.named(&["DMO2", "mText", "labelText"])?;
            let multiline = matches!(field.definition(assets)?, Character::Text(text) if text.multiline);
            let wrapped = multiline && super::tutorial_movie::max_scroll(field, assets)? > 1;
            super::objective_movie::set_y(child(panel.named(&["DMO2"])?, "mText")?, if wrapped { -8. } else { 2. });
        }
        Ok(())
    }

    /// `SetSymbol`: `mButtonX = -4 - signup._width / 2`, the D-pad button left
    /// of the centred symbol text.
    fn set_symbol(&mut self, assets: &Assets) -> RenderResult<()> {
        let width = text_field_width(self.root.named(&["mSignup", "mOverlaySymbol", "signup"])?, assets)?;
        set_x(child(self.root.named(&["mSignup"])?, "mButtonItem")?, -4. - width / 2.);
        Ok(())
    }

    /// `ShowChallengeNameText(show)`: the title label and its glow.
    fn show_challenge_name_text(&mut self, show: bool) -> RenderResult<()> {
        for name in ["labelGlow", "label"] {
            self.root.named(&["mSignup", name])?.visible = show;
        }
        Ok(())
    }

    /// `SetState`: 0 (or -1) hides; any other state hides the title, shows the
    /// overlay and plays `button_intro` unless a state picks another intro.
    /// Challenge shows `Sign up` and the title, Teleport the title as the
    /// symbol, OTS the title in the Own the Spot panel. The `SignUpHUD`
    /// component never sets DMO grab for the challenge sign-up's data.
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &SignUpHudView) -> RenderResult<()> {
        if state == state::OFF || state == -1 {
            return self.show(assets, false);
        }
        self.show_challenge_name_text(false)?;
        self.show(assets, true)?;
        let mut intro = "button_intro";
        match state {
            state::CHALLENGE => {
                set_button(self.root.named(&["mSignup", "mButtonItem"])?, "DPad_Up")?;
                self.show_challenge_name_text(true)?;
                let title = title_text(assets, view)?;
                let sign_up = assets.initial_text("ID_COMMON_SIGNUP");
                for name in ["signup", "signuplabel"] {
                    self.root.named(&["mSignup", "mOverlaySymbol", name])?.text = Some(sign_up.clone());
                }
                for name in ["labelGlow", "label"] {
                    self.root.named(&["mSignup", name])?.text = Some(title.clone());
                }
                self.dmo_hud = false;
                self.ots_hud = false;
                self.set_symbol(assets)?;
            }
            state::TELEPORT => {
                self.show_challenge_name_text(false)?;
                let title = title_text(assets, view)?;
                for name in ["signup", "signuplabel"] {
                    self.root.named(&["mSignup", "mOverlaySymbol", name])?.text = Some(title.clone());
                }
                self.dmo_hud = false;
                self.ots_hud = false;
                self.set_symbol(assets)?;
            }
            state::PRESS_START => {
                self.dmo_hud = false;
                self.ots_hud = false;
                self.set_symbol(assets)?;
            }
            state::DMO_GRAB => {
                intro = "dmo_intro";
                self.dmo_hud = true;
                self.ots_hud = false;
                self.init_dmo(assets, view.dmo.ok_or("DMO HUD parameters absent")?)?;
            },
            state::OTS => {
                set_button(self.root.named(&["mSignup", "mButtonItem"])?, "DPad_Up")?;
                intro = "ots_intro";
                let title = title_text(assets, view)?;
                self.root.named(&["ots_mc", "mChallengeInstanceName"])?.text = Some(title);
                self.dmo_hud = false;
                self.ots_hud = true;
                self.set_symbol(assets)?;
            }
            _ => {}
        }
        self.root.goto(assets, intro)
    }
}

/// `SignUp_GetChallengeTitle`'s text: a formatted challenge title shows as
/// is (`FormatStringCustom` with no arguments), a HAL key is looked up.
fn title_text(assets: &Assets, view: &SignUpHudView) -> RenderResult<Vec<u16>> {
    if view.formatted { localizer::format_custom(assets, &view.title, "") } else { Ok(assets.initial_text(&view.title)) }
}

/// `Button Item.SetButtonType`: the PS3 circle-enter swap never applies on
/// the 360, so the render target takes the button name.
fn set_button(item: &mut Clip, button: &str) -> RenderResult<()> {
    item.named(&["mButtonRender"])?.properties.insert("_test".into(), button.into());
    Ok(())
}

/// `autoSize = "center"`: the field keeps its centre and shrinks to its text
/// (`EnsureStringAllocated` `0x82a12220`); the symbol fields are centred
/// single lines, so they draw unchanged and `_width` is `textWidth + 4`.
fn auto_size_center(field: &mut Clip, assets: &Assets) -> RenderResult<()> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("action overlay auto-size target is not text".into());
    };
    if text.multiline || text.alignment != 2 {
        return Err("action overlay auto-size field differs from the verified layout".into());
    }
    field.properties.insert("text_autosize".into(), "center".into());
    Ok(())
}

/// actionoverlay2.apt's sole InitAction defines the guarded global class
/// translated above; root action 3634 constructs it.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (32, 48, 0x366c))
}

/// Timeline actions besides `Stop`: root action 3634 constructs the class.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x3634 => clip.calls.push("actionoverlay2"),
        _ => return Err(format!("untranslated action overlay APT action {offset:#x}").into()),
    }
    Ok(())
}
