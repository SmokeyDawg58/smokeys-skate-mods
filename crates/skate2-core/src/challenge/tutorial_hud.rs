//! `Sk8::HUD::TutorialInstructionHUD`, the `tutorialinstruction` HUD
//! component (ctor `0x826b5a50`), driven by its group's
//! `TutorialInstructionHUDData`. The screen `hud2/hudtutorial` reads the bound
//! data through `HUDComponentsCmdTranslator` (`GetTutorialHalText`
//! `0x826bae30`, `GetTutorialGesturesButtons` `0x826bae70`); `SetScreenState`
//! runs its `SetState` at once, so each state carries what the screen reads.

use super::system::World;

/// The component's name (`kHUDName`).
pub const COMPONENT: &str = "tutorialinstruction";

/// The `SetScreenState` values the component sets.
pub mod screen_state {
    /// `Show(true)`.
    pub const SHOW: i32 = -10;
    /// `Show(false)`.
    pub const HIDE: i32 = -20;
    /// Any other instruction: its text and the trick guide's gestures.
    pub const INSTRUCTION: i32 = 10;
    pub const NONE: i32 = 11;
    pub const OLLIE: i32 = 12;
    pub const NOLLIE: i32 = 13;
    pub const KICKFLIP: i32 = 14;
    pub const POPSHUVIT: i32 = 15;
    pub const POPSHUVIT_360: i32 = 16;
    /// The stance's gestures flipped.
    pub const STANCE: i32 = 77;
}

/// `GetTutorialGesturesButtons`: the trick guide collection
/// (`StringToLowerCaseKey(mInstruction)`) in the trick class, else in the
/// tutorial class, field `0x03acc9f200807b87`.
const TRICK_CLASS: u64 = 0x6918_4699_84a8_c596;
const TUTORIAL_CLASS: u64 = 0xc294_e3d4_6e9a_4cac;
const TRICK_SLOTS: u64 = 0x03ac_c9f2_0080_7b87;

/// `gFETrickGuideButtons` (`ETrickInput` order).
const BUTTONS: [&str; 29] = [
    "", "Back", "Start", "A", "B", "Y", "X", "R_Trigger", "L_Trigger", "L_Bumper", "R_Bumper", "L3_Click", "R3_Click",
    "Stick_Right_Up", "Stick_Right_Down", "Stick_Right_L", "Stick_Right_R", "Stick_Right", "DPad_Up", "DPad_Down",
    "DPad_Left", "DPad_Right", "Stick_Left_Up", "Stick_Left_Down", "Stick_Left_L", "Stick_Left_R", "Stick_Left", "DPad",
    "",
];

/// `gFETrickGuideGestures` (`eSk8ControllerGesture` order); `GetGestureString`
/// reads `Unknown` (47) and `Max` (48) as `None`.
const GESTURES: [&str; 47] = [
    "", "grind_5_0_Grind", "grind_fs_5_0", "grind_fs_bluntslide", "grind_fs_boardslide", "grind_fs_crooks",
    "grind_fs_lipslide", "grind_fs_noseblunt", "grind_fs_overcrooks", "grind_fs_salad", "grind_fs_noseslide",
    "grind_fs_tailslide", "grind_nose_grind", "grind_fs_feeble", "grind_fs_smith", "grind_fs_willie",
    "grind_fs_overwillie", "nollie", "nollie_360_flip", "nollie_360_hardflip", "nollie_360_heelflip",
    "nollie_360_inward_heel", "nollie_360popshuvit", "nollie_fs_360popshuvit", "nollie_fs_popshuvit",
    "nollie_hardflip", "nollie_heelflip", "nollie_inward_heelflip", "nollie_kickflip", "nollie_popshuvit",
    "nollie_varial_flip", "nollie_varial_heelflip", "ollie", "ollie_360_flip", "ollie_360_hardflip",
    "ollie_360_heelflip", "ollie_360_inward_heelflip", "ollie_360popshuvit", "ollie_fs_360popshuvit",
    "ollie_fs_popshuvit", "ollie_hardflip", "ollie_heelflip", "ollie_inward_heelflip", "ollie_kickflip",
    "ollie_popshuvit", "ollie_varial_heelflip", "ollie_varial_kickflip",
];

/// `FrontEndState_TrickGuide::GetGestureString` (`0x82644cf8`).
#[must_use]
pub fn gesture_string(gesture: u32) -> &'static str {
    usize::try_from(gesture).ok().and_then(|i| GESTURES.get(i)).copied().unwrap_or(GESTURES[0])
}

/// One trick guide slot as the screen reads it: gesture, button and
/// connector (`GetConnectorString`: 1 Plus, 2 Arrow, 3 Or).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TrickSlot {
    pub gesture: String,
    pub button: String,
    pub connector: String,
}

/// What the screen reads with a state.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TutorialHudView {
    /// `mTutorialInstructionHALID` (`GetTutorialHalText`).
    pub hal_id: String,
    /// `GetTutorialGesturesButtons`: at most four slots.
    pub slots: Vec<TrickSlot>,
    /// `HUDComponents.PlayerStance_IsRegular`.
    pub stance_regular: bool,
    /// `Tricks.IsStanceTrickGesturesFlipped`.
    pub stance_flipped: bool,
}

/// `TutorialInstructionHUD`.
#[derive(Clone, Debug, Default)]
pub struct TutorialHudComponent {
    /// `mVisible`.
    pub visible: bool,
    /// `mData`: the hash of the HUD group whose data is bound.
    pub bound: Option<u32>,
    /// `mLastStanceGesturesFlipped`.
    last_stance_flipped: bool,
    /// `SetScreenState` calls the screen has not run, with what it reads.
    pub screen_states: Vec<(i32, TutorialHudView)>,
}

fn data(world: &World) -> Option<&super::hud::TutorialInstructionHud> {
    let hash = world.tutorial_hud.bound?;
    world.hud.groups.iter().find(|g| g.hash == hash)?.tutorial.as_ref()
}

fn data_mut(world: &mut World) -> Option<&mut super::hud::TutorialInstructionHud> {
    let hash = world.tutorial_hud.bound?;
    world.hud.groups.iter_mut().find(|g| g.hash == hash)?.tutorial.as_mut()
}

/// `GetTutorialGesturesButtons` (`0x826bae70`): slots until one has neither
/// gesture nor button. The collection is `Attrib::StringToLowerCaseKey`
/// (`0x82976640`: `StringHash64` of the `tolower`ed name).
fn slots(world: &World, instruction: &str) -> Vec<TrickSlot> {
    let collection = super::keys::hash64(&instruction.to_ascii_lowercase());
    let class = if world.db.has_collection(TRICK_CLASS, collection) { TRICK_CLASS } else { TUTORIAL_CLASS };
    collection_slots(&world.db, class, collection)
}

/// Shared native four-slot format, also consumed by FETrickTutorial.
pub fn collection_slots(db: &super::attrib_db::AttribDb, class: u64, collection: u64) -> Vec<TrickSlot> {
    let mut slots = Vec::new();
    for i in 0..4 {
        let entry = db.attribute(class, collection, TRICK_SLOTS).and_then(|a| a.data(i)).and_then(|at| db.bytes(at, 12));
        let word = |n: usize| entry.map_or(0, |b| u32::from_be_bytes([b[n * 4], b[n * 4 + 1], b[n * 4 + 2], b[n * 4 + 3]]));
        let (gesture, button, connector) = (word(0), word(1), word(2));
        if gesture == 0 && button == 0 {
            break;
        }
        let gesture = if matches!(gesture, 47 | 48) { 0 } else { gesture };
        slots.push(TrickSlot {
            gesture: GESTURES.get(gesture as usize).copied().unwrap_or_default().to_owned(),
            button: BUTTONS.get(button as usize).copied().unwrap_or_default().to_owned(),
            connector: match connector {
                1 => "Plus",
                2 => "Arrow",
                3 => "Or",
                _ => "",
            }
            .to_owned(),
        });
    }
    slots
}

/// `FrontEndManager::SetScreenState` on the component's screen.
fn set_screen_state(world: &mut World, state: i32) {
    let (hal_id, instruction) = data(world).map(|d| (d.hal_id.clone(), d.instruction.clone())).unwrap_or_default();
    let view = TutorialHudView {
        hal_id,
        slots: slots(world, &instruction),
        stance_regular: world.stance_regular,
        stance_flipped: world.stance_gestures_flipped,
    };
    world.tutorial_hud.screen_states.push((state, view));
}

/// `HUDManager2::Update` (`0x82cee4f0`) for this component, after the
/// shared `FixupActiveComponents` when the active groups changed.
pub fn update(world: &mut World, active_groups_changed: bool) {
    if active_groups_changed {
        fixup(world);
    }
    let Some(hash) = world.tutorial_hud.bound else { return };
    let shown = world
        .hud
        .groups
        .iter()
        .find(|g| g.hash == hash)
        .and_then(|g| g.instances.iter().find(|i| i.component == COMPONENT))
        .is_some_and(|i| i.shown);
    if world.tutorial_hud.visible != shown {
        show(world, shown);
    }
    component_update(world);
}

/// `FixupActiveComponents` (`0x82cedff0`) for this component.
fn fixup(world: &mut World) {
    let front = if world.hud.hidden_flags == 0 {
        world.hud.active.iter().copied().find(|&hash| world.hud.groups.iter().any(|g| g.hash == hash && g.tutorial.is_some()))
    } else {
        None
    };
    if let Some(hash) = front {
        world.tutorial_hud.bound = Some(hash);
        if let Some(d) = data_mut(world) {
            d.dirty = true;
        }
    } else if world.tutorial_hud.bound.take().is_some() {
        show(world, false);
    }
}

/// `TutorialInstructionHUD::Show` (`0x826be510`).
fn show(world: &mut World, show_it: bool) {
    if world.tutorial_hud.visible != show_it {
        set_screen_state(world, if show_it { screen_state::SHOW } else { screen_state::HIDE });
        world.tutorial_hud.visible = show_it;
    }
}

/// `TutorialInstructionHUD::Update` (`0x826be7d0`).
fn component_update(world: &mut World) {
    if !world.tutorial_hud.visible {
        return;
    }
    if data(world).is_some_and(|d| d.tutorial_dirty) {
        refresh(world);
    }
    let flipped = world.stance_gestures_flipped;
    if flipped != world.tutorial_hud.last_stance_flipped {
        set_screen_state(world, screen_state::STANCE);
    }
    world.tutorial_hud.last_stance_flipped = flipped;
}

/// `RefreshTutorialHUD` (`0x826be578`). `"popshuvit"` is compared twice, so
/// `"tutorial_popshuvit"` is an ordinary instruction.
fn refresh(world: &mut World) {
    let instruction = data(world).map(|d| d.instruction.clone()).unwrap_or_default();
    let state = match instruction.as_str() {
        "clear" | "tutorial_clear" => None,
        "none" | "tutorial_none" => Some(screen_state::NONE),
        "ollie" | "tutorial_ollie" => Some(screen_state::OLLIE),
        "nollie" | "tutorial_nollie" => Some(screen_state::NOLLIE),
        "kickflip" | "tutorial_kickflip" => Some(screen_state::KICKFLIP),
        "popshuvit" => Some(screen_state::POPSHUVIT),
        "360popshuvit" | "tutorial_360popshuvit" => Some(screen_state::POPSHUVIT_360),
        _ => Some(screen_state::INSTRUCTION),
    };
    match state {
        Some(state) => set_screen_state(world, state),
        None => show(world, false),
    }
    if let Some(d) = data_mut(world) {
        d.tutorial_dirty = false;
    }
}
