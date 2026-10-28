//! `Sk8::HUD::SignUpHUD`, the `signup` HUD component of the sign-up
//! manager's `cChallengeSignUpManager-DPadUp` group, driven by its
//! `SignUpData` (the manager's prompt: `world.signup.prompt` and its dirty
//! flag). The screen is hud2/actionoverlay2 (template entry `0x82fdb8f8`,
//! placement 0); `SetScreenState` runs its `SetState` at once, so each state
//! carries what the screen reads then.

use super::system::{Output, World};

/// `SignUpHUD::Show` (`0x826c0470`): frames before a sign-up press is taken.
const SIGN_UP_DELAY: i32 = 15;

/// `SignUpData::kHUDName`.
pub const COMPONENT: &str = "signup";
/// `cChallengeSignUpManager::kSignUpDPadUpGroupName`.
pub const GROUP: &str = "cChallengeSignUpManager-DPadUp";
pub const PROP_GROUP: &str = "UpdateObjectGrabbing";

/// PhysOutModule_Hud::UpdateHUDsAndObjectGrabbing827c6ee0.
pub fn update_prop(world:&mut World,prop:Option<crate::ui::phonelist::PropHudSnapshot>,phone_hidden:bool) {
    let was=world.signup_hud.prop.is_some();
    world.signup_hud.prop=prop;
    if prop.is_some() {
        if world.hud.group(PROP_GROUP).is_none() {world.hud.create_group(PROP_GROUP,&[COMPONENT]);}
        world.hud.add_group_to_active(PROP_GROUP);
        world.hud.show(PROP_GROUP,COMPONENT,true);
    } else if was {
        world.hud.show(PROP_GROUP,COMPONENT,false);
        world.hud.remove_group_from_active(PROP_GROUP);
    }
    world.hud.show("phonelist","phonelist",!phone_hidden);
}

/// `SignUpHUD::eSignUpSetStates`, the overlay's `kActionOverlayState_*`.
pub mod screen_state {
    pub const HIDE: i32 = 0;
    pub const SHOW: i32 = 1;
    pub const TELEPORT: i32 = 2;
    pub const DMO_GRAB: i32 = 4;
    pub const OTS: i32 = 5;
}

/// `challenges` `Title` (`0xe2a50788b6c20c92`).
const CHALLENGE_TITLE: u64 = 0xe2a5_0788_b6c2_0c92;

/// `eChallengeTypes` `OwnTheSpot` and `CreateASpot`.
const OWN_THE_SPOT: i32 = 9;
const CREATE_A_SPOT: i32 = 35;

/// `mChallengeNameBuf` as `SignUp_GetChallengeTitle` (`0x826b8210`) hands it
/// to the screen.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SignUpHudView {
    /// One challenge's `Title` HAL key, which `ShowChallengeTitle` formats
    /// (`FormatStringCustom` with no arguments, a literal for the screen),
    /// or `ID_SIGNUP_MULTIPLIESIGNUP_SIGNUPHUD`, which the screen looks up.
    pub title: String,
    /// The title is a formatted challenge title.
    pub formatted: bool,
    /// DMO_GetParams, read by the same actionoverlay2 screen in prop mode.
    pub dmo: Option<[bool; 5]>,
}

/// `SignUpHUD` (ctor `0x826b5228`).
#[derive(Clone, Debug, Default)]
pub struct SignUpHudComponent {
    /// `mVisible`.
    pub visible: bool,
    /// `mData` is bound (the group is the front active one with the
    /// component).
    pub bound: bool,
    /// The bound data is a teleporter's (`HUD2.SetupTeleportHUD`, `eSignUpType_Teleport`): its
    /// group.
    pub teleport: Option<String>,
    pub prop: Option<crate::ui::phonelist::PropHudSnapshot>,
    pub prop_bound: bool,
    /// `mDelaySignUp` (+0x74) and `mIsDpadUp` (+0x78) belong to the shared
    /// component, including when a visibility change unbinds it.
    delay: i32,
    dpad_held: bool,
    /// `mSignupState`.
    state: i32,
    /// `mChallengeNameBuf`: kept until `ShowChallengeTitle` writes it again.
    title: SignUpHudView,
    /// `SetScreenState` calls the screen has not run, with what it reads.
    pub screen_states: Vec<(i32, SignUpHudView)>,
}

/// `HUDManager2::Update` (`0x82cee4f0`) for this component, after the
/// shared `FixupActiveComponents` when the active groups changed.
pub fn update(world: &mut World, active_groups_changed: bool) {
    if active_groups_changed {
        fixup(world);
    }
    if !world.signup_hud.bound {
        return;
    }
    let group = if world.signup_hud.prop_bound { PROP_GROUP.to_owned() } else { world.signup_hud.teleport.clone().unwrap_or_else(|| GROUP.to_owned()) };
    let shown = world.hud.group(&group).and_then(|g| g.instances.iter().find(|i| i.component == COMPONENT)).is_some_and(|i| i.shown);
    if world.signup_hud.visible != shown {
        show(world, shown);
    }
    // SignUpHUD::Update 0x826bd3e8: refresh first, then count down while
    // D-pad up is not held. Only a bound component receives this update.
    if data_dirty(world) {
        refresh(world);
    }
    if !world.signup_hud.dpad_held {
        world.signup_hud.delay = world.signup_hud.delay.wrapping_sub(1);
    }
}

/// The bound data's `IsDirty`.
fn data_dirty(world: &World) -> bool {
    if world.signup_hud.prop_bound { return true; }
    match &world.signup_hud.teleport {
        Some(group) => world.teleporters.hud(group).is_some_and(|hud| hud.dirty),
        None => world.signup.prompt_dirty,
    }
}

/// `FixupActiveComponents` (`0x82cedff0`): bound (`BindData`, then the
/// data's `MakeDirty`) while no visibility flag hides the HUD and the
/// sign-up group or a teleporter's group is the front active group with the
/// component; otherwise unbound and `Show(false)`.
fn fixup(world: &mut World) {
    let front = if world.hud.hidden_flags == 0 {
        world
            .hud
            .active
            .iter()
            .find_map(|&hash| world.hud.groups.iter().find(|g| g.hash == hash && g.instances.iter().any(|i| i.component == COMPONENT)))
            .map(|g| g.name.clone())
    } else {
        None
    };
    world.signup_hud.prop_bound = front.as_deref() == Some(PROP_GROUP);
    match front {
        Some(name) if name == PROP_GROUP => {
            world.signup_hud.bound = true;
            world.signup_hud.teleport = None;
        }
        Some(name) if name == GROUP => {
            world.signup_hud.bound = true;
            world.signup_hud.teleport = None;
            world.signup.prompt_dirty = true;
        }
        Some(name) if world.teleporters.hud(&name).is_some() => {
            if let Some(hud) = world.teleporters.hud_mut(&name) {
                hud.dirty = true;
            }
            world.signup_hud.bound = true;
            world.signup_hud.teleport = Some(name);
        }
        _ => {
            world.signup_hud.teleport = None;
            if std::mem::take(&mut world.signup_hud.bound) {
                show(world, false);
            }
        }
    }
}

/// `SignUpHUD::Show` (`0x826c0430`): shown, refresh the screen, set its
/// state again and arm the press delay; hidden, clear the delay/held latch
/// and set state 0. This also covers HUD suppression and direct popup calls.
pub(crate) fn show(world: &mut World, show_it: bool) {
    if show_it {
        world.signup_hud.visible = true;
        refresh(world);
        let state = world.signup_hud.state;
        set_screen_state(world, state);
        world.signup_hud.delay = SIGN_UP_DELAY;
    } else {
        world.signup_hud.visible = false;
        world.signup_hud.delay = 0;
        world.signup_hud.dpad_held = false;
        set_screen_state(world, screen_state::HIDE);
    }
}

/// `UpdateComponentInput` (`0x82cebc38`) only dispatches to a bound,
/// visible component. The props adapter handles the DMO input separately.
/// The return value tells the host whether the several-challenges popup paused it.
pub(crate) fn dpad_up(world: &mut World) -> bool {
    if !world.signup_hud.bound || !world.signup_hud.visible || world.signup_hud.prop_bound {
        return false;
    }
    // `UpdateDPadUp` (`0x826bd450`): teleporters bypass the sign-up delay.
    if let Some(group) = world.signup_hud.teleport.clone() {
        super::teleporter::sign_up(world, &group);
        return false;
    }
    if world.signup_hud.delay > 0 {
        if world.signup_hud.dpad_held {
            world.signup_hud.delay = SIGN_UP_DELAY;
        }
        world.signup_hud.dpad_held = true;
        return false;
    }
    match world.signup.prompt.len() {
        0 => false,
        1 => {
            super::signup::player_trigger_sign_up(world, world.signup.prompt[0], false);
            false
        }
        _ => {
            let options = world.signup.prompt.iter().take(4).map(|&a| world.activities.activities[a].name.clone()).collect();
            world.signup.prompt_visible = false;
            world.output(Output::SignUpChoice { options });
            // The component's own Show(false); its instance stays shown.
            show(world, false);
            true
        }
    }
}

/// `SignUpHUD::UpdateInput(0x4000)` (`0x826bd938`), after the same dispatch gate.
pub(crate) fn dpad_released(world: &mut World) {
    if world.signup_hud.bound && world.signup_hud.visible {
        world.signup_hud.delay = 0;
        world.signup_hud.dpad_held = false;
    }
}

fn set_screen_state(world: &mut World, state: i32) {
    let view = world.signup_hud.title.clone();
    world.signup_hud.screen_states.push((state, view));
}

/// `SignUpHUD::RefreshFEScreen` (`0x826bd2d0`) for the manager's
/// `eSignUpType_SignUp` data: hidden without challenges, Own the Spot for a
/// single Own the Spot challenge, else shown; the title is rewritten and a
/// dirty data sets the screen state.
fn refresh(world: &mut World) {
    if !world.signup_hud.bound {
        return;
    }
    if world.signup_hud.prop_bound {
        world.signup_hud.state = screen_state::DMO_GRAB;
        world.signup_hud.title.dmo = world.signup_hud.prop.map(|p| p.params);
        set_screen_state(world, screen_state::DMO_GRAB);
        return;
    }
    world.signup_hud.title.dmo = None;
    if let Some(group) = world.signup_hud.teleport.clone() {
        // eSignUpType_Teleport: the teleport state and its one challenge's title.
        world.signup_hud.state = screen_state::TELEPORT;
        if let Some(activity) = world.teleporters.hud(&group).map(|hud| hud.activity) {
            show_challenge_title(world, &[activity]);
        }
        if world.teleporters.hud_mut(&group).is_some_and(|hud| std::mem::take(&mut hud.dirty)) {
            set_screen_state(world, screen_state::TELEPORT);
        }
        return;
    }
    let state = match world.signup.prompt.as_slice() {
        [] => screen_state::HIDE,
        [single] if challenge_type(world, *single) == OWN_THE_SPOT => screen_state::OTS,
        _ => screen_state::SHOW,
    };
    world.signup_hud.state = state;
    let prompt = world.signup.prompt.clone();
    show_challenge_title(world, &prompt);
    if world.signup.prompt_dirty {
        let state = world.signup_hud.state;
        set_screen_state(world, state);
    }
    world.signup.prompt_dirty = false;
}

/// `SignUpHUD::ShowChallengeTitle` (`0x826bd1f0`): one challenge's `Title`
/// (`FormatStringCustom` with no arguments), several challenges'
/// `ID_SIGNUP_MULTIPLIESIGNUP_SIGNUPHUD`; none leaves the buffer. A
/// Create-a-Spot challenge's name comes from `CreateASpotDataManager`,
/// which the career does not reach.
fn show_challenge_title(world: &mut World, challenges: &[usize]) {
    let title = match challenges {
        [] => return,
        [single] => {
            if challenge_type(world, *single) == CREATE_A_SPOT {
                return;
            }
            let key = world.activities.activities[*single].key;
            let title = world
                .db
                .attribute(super::keys::class::CHALLENGES, key, CHALLENGE_TITLE)
                .and_then(|a| a.text(0))
                .unwrap_or_default()
                .to_owned();
            SignUpHudView { title, formatted: true, dmo: None }
        }
        _ => SignUpHudView { title: "ID_SIGNUP_MULTIPLIESIGNUP_SIGNUPHUD".into(), formatted: false, dmo: None },
    };
    world.signup_hud.title = title;
}

fn challenge_type(world: &World, activity: usize) -> i32 {
    super::signup::challenge_type(world, activity)
}
