//! FrontEndState_AnnalsOfMeat77: OnEnter826ae5a0 and queries82693b90..826941a8.
use super::{hom::HomStats, keys::class, system::World};

#[derive(Clone, Debug, PartialEq)]
pub struct ChallengeRow {
    pub description: (String, String),
    pub state: &'static str,
    pub cash: Option<i32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ChallengePath {
    pub current: (String, String),
    pub completed: usize,
    pub cash: f32,
    pub rows: Vec<ChallengeRow>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AnnalsView {
    pub stats: HomStats,
    pub paths: [ChallengePath; 4],
    pub tab: usize,
    pub bail_index: usize,
    pub path_index: usize,
}
fn unknown() -> (String, String) {
    ("ID_ANNALSOFMEAT_UNKNOWN_DESC".into(), String::new())
}
fn description(world: &World, challenge: u64) -> (String, String) {
    world
        .db
        .attribute(
            class::CHALLENGES,
            challenge,
            super::signup_poster::challenge_field::OBJECTIVES,
        )
        .and_then(|a| a.ref_spec(0))
        .map(|(c, k)| super::objectives::title(&world.db, c, k))
        .unwrap_or_else(unknown)
}
fn path(world: &World, list: i32) -> ChallengePath {
    let mut entries = super::stuff_menu::challenges_in_list(world, list)
        .into_iter()
        .filter_map(|challenge| {
            super::signpost::signpost_key_for_challenge(world, challenge).map(|s| (challenge, s))
        })
        .filter(|&(_, s)| {
            world.signpost.is_challenge_complete(s) || world.signpost.is_challenge_active(s)
        })
        .collect::<Vec<_>>();
    // GetAllChallengesInListSorted825d6280: completed in ascending completion
    // order (authored order breaks ties), followed by active entries in authored order.
    entries.sort_by_key(|&(_, s)| {
        world
            .signpost
            .completed
            .get(&s)
            .map_or((1, 0), |c| (0, c.order))
    });
    let current = match entries.len() {
        0 => unknown(),
        16..=usize::MAX => (
            "ID_ANNALSOFMEAT_ALL_CHALLENGES_COMPLETE".into(),
            String::new(),
        ),
        _ => description(world, entries.last().unwrap().0),
    };
    let mut cash = 0.;
    for &(_, s) in &entries {
        if world.signpost.is_challenge_complete(s) {
            cash += super::hom_results::cash_award(world, s) as f32;
        }
    }
    let rows = (0..15)
        .map(|i| {
            if let Some(&(c, s)) = entries.get(i) {
                let complete = world.signpost.is_challenge_complete(s);
                ChallengeRow {
                    description: description(world, c),
                    state: if complete { "done" } else { "active" },
                    cash: complete.then(|| super::hom_results::cash_award(world, s)),
                }
            } else {
                ChallengeRow {
                    description: unknown(),
                    state: "undone",
                    cash: None,
                }
            }
        })
        .collect();
    ChallengePath {
        current,
        completed: entries.len(),
        cash,
        rows,
    }
}
pub(crate) fn view(world: &World) -> AnnalsView {
    let stats = world.hom_stats.lock().expect("Annals stats lock").clone();
    AnnalsView {
        tab: usize::from(stats.top.is_empty()),
        stats,
        paths: [70, 69, 68, 67].map(|list| path(world, list)),
        bail_index: 0,
        path_index: 0,
    }
}
