//! Actual first-spawn prefix proof, separate from scoped component tick coverage.
use super::*;
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_startup_retains_actor_rosters_before_pending_observers() {
    with_original_inputs(ground_startup::check_actual_roster_startup);
}
