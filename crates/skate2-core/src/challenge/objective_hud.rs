//! `Sk8::HUD::ObjectiveHUD`, the `objective` HUD component. The screen
//! `hud2/hudobjective2` reads it through `HUDComponentsCmdTranslator`
//! (`sMethod_Objective_*`), and the component drives the screen with
//! `FrontEndManager::SetScreenState` (1 intro, 2 outro, 3 the objective list,
//! 4 + row one objective's result, 50 scoring). `SetScreenState` runs the
//! screen's `SetState` at once (`ScreenManagerBase::CallASFunction`), so each
//! state carries what the screen reads at that moment.
//! `HUDManager2::Update` (`0x82cee4f0`) binds the component to the front active
//! group that has it and keeps its visibility in step with the group's
//! instance.

use super::{
    hud::ObjectiveHud,
    keys::class,
    objectives::{self, Status, field, result_style},
    system::World,
};

/// The component's name (`kHUDName`).
pub const COMPONENT: &str = "objective";

/// `challenges` `Title`, copied into `mTitle` (`0xe2a50788b6c20c92`).
pub(crate) const CHALLENGE_TITLE: u64 = 0xe2a5_0788_b6c2_0c92;

/// `eObjectiveState`.
pub mod objective_state {
    pub const FAILED: i32 = 0;
    pub const COMPLETE: i32 = 1;
    pub const INCOMPLETE: i32 = 2;
    pub const ACTIVE: i32 = 3;
    pub const DISABLED: i32 = 4;
    /// The constructor's value before any refresh.
    pub const NONE: i32 = 5;
}

/// The `SetScreenState` values the component sets.
pub mod screen_state {
    pub const INTRO: i32 = 1;
    pub const OUTRO: i32 = 2;
    pub const OBJECTIVES: i32 = 3;
    /// Plus the objective's row.
    pub const RESULT: i32 = 4;
    pub const SCORING: i32 = 50;
}

/// `ObjectiveHUD::MapObjectiveState` (`0x826b2bd8`).
fn map_state(status: Status) -> i32 {
    match status {
        Status::Disabled => objective_state::DISABLED,
        Status::Enabled => objective_state::INCOMPLETE,
        Status::Active => objective_state::ACTIVE,
        Status::Complete => objective_state::COMPLETE,
        Status::Failed => objective_state::FAILED,
    }
}

/// One listed objective: `mState`, `mType`, `mObjectiveTitle`, `mValue`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveRow {
    pub state: i32,
    pub result_style: i32,
    /// The objective's `mTitle`: HAL key and format arguments.
    pub title: (String, String),
    /// `"#%d/%d"`: the fraction's numerator and denominator.
    pub value: String,
}

impl Default for ObjectiveRow {
    /// The constructor (`0x826b5018`).
    fn default() -> Self {
        Self { state: objective_state::NONE, result_style: result_style::CHECK_BOX, title: Default::default(), value: String::new() }
    }
}

/// What the screen's `HUDComponents.Objective_*` calls read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "native flags")]
pub struct ObjectiveHudView {
    /// `mTitle`: the challenge's `Title` HAL key.
    pub title: String,
    pub num_objectives: i32,
    pub rows: [ObjectiveRow; 4],
    pub is_modified_position: bool,
    /// The bound data's `mVariantHalId` and visibility flags.
    pub variant_name: String,
    pub show_activity_type: bool,
    pub show_activity_name: bool,
    pub show_curr_and_req: bool,
}

impl Default for ObjectiveHudView {
    fn default() -> Self {
        let data = ObjectiveHud::default();
        Self {
            title: String::new(),
            num_objectives: 0,
            rows: Default::default(),
            is_modified_position: false,
            variant_name: data.variant_hal_id,
            show_activity_type: data.show_activity_type,
            show_activity_name: data.show_activity_name,
            show_curr_and_req: data.show_curr_and_req,
        }
    }
}

/// `ObjectiveHUD` (`0x2f0` bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveHudComponent {
    /// `mVisible`.
    pub visible: bool,
    pub score: u32,
    pub required_score: u32,
    pub score_enabled: bool,
    /// `mIsFrozen`.
    pub frozen: bool,
    pub view: ObjectiveHudView,
    /// `mData`: the hash of the HUD group whose data is bound.
    pub bound: Option<u32>,
    /// `SetScreenState` calls the screen has not run, with what it reads.
    pub screen_states: Vec<(i32, ObjectiveHudView)>,
}

impl Default for ObjectiveHudComponent {
    fn default() -> Self {
        Self {
            visible: false,
            score: 0,
            required_score: 0,
            score_enabled: true,
            frozen: false,
            view: ObjectiveHudView::default(),
            bound: None,
            screen_states: Vec::new(),
        }
    }
}

fn data(world: &World) -> Option<&ObjectiveHud> {
    let hash = world.objective_hud.bound?;
    world.hud.groups.iter().find(|g| g.hash == hash)?.objective.as_ref()
}

fn data_mut(world: &mut World) -> Option<&mut ObjectiveHud> {
    let hash = world.objective_hud.bound?;
    world.hud.groups.iter_mut().find(|g| g.hash == hash)?.objective.as_mut()
}

/// `FrontEndManager::SetScreenState` on the component's screen.
fn set_screen_state(world: &mut World, state: i32) {
    let mut view = world.objective_hud.view.clone();
    if let Some(d) = data(world) {
        view.variant_name.clone_from(&d.variant_hal_id);
        view.show_activity_type = d.show_activity_type;
        view.show_activity_name = d.show_activity_name;
        view.show_curr_and_req = d.show_curr_and_req;
    }
    world.objective_hud.screen_states.push((state, view));
}

/// `HUDManager2::Update` (`0x82cee4f0`) for this component, after the
/// shared `FixupActiveComponents` when the active groups changed.
pub fn update(world: &mut World, active_groups_changed: bool) {
    if active_groups_changed {
        fixup(world);
    }
    let Some(hash) = world.objective_hud.bound else { return };
    let shown = world
        .hud
        .groups
        .iter()
        .find(|g| g.hash == hash)
        .and_then(|g| g.instances.iter().find(|i| i.component == COMPONENT))
        .is_some_and(|i| i.shown);
    if world.objective_hud.visible != shown {
        show(world, shown);
    }
    component_update(world);
}

/// `FixupActiveComponents` (`0x82cedff0`) for this component: while no
/// visibility flag hides the HUD it is bound to the front active group that
/// has it (`BindData`, then the data's `MakeDirty`); otherwise it is unbound
/// and hidden.
fn fixup(world: &mut World) {
    let front = if world.hud.hidden_flags == 0 {
        world.hud.active.iter().copied().find(|&hash| world.hud.groups.iter().any(|g| g.hash == hash && g.objective.is_some()))
    } else {
        None
    };
    if let Some(hash) = front {
        world.objective_hud.bound = Some(hash);
        if let Some(d) = data_mut(world) {
            d.dirty = true;
        }
    } else if world.objective_hud.bound.take().is_some() {
        show(world, false);
    }
}

/// `ObjectiveHUD::Show` (`0x826bc780`, shared with `OnlineOTSStatHUD`).
fn show(world: &mut World, show_it: bool) {
    if show_it {
        refresh(world);
    }
    if world.objective_hud.visible != show_it {
        set_screen_state(world, if show_it { screen_state::INTRO } else { screen_state::OUTRO });
        world.objective_hud.visible = show_it;
    }
}

/// `ObjectiveHUD::Refresh` (`0x826bc800`).
fn refresh(world: &mut World) {
    let Some(d) = data(world).cloned() else { return };
    // ObjectiveHUDData::RefreshObjectives 0x82d829c0.
    if let Some(group) = d.group {
        let listed = objectives::displayed_objectives(world, group, 4);
        if let Some(data) = data_mut(world) {
            data.listed = listed;
        }
    }
    let Some(d) = data(world).cloned() else { return };
    let title = world.db.attribute(class::CHALLENGES, d.challenge, CHALLENGE_TITLE).and_then(|a| a.text(0)).unwrap_or_default().to_owned();
    let c = &mut world.objective_hud;
    c.view.title = title;
    c.score_enabled = d.enable_score;
    c.score = d.score;
    c.view.num_objectives = i32::try_from(d.listed.len()).unwrap_or(4);
    c.required_score = d.score_required;
    if d.listed.is_empty() {
        show(world, false);
    }
    let gi = d.group.and_then(|g| world.objectives.group_index(g));
    for (row, &index) in d.listed.iter().enumerate() {
        let Some(o) = gi.and_then(|gi| world.objectives.groups[gi].objectives.get(index)) else { continue };
        let (style, numerator, status, key, title) = (o.result_style, o.fraction_numerator, o.state, o.key, o.title.clone());
        let denominator = world.db.attribute(class::CHALLENGE_OBJECTIVE, key, field::FRACTION_DENOMINATOR).and_then(|a| a.i16(0)).unwrap_or(0);
        let state = map_state(status);
        let r = &mut world.objective_hud.view.rows[row];
        r.result_style = style;
        r.value = format!("#{numerator}/{denominator}");
        if r.state != state {
            r.state = state;
            set_screen_state(world, screen_state::RESULT + i32::try_from(row).unwrap_or(0));
        }
        world.objective_hud.view.rows[row].title = title;
    }
    world.objective_hud.view.is_modified_position = d.is_modified_position;
    set_screen_state(world, screen_state::OBJECTIVES);
}

/// `ObjectiveHUD::RefreshScoringDisplay` (`0x826bcb78`).
fn refresh_scoring(world: &mut World) {
    if let Some(d) = data(world) {
        let (score, required) = (d.score, d.score_required);
        world.objective_hud.score = score;
        world.objective_hud.required_score = required;
    }
    set_screen_state(world, screen_state::SCORING);
}

/// `ObjectiveHUD::Update` (`0x826b2a88`).
fn component_update(world: &mut World) {
    let Some(group) = data(world).and_then(|d| d.group) else { return };
    let points = objectives::accumulated_points(world, group);
    let required = world.objectives.group_index(group).map_or(0, |gi| world.objectives.groups[gi].points_requirement);
    if let Some(d) = data_mut(world) {
        if points != d.score {
            d.score = points;
            d.scoring_dirty = true;
        }
        if required != d.score_required {
            d.score_required = required;
            d.scoring_dirty = true;
        }
    }
    if !world.objective_hud.frozen {
        // ObjectiveHUDData::IsDirty 0x82d82960 with a group.
        if objectives::any_dirty(world, group) {
            refresh(world);
            // ResetDirty 0x82d82980.
            objectives::reset_dirty(world, group);
            if let Some(d) = data_mut(world) {
                d.dirty = false;
            }
        }
        if data(world).is_some_and(|d| d.scoring_dirty) {
            refresh_scoring(world);
            if let Some(d) = data_mut(world) {
                d.scoring_dirty = false;
            }
        }
    }
    world.objective_hud.frozen = data(world).is_some_and(|d| d.is_frozen);
}
