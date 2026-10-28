//! Original hud2/hudtutorial (asset 30, fingerprint 5ca788c6) and its
//! `hudtutorial` class (InitAction 9e6c), driven by `TutorialInstructionHUD`
//! (`skate2_core::challenge::tutorial_hud`). `SetScreenState` runs the
//! class's `SetState` at once, so each state is applied with the component
//! view it carries. The trick guide's gestures are controls/gesture_item
//! (asset 31, fingerprint 3577d433) instances; their `gesture_item` class is
//! translated here as well. Apt finds members case-insensitively
//! (`AptNativeHash::HashFindKey` `0x829da970`), so the class's
//! `mAnim360Popshuvit` is the authored `mAnim360popshuvit`.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    objective_movie::{set_x, set_xscale, set_y},
    pause_movie::{bounds, child},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    challenge::tutorial_hud::{TutorialHudView, screen_state},
    ui::apt_timeline::{DisplayEntry, IDENTITY, PlacementIdentity},
};
use skate2_formats::apt::Character;

/// `hudtutorial.mMAX_GESTURES`.
const MAX_GESTURES: usize = 4;
/// `hudtutorial.prototype`'s `buttonKerning`, `slashKerning` and
/// `verticalOffset`.
const BUTTON_KERNING: f32 = 28.;
const SLASH_KERNING: f32 = 17.;
const VERTICAL_OFFSET: f32 = 32.;
/// `SethudtutorialItem`'s background growth per extra line.
const LINE_HEIGHT: f32 = 18.;
/// The `ShowAnimated*` clips in screen-state order (12 to 16).
const ANIMATIONS: [&str; 5] = ["mAnimOllie", "mAnimNollie", "mAnimKickflip", "mAnimPopshuvit", "mAnim360Popshuvit"];
/// controls/gesture_item's `Gesture Item` export.
pub(super) const GESTURE_ITEM: u32 = 125;
/// `attachMovie` and `createEmptyMovieClip` add 0x4000 to the depth
/// (`sMethod_attachMovie` `0x82a1d088`), above every authored child.
const DYNAMIC_DEPTH: i32 = 0x4000;
/// `SetProceduralProperty`'s smallest scale (`0x829ea608`).
const MIN_SCALE: f32 = f32::from_bits(0x38d1_b717);

pub(super) struct TutorialMovie {
    pub root: Clip,
    /// The instance's `mGestureButtonGroup_x_pos` and
    /// `mGestureButtonGroup_y_pos`.
    group_position: [f32; 2],
}

impl TutorialMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, 30, 0)?, group_position: [0.; 2] };
        movie.calls(assets)?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls(assets)
    }

    /// Constructors and the frame scripts' `gotoAndPlay`s, which Apt runs
    /// with the queued actions after the frame's display lists.
    fn calls(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "hudtutorial" => self.construct()?,
                "TutorialContainerIntro" => self.root.named(&["container_mc"])?.goto(assets, "intro")?,
                "TutorialContainerOutro" => self.root.named(&["container_mc"])?.goto(assets, "outro")?,
                "TutorialContainerIdle" => self.root.named(&["container_mc"])?.goto(assets, "idle")?,
                _ => return Err(format!("untranslated tutorial HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action 9e3c's `new hudtutorial(this)`: `mScreen._x -=
    /// Screen_EdgeOffset` subtracts zero, `mScreen.stop()` holds the root on
    /// its first frame, the group's authored position is kept and
    /// `ClearTricks` runs. The constructor's `autoSave = "center"` on the
    /// labels sets an ordinary member, not `autoSize`.
    fn construct(&mut self) -> RenderResult<()> {
        self.root.playback.stop();
        let group = self.group()?;
        self.group_position = [group.entry.matrix.tx, group.entry.matrix.ty];
        self.clear_tricks()
    }

    fn group(&mut self) -> RenderResult<&mut Child> {
        child(self.root.named(&["container_mc"])?, "mcGestureButtonGroup")
    }

    /// `SetState`: -10 shows, -20 hides, -1 `ClearTricks`, 10 the text and
    /// trick guide, 11 the text alone, 12 to 16 the text and an animated
    /// trick, 77 the stance; others only trace.
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &TutorialHudView) -> RenderResult<()> {
        match state {
            screen_state::SHOW => self.root.visible = true,
            screen_state::HIDE => self.root.visible = false,
            -1 => self.clear_tricks()?,
            screen_state::INSTRUCTION => {
                self.clear_tricks()?;
                self.set_item(assets, &view.hal_id)?;
                self.fill_trick_data(assets, view)?;
            }
            screen_state::NONE => {
                self.set_item(assets, &view.hal_id)?;
                self.clear_tricks()?;
            }
            screen_state::OLLIE..=screen_state::POPSHUVIT_360 => {
                self.set_item(assets, &view.hal_id)?;
                self.clear_tricks()?;
                let index = usize::try_from(state - screen_state::OLLIE)?;
                self.show_animated(index, true, view)?;
            }
            screen_state::STANCE => self.update_trick_gestures_for_stance(view)?,
            _ => {}
        }
        Ok(())
    }

    /// `SethudtutorialItem(HalText)`: the label and its glow show the text;
    /// a text the large field wraps (`maxscroll > 1`) moves `mTextMC` to
    /// `smalltext` for good, and the background grows 18 per further line
    /// there.
    fn set_item(&mut self, assets: &Assets, hal_id: &str) -> RenderResult<()> {
        let text = assets.initial_text(hal_id);
        let container = self.root.named(&["container_mc"])?;
        let label = container.named(&["mTextMC"])?;
        label.visible = true;
        set_text(label, &text)?;
        let mut extra = 0.;
        if max_scroll(label.named(&["label"])?, assets)? > 1 {
            label.goto(assets, "smalltext")?;
            set_text(label, &text)?;
            let lines = max_scroll(label.named(&["label"])?, assets)?;
            extra = (lines - 1) as f32 * LINE_HEIGHT;
        }
        let background = container.named(&["mTutBackground"])?;
        set_y(child(background, "mBackerreflect")?, 15. + extra);
        set_y(child(background, "mBackertop")?, 92.);
        set_y(child(background, "mBackercore")?, 92.);
        set_height(child(background, "mBackercore")?, assets, 16. + extra)?;
        set_y(child(background, "mBackerbottom")?, 108. + extra);
        Ok(())
    }

    /// `ClearTricks`: the group back at its authored position, the
    /// animations hidden and every guide item hidden at its slot.
    fn clear_tricks(&mut self) -> RenderResult<()> {
        let [x, y] = self.group_position;
        let group = self.group()?;
        set_x(group, x);
        set_y(group, y);
        let group = &mut group.clip;
        // ShowAnimated*(false): a hidden animation's stance is not updated.
        for name in ANIMATIONS {
            child(group, name)?.clip.visible = false;
        }
        for i in 0..MAX_GESTURES {
            let slot = i as f32 * 130.;
            for (name, x) in [("mcGestureItem", 48.), ("mcButtonItem", 47.)] {
                let item = child(group, &format!("{name}{i}"))?;
                set_x(item, slot + x);
                item.clip.visible = false;
            }
            if i < MAX_GESTURES - 1 {
                for name in ["mcConnectorArrow", "mcConnectorPlus", "mcConnectorEither"] {
                    let connector = child(group, &format!("{name}{i}"))?;
                    set_x(connector, slot + 96.);
                    connector.clip.visible = false;
                }
            }
        }
        Ok(())
    }

    /// `ShowAnimated*(isVisible)` then its `UpdateStanceAnimated*`.
    fn show_animated(&mut self, index: usize, visible: bool, view: &TutorialHudView) -> RenderResult<()> {
        child(&mut self.group()?.clip, ANIMATIONS[index])?.clip.visible = visible;
        self.update_stance_animated(index, view)
    }

    /// `UpdateStanceAnimated*`: a shown animation and its `mCommonText` face
    /// left (`_xscale` -100) unless `Tricks.IsStanceTrickGesturesFlipped()`.
    fn update_stance_animated(&mut self, index: usize, view: &TutorialHudView) -> RenderResult<()> {
        let animation = child(&mut self.group()?.clip, ANIMATIONS[index])?;
        if !animation.clip.visible {
            return Ok(());
        }
        let scale = if view.stance_flipped { 100. } else { -100. };
        set_xscale(animation, scale);
        set_xscale(child(&mut animation.clip, "mCommonText")?, scale);
        Ok(())
    }

    /// `FillTrickData`: the animations hidden, then the guide's slots from
    /// `GetTutorialGesturesButtons`. A gesture lifts the group by
    /// `verticalOffset`; a button and an `Or` connector add kerning (two
    /// `buttonKerning`s, two `slashKerning`s) that pulls the following items
    /// left; the group then moves right by its slot count's offset and half
    /// the kerning. Counts other than 1 to 4 lay nothing out.
    fn fill_trick_data(&mut self, assets: &Assets, view: &TutorialHudView) -> RenderResult<()> {
        for index in 0..ANIMATIONS.len() {
            self.show_animated(index, false, view)?;
        }
        let mut offset: f32 = match view.slots.len() {
            1 => 195.,
            2 => 130.,
            3 => 65.,
            4 => 0.,
            _ => return Ok(()),
        };
        let [_, y] = self.group_position;
        let group = self.group()?;
        let mut kerning = 0.;
        let mut connector = "";
        for (i, slot) in view.slots.iter().enumerate() {
            let shown = if !slot.gesture.is_empty() {
                set_y(group, y - VERTICAL_OFFSET);
                let item = child(&mut group.clip, &format!("mcGestureItem{i}"))?;
                set_x(item, item.entry.matrix.tx - kerning);
                // SetUseInGameStance(true), SetGestureType (whose own
                // UpdateGestureForStance gives the same scale) and
                // UpdateGestureForStance.
                item.clip.properties.insert("mUseInGameStance".into(), "true".into());
                set_gesture_type(&mut item.clip, assets, &slot.gesture)?;
                update_gesture_for_stance(&mut item.clip, view);
                item.clip.visible = true;
                true
            } else if !slot.button.is_empty() {
                kerning += BUTTON_KERNING;
                let item = child(&mut group.clip, &format!("mcButtonItem{i}"))?;
                set_x(item, item.entry.matrix.tx - kerning);
                // DisableSwap(true), SetButtonType: the A/B swap is PS3-only.
                item.clip.named(&["mButtonRender"])?.properties.insert("_test".into(), slot.button.clone());
                item.clip.visible = true;
                kerning += BUTTON_KERNING;
                true
            } else {
                false
            };
            if shown && i > 0 {
                let previous = i - 1;
                match connector {
                    "Arrow" => child(&mut group.clip, &format!("mcConnectorArrow{previous}"))?.clip.visible = true,
                    "Plus" => child(&mut group.clip, &format!("mcConnectorPlus{previous}"))?.clip.visible = true,
                    "Or" => {
                        let either = child(&mut group.clip, &format!("mcConnectorEither{previous}"))?;
                        either.clip.visible = true;
                        set_x(either, either.entry.matrix.tx - SLASH_KERNING);
                        let button = child(&mut group.clip, &format!("mcButtonItem{i}"))?;
                        set_x(button, SLASH_KERNING.mul_add(-2., button.entry.matrix.tx));
                        let gesture = child(&mut group.clip, &format!("mcGestureItem{i}"))?;
                        set_x(gesture, gesture.entry.matrix.tx - SLASH_KERNING.mul_add(2., 5.));
                        kerning += SLASH_KERNING * 2.;
                    }
                    _ => {}
                }
            }
            // The fourth slot has no connectors: undefined._x is not set.
            if i < MAX_GESTURES - 1 {
                for name in ["mcConnectorEither", "mcConnectorArrow", "mcConnectorPlus"] {
                    let following = child(&mut group.clip, &format!("{name}{i}"))?;
                    set_x(following, following.entry.matrix.tx - kerning);
                }
            }
            connector = &slot.connector;
        }
        if view.slots.len() == 4 && view.slots.last().is_some_and(|s| !s.gesture.is_empty()) {
            offset -= 15.;
        }
        offset += kerning / 2.;
        set_x(group, group.entry.matrix.tx + offset);
        Ok(())
    }

    /// `UpdateTrickGesturesForStance` (state 77): the shown animation and
    /// the guide's gestures turned for the stance.
    fn update_trick_gestures_for_stance(&mut self, view: &TutorialHudView) -> RenderResult<()> {
        for index in 0..ANIMATIONS.len() {
            self.update_stance_animated(index, view)?;
        }
        let group = &mut self.group()?.clip;
        for (i, slot) in view.slots.iter().enumerate() {
            if !slot.gesture.is_empty() {
                let item = &mut child(group, &format!("mcGestureItem{i}"))?.clip;
                item.properties.insert("mUseInGameStance".into(), "true".into());
                update_gesture_for_stance(item, view);
            }
        }
        Ok(())
    }
}

/// `label.text` and `labelGlow.text`.
fn set_text(holder: &mut Clip, text: &[u16]) -> RenderResult<()> {
    for name in ["label", "labelGlow"] {
        holder.named(&[name])?.text = Some(text.to_vec());
    }
    Ok(())
}

/// A multiline field's `maxscroll` (`objectMemberLookup` text member 9,
/// filled by `EnsureStringAllocated` `0x82a12220`): `SkateAptString`'s
/// (`0x82a70b28`) line count for the field's width.
pub(super) fn max_scroll(field: &Clip, assets: &Assets) -> RenderResult<usize> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("maxscroll target is not text".into());
    };
    if !text.multiline {
        return Err("maxscroll target is a single line".into());
    }
    let Some(Character::Font { name, .. }) = assets.movies[field.asset].movie.characters.get(text.font_id as usize).and_then(Option::as_ref) else {
        return Err("maxscroll target font absent".into());
    };
    let font = assets.apt_font(name)?;
    let scale = font.scale[0] * text.height;
    let units = field.text.as_deref().unwrap_or_default();
    let width = text.bounds.right - text.bounds.left;
    Ok(super::native_text_lines(units, true, width, |c| font.metrics.mapped_glyph(c).map_or(0., |g| g.advance * scale)).len())
}

/// `SetProceduralProperty(_height)` from ActionScript (`0x829ea608`): the y
/// scale becomes `d * value / height` (at least 1.1322573 percent), the
/// height being the piece's bounds through its own matrix
/// (`GetBoundingRect` `0x829e24e8`).
pub(super) fn set_height(piece: &mut Child, assets: &Assets, value: f32) -> RenderResult<()> {
    piece.clip.script_transform = true;
    if value < 0. {
        return Ok(());
    }
    let value = if value == 0. { MIN_SCALE } else { value };
    let m = piece.entry.matrix;
    if m.b.abs() >= MIN_SCALE || m.c.abs() >= MIN_SCALE {
        return Err("tutorial HUD _height target is rotated".into());
    }
    let r = bounds(&piece.clip, assets)?.ok_or("tutorial HUD _height target has no bounds")?;
    let height = ((r.bottom - r.top) * m.d).abs();
    if height == 0. {
        return Ok(());
    }
    let ratio = (value / height).max(MIN_SCALE);
    let scale = (m.d * ratio * 100.).max(f32::from_bits(0x3f90_edcf));
    piece.entry.matrix.d = scale * f32::from_bits(0x3c23_d70a);
    Ok(())
}

/// The `gesture_item` constructor, run as a `Gesture Item` is placed:
/// `placeholder` hidden and `SetGestureType(mGestureImage)`. Its
/// `UpdateGestureForStance` reads `PlayerStance_IsRegular` at screen load;
/// `hudtutorial`'s constructor hides every item and each path that shows
/// one sets the gesture's scale again first, so that scale is never drawn.
pub(super) fn construct_gesture_item(item: &mut Clip, assets: &Assets) -> RenderResult<()> {
    item.named(&["placeholder"])?.visible = false;
    let gesture = item.properties.get("mGestureImage").cloned().ok_or("Gesture Item has no mGestureImage")?;
    set_gesture_type(item, assets, &gesture)
}

/// `gesture_item.SetGestureType(gesture)`: the previous gesture goes
/// (`removeMovieClip`, and the empty `createEmptyMovieClip("gesture", 0)`
/// the attach replaces) and `attachMovie("g_" + gesture, "gestureitem", 0)`
/// places the export at depth 0x4000. `findCharacterInLibrary`
/// (`0x82a198e8`) matches the control's exports case-insensitively; an
/// absent one attaches nothing.
pub(super) fn set_gesture_type(item: &mut Clip, assets: &Assets, gesture: &str) -> RenderResult<()> {
    item.children.remove(&DYNAMIC_DEPTH);
    item.properties.insert("mGestureImage".into(), gesture.into());
    let name = format!("g_{gesture}");
    let Some(export) = assets.movies[item.asset].movie.exports.iter().find(|e| e.name.eq_ignore_ascii_case(&name)) else {
        return Ok(());
    };
    let clip = Clip::new(assets, item.asset, export.character_id)?;
    let entry = DisplayEntry {
        identity: PlacementIdentity { character: export.character_id, frame: 0 },
        matrix: IDENTITY,
        colour_scale: u32::MAX,
        colour_bias: 0,
        ratio: 0.,
        name: Some("gestureitem".into()),
        clip_depth: -1,
        events: Vec::new(),
        blend_mode: None,
    };
    item.children.insert(DYNAMIC_DEPTH, Child { entry, clip });
    Ok(())
}

/// `gesture_item.UpdateGestureForStance`: the attached gesture faces left
/// (`_xscale` -100) for a regular stance, `!IsStanceTrickGesturesFlipped()`
/// with `mUseInGameStance`, else `PlayerStance_IsRegular()`.
/// `mUseSkateSettings` stays false: only `SetUseSkateChallengeSettings` sets
/// it, which this screen never calls.
fn update_gesture_for_stance(item: &mut Clip, view: &TutorialHudView) {
    let in_game = item.properties.get("mUseInGameStance").is_some_and(|v| v == "true");
    face_gesture(item, if in_game { !view.stance_flipped } else { view.stance_regular });
}

/// The end of `UpdateGestureForStance`: `mMovieClip`, the attached gesture, faces left
/// (`_xscale` -100) for a regular stance.
pub(super) fn face_gesture(item: &mut Clip, regular: bool) {
    if let Some(gesture) = item.children.get_mut(&DYNAMIC_DEPTH) {
        set_xscale(gesture, if regular { -100. } else { 100. });
    }
}

/// hudtutorial.apt's sole InitAction defines the guarded global class
/// translated above; root action 9e3c constructs it.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (30, 71, 0x9e6c))
}

/// Timeline actions: the root's construction (9e3c) and its intro and
/// outro frames, which the constructor's `stop()` keeps it from reaching,
/// and `container_mc`'s idle loop. A `gotoAndPlay` is queued as a call.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (30, 0x9e3c) => {
            clip.calls.push("hudtutorial");
            clip.visible = false;
        }
        (30, 0xadc8) => {
            clip.visible = true;
            clip.calls.push("TutorialContainerIntro");
        }
        (30, 0xadfc) => clip.calls.push("TutorialContainerOutro"),
        (30, 0xae24) => {
            clip.visible = false;
            clip.playback.stop();
        }
        (30, 0xb0fc) => clip.calls.push("TutorialContainerIdle"),
        _ => return Err(format!("untranslated tutorial HUD APT action {}:{offset:#x}", clip.asset).into()),
    }
    Ok(())
}
