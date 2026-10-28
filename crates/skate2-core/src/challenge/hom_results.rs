//! FrontEndState_HOMResults82642348/826591e8 and HUDComponents' result natives.
use super::{keys::class, mission::HomBail, system::World};

#[derive(Clone, Debug, PartialEq)]
pub struct CompletedChallenge {
    pub title: (String, String),
    pub cash: i32,
    pub icon: &'static str,
    pub progress: [i32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct HomResultsView {
    pub bail: Option<HomBail>,
    pub top_bail: bool,
    pub challenges: Vec<CompletedChallenge>,
    pub total_cash: i32,
    pub player_cash: i32,
    pub animation_seconds: f32,
    pub points_page: bool,
    pub selection: usize,
}

/// GetChallengeAward825cf6a0: first matching award in the attribute table.
pub(super) fn cash_award(world: &World, signpost: u64) -> i32 {
    use super::signpost_progress::classes;
    let db = &world.db;
    let id = db.layout_word(class::PROGRESSION_CHALLENGES, signpost, 12);
    let cash = db
        .table_order(classes::POINT_POOLS)
        .into_iter()
        .find(|&key| db.layout_text(classes::POINT_POOLS, key, 0) == Some("Cash"))
        .map_or(0, |key| db.layout_word(classes::POINT_POOLS, key, 1));
    db.table_order(classes::AWARDS)
        .into_iter()
        .find(|&key| {
            db.layout_word(classes::AWARDS, key, 1) == id
                && db.layout_word(classes::AWARDS, key, 0) == cash
        })
        .map_or(0, |key| {
            db.layout_word(classes::AWARDS, key, 2).cast_signed()
        })
}

/// HOM_GetChallengesIconDamage826b7108 / GetChallengesProgress826b7348.
fn category(world: &World, signpost: u64) -> (&'static str, [i32; 2]) {
    for (id, icon) in [
        (70, "points"),
        (69, "damage"),
        (68, "style"),
        (67, "wildcard"),
    ] {
        let key = super::stuff_menu::list_key(world, id);
        let contains = world
            .db
            .attribute(class::PROGRESSION_LISTS, key, 0xec1c_213b_bdbf_1a68)
            .is_some_and(|a| {
                (0..a.len()).any(|i| a.ref_spec(i).is_some_and(|(_, k)| k == signpost))
            });
        if contains {
            let count = super::stuff_menu::challenges_in_list(world, id)
                .into_iter()
                .filter_map(|k| super::signpost::signpost_key_for_challenge(world, k))
                .filter(|&k| world.signpost.is_challenge_complete(k))
                .count() as i32;
            return (icon, [count, 15]);
        }
    }
    ("none", [0, 0])
}

pub(crate) fn view(world: &World, animation_seconds: f32) -> HomResultsView {
    let stats = world.hom_stats.lock().expect("HoM results stats lock");
    let challenges = world
        .hom
        .completed
        .iter()
        .take(4)
        .map(|&activity| {
            let challenge = world.activities.activities[activity].key;
            let signpost =
                super::signpost::signpost_key_for_challenge(world, challenge).unwrap_or(0);
            let title = world
                .db
                .attribute(
                    class::CHALLENGES,
                    challenge,
                    super::signup_poster::challenge_field::OBJECTIVES,
                )
                .and_then(|a| a.ref_spec(0))
                .map(|(c, k)| super::objectives::title(&world.db, c, k))
                .unwrap_or_default();
            let (icon, progress) = category(world, signpost);
            CompletedChallenge {
                title,
                cash: cash_award(world, signpost),
                icon,
                progress,
            }
        })
        .collect::<Vec<_>>();
    let total_cash = world
        .hom
        .completed
        .iter()
        .map(|&a| {
            super::signpost::signpost_key_for_challenge(world, world.activities.activities[a].key)
                .map_or(0, |k| cash_award(world, k))
        })
        .fold(0i32, i32::wrapping_add);
    HomResultsView {
        bail: stats.last,
        top_bail: stats.last_top_bail,
        points_page: challenges.is_empty(),
        challenges,
        total_cash,
        player_cash: super::signpost_progress::cash(world),
        animation_seconds,
        selection: 0,
    }
}

pub(crate) fn continue_results(world: &mut World) {
    let name = if world.signpost.message_setting_value("FMV_HOM").unwrap_or(1) != 0 {
        "HOM_Give_Reward"
    } else {
        "END_HOM_TUTORIAL"
    };
    let key = world
        .db
        .table_order(class::PROGRESSION_STATE_MACHINES)
        .into_iter()
        .find(|&key| {
            world
                .db
                .layout_text(class::PROGRESSION_STATE_MACHINES, key, 0)
                == Some(name)
        });
    if let Some(key) = key {
        super::signpost::queue_triggered_state_machine(world, key);
    }
}
