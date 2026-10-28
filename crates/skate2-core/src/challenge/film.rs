//! cFilmManager82da9c48: authored AnywhereFilm activities and Contacts lists.
use super::{activity, keys::class, signpost, signup, system::World};

#[derive(Debug, Default)]
pub struct FilmManager {
    all: Vec<usize>,
    pub available: Vec<usize>,
    pub completed: Vec<usize>,
    /// UpdateState82da35b0 has no HUD side effect in this executable.
    pub overlay_active: bool,
}

/// AddActivity82daf210, called in the original activity creation order.
pub(crate) fn init(world: &mut World) {
    world.films.all = world.activities.activities.iter().enumerate().filter_map(|(i,a)| {
        let global = |key| a.global_data.and_then(|(c,k)|world.db.attribute(c,k,key));
        (!global(0xfd40_e4a3_1027_c984).and_then(|a|a.bool(0)).unwrap_or(false)
            && signup::challenge_type(world,i)==39).then_some(i)
    }).collect();
}

/// ReevaluateActivityPrereqs82db0f88 / EvaluateActivitiesForPass82cda098.
pub(crate) fn reevaluate(world: &mut World) {
    let all = &world.films.all;
    let available = signup::evaluate_for_prereqs(world, all, world.game_mode_flags, "");
    let completed = all.iter().copied().filter(|&i| {
        world.enable_progression && signpost::signpost_key_for_challenge(world, world.activities.activities[i].key)
            .is_some_and(|k|world.signpost.is_challenge_complete(k))
    }).collect();
    world.films.available = available;
    world.films.completed = completed;
}

/// CanStart[Completed]FilmChallenge82da33e0/3410: mode and nonempty list only.
pub fn available(world: &World, completed: bool) -> bool {
    world.game_mode_flags == 4 && !list(world,completed).is_empty()
}
pub fn list(world: &World, completed: bool) -> &[usize] {
    if completed { &world.films.completed } else { &world.films.available }
}
/// Get[Completed]FilmChallengeInfo82dac090/c200.
pub fn info(world: &World, activity: usize) -> (String,String) {
    let key=world.activities.activities[activity].key;
    let text=|field|world.db.attribute(class::CHALLENGES,key,field)
        .and_then(|a|a.text(0)).unwrap_or_default().to_owned();
    (text(super::signup_poster::challenge_field::TITLE),
     text(super::signup_poster::challenge_field::SIGN_UP_DESCRIPTION))
}
/// Start[Completed]FilmChallenge82da3470/3510 bypasses the sign-up poster.
pub fn start(world: &mut World, activity: usize) {
    if activity::can_start_main_challenge(world) {
        activity::start_main_challenge(world,world.activities.activities[activity].key);
    }
}
pub(crate) fn update(world: &mut World) {
    if activity::can_start_main_challenge(world) && world.game_mode_flags & 8 != 8
        && world.hud.group_exists("main") {
        world.films.overlay_active = available(world,false);
    }
}
