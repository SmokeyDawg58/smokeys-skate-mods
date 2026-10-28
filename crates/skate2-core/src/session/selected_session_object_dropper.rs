//! PC-only Object Dropper, laid out after Skate 3's park editor (Skate 2 has none) and placing
//! Skate 2's own props. In offline free skate, LB + B opens it: a free camera takes over while
//! the world keeps running (the skater's input is held neutral, so it stands where it was).
//! Left stick moves the camera across, right stick looks, D-pad Up/Down raise and lower it, and
//! a crosshair marks the middle of the screen. X opens the object list: LT/RT pick the tab
//! (Terrain, Other, Props; only Props has anything yet), LB/RB the subcategory, D-pad Up/Down
//! the item, A takes it into the editor's hands in front of the crosshair, B closes the list.
//! A held prop: left stick moves it, D-pad Up/Down raise and lower it, right stick orbits the
//! camera round it, D-pad Right/Left bring the camera closer or farther (closer moves it more
//! finely), RT + a left stick flick turns it 90 degrees, LT + RT + left stick turns it smoothly,
//! L3 stands it back up as it was taken, and A lets go of it so it falls. With nothing held, A
//! picks the placed prop under the crosshair back up and Y deletes it. B leaves. A placed prop
//! is kept as Skate 2 keeps its own (`try_drop_on_locator`): an ordinary object on a locator of
//! its own, which the skater can grab and drag; far away the census culls it and later respawns
//! it where it was left.

use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage, SelectedVisualSnapshot};
use crate::input_pad::XboxPadSample;
use crate::nis::manager::{FrozenFocus, NisCamera};
use crate::simulation_assembly_backend::constraint_session::skateboard_pool::riderless::RiderlessBoardHandle;
use crate::ui::object_dropper::{EditorSettings, PROPS, YOUR_BOARD};

const DPAD_UP: u16 = 1 << 0;
const DPAD_DOWN: u16 = 1 << 1;
const DPAD_LEFT: u16 = 1 << 2;
const DPAD_RIGHT: u16 = 1 << 3;
const START: u16 = 1 << 4;
const L3: u16 = 1 << 6;
const LEFT_BUMPER: u16 = 1 << 8;
const RIGHT_BUMPER: u16 = 1 << 9;
const A: u16 = 1 << 12;
const B: u16 = 1 << 13;
const X: u16 = 1 << 14;
const Y: u16 = 1 << 15;
/// Dropped props kept at once (their own locators; the census decides which are out).
const MAX_DROPPED: usize = 24;
/// Dropped props' locator IDs: a range no streamed data uses ("PCDR"), then a count.
const LOCATOR_BASE: u64 = 0x5043_4452_0000_0000;
/// How far the crosshair reaches for a surface (metres).
const AIM_REACH: f32 = 60.0;
/// A prop taken from the list appears this far along the crosshair, or on the first surface
/// the crosshair meets nearer than that (metres).
const SPAWN_DISTANCE: f32 = 6.0;
/// The crosshair targets the placed prop whose middle passes nearest its line, within this
/// (metres) plus this much per metre away.
const TARGET_WIDTH: f32 = 1.0;
const TARGET_SPREAD: f32 = 0.06;
/// A prop's middle above its origin (its base), for aiming at it and orbiting it (metres).
const PROP_MIDDLE: f32 = 0.5;
/// The free camera: across and up/down (m/s), and looking (rad/s, Skate 3's manual camera's).
const FLY_SPEED: f32 = 8.0;
const RISE_SPEED: f32 = 6.0;
const LOOK_SPEED: f32 = 1.75;
/// A held prop moves across and up/down this fast per metre the camera is from it (1/s).
const MOVE_RATE: f32 = 0.8;
const LIFT_RATE: f32 = 0.4;
/// The orbit camera's distance from a held prop (metres), changed this fast (1/s, scaled).
const ORBIT_MIN: f32 = 1.5;
const ORBIT_MAX: f32 = 25.0;
const ZOOM_RATE: f32 = 1.2;
/// LT + RT + left stick turn a held prop at full tilt from this speed, rising to the top speed
/// over the ramp while the stick stays pushed (rad/s: 15 to 120 degrees; seconds).
const FINE_TURN_START: f32 = 15.0 * std::f32::consts::PI / 180.0;
const FINE_TURN_TOP: f32 = 120.0 * std::f32::consts::PI / 180.0;
const FINE_TURN_RAMP: f32 = 1.5;
/// An RT flick spins the prop 90 degrees at this speed (rad/s: a quarter turn in 0.2 s).
const SPIN_SPEED: f32 = 2.5 * std::f32::consts::PI;
/// A flick: the left stick leaves the middle (inside REST) and passes FLICK.
const STICK_REST: f32 = 0.3;
const STICK_FLICK: f32 = 0.7;
/// The editor holds B (the foot brake) for a riding skater from this many frames after opening.
const BRAKE_AFTER_FRAMES: u32 = 2;
/// A trigger counts as held from here (of 255).
const TRIGGER_HELD: u8 = 128;
/// Analog moves are cubed, as Skate 3's manual camera does; smaller results count as none.
const DEADZONE: f32 = 0.01;
const FIELD_OF_VIEW: f32 = 55.0;
const NEAR_PLANE: f32 = 0.2;
/// The object list's tabs, as Skate 3's; only Props holds anything in Skate 2.
const TABS: [&str; 3] = ["Terrain", "Other", "Props"];
const PROPS_TAB: usize = 2;
/// Object list rows shown around the selected one.
const LIST_ROWS: usize = 10;
/// Under the Props tab, as Skate 3's says it.
const PROPS_INFO: [&str; 2] = ["Props always fall and settle to the ground.", "You can also knock them around while skating."];

/// Copies of the player's board out at once: each is a real board in the skater pool, which it
/// shares with AI and party skaters (16 members, two per skater).
const MAX_BOARDS: usize = 2;
/// A resting board's deck above the ground (the riderless board corpus: 0.087 m).
const BOARD_REST_DECK_HEIGHT: f32 = 0.087;
/// Prop-menu object IDs for dropped boards: living-world entity IDs carry their small entity
/// type in the top byte, so this type byte names no entity; the board's key follows.
const BOARD_PROP_ID: u32 = 0xB0 << 24;
/// An on-foot skater gets a dropped board's prop menu (the one Skate 2 shows by a grabbable
/// prop) within this of its deck, across the ground and up (metres).
const BOARD_MENU_REACH: f32 = 1.2;
const BOARD_MENU_HEIGHT: f32 = 1.5;
/// A deck whose up axis has at least this world-up component counts as upright.
const BOARD_UPRIGHT_UP_Y: f32 = 0.9;
/// Righting a board starts it this far above its deck and lets it fall (metres).
const BOARD_RIGHT_LIFT: f32 = 0.15;

#[derive(Clone)]
struct Dropped {
    /// A prop's own locator; unused for a board.
    locator: u64,
    /// A copy of the player's board: its riderless board in the skater pool.
    board: Option<RiderlessBoardHandle>,
    prop: usize,
    /// Where it was placed, then (in the editor) where it is now; a board's is its deck.
    transform: [f32; 16],
    /// A board's key in its prop-menu object ID (`BOARD_PROP_ID`).
    key: u32,
    /// A board's live deck as of the last frame.
    deck: Option<[f32; 16]>,
    /// A board added to the session marker: the deck transform the marker puts it back at.
    marker: Option<[f32; 16]>,
}

/// The editor's camera: where it is and the way it looks.
#[derive(Clone, Copy, Debug)]
struct View {
    position: [f32; 3],
    at: [f32; 3],
}

/// A prop in the editor's hands: drawn, not yet in the world.
#[derive(Clone, Copy, Debug)]
struct Held {
    prop: usize,
    /// Row-major (axis rows, translation row 3); a board's is its deck.
    transform: [f32; 16],
    /// The heading L3 stands it back up at.
    home: f32,
    /// Picked up from where it was placed: B puts it back there.
    picked: Option<[f32; 16]>,
}

#[derive(Default)]
pub(super) struct ObjectDropper {
    view: Option<View>,
    /// Leaving waits for an idle pad, so the B that left does not reach the skater.
    leaving: bool,
    list_open: bool,
    /// The settings page (Start), over whatever the editor was doing.
    settings_open: bool,
    settings_row: usize,
    settings: EditorSettings,
    /// The settings as last loaded or saved: closing the page saves only a change.
    settings_saved: EditorSettings,
    /// The list shows the selected subcategory's items, not the subcategories.
    list_items: bool,
    tab: usize,
    /// The catalog entry selected in the Props tab.
    selected: usize,
    held: Option<Held>,
    orbit_distance: f32,
    /// The left stick has been back in the middle since its last flick.
    flick_ready: bool,
    /// Frames since the editor opened.
    open_frames: u32,
    /// A flick's quarter turn still to spin: its axis and the angle left (radians).
    spin: Option<([f32; 3], f32)>,
    /// How long the stick has been turning the prop finely (seconds).
    fine_hold: f32,
    triggers: [u8; 2],
    /// The placed prop under the crosshair (index into `dropped`).
    target: Option<usize>,
    note: &'static str,
    dropped: Vec<Dropped>,
    buttons: u16,
    next_locator: u64,
    next_board_key: u32,
    /// The local board's deck (Part 6, world) and each dropped board's, as of the last frame.
    local_board_deck: Option<[f32; 16]>,
    dropped_board_decks: Vec<[f32; 16]>,
    /// The held prop's bank asset and RModel entry, for drawing it.
    preview_model: Option<(usize, u64, u32)>,
    /// Each looked-up prop's bounding box centre (model space), which it turns about, and half its
    /// diagonal, how far from that centre the crosshair still finds it.
    centers: std::collections::BTreeMap<usize, ([f32; 3], f32)>,
}

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual, format!("object dropper: {e}"))
}

fn axis(value: i16) -> f32 {
    (f32::from(value) / 32767.0).clamp(-1.0, 1.0)
}

fn cubic(value: f32) -> f32 {
    let cubed = value * value * value;
    if cubed.abs() > DEADZONE { cubed } else { 0.0 }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    if length > 1e-6 { scale(v, 1.0 / length) } else { v }
}

/// `v` turned `angle` radians about the unit `axis` (right-handed, as the manual camera turns).
fn rotate(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    add(add(scale(v, cos), scale(cross(axis, v), sin)), scale(axis, dot(axis, v) * (1.0 - cos)))
}

const UP: [f32; 3] = [0.0, 1.0, 0.0];

/// The camera's way turned by `yaw` about the vertical, then `pitch` about its own side, as Skate
/// 3's manual camera looks (never straight up or down).
fn look(at: [f32; 3], yaw: f32, pitch: f32) -> [f32; 3] {
    let at = rotate(at, UP, yaw);
    let side = normalize(cross(UP, at));
    let pitched = normalize(rotate(at, side, pitch));
    if pitched[1].abs() < 0.97 { pitched } else { normalize(at) }
}

/// The camera's way across the ground, and the way the left stick's right is (Skate 3's manual
/// camera strafes along `at x up`).
fn ground_axes(at: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let flat = [at[0], 0.0, at[2]];
    let flat = if dot(flat, flat) > 1e-8 { normalize(flat) } else { [0.0, 0.0, 1.0] };
    (flat, normalize(cross(flat, UP)))
}

fn heading(transform: &[f32; 16]) -> f32 {
    if transform[8].hypot(transform[10]) > 1e-4 { transform[8].atan2(transform[10]) } else { 0.0 }
}

/// A transform's axis rows turned `angle` about `axis`, then made orthonormal again.
fn turn(transform: &mut [f32; 16], axis: [f32; 3], angle: f32) {
    let row = |t: &[f32; 16], i: usize| [t[i * 4], t[i * 4 + 1], t[i * 4 + 2]];
    let x = rotate(row(transform, 0), axis, angle);
    let z = normalize(rotate(row(transform, 2), axis, angle));
    let y = normalize(cross(z, x));
    let x = normalize(cross(y, z));
    for (i, axis) in [x, y, z].into_iter().enumerate() {
        transform[i * 4..i * 4 + 3].copy_from_slice(&axis);
    }
}

fn origin(transform: &[f32; 16]) -> [f32; 3] {
    [transform[12], transform[13], transform[14]]
}

/// `local` (model space) placed by a row-major transform.
fn place(transform: &[f32; 16], local: [f32; 3]) -> [f32; 3] {
    let mut point = origin(transform);
    for (row, value) in local.into_iter().enumerate() {
        point = add(point, scale([transform[row * 4], transform[row * 4 + 1], transform[row * 4 + 2]], value));
    }
    point
}

/// A prop's middle in its own model space: its bounding box's centre once its model has been
/// looked up, else `PROP_MIDDLE` above its origin; a board's deck is its middle.
fn local_middle(centers: &std::collections::BTreeMap<usize, ([f32; 3], f32)>, prop: usize) -> [f32; 3] {
    if PROPS[prop].2 == YOUR_BOARD {
        return [0.0; 3];
    }
    centers.get(&prop).map_or([0.0, PROP_MIDDLE, 0.0], |center| center.0)
}

/// How far from its middle the crosshair finds a placed prop: half its bounding box's diagonal,
/// at least `TARGET_WIDTH`.
fn target_reach(centers: &std::collections::BTreeMap<usize, ([f32; 3], f32)>, prop: usize) -> f32 {
    centers.get(&prop).map_or(TARGET_WIDTH, |center| center.1.max(TARGET_WIDTH))
}

/// Where a placed or held thing is aimed at, orbited and turned about: its middle.
fn middle(centers: &std::collections::BTreeMap<usize, ([f32; 3], f32)>, prop: usize, transform: &[f32; 16]) -> [f32; 3] {
    place(transform, local_middle(centers, prop))
}

/// `turn` about the prop's middle (`local`, model space) rather than its origin.
fn turn_about(transform: &mut [f32; 16], axis: [f32; 3], angle: f32, local: [f32; 3]) {
    let fixed = place(transform, local);
    turn(transform, axis, angle);
    let moved = place(transform, local);
    for lane in 0..3 {
        transform[12 + lane] += fixed[lane] - moved[lane];
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// Whether the editor's camera holds the screen (the skater's input is held neutral).
    #[must_use]
    pub fn object_dropper_open(&self) -> bool {
        self.object_dropper.view.is_some()
    }

    /// Whether to hold B for the local skater: on its board (the foot brake), from the editor's
    /// third frame, so the game sees B let go after the LB + B that opened it and pressed anew.
    pub(super) fn object_dropper_skater_riding(&self) -> bool {
        if self.object_dropper.open_frames < BRAKE_AFTER_FRAMES {
            return false;
        }
        let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()) else {
            return false;
        };
        let Ok(physical) = actor.cached_physical_allocation() else {
            return false;
        };
        let Ok(output) = physical.owner().phys_out().lock() else {
            return false;
        };
        output.try_state_output().is_ok_and(|state| !state.is_offboard)
    }

    /// The board decks the visual snapshot draws copies of the player's board with.
    pub(super) fn object_dropper_board_poses(&self) -> (Option<[f32; 16]>, Vec<[f32; 16]>) {
        (self.object_dropper.local_board_deck, self.object_dropper.dropped_board_decks.clone())
    }

    /// Once a frame after the simulation: where the local board's deck is now (physics Part 6),
    /// the frame of reference for drawing dropped copies of the player's board.
    pub(super) fn refresh_object_dropper_boards(&mut self) -> Result<(), SelectedLiveError> {
        let deck = self
            .inner
            .try_with_selected_queued_frame(|frame, _, _| {
                frame
                    .world
                    .board(frame.board)
                    .ok()
                    .and_then(|board| board.try_live_part_transform_host(frame.world.backend(), 6).ok())
            })
            .map_err(|e| error(format!("{e:?}")))?;
        self.object_dropper.local_board_deck = deck;
        let mut decks = Vec::new();
        if self.object_dropper.dropped.iter().any(|dropped| dropped.board.is_some()) {
            let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            for dropped in &mut self.object_dropper.dropped {
                dropped.deck = dropped
                    .board
                    .as_ref()
                    .and_then(|board| world.riderless_board_part_transforms(board).ok().map(|parts| parts[6]));
                decks.extend(dropped.deck);
            }
        }
        self.object_dropper.dropped_board_decks = decks;
        Ok(())
    }

    /// Opens the editor on LB + B. `allowed`: offline free skate with the game's focus.
    /// True when it opened (the caller skips this frame's simulation).
    pub(super) fn try_open_object_dropper(&mut self, pad: Option<XboxPadSample>, allowed: bool) -> bool {
        let buttons = pad.map_or(0, |pad| pad.buttons);
        let pressed = buttons & !self.object_dropper.buttons;
        self.object_dropper.buttons = buttons;
        if !allowed || buttons & LEFT_BUMPER == 0 || pressed & B == 0 {
            return false;
        }
        let Some(matrix) = self.game_camera_matrix() else {
            return false;
        };
        let dropper = &mut self.object_dropper;
        dropper.view = Some(View {
            position: [matrix[12], matrix[13], matrix[14]],
            at: look(normalize([matrix[8], matrix[9], matrix[10]]), 0.0, 0.0),
        });
        dropper.leaving = false;
        dropper.list_open = false;
        dropper.list_items = false;
        dropper.tab = PROPS_TAB;
        dropper.held = None;
        dropper.target = None;
        dropper.flick_ready = false;
        dropper.open_frames = 0;
        dropper.triggers = [255; 2];
        dropper.settings_open = false;
        dropper.settings = EditorSettings::load();
        dropper.settings.publish();
        dropper.settings_saved = dropper.settings;
        dropper.note = "LB + B opened the park editor";
        self.show_object_dropper_menu();
        true
    }

    /// One frame of the editor, after the world's (held neutral) simulation tick.
    pub(super) fn update_object_dropper(&mut self, pad: Option<XboxPadSample>, dt: f32) -> Result<(), SelectedLiveError> {
        let pad = pad.unwrap_or(XboxPadSample {
            buttons: 0,
            left_trigger: 0,
            right_trigger: 0,
            left_x: 0,
            left_y: 0,
            right_x: 0,
            right_y: 0,
        });
        let dropper = &mut self.object_dropper;
        dropper.open_frames = dropper.open_frames.saturating_add(1);
        let pressed = pad.buttons & !dropper.buttons;
        dropper.buttons = pad.buttons;
        let held = [pad.left_trigger >= TRIGGER_HELD, pad.right_trigger >= TRIGGER_HELD];
        let triggers_pressed = [held[0] && dropper.triggers[0] < TRIGGER_HELD, held[1] && dropper.triggers[1] < TRIGGER_HELD];
        dropper.triggers = [pad.left_trigger, pad.right_trigger];
        let stick = [axis(pad.left_x), axis(pad.left_y)];
        let flick = if stick[0].hypot(stick[1]) < STICK_REST {
            dropper.flick_ready = true;
            None
        } else if dropper.flick_ready && (stick[0].abs() > STICK_FLICK || stick[1].abs() > STICK_FLICK) {
            dropper.flick_ready = false;
            Some(if stick[0].abs() > stick[1].abs() { [stick[0].signum(), 0.0] } else { [0.0, stick[1].signum()] })
        } else {
            None
        };
        if dropper.leaving {
            let idle = pad.buttons == 0 && pad.left_trigger < 30 && pad.right_trigger < 30;
            if idle {
                dropper.view = None;
                dropper.leaving = false;
                crate::ui::object_dropper::set_menu(None);
            }
            return Ok(());
        }
        if dropper.settings_open {
            self.update_editor_settings(pressed);
        } else if dropper.list_open {
            self.update_object_list(pressed, triggers_pressed, flick)?;
        } else if pressed & START != 0 {
            dropper.settings_open = true;
            dropper.note = "";
        } else if dropper.held.is_some() {
            self.update_held_prop(&pad, pressed, held, flick, dt)?;
        } else {
            self.update_free_camera(&pad, pressed, dt)?;
        }
        self.refresh_dropper_preview_model()?;
        self.show_object_dropper_menu();
        Ok(())
    }

    /// The object list, as Skate 3's: LT/RT tab; the subcategories (D-pad Up/Down, A opens one,
    /// B or X closes the list), then its items (D-pad Up/Down or a stick flick, LB/RB the next
    /// subcategory's, A takes it, B back to the subcategories).
    fn update_object_list(&mut self, pressed: u16, triggers: [bool; 2], flick: Option<[f32; 2]>) -> Result<(), SelectedLiveError> {
        let dropper = &mut self.object_dropper;
        dropper.note = "";
        if triggers[0] {
            dropper.tab = (dropper.tab + TABS.len() - 1) % TABS.len();
            dropper.list_items = false;
        }
        if triggers[1] {
            dropper.tab = (dropper.tab + 1) % TABS.len();
            dropper.list_items = false;
        }
        let step = flick.map_or(0.0, |flick| -flick[1]);
        let down = pressed & DPAD_DOWN != 0 || step > 0.0;
        let up = pressed & DPAD_UP != 0 || step < 0.0;
        let count = PROPS.len();
        let (start, len) = category_run(dropper.selected);
        if !dropper.list_items {
            if pressed & (B | X) != 0 {
                dropper.list_open = false;
            } else if dropper.tab == PROPS_TAB {
                if down {
                    dropper.selected = (start + len) % count;
                }
                if up {
                    dropper.selected = category_run((start + count - 1) % count).0;
                }
                if pressed & A != 0 {
                    dropper.list_items = true;
                }
            }
            return Ok(());
        }
        let within = dropper.selected - start;
        if pressed & B != 0 {
            dropper.list_items = false;
            dropper.selected = start;
            return Ok(());
        }
        if pressed & RIGHT_BUMPER != 0 {
            dropper.selected = (start + len) % count;
        }
        if pressed & LEFT_BUMPER != 0 {
            dropper.selected = category_run((start + count - 1) % count).0;
        }
        if down {
            dropper.selected = start + (within + 1) % len;
        }
        if up {
            dropper.selected = start + (within + len - 1) % len;
        }
        if pressed & A != 0 {
            self.take_from_list()?;
        }
        Ok(())
    }

    /// Nothing held: fly and look, X opens the list, A picks up and Y deletes the placed prop
    /// under the crosshair, B leaves.
    fn update_free_camera(&mut self, pad: &XboxPadSample, pressed: u16, dt: f32) -> Result<(), SelectedLiveError> {
        let dropper = &mut self.object_dropper;
        let Some(mut view) = dropper.view else {
            return Ok(());
        };
        let (flat, side) = ground_axes(view.at);
        let speed = FLY_SPEED * dropper.settings.scale(EditorSettings::CAMERA_MOVE) * dt;
        view.position = add(view.position, scale(flat, cubic(axis(pad.left_y)) * speed));
        view.position = add(view.position, scale(side, cubic(axis(pad.left_x)) * speed));
        let rise = f32::from(pad.buttons & DPAD_UP != 0) - f32::from(pad.buttons & DPAD_DOWN != 0);
        view.position[1] += rise * RISE_SPEED * dropper.settings.scale(EditorSettings::CAMERA_MOVE) * dt;
        let look_speed = LOOK_SPEED * dropper.settings.scale(EditorSettings::CAMERA_LOOK) * dt;
        view.at = look(view.at, cubic(-axis(pad.right_x)) * look_speed, cubic(-axis(pad.right_y)) * look_speed);
        dropper.view = Some(view);
        self.refresh_dropper_target()?;
        let dropper = &mut self.object_dropper;
        if pressed & B != 0 {
            dropper.leaving = true;
            crate::ui::object_dropper::set_menu(Some(leaving_menu()));
            return Ok(());
        }
        if pressed & X != 0 {
            dropper.list_open = true;
            dropper.list_items = false;
            dropper.note = "";
        } else if pressed & A != 0 {
            let note = match dropper.target {
                Some(index) => {
                    self.pick_up(index)?;
                    "picked it up"
                }
                None => "nothing under the crosshair to pick up",
            };
            self.object_dropper.note = note;
        } else if pressed & Y != 0 {
            let note = match dropper.target.take() {
                Some(index) => {
                    let dropped = self.object_dropper.dropped.remove(index);
                    self.remove_dropped(&dropped)?;
                    "deleted it"
                }
                None => "nothing under the crosshair to delete",
            };
            self.object_dropper.note = note;
        }
        Ok(())
    }

    /// A prop held: move, raise, orbit, zoom, turn; A places it, B places it and leaves.
    fn update_held_prop(
        &mut self,
        pad: &XboxPadSample,
        pressed: u16,
        triggers: [bool; 2],
        flick: Option<[f32; 2]>,
        dt: f32,
    ) -> Result<(), SelectedLiveError> {
        let dropper = &mut self.object_dropper;
        let (Some(mut view), Some(mut held)) = (dropper.view, dropper.held) else {
            return Ok(());
        };
        let (flat, side) = ground_axes(view.at);
        // Turning "up" tips the prop's near side up: about the camera's left across the ground.
        let tip = scale(side, -1.0);
        let settings = dropper.settings;
        let stick = [axis(pad.left_x), axis(pad.left_y)];
        let local = local_middle(&dropper.centers, held.prop);
        if let Some((axis, left)) = dropper.spin {
            let step = left.signum() * (SPIN_SPEED * settings.scale(EditorSettings::SPIN) * dt).min(left.abs());
            turn_about(&mut held.transform, axis, step, local);
            dropper.spin = (left - step).abs().gt(&1e-5).then_some((axis, left - step));
        }
        let fine = triggers == [true, true] && stick[0].hypot(stick[1]) > STICK_REST;
        dropper.fine_hold = if fine { dropper.fine_hold + dt } else { 0.0 };
        match triggers {
            [true, true] => {
                let ramp = (dropper.fine_hold / FINE_TURN_RAMP).min(1.0);
                let speed = (FINE_TURN_START + (FINE_TURN_TOP - FINE_TURN_START) * ramp * ramp)
                    * settings.scale(EditorSettings::FINE_TURN) * dt;
                turn_about(&mut held.transform, UP, stick[0] * speed, local);
                turn_about(&mut held.transform, tip, stick[1] * speed, local);
            }
            [false, true] => {
                if let Some([x, y]) = flick.filter(|_| dropper.spin.is_none()) {
                    let quarter = std::f32::consts::FRAC_PI_2;
                    dropper.spin = Some(if x != 0.0 { (UP, x * quarter) } else { (tip, y * quarter) });
                }
            }
            _ => {
                let speed = dropper.orbit_distance * MOVE_RATE * settings.scale(EditorSettings::PROP_MOVE) * dt;
                let across = add(scale(flat, cubic(stick[1]) * speed), scale(side, cubic(stick[0]) * speed));
                held.transform[12] += across[0];
                held.transform[14] += across[2];
            }
        }
        let rise = f32::from(pad.buttons & DPAD_UP != 0) - f32::from(pad.buttons & DPAD_DOWN != 0);
        held.transform[13] += rise * dropper.orbit_distance * LIFT_RATE * settings.scale(EditorSettings::PROP_MOVE) * dt;
        let zoom = f32::from(pad.buttons & DPAD_LEFT != 0) - f32::from(pad.buttons & DPAD_RIGHT != 0);
        dropper.orbit_distance = (dropper.orbit_distance * (zoom * ZOOM_RATE * dt).exp()).clamp(ORBIT_MIN, ORBIT_MAX);
        if pressed & L3 != 0 {
            dropper.spin = None;
            let fixed = place(&held.transform, local);
            held.transform = crate::ui::object_dropper::prop_transform(origin(&held.transform), held.home);
            let moved = place(&held.transform, local);
            for lane in 0..3 {
                held.transform[12 + lane] += fixed[lane] - moved[lane];
            }
        }
        let look_speed = LOOK_SPEED * settings.scale(EditorSettings::CAMERA_LOOK) * dt;
        view.at = look(view.at, cubic(-axis(pad.right_x)) * look_speed, cubic(-axis(pad.right_y)) * look_speed);
        view.position = add(place(&held.transform, local), scale(view.at, -dropper.orbit_distance));
        dropper.view = Some(view);
        dropper.held = Some(held);
        if pressed & A != 0 {
            self.object_dropper.note = self.place_held()?;
        } else if pressed & B != 0 {
            self.cancel_held()?;
        } else if pressed & Y != 0 {
            let dropper = &mut self.object_dropper;
            dropper.held = None;
            dropper.spin = None;
            dropper.note = "deleted it";
        }
        Ok(())
    }

    /// Restore editor props after challenge/game resets in any mode. The native
    /// census keeps its ordinary distance/capacity rules when enabled; modes that
    /// disable it use the same explicit drop path as the editor.
    pub(super) fn refresh_dropped_props(&mut self) -> Result<(), SelectedLiveError> {
        let locators: Vec<u64> = self.object_dropper.dropped.iter().filter(|dropped| dropped.board.is_none()).map(|dropped| dropped.locator).collect();
        if locators.is_empty() {
            return Ok(());
        }
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Ok(());
                };
                living
                    .try_respawn_dropped_locators(frame.world, frame.actor, frame.cvars, recipes, &locators)
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }

    /// Explicitly leave the editor, returning a picked-up prop to its last placement.
    pub(super) fn close_object_dropper(&mut self) -> Result<(), SelectedLiveError> {
        self.cancel_held()?;
        let dropper = &mut self.object_dropper;
        dropper.view = None;
        dropper.leaving = false;
        dropper.list_open = false;
        dropper.settings_open = false;
        crate::ui::object_dropper::set_menu(None);
        Ok(())
    }

    fn cancel_held(&mut self) -> Result<(), SelectedLiveError> {
        let Some(held) = self.object_dropper.held.take() else {
            return Ok(());
        };
        if let Some(transform) = held.picked {
            if PROPS[held.prop].2 == YOUR_BOARD {
                self.drop_board(held.prop, transform)?;
            } else {
                self.drop_prop(held.prop, transform)?;
            }
        }
        let dropper = &mut self.object_dropper;
        dropper.spin = None;
        dropper.list_open = true;
        dropper.list_items = true;
        dropper.tab = PROPS_TAB;
        dropper.selected = held.prop;
        dropper.note = "";
        Ok(())
    }

    /// The settings page: D-pad Up/Down pick a setting, Left/Right change it, X sets it back to
    /// its default, B (or Start) closes the page, saving any change. The board weight takes
    /// effect at once.
    fn update_editor_settings(&mut self, pressed: u16) {
        let dropper = &mut self.object_dropper;
        let rows = EditorSettings::ROWS.len();
        if pressed & DPAD_DOWN != 0 {
            dropper.settings_row = (dropper.settings_row + 1) % rows;
        }
        if pressed & DPAD_UP != 0 {
            dropper.settings_row = (dropper.settings_row + rows - 1) % rows;
        }
        let (min, max, step) = EditorSettings::limits(dropper.settings_row);
        let value = &mut dropper.settings.percent[dropper.settings_row];
        if pressed & DPAD_RIGHT != 0 {
            *value = (*value + step).min(max);
        }
        if pressed & DPAD_LEFT != 0 {
            *value = value.saturating_sub(step).max(min);
        }
        if pressed & X != 0 {
            *value = EditorSettings::DEFAULTS[dropper.settings_row];
        }
        dropper.settings.publish();
        if pressed & (B | START) != 0 {
            dropper.settings_open = false;
            dropper.note = "";
            if dropper.settings != dropper.settings_saved {
                dropper.note = match dropper.settings.save() {
                    Ok(()) => "settings saved",
                    Err(_) => "could not save park-editor.json",
                };
                dropper.settings_saved = dropper.settings;
            }
        }
    }

    fn show_object_dropper_menu(&self) {
        use crate::ui::object_dropper::Menu;
        let dropper = &self.object_dropper;
        if dropper.leaving {
            return;
        }
        let placed = format!("{} of {MAX_DROPPED} props placed", dropper.dropped.len());
        let mut menu = if dropper.settings_open {
            Menu {
                title: "Editor Settings".into(),
                rows: EditorSettings::ROWS
                    .iter()
                    .zip(dropper.settings.percent)
                    .map(|((_, label), value)| (format!("{label}   {value}%"), None))
                    .collect(),
                selected: Some(dropper.settings_row),
                info: vec!["D-pad Up/Down: choose      D-pad Left/Right: slower / faster".into()],
                hints: vec![("X", "Default".into()), ("B", "Back".into())],
                ..Menu::default()
            }
        } else if dropper.list_open {
            let tabs = Some((TABS.iter().map(|tab| (*tab).to_owned()).collect(), dropper.tab));
            let (start, len) = category_run(dropper.selected);
            if dropper.tab != PROPS_TAB {
                Menu {
                    title: "New Object".into(),
                    tabs,
                    info: vec![format!("{} objects are not in Skate 2 yet.", TABS[dropper.tab])],
                    hints: vec![("B", "Back".into())],
                    ..Menu::default()
                }
            } else if !dropper.list_items {
                let runs = categories();
                Menu {
                    title: "New Object".into(),
                    tabs,
                    rows: runs.iter().map(|&(first, len)| (PROPS[first].0.to_owned(), Some(len))).collect(),
                    selected: runs.iter().position(|&(first, _)| first == start),
                    info: PROPS_INFO.iter().map(|line| (*line).to_owned()).collect(),
                    hints: vec![("A", "Select".into()), ("B", "Back".into())],
                    ..Menu::default()
                }
            } else {
                let within = dropper.selected - start;
                let rows = LIST_ROWS.min(len);
                let first = within.saturating_sub(rows / 2).min(len - rows);
                Menu {
                    title: format!("{}  ({}/{len})", PROPS[start].0, within + 1),
                    tabs,
                    rows: (start + first..start + first + rows).map(|index| (PROPS[index].1.to_owned(), None)).collect(),
                    selected: Some(within - first),
                    info: PROPS_INFO.iter().map(|line| (*line).to_owned()).collect(),
                    hints: vec![("A", "Take".into()), ("B", "Back".into()), ("RB", "Next group".into())],
                    ..Menu::default()
                }
            }
        } else if let Some(held) = dropper.held {
            Menu {
                title: PROPS[held.prop].1.into(),
                info: vec![
                    "Left stick: move      D-pad Up/Down: raise / lower".into(),
                    "Right stick: orbit      D-pad Right/Left: closer / farther".into(),
                    "RT + flick left stick: turn 90 degrees".into(),
                    "LT + RT + left stick: fine turn      L3: reset turn".into(),
                ],
                hints: vec![("A", "Place".into()), ("B", "Back".into()), ("Y", "Delete".into()), ("Start", "Settings".into())],
                crosshair: true,
                ..Menu::default()
            }
        } else {
            let target = dropper.target.and_then(|index| dropper.dropped.get(index));
            Menu {
                title: "Park Editor".into(),
                info: vec![
                    match target {
                        Some(target) => format!("Crosshair on: {}", PROPS[target.prop].1),
                        None => "Crosshair on: nothing".into(),
                    },
                    placed,
                    "Left stick: move      Right stick: look".into(),
                    "D-pad Up/Down: raise / lower".into(),
                ],
                hints: vec![("X", "Objects".into()), ("A", "Pick up".into()), ("Y", "Delete".into()), ("B", "Exit".into()), ("Start", "Settings".into())],
                crosshair: true,
                ..Menu::default()
            }
        };
        if !dropper.note.is_empty() {
            menu.info.push(dropper.note.into());
        }
        crate::ui::object_dropper::set_menu(Some(menu));
    }

    /// How far along the crosshair the first surface is, within reach.
    fn crosshair_hit(&mut self) -> Result<Option<f32>, SelectedLiveError> {
        let Some(view) = self.object_dropper.view else {
            return Ok(None);
        };
        let start = [view.position[0], view.position[1], view.position[2], 1.0];
        let end = std::array::from_fn(|i| if i < 3 { view.position[i] + view.at[i] * AIM_REACH } else { 1.0 });
        self.inner
            .try_with_selected_queued_frame(|frame, ground, _| -> Result<Option<f32>, String> {
                use crate::skeleton_root_transforms::SkeletonRootVector;
                let hit = frame
                    .world
                    .try_world_line_test(
                        ground,
                        crate::collision_line_collection::CollisionLineInput {
                            start: SkeletonRootVector::known(start),
                            end: SkeletonRootVector::known(end),
                            fatness: 0.0,
                        },
                        "ObjectDropper::Aim",
                        &mut crate::collision_line_numeric::HostCollisionLineNumeric,
                    )
                    .map_err(|e| format!("{e:?}"))?;
                Ok(hit
                    .and_then(|hit| hit.position.known_lanes())
                    .map(|p| dot([p[0] - start[0], p[1] - start[1], p[2] - start[2]], view.at)))
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }

    /// The selected prop into the editor's hands, in front of the crosshair: on the first
    /// surface it meets, or `SPAWN_DISTANCE` along it.
    fn take_from_list(&mut self) -> Result<(), SelectedLiveError> {
        let Some(view) = self.object_dropper.view else {
            return Ok(());
        };
        let distance = self.crosshair_hit()?.map_or(SPAWN_DISTANCE, |hit| hit.clamp(0.0, SPAWN_DISTANCE));
        let mut at = add(view.position, scale(view.at, distance));
        let prop = self.object_dropper.selected;
        if PROPS[prop].2 == YOUR_BOARD {
            at[1] += BOARD_REST_DECK_HEIGHT;
        }
        let home = view.at[0].atan2(view.at[2]);
        self.hold(Held { prop, transform: crate::ui::object_dropper::prop_transform(at, home), home, picked: None })?;
        let dropper = &mut self.object_dropper;
        dropper.list_open = false;
        dropper.note = "";
        Ok(())
    }

    /// `held` into the editor's hands, the camera turned onto it from where it is.
    fn hold(&mut self, held: Held) -> Result<(), SelectedLiveError> {
        self.object_dropper.held = Some(held);
        self.refresh_dropper_preview_model()?;
        let dropper = &mut self.object_dropper;
        let Some(view) = dropper.view.as_mut() else {
            return Ok(());
        };
        let middle = middle(&dropper.centers, held.prop, &held.transform);
        let toward = [middle[0] - view.position[0], middle[1] - view.position[1], middle[2] - view.position[2]];
        let distance = dot(toward, toward).sqrt();
        if distance > 1e-3 {
            view.at = look(normalize(toward), 0.0, 0.0);
        }
        dropper.orbit_distance = distance.clamp(ORBIT_MIN, ORBIT_MAX);
        view.position = add(middle, scale(view.at, -dropper.orbit_distance));
        dropper.held = Some(held);
        dropper.target = None;
        dropper.flick_ready = false;
        dropper.spin = None;
        Ok(())
    }

    /// Takes the placed prop `index` out of the world and into the editor's hands.
    fn pick_up(&mut self, index: usize) -> Result<(), SelectedLiveError> {
        let dropped = self.object_dropper.dropped.remove(index);
        self.object_dropper.target = None;
        self.remove_dropped(&dropped)?;
        let home = heading(&dropped.transform);
        self.hold(Held { prop: dropped.prop, transform: dropped.transform, home, picked: Some(dropped.transform) })?;
        Ok(())
    }

    /// Lets go of the held prop where it is: it joins the world and falls. Kept in hand if the
    /// world takes no more.
    fn place_held(&mut self) -> Result<&'static str, SelectedLiveError> {
        let Some(held) = self.object_dropper.held else {
            return Ok("");
        };
        if self.object_dropper.dropped.len() >= MAX_DROPPED {
            let oldest = self.object_dropper.dropped.remove(0);
            self.remove_dropped(&oldest)?;
        }
        let note = if PROPS[held.prop].2 == YOUR_BOARD {
            self.drop_board(held.prop, held.transform)?
        } else {
            self.drop_prop(held.prop, held.transform)?
        };
        if note.starts_with("placed") {
            self.object_dropper.held = None;
        }
        Ok(note)
    }

    fn drop_prop(&mut self, prop: usize, transform: [f32; 16]) -> Result<&'static str, SelectedLiveError> {
        let guid = PROPS[prop].2;
        let locator = LOCATOR_BASE + self.object_dropper.next_locator;
        self.object_dropper.next_locator += 1;
        let added = self
            .inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Ok(false);
                };
                living
                    .try_drop_on_locator(frame.world, frame.actor, frame.cvars, recipes, locator, guid, &transform)
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)?;
        Ok(if added {
            self.object_dropper.dropped.push(Dropped { locator, board: None, prop, transform, key: 0, deck: None, marker: None });
            "placed"
        } else {
            "could not place it there"
        })
    }

    /// Where every placed prop is now (the world moves them), and which one the crosshair is on.
    fn refresh_dropper_target(&mut self) -> Result<(), SelectedLiveError> {
        let Some(view) = self.object_dropper.view else {
            return Ok(());
        };
        let locators: Vec<u64> = self.object_dropper.dropped.iter().map(|dropped| dropped.locator).collect();
        let now = {
            let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            let mut now = crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_transforms(&world, &locators)
                .map_err(error)?;
            for (dropped, now) in self.object_dropper.dropped.iter().zip(now.iter_mut()) {
                if let Some(board) = &dropped.board {
                    *now = world.riderless_board_part_transforms(board).ok().map(|parts| parts[6]);
                }
            }
            now
        };
        let dropper = &mut self.object_dropper;
        for (dropped, now) in dropper.dropped.iter_mut().zip(&now) {
            if let Some(transform) = now {
                dropped.transform = *transform;
            }
        }
        dropper.target = dropper
            .dropped
            .iter()
            .enumerate()
            .filter_map(|(index, dropped)| {
                let middle = middle(&dropper.centers, dropped.prop, &dropped.transform);
                let toward = [middle[0] - view.position[0], middle[1] - view.position[1], middle[2] - view.position[2]];
                let along = dot(toward, view.at);
                let off = (dot(toward, toward) - along * along).max(0.0).sqrt();
                let reach = target_reach(&dropper.centers, dropped.prop) + TARGET_SPREAD * along;
                (along > 0.0 && along < AIM_REACH && off <= reach).then_some((index, off / reach))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index);
        Ok(())
    }

    /// A copy of the player's board: a real riderless board with its deck at `transform`.
    fn drop_board(&mut self, prop: usize, transform: [f32; 16]) -> Result<&'static str, SelectedLiveError> {
        let boards: Vec<usize> = (0..self.object_dropper.dropped.len())
            .filter(|&index| self.object_dropper.dropped[index].board.is_some())
            .collect();
        if boards.len() >= MAX_BOARDS {
            let oldest = self.object_dropper.dropped.remove(boards[0]);
            self.remove_dropped(&oldest)?;
        }
        let board = self
            .inner
            .try_with_selected_queued_frame(|frame, _, _| -> Result<_, String> {
                let vault = std::sync::Arc::clone(
                    &frame.actor.cached_physical_allocation().map_err(|e| format!("{e:?}"))?.owner().inputs().vault,
                );
                Ok(frame.world.try_spawn_riderless_board(&vault, transform).ok())
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)?;
        Ok(match board {
            Some(board) => {
                let key = self.object_dropper.next_board_key & 0x00ff_ffff;
                self.object_dropper.next_board_key = self.object_dropper.next_board_key.wrapping_add(1);
                self.object_dropper.dropped.push(Dropped {
                    locator: 0,
                    board: Some(board),
                    prop,
                    transform,
                    key,
                    deck: None,
                    marker: None,
                });
                "placed your board"
            }
            None => "no room for another board (AI or party skaters use the slots)",
        })
    }

    /// Takes one dropped thing back: a board leaves the skater pool, a prop its locator.
    fn remove_dropped(&mut self, dropped: &Dropped) -> Result<(), SelectedLiveError> {
        if let Some(board) = &dropped.board {
            return self
                .inner
                .try_with_selected_queued_frame(|frame, _, _| frame.world.try_remove_riderless_board(board).map_err(|e| format!("{e:?}")))
                .map_err(|e| error(format!("{e:?}")))?
                .map_err(error);
        }
        self.remove_dropped_objects(&[dropped.locator])
    }

    fn remove_dropped_objects(&mut self, locators: &[u64]) -> Result<(), SelectedLiveError> {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Ok(());
                };
                living
                    .try_remove_dropped_locators(frame.world, frame.actor, frame.cvars, recipes, locators)
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }

    /// The world through the editor's camera, with the held prop drawn where it is.
    ///
    /// # Errors
    ///
    /// Returns [`SelectedLiveError`] when the editor is not open or the world cannot be drawn.
    pub fn try_object_dropper_visual_snapshot(&self) -> Result<SelectedVisualSnapshot<'_>, SelectedLiveError> {
        let view = self.object_dropper.view.ok_or_else(|| error("not open"))?;
        let right = normalize(cross(UP, view.at));
        let up = normalize(cross(view.at, right));
        let right = normalize(cross(up, view.at));
        let (p, at) = (view.position, view.at);
        let matrix = [
            right[0], right[1], right[2], 0.0,
            up[0], up[1], up[2], 0.0,
            at[0], at[1], at[2], 0.0,
            p[0], p[1], p[2], 1.0,
        ];
        let mut visual = self.visual_snapshot(
            Some(NisCamera { matrix: Some(matrix), fov: FIELD_OF_VIEW, near_plane: NEAR_PLANE }),
            Some(FrozenFocus::At(p)),
        )?;
        let Some(held) = self.object_dropper.held else {
            return Ok(visual);
        };
        if PROPS[held.prop].2 == YOUR_BOARD {
            visual.dropped_boards.push(held.transform);
        } else if let (Some((model_prop, asset_id, render_model_entry)), Some(template)) =
            (self.object_dropper.preview_model, visual.dynamic_objects.first().copied())
            && model_prop == held.prop
        {
            visual.dynamic_objects.push(crate::SelectedDynamicObjectVisual {
                entity_id: u32::MAX,
                asset_id,
                render_model_entry,
                part_transform: held.transform,
                ..template
            });
        }
        Ok(visual)
    }

    /// Looks up the held prop's model when it changes.
    fn refresh_dropper_preview_model(&mut self) -> Result<(), SelectedLiveError> {
        let Some(held) = self.object_dropper.held else {
            return Ok(());
        };
        if self.object_dropper.preview_model.is_some_and(|(cached, _, _)| cached == held.prop) || PROPS[held.prop].2 == YOUR_BOARD {
            return Ok(());
        }
        let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
        let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
        let model = crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_preview_model(&world, PROPS[held.prop].2)
            .map_err(error)?;
        drop(world);
        if let Some((_, _, center, reach)) = model {
            self.object_dropper.centers.insert(held.prop, (center, reach));
        }
        self.object_dropper.preview_model = model.map(|(asset_id, entry, _, _)| (held.prop, asset_id, entry));
        Ok(())
    }
}

/// The panel while leaving waits for the buttons to be let go.
fn leaving_menu() -> crate::ui::object_dropper::Menu {
    crate::ui::object_dropper::Menu {
        title: "Park Editor".into(),
        info: vec!["Let go of the buttons to skate.".into()],
        ..Default::default()
    }
}

/// Every subcategory: its first catalog entry and length, in order.
fn categories() -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < PROPS.len() {
        let run = category_run(index);
        runs.push(run);
        index = run.0 + run.1;
    }
    runs
}

/// The subcategory holding catalog entry `index`: its first entry and length.
fn category_run(index: usize) -> (usize, usize) {
    let category = PROPS[index].0;
    let start = PROPS[..index].iter().rposition(|entry| entry.0 != category).map_or(0, |last| last + 1);
    let len = PROPS[start..].iter().take_while(|entry| entry.0 == category).count();
    (start, len)
}

// ---- Dropped boards in the session marker ---------------------------------------------------
// Skate 2 adds a grabbable prop to the session marker through the prop menu an on-foot skater
// gets beside it (`PropHudSnapshot`); using the marker puts every member back where it was
// added. A dropped board is no living-world object, so it gets that same menu under an object
// ID of its own and keeps its membership here.

/// Whether a prop-menu object ID names a dropped board rather than a living-world object.
pub(super) fn is_board_prop(object: u32) -> bool {
    object & 0xff00_0000 == BOARD_PROP_ID
}

/// The deck turned upright about its own heading, lifted to fall back onto its wheels.
fn upright_deck(deck: [f32; 16]) -> [f32; 16] {
    // The deck's Z row is its length; keep it level, or use X when the board stands on end.
    let (mut x, mut z) = (deck[8], deck[10]);
    if x.hypot(z) < 1e-3 {
        (x, z) = (-deck[2], deck[0]);
    }
    let length = x.hypot(z).max(1e-6);
    let (zx, zz) = (x / length, z / length);
    [
        zz, 0.0, -zx, 0.0,
        0.0, 1.0, 0.0, 0.0,
        zx, 0.0, zz, 0.0,
        deck[12], deck[13] + BOARD_RIGHT_LIFT, deck[14], 1.0,
    ]
}

impl ObjectDropper {
    /// The prop menu for the dropped board nearest an on-foot skater at `skater`, if one is
    /// in reach. `marker_exists`: the session marker has been set (the menu's third value).
    pub(super) fn board_prop_hud(
        &self,
        skater: [f32; 3],
        marker_exists: bool,
    ) -> Option<crate::ui::phonelist::PropHudSnapshot> {
        // Like a living-world prop, only while RB (interact) is held, and not in the editor.
        if self.view.is_some() || self.buttons & RIGHT_BUMPER == 0 {
            return None;
        }
        self.dropped
            .iter()
            .filter(|dropped| dropped.board.is_some())
            .filter_map(|dropped| {
                let deck = dropped.deck?;
                let across = (deck[12] - skater[0]).hypot(deck[14] - skater[2]);
                (across <= BOARD_MENU_REACH && (deck[13] - skater[1]).abs() <= BOARD_MENU_HEIGHT)
                    .then_some((dropped, deck, across))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(dropped, deck, _)| crate::ui::phonelist::PropHudSnapshot {
                object: BOARD_PROP_ID | dropped.key,
                // Upright disabled, reset disabled, marker valid, in the marker, maximum reached.
                params: [deck[5] >= BOARD_UPRIGHT_UP_Y, false, marker_exists, dropped.marker.is_some(), false],
            })
    }

    /// `ClearDynamicObjectLocatorsInSessionMarker`'s counterpart: no board stays a member.
    pub(super) fn clear_board_markers(&mut self) {
        for dropped in &mut self.dropped {
            dropped.marker = None;
        }
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The prop menu's actions on dropped boards (0 upright, 1 reset, 2 add to the marker,
    /// 3 remove), then, when the marker is being used (`restore`), every member board put back
    /// at rest where it was added. `has_marker`: a session marker exists to add to.
    pub(super) fn apply_board_prop_actions(
        &mut self,
        actions: &[(u32, u8)],
        restore: bool,
        has_marker: bool,
    ) -> Result<(), SelectedLiveError> {
        for &(object, action) in actions {
            let key = object & 0x00ff_ffff;
            let Some(index) = self
                .object_dropper
                .dropped
                .iter()
                .position(|dropped| dropped.board.is_some() && dropped.key == key)
            else {
                continue;
            };
            let board = self.object_dropper.dropped[index].board.clone().expect("filtered board");
            let deck = self.board_deck(&board)?;
            match action {
                0 if deck[5] < BOARD_UPRIGHT_UP_Y => self.place_board(&board, upright_deck(deck))?,
                1 => {
                    let start = self.object_dropper.dropped[index].transform;
                    self.place_board(&board, start)?;
                }
                2 if has_marker => {
                    self.object_dropper.dropped[index].marker =
                        Some(if deck[5] >= BOARD_UPRIGHT_UP_Y { deck } else { upright_deck(deck) });
                }
                3 => self.object_dropper.dropped[index].marker = None,
                _ => {}
            }
        }
        if restore {
            let members: Vec<_> = self
                .object_dropper
                .dropped
                .iter()
                .filter_map(|dropped| Some((dropped.board.clone()?, dropped.marker?)))
                .collect();
            for (board, saved) in members {
                self.place_board(&board, saved)?;
            }
        }
        Ok(())
    }

    fn board_deck(&mut self, board: &RiderlessBoardHandle) -> Result<[f32; 16], SelectedLiveError> {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                frame.world.riderless_board_part_transforms(board).map(|parts| parts[6]).map_err(|e| format!("{e:?}"))
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }

    /// Teleports a dropped board (deck at `transform`) at rest.
    fn place_board(&mut self, board: &RiderlessBoardHandle, transform: [f32; 16]) -> Result<(), SelectedLiveError> {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                frame.world.try_place_riderless_board(board, transform).map_err(|e| format!("{e:?}"))
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)
    }
}

/// Test access for the facade corpus (which drives the whole live session).
#[cfg(test)]
impl<T, C> SelectedLiveSession<T, C> {
    /// The living world reset an online game's entry runs (`try_reset_game` as a network game).
    pub(crate) fn test_network_game_reset(&mut self) {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Err("no living world".to_string());
                };
                living.try_reset_game(frame.world, frame.actor, frame.cvars, recipes, true).map_err(|e| e.to_string())
            })
            .unwrap()
            .unwrap();
    }
    pub(crate) fn test_respawn_online_props(&mut self) {
        self.refresh_dropped_props().unwrap();
    }
    /// Whether each placed prop has its object in the world.
    pub(crate) fn test_dropped_objects_out(&mut self) -> Vec<bool> {
        let locators: Vec<u64> = self.object_dropper.dropped.iter().map(|dropped| dropped.locator).collect();
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_objects_out(frame.world, &locators).map_err(|e| e.to_string())
            })
            .unwrap()
            .unwrap()
    }
    pub(crate) fn test_drop_board(&mut self, transform: [f32; 16]) -> bool {
        let board = PROPS.iter().position(|entry| entry.2 == YOUR_BOARD).expect("Your board entry");
        self.drop_board(board, transform).expect("drop board") == "placed your board"
    }
    /// The local skater's root (world XYZ, then its forward Z axis XYZ).
    pub(crate) fn test_skater_root(&mut self) -> ([f32; 3], [f32; 3]) {
        self.inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let output = physical.owner().phys_out().lock().unwrap();
                let root = &output.try_skeleton_output().unwrap().skeleton_root_matrix;
                (root.columns()[3].xyz, root.columns()[2].xyz)
            })
            .unwrap()
    }
    /// Each dropped board's (prop-menu object ID, live deck, marker transform).
    pub(crate) fn test_dropped_boards(&self) -> Vec<(u32, Option<[f32; 16]>, Option<[f32; 16]>)> {
        self.object_dropper
            .dropped
            .iter()
            .filter(|dropped| dropped.board.is_some())
            .map(|dropped| (BOARD_PROP_ID | dropped.key, dropped.deck, dropped.marker))
            .collect()
    }
    pub(crate) fn test_place_dropped_board(&mut self, object: u32, transform: [f32; 16]) {
        let board = self
            .object_dropper
            .dropped
            .iter()
            .find(|dropped| dropped.board.is_some() && BOARD_PROP_ID | dropped.key == object)
            .and_then(|dropped| dropped.board.clone())
            .expect("dropped board");
        self.place_board(&board, transform).unwrap();
    }
    /// The prop menu the HUD shows now, if any.
    pub(crate) fn test_prop_menu(&self) -> Option<crate::ui::phonelist::PropHudSnapshot> {
        self.inner.session_marker.as_ref().and_then(|marker| marker.hud.prop)
    }
    /// Queues a prop-menu action as the pad would (0 upright, 1 reset, 2 add, 3 remove).
    pub(crate) fn test_queue_prop_action(&mut self, object: u32, action: u8) {
        self.inner.session_marker.as_mut().expect("session marker").prop_actions.push((object, action));
    }
    /// Uses the marker's prop restore, as a marker teleport request does.
    pub(crate) fn test_restore_marker_props(&mut self) {
        self.inner.session_marker.as_mut().expect("session marker").restore_props = true;
    }
    /// The park editor as the test pad sees it: (open, list open, items shown, held prop name and
    /// transform, every placed prop's live transform, the crosshair's target, camera position).
    pub(crate) fn test_editor(&mut self) -> TestEditor {
        self.refresh_dropper_target().unwrap();
        let dropper = &self.object_dropper;
        TestEditor {
            open: dropper.view.is_some(),
            list_open: dropper.list_open,
            list_items: dropper.list_items,
            held: dropper.held.map(|held| (PROPS[held.prop].1, held.transform)),
            held_middle: dropper.held.map(|held| (local_middle(&dropper.centers, held.prop), middle(&dropper.centers, held.prop, &held.transform))),
            placed: dropper.dropped.iter().map(|dropped| dropped.transform).collect(),
            target: dropper.target,
            camera: dropper.view.map(|view| view.position),
            settings_open: dropper.settings_open,
            settings: dropper.settings.percent.to_vec(),
        }
    }
    pub(crate) fn test_has_session_marker(&self) -> bool {
        self.inner.session_marker.as_ref().is_some_and(|marker| marker.marker.is_some())
    }
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct TestEditor {
    pub open: bool,
    pub list_open: bool,
    pub list_items: bool,
    pub held: Option<(&'static str, [f32; 16])>,
    /// The held prop's middle in model space and in the world.
    pub held_middle: Option<([f32; 3], [f32; 3])>,
    pub placed: Vec<[f32; 16]>,
    pub target: Option<usize>,
    pub camera: Option<[f32; 3]>,
    pub settings_open: bool,
    pub settings: Vec<u16>,
}
