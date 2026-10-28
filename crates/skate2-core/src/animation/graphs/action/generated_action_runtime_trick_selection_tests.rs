use super::*;
use crate::generated_action_load::construct::ComparisonOp;
use crate::generated_action_load::{conditions::parent_time_condition, construct::Comparison};
use std::collections::BTreeMap;

#[test]
fn gesture_success_and_hold_absence_skip_all_later_conditions() {
    for mask in [Mask::Pre, Mask::Sustain] {
        for (expression_index, gesture) in [(91, 108), (92, 111), (93, 112)] {
            let mut calls = Vec::new();
            assert!(
                expression(mask, expression_index, |i| {
                    calls.push(i);
                    Ok(true)
                })
                .unwrap()
            );
            assert_eq!(calls, [gesture]);
            calls.clear();
            assert!(
                !expression(mask, expression_index, |i| {
                    calls.push(i);
                    Ok(false)
                })
                .unwrap()
            );
            assert_eq!(calls, [gesture, 109]);
        }
    }
}

#[test]
fn air_fallback_is_only_read_after_held_timer_failure() {
    for timer in [false, true] {
        let mut calls = Vec::new();
        assert!(
            expression(Mask::Pre, 91, |i| {
                calls.push(i);
                Ok(match i {
                    108 => false,
                    110 => timer,
                    _ => true,
                })
            })
            .unwrap()
        );
        assert_eq!(
            calls,
            if timer {
                vec![108, 109, 110]
            } else {
                vec![108, 109, 110, 36]
            }
        );
    }
}

#[test]
fn time_query_uses_selected_tag_and_missing_sentinel_before_comparison() {
    let comparison = Comparison {
        op: ComparisonOp::Less,
        absolute: false,
        reference: 0.4,
    };
    let times = BTreeMap::from([(20, 0.1), (52, 0.5)]);
    assert!(
        !parent_time_condition(&times, 224, Some(52), comparison, Error::InvalidState).unwrap()
    );
    assert!(parent_time_condition(&times, 224, Some(20), comparison, Error::InvalidState).unwrap());
    for missing in [None, Some(u16::MAX), Some(53)] {
        assert!(
            !parent_time_condition(&times, 224, missing, comparison, Error::InvalidState).unwrap()
        );
    }
    assert!(matches!(
        parent_time_condition(&times, 224, Some(224), comparison, Error::InvalidState),
        Err(Error::InvalidState(224))
    ));
}

#[test]
fn unordered_age_reaches_comparison_but_negative_age_does_not() {
    let comparison = Comparison {
        op: ComparisonOp::NotEqual,
        absolute: false,
        reference: 0.4,
    };
    let times = BTreeMap::from([(52, f32::NAN), (53, -1.0)]);
    assert!(parent_time_condition(&times, 224, Some(52), comparison, Error::InvalidState).unwrap());
    assert!(
        !parent_time_condition(&times, 224, Some(53), comparison, Error::InvalidState).unwrap()
    );
}
