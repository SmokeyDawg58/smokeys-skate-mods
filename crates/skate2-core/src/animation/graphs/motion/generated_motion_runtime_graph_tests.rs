use super::*;
use crate::generated_motion_load::data;

fn original(current: u16) -> Graph<'static> {
    Graph {
        states: &data::STATES,
        transitions: &data::TRANSITIONS,
        indices: &data::INDICES,
        current,
    }
}

#[test]
fn original_default_tick_two_keeps_ancestors_and_has_one_lazy_root_transition() {
    let graph = original(6);
    let mut calls = Vec::new();
    let selected = graph
        .next(&mut |mask, expression, parent| {
            calls.push((mask, expression, parent));
            Ok(false)
        })
        .unwrap();
    assert_eq!(
        selected,
        Selected {
            state: 6,
            transition: NULL_INDEX
        }
    );
    assert_eq!(calls[0], (Mask::Pre, 273, 0));
    assert_eq!(
        calls
            .iter()
            .filter(|&&(_, expression, _)| expression == 273)
            .count(),
        1
    );
    assert!(calls.iter().all(|&(mask, _, _)| mask == Mask::Pre));
    assert_eq!(graph.changes(&selected).unwrap(), (vec![], vec![]));
}

#[test]
fn rejected_root_transition_retains_error_before_reselection() {
    let graph = original(6);
    let mut calls = Vec::new();
    let error = graph
        .next(&mut |mask, expression, parent| {
            calls.push((mask, expression, parent));
            Err(Error::PendingExpression(expression))
        })
        .unwrap_err();
    assert!(matches!(error, Error::PendingExpression(273)));
    assert_eq!(calls, [(Mask::Pre, 273, 0)]);
}

#[test]
fn motion_common_ancestors_use_capacity_eleven_and_exclusive_pivot() {
    let graph = original(6);
    let selected = Selected {
        state: 6,
        transition: NULL_INDEX,
    };
    assert_eq!(graph.ancestors(6).unwrap(), [0, 6]);
    assert_eq!(
        graph.ancestors(1342).unwrap(),
        [0, 3, 21, 78, 250, 686, 989, 1145, 1232, 1298, 1342]
    );
    assert_eq!(graph.changes(&selected).unwrap(), (vec![], vec![]));
    // Original root transition0 targets Teleport5, so Default6 exits and5 enters.
    assert_eq!(
        graph
            .changes(&Selected {
                state: 5,
                transition: 0
            })
            .unwrap(),
        (vec![6], vec![5])
    );
}

#[test]
fn air_cycle_target_checks_its_inactive_leaf_then_holding_ancestor_lazily() {
    let graph = original(691);
    assert!(!data::STATES[1334].active);
    let mut expressions = Vec::new();
    let mut conditions = Vec::new();
    assert!(
        !graph
            .valid_target(1334, &mut |mask, index, parent| {
                assert_eq!(mask, Mask::Pre);
                expressions.push((index, parent));
                super::super::selection::expression(index, &mut |condition| {
                    conditions.push(condition);
                    assert!(matches!(condition, 351 | 355));
                    Ok(false)
                })
            })
            .unwrap()
    );
    assert_eq!(expressions, [(243, 1334), (249, 1273)]);
    assert_eq!(conditions, [351, 355]);
}
#[test]
fn holding_manual_intent_pre_and_sustain_keep_result_and_failure() {
    for mask in [Mask::Pre, Mask::Sustain] {
        for result in [false, true] {
            let mut reads = Vec::new();
            let mut condition = |i| {
                reads.push(i);
                Ok(result)
            };
            let actual = match mask {
                Mask::Pre => super::super::selection::expression(249, &mut condition),
                Mask::Sustain => super::super::sustain::expression(249, &mut condition),
            }
            .unwrap();
            assert_eq!(actual, result);
            assert_eq!(reads, [355]);
        }
        let mut fail = |i| {
            assert_eq!(i, 355);
            Err(Error::MissingOwner("intent source"))
        };
        let result = match mask {
            Mask::Pre => super::super::selection::expression(249, &mut fail),
            Mask::Sustain => super::super::sustain::expression(249, &mut fail),
        };
        assert!(matches!(result, Err(Error::MissingOwner("intent source"))));
    }
}
