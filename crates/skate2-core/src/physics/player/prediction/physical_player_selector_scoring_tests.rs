#![expect(clippy::float_cmp, reason = "exact fixture publications")]
use super::super::selector_update::tests::{Source, owner};
use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
#[test]
fn unordered_transition_and_apex_threshold_choose_positive_without_penalty() {
    let mut s = owner();
    s.num_trajectories = 1;
    s.on_vert = true;
    s.queries[0].result.ground_normal = V::known([1.0, 0.0, 0.0, 0.0]);
    let mut src = Source {
        scalar_nan: Some(0x1ec),
        ..Source::default()
    };
    calculate_scores(
        &mut s,
        &mut src,
        &mut HostReckoningNumeric,
        SelectorGrindPolicy::ParkedForFirstSkatingTest,
    )
    .unwrap();
    assert_eq!(s.scores[0], 3.0);
}
#[test]
fn native_negative_penalty_still_probes_the_wall() {
    let mut s = owner();
    s.num_trajectories = 1;
    s.pass = 2;
    let mut src = Source {
        penalty: -3.0,
        ..Source::default()
    };
    calculate_scores(
        &mut s,
        &mut src,
        &mut HostReckoningNumeric,
        SelectorGrindPolicy::Native,
    )
    .unwrap();
    assert_eq!(s.scores[0], -2.0);
    let order: Vec<&str> = src
        .calls
        .iter()
        .map(String::as_str)
        .filter(|x| x.starts_with("grind") || x.starts_with("avoid") || x.starts_with("feet"))
        .collect();
    assert_eq!(order[..2], ["avoidGrinds", "grindPenalty"]);
    assert!(order[2].starts_with("feet"));
}
#[test]
fn reversing_on_vert_forfeits_the_grind_bonus() {
    let mut s = owner();
    s.num_trajectories = 1;
    s.pass = 1;
    s.on_vert = true;
    let mut src = Source {
        grind: 5.0,
        transition: 0.0,
        ..Source::default()
    };
    calculate_scores(
        &mut s,
        &mut src,
        &mut HostReckoningNumeric,
        SelectorGrindPolicy::Native,
    )
    .unwrap();
    assert!(!s.grind_bonus_awarded);
    assert!(src.calls.iter().any(|x| x == "graphb0"));
    assert!(!src.calls.iter().any(|x| x.starts_with("feet")));
    assert_eq!(s.scores[0], 1.0);
}
#[test]
fn invalid_fourth_time_lane_skips_all_candidate_reads() {
    let mut s = owner();
    s.num_trajectories = 1;
    s.queries[0].result.time[3] = f32::NAN;
    let mut src = Source::default();
    calculate_scores(
        &mut s,
        &mut src,
        &mut HostReckoningNumeric,
        SelectorGrindPolicy::ParkedForFirstSkatingTest,
    )
    .unwrap();
    assert_eq!(s.scores[0], 0.0);
    assert!(!src.calls.iter().any(|x| x == "central"));
}
#[test]
fn high_nonwall_candidate_still_gets_two_thousand() {
    let mut s = owner();
    s.num_trajectories = 1;
    s.queries[0].result.trajectory.position.xyz[1] = 3.0;
    add_wall_scores::<_, ()>(&mut s, &mut Source::default()).unwrap();
    assert_eq!(s.wallride_scores[0], 2000.0);
    assert_eq!(s.scores[0], 2000.0);
    assert!(!s.is_wallride[0]);
}
#[test]
fn surface_failed_scalar_does_not_publish_up() {
    let mut s = owner();
    s.num_trajectories = 1;
    s.queries[0].result.surface = 6 << 7;
    let mut src = Source {
        fail_scalar: Some(0x1cc),
        ..Source::default()
    };
    assert!(
        calculate_scores(
            &mut s,
            &mut src,
            &mut HostReckoningNumeric,
            SelectorGrindPolicy::ParkedForFirstSkatingTest
        )
        .is_err()
    );
    assert_eq!(s.queries[0].result.normal, UP);
    let tail = &src.calls[src.calls.len() - 2..];
    assert_eq!(tail, ["up", "scalar1cc"]);
}
#[test]
fn wall_probe_uses_stored_position_and_nan_slope_accepts() {
    let mut s = owner();
    s.num_trajectories = 1;
    s.queries[0].result.normal = V::known([1.0, 0.0, 0.0, 0.0]);
    s.queries[0].result.position.xyz[1] = 7.0;
    let mut src = Source {
        scalar_nan: Some(0x1a8),
        hit: Some((V::known([0.0, 2.0, 0.0, 0.0]), UP)),
        ..Source::default()
    };
    assert_eq!(
        wall_score(&mut s, 0, &mut src, &mut HostReckoningNumeric, false).unwrap(),
        10.0
    );
    assert!(src.calls.iter().any(|x| x == "lineY7"));
    assert!(src.calls.iter().any(|x| x == "graph180"));
}
#[test]
fn nan_minimum_height_still_reaches_boost() {
    let mut s = owner();
    s.queries[0].result.normal = V::known([1.0, 0.0, 0.0, 0.0]);
    let mut src = Source {
        scalar_nan: Some(0x1a4),
        hit: Some((V::known([0.0, 2.0, 0.0, 0.0]), UP)),
        ..Source::default()
    };
    assert_eq!(
        wall_score(&mut s, 0, &mut src, &mut HostReckoningNumeric, false).unwrap(),
        4.0
    );
    assert!(src.calls.iter().any(|x| x == "graph180"));
}
