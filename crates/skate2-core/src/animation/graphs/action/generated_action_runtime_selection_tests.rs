use super::super::super::{Section, data};
use super::*;

#[test]
fn braking_entry_and_exit_use_native_speed_then_parent_time_conditions() {
    // Pre828ff990 cases0..2 and Sustain82906048 default return1.
    // A false condition must keep waiting; admitting the missing expression
    // must not turn it into an unconditional transition.
    for (expression, condition) in [(0, 3), (1, 4), (2, 5)] {
        for value in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(evaluate_expression(Mask::Pre, expression, |index| {
                reads.push(index);
                Ok(value)
            }).unwrap(), value);
            assert_eq!(reads, [condition]);
        }
        assert!(evaluate_expression(Mask::Sustain, expression, |_| {
            panic!("native braking sustain does not read conditions")
        }).unwrap());
    }
    // Live kConditions8217c460 and kAttribs82174cd8: keep the existing
    // native thresholds, including the unusual ten-second wait timeout.
    for (condition, name, attribute, key, value) in [
        (3, "PhysicsSpeedCompare", 317, "greater", "5.0"),
        (4, "PhysicsSpeedCompare", 318, "less", "0.5"),
        (5, "InParentStateForTime", 316, "greater", "10.0"),
    ] {
        assert_eq!(data::CONDITIONS[condition].name, name);
        assert_eq!(data::CONDITIONS[condition].attributes.index as usize, attribute);
        assert_eq!((data::ATTRIBUTES[attribute].key, data::ATTRIBUTES[attribute].value), (key, value));
    }
}

#[test]
fn expression104_checks_six_intents_in_source_order_for_both_masks() {
    let ordered = [59, 53, 54, 60, 52, 55];
    for mask in [Mask::Pre, Mask::Sustain] {
        for stop in 0..ordered.len() {
            let mut reads = Vec::new();
            assert!(
                evaluate_expression(mask, 104, |c| {
                    reads.push(c);
                    Ok(c == ordered[stop])
                })
                .unwrap()
            );
            assert_eq!(reads, ordered[..=stop]);
        }
    }
}

#[test]
fn push_alternative_83_sustain_omits_entry_and_mirror_conditions() {
    let mut reads = Vec::new();
    assert!(
        !evaluate_expression(Mask::Pre, 83, |c| {
            reads.push(c);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [58]);
    reads.clear();
    assert!(
        evaluate_expression(Mask::Sustain, 83, |c| {
            reads.push(c);
            Ok(c == 38)
        })
        .unwrap()
    );
    assert_eq!(reads, [39, 38]);
}

#[test]
fn coffin_sustain_preserves_required_intents_and_lazy_physical_alternatives() {
    for mask in [Mask::Pre, Mask::Sustain] {
        let mut reads = Vec::new();
        assert!(
            !evaluate_expression(mask, 101, |c| {
                reads.push(c);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [45]);
        reads.clear();
        assert!(
            evaluate_expression(mask, 101, |c| {
                reads.push(c);
                Ok(matches!(c, 45 | 44 | 36 | 38 | 39))
            })
            .unwrap()
        );
        assert_eq!(reads, [45, 44, 74, 36, 38, 39]);
    }
}

#[test]
fn fingerflip_pre_and_sustain_check_five_intents_in_native_order() {
    let ordered = [47, 48, 49, 50, 51];
    for mask in [Mask::Pre, Mask::Sustain] {
        for (stop_index, stop) in ordered.iter().enumerate() {
            let mut reads = Vec::new();
            assert!(
                evaluate_expression(mask, 100, |condition| {
                    reads.push(condition);
                    Ok(condition == *stop)
                })
                .unwrap()
            );
            assert_eq!(reads, ordered[..=stop_index]);
        }
        let mut reads = Vec::new();
        assert!(
            !evaluate_expression(mask, 100, |condition| {
                reads.push(condition);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, ordered);
    }
}

#[test]
fn transitioning_offboard_checks_animation_before_physical_exclusions() {
    let mut seen = Vec::new();
    let result = evaluate_expression(Mask::Pre, 71, |condition| {
        seen.push(condition);
        Ok(matches!(condition, 132))
    })
    .unwrap();
    assert!(result);
    assert_eq!(seen, [131, 132, 72, 127, 133]);

    seen.clear();
    let result = evaluate_expression(Mask::Sustain, 71, |condition| {
        seen.push(condition);
        Ok(condition == 131)
    })
    .unwrap();
    assert!(result);
    assert_eq!(seen, [131]);
}

#[test]
fn coffin_pre101_checks_required_intents_before_physical_alternatives() {
    let mut reads = Vec::new();
    assert!(
        !evaluate_expression(Mask::Pre, 101, |condition| {
            reads.push(condition);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [45]);

    reads.clear();
    assert!(
        evaluate_expression(Mask::Pre, 101, |condition| {
            reads.push(condition);
            Ok(matches!(condition, 45 | 44 | 74 | 15 | 16))
        })
        .unwrap()
    );
    assert_eq!(reads, [45, 44, 74, 15, 16]);

    reads.clear();
    assert!(
        evaluate_expression(Mask::Pre, 101, |condition| {
            reads.push(condition);
            Ok(matches!(condition, 45 | 44 | 36 | 38 | 39))
        })
        .unwrap()
    );
    assert_eq!(reads, [45, 44, 74, 36, 38, 39]);
}

#[test]
fn regular_and_mirrored_selection_share_condition_only_in_pre() {
    for mirrored in [false, true] {
        for (expression, expected) in [(13, !mirrored), (14, mirrored)] {
            let mut reads = Vec::new();
            assert_eq!(
                evaluate_expression(Mask::Pre, expression, |index| {
                    reads.push(index);
                    assert_eq!(index, 42);
                    Ok(mirrored)
                })
                .unwrap(),
                expected
            );
            assert_eq!(reads, [42]);
            assert!(
                evaluate_expression(Mask::Sustain, expression, |_| {
                    panic!("native Sustain does not read IsMirrored")
                })
                .unwrap()
            );
        }
    }
}

#[test]
fn handplant_expression_uses_condition37_in_both_masks_and_propagates_failure() {
    for mask in [Mask::Pre, Mask::Sustain] {
        for value in [false, true] {
            let mut reached = Vec::new();
            assert_eq!(
                evaluate_expression(mask, 7, |index| {
                    reached.push(index);
                    Ok(value)
                })
                .unwrap(),
                value
            );
            assert_eq!(reached, [37]);
        }
        assert!(matches!(
            evaluate_expression(mask, 7, |_| Err(Error::PendingExpression(700))),
            Err(Error::PendingExpression(700))
        ));
    }
}

#[test]
fn grab_selection_preserves_mask_specific_mirroring_and_lazy_order() {
    for mask in [Mask::Pre, Mask::Sustain] {
        for left in [false, true] {
            for right in [false, true] {
                for mirrored in [false, true] {
                    for expression in [42, 43] {
                        let mut reads = Vec::new();
                        let actual = evaluate_expression(mask, expression, |index| {
                            reads.push(index);
                            Ok(match index {
                                38 => left,
                                39 => right,
                                42 => mirrored,
                                _ => panic!("unexpected condition"),
                            })
                        })
                        .unwrap();
                        let (expected, expected_reads) =
                            match (expression, mask, left, right, mirrored) {
                                (42, _, false, _, _) => (false, vec![38]),
                                (42, _, true, _, _) | (43, Mask::Sustain, false, _, _) => {
                                    (right, vec![38, 39])
                                }
                                (43, Mask::Sustain, true, _, _) => (true, vec![38]),
                                (43, Mask::Pre, true, _, false) => (true, vec![38, 42]),
                                (43, Mask::Pre, false, false, _) => (false, vec![38, 39]),
                                (43, Mask::Pre, false, true, _) => (mirrored, vec![38, 39, 42]),
                                (43, Mask::Pre, true, false, true) => (false, vec![38, 42, 39]),
                                (43, Mask::Pre, true, true, true) => (true, vec![38, 42, 39, 42]),
                                _ => unreachable!(),
                            };
                        assert_eq!(actual, expected);
                        assert_eq!(reads, expected_reads);
                    }
                }
            }
        }
        assert!(matches!(
            evaluate_expression(mask, 42, |index| if index == 38 {
                Ok(true)
            } else {
                Err(Error::PendingExpression(701))
            }),
            Err(Error::PendingExpression(701))
        ));
    }
}

#[test]
fn nose_grab_pre50_preserves_gate_and_mirrored_grab_call_order() {
    let cases: &[(bool, bool, bool, bool, bool, bool, &[u16])] = &[
        (false, false, false, false, false, false, &[58, 63]),
        (true, false, false, true, false, true, &[58, 39, 42]),
        (false, true, true, false, true, true, &[58, 63, 39, 38, 42]),
        (true, false, true, true, true, true, &[58, 39, 42, 38, 42]),
    ];
    for &(adjust, state, mirrored, right, left, expected, calls) in cases {
        let mut observed = Vec::new();
        let actual = evaluate_expression(Mask::Pre, 50, |index| {
            observed.push(index);
            Ok(match index {
                58 => adjust,
                63 => state,
                39 => right,
                38 => left,
                42 => mirrored,
                _ => panic!("unexpected condition {index}"),
            })
        })
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(observed, calls);
    }

    let mut observed = Vec::new();
    assert!(matches!(
        evaluate_expression(Mask::Pre, 50, |index| {
            observed.push(index);
            if index == 42 {
                Err(Error::PendingExpression(700))
            } else {
                Ok(index == 58 || index == 39)
            }
        }),
        Err(Error::PendingExpression(700))
    ));
    assert_eq!(observed, [58, 39, 42]);
}

#[test]
fn crail_grab_pre84_preserves_gate_and_mirrored_grab_call_order() {
    let cases: &[(bool, bool, bool, bool, bool, &[u16])] = &[
        (false, false, false, false, false, &[58]),
        (true, true, false, false, true, &[58, 38, 42]),
        (true, false, true, true, true, &[58, 38, 39, 42]),
        (true, true, true, true, true, &[58, 38, 42, 39, 42]),
    ];
    for &(adjust, left, right, mirrored, expected, calls) in cases {
        let mut observed = Vec::new();
        let actual = evaluate_expression(Mask::Pre, 84, |index| {
            observed.push(index);
            Ok(match index {
                58 => adjust,
                38 => left,
                39 => right,
                42 => mirrored,
                _ => panic!("unexpected condition {index}"),
            })
        })
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(observed, calls);
    }

    let mut observed = Vec::new();
    assert!(matches!(
        evaluate_expression(Mask::Pre, 84, |index| {
            observed.push(index);
            if index == 42 {
                Err(Error::PendingExpression(701))
            } else {
                Ok(index == 58 || index == 38)
            }
        }),
        Err(Error::PendingExpression(701))
    ));
    assert_eq!(observed, [58, 38, 42]);
}

#[test]
fn rocket_air_and_down_idle_preconditions_use_distinct_cached_conditions() {
    let mut reads = Vec::new();
    assert!(
        !evaluate_expression(Mask::Pre, 51, |index| {
            reads.push(index);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [39]);

    reads.clear();
    assert!(
        evaluate_expression(Mask::Pre, 51, |index| {
            reads.push(index);
            Ok(true)
        })
        .unwrap()
    );
    assert_eq!(reads, [39, 38]);

    reads.clear();
    assert!(matches!(
        evaluate_expression(Mask::Pre, 51, |index| {
            reads.push(index);
            if index == 38 {
                Err(Error::PendingExpression(702))
            } else {
                Ok(true)
            }
        }),
        Err(Error::PendingExpression(702))
    ));
    assert_eq!(reads, [39, 38]);

    for value in [false, true] {
        reads.clear();
        assert_eq!(
            evaluate_expression(Mask::Pre, 12, |index| {
                reads.push(index);
                Ok(value)
            })
            .unwrap(),
            value
        );
        assert_eq!(reads, [61]);
    }
}

#[test]
fn down_idle_sustain_uses_condition61_and_propagates_its_failure() {
    for value in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            evaluate_expression(Mask::Sustain, 12, |index| {
                reads.push(index);
                Ok(value)
            })
            .unwrap(),
            value
        );
        assert_eq!(reads, [61]);
    }

    let mut reads = Vec::new();
    assert!(matches!(
        evaluate_expression(Mask::Sustain, 12, |index| {
            reads.push(index);
            Err(Error::PendingExpression(703))
        }),
        Err(Error::PendingExpression(703))
    ));
    assert_eq!(reads, [61]);
}

fn state(parent: u16, expression: u16) -> StateDeclaration {
    StateDeclaration {
        name: "controlled topology",
        parent,
        expression,
        interrupt_type: 1,
        active: true,
        interrupt_target: NULL_INDEX,
        children: Section { count: 0, index: 0 },
        behaviours: Section { count: 0, index: 0 },
        transitions: Section { count: 0, index: 0 },
        update_address: 0,
        trailing: 0,
    }
}
fn transition(priority: u8, target: u16, expression: u16) -> TransitionDeclaration {
    TransitionDeclaration {
        name: "controlled edge",
        priority,
        target,
        expression,
        hooks: Section { count: 0, index: 0 },
    }
}

#[test]
fn ancestor_high_priority_beats_child_low_priority_and_preserves_indirection() {
    let mut states = [
        state(NULL_INDEX, NULL_INDEX),
        state(0, NULL_INDEX),
        state(0, NULL_INDEX),
    ];
    states[0].transitions = Section { count: 1, index: 0 };
    states[1].transitions = Section { count: 1, index: 1 };
    let transitions = [transition(0, 1, 10), transition(3, 2, 11)];
    let graph = Graph {
        states: &states,
        transitions: &transitions,
        indices: &[1, 0],
        current: 1,
    };
    let mut reached = Vec::new();
    let selected = graph
        .next(&mut |mask, expression, parent| {
            reached.push((mask, expression, parent));
            Ok(true)
        })
        .unwrap();
    assert_eq!(
        selected,
        Selected {
            state: 2,
            transition: 1
        }
    );
    assert_eq!(reached, [(Mask::Pre, 11, 0)]);
}

#[test]
fn failed_target_leaf_continues_descriptor_order_after_reached_side_effects() {
    let mut states = [
        state(NULL_INDEX, NULL_INDEX),
        state(0, NULL_INDEX),
        state(0, NULL_INDEX),
        state(0, NULL_INDEX),
        state(2, 8),
    ];
    states[1].transitions = Section { count: 2, index: 0 };
    states[2].children = Section { count: 1, index: 4 };
    let transitions = [transition(3, 2, 9), transition(3, 3, 10)];
    let graph = Graph {
        states: &states,
        transitions: &transitions,
        indices: &[0, 1],
        current: 1,
    };
    let mut reached = Vec::new();
    let selected = graph
        .next(&mut |_, expression, _| {
            reached.push(expression);
            Ok(expression != 8)
        })
        .unwrap();
    assert_eq!(
        selected,
        Selected {
            state: 3,
            transition: 1
        }
    );
    assert_eq!(reached, [9, 8, 10]);
}

#[test]
fn sustain_walks_current_ancestry_but_preconditions_other_candidates() {
    let mut states = [state(NULL_INDEX, 10), state(0, 11), state(0, 12)];
    states[0].children = Section { count: 2, index: 1 };
    let graph = Graph {
        states: &states,
        transitions: &[],
        indices: &[],
        current: 2,
    };
    let mut reached = Vec::new();
    let selected = graph
        .next(&mut |mask, expression, parent| {
            reached.push((mask, expression, parent));
            Ok(expression != 11)
        })
        .unwrap();
    assert_eq!(selected.state, 2);
    assert_eq!(
        reached,
        [
            (Mask::Sustain, 10, 0),
            (Mask::Pre, 11, 1),
            (Mask::Sustain, 12, 2)
        ]
    );
    assert_eq!(graph.changes(&selected).unwrap(), (vec![], vec![]));
}

#[test]
fn noninterruptible_checks_child_before_parent_and_short_circuits_failure() {
    let mut states = [state(NULL_INDEX, 10), state(0, 11)];
    states[1].interrupt_type = 0;
    let graph = Graph {
        states: &states,
        transitions: &[],
        indices: &[],
        current: 1,
    };
    let mut reached = Vec::new();
    let selected = graph
        .next(&mut |mask, expression, _| {
            reached.push((mask, expression));
            Ok(true)
        })
        .unwrap();
    assert_eq!(selected.state, 1);
    assert_eq!(reached, [(Mask::Sustain, 11), (Mask::Sustain, 10)]);
    reached.clear();
    let error = graph
        .next(&mut |mask, expression, _| {
            reached.push((mask, expression));
            Err(Error::PendingExpression(expression))
        })
        .unwrap_err();
    assert!(matches!(error, Error::PendingExpression(11)));
    assert_eq!(reached, [(Mask::Sustain, 11)]);
}

#[test]
fn explicit_self_transition_uses_parent_pivot_and_reenters_the_parent_too() {
    let states = [
        state(NULL_INDEX, NULL_INDEX),
        state(0, NULL_INDEX),
        state(1, NULL_INDEX),
    ];
    let transitions = [transition(3, 2, NULL_INDEX)];
    let graph = Graph {
        states: &states,
        transitions: &transitions,
        indices: &[],
        current: 2,
    };
    // Native passes target.parent as an EXCLUSIVE numeric pivot for target
    // descending from current, so parent1 also exits/enters on this self edge.
    assert_eq!(
        graph
            .changes(&Selected {
                state: 2,
                transition: 0
            })
            .unwrap(),
        (vec![1, 2], vec![1, 2])
    );
    assert_eq!(
        graph
            .changes(&Selected {
                state: 2,
                transition: NULL_INDEX
            })
            .unwrap(),
        (vec![], vec![])
    );
}

#[test]
fn source_sustain_expressions_omit_precondition_callbacks_and_keep_lazy_order() {
    let mut reached = Vec::new();
    assert!(
        evaluate_expression(Mask::Sustain, 27, |i| {
            reached.push(i);
            Ok(false)
        })
        .unwrap()
    );
    assert!(reached.is_empty());
    assert!(
        evaluate_expression(Mask::Sustain, 31, |i| {
            reached.push(i);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reached, [96]);
    reached.clear();
    assert!(
        !evaluate_expression(Mask::Pre, 31, |i| {
            reached.push(i);
            Ok(i == 70 || i == 127)
        })
        .unwrap()
    );
    assert_eq!(reached, [70, 127]);
}

#[test]
fn offboard_default_expression_reads_landing_time_only_for_offboard_air() {
    for mask in [Mask::Pre, Mask::Sustain] {
        let mut calls = Vec::new();
        assert!(
            evaluate_expression(mask, 88, |index| {
                calls.push(index);
                Ok(index == 134)
            })
            .unwrap()
        );
        assert_eq!(calls, [134]);

        calls.clear();
        assert!(
            !evaluate_expression(mask, 88, |index| {
                calls.push(index);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(calls, [134, 135]);

        calls.clear();
        assert!(
            evaluate_expression(mask, 88, |index| {
                calls.push(index);
                Ok(index == 135 || index == 143)
            })
            .unwrap()
        );
        assert_eq!(calls, [134, 135, 143]);
    }
}

#[test]
fn original_default_second_selection_uses_real_table_and_retains_instances_domain() {
    let graph = Graph {
        states: &data::STATES,
        transitions: &data::TRANSITIONS,
        indices: &data::INDICES,
        current: 3,
    };
    assert!(data::TRANSITIONS.iter().all(|t| t.hooks.count == 0));
    let mut reached = Vec::new();
    let selected = graph
        .next(&mut |mask, expression, parent| {
            reached.push((mask, expression, parent));
            // Controlled predicate oracle, not a production Actor or parity claim.
            Ok(false)
        })
        .unwrap();
    assert_eq!(
        selected,
        Selected {
            state: 3,
            transition: NULL_INDEX
        }
    );
    assert_eq!(graph.changes(&selected).unwrap(), (vec![], vec![]));
    assert_eq!(
        reached,
        [
            (Mask::Pre, 37, 4),
            (Mask::Pre, 38, 5),
            (Mask::Pre, 3, 16),
            (Mask::Pre, 6, 17),
            (Mask::Pre, 56, 19),
            (Mask::Pre, 27, 23),
            (Mask::Pre, 28, 24),
            (Mask::Pre, 29, 7),
            (Mask::Pre, 31, 8)
        ]
    );
}

#[test]
fn interrupt_target_sustain_failure_reevaluates_root_without_entering_target() {
    let mut states = [state(NULL_INDEX, 10), state(0, 11), state(0, 12)];
    states[0].children = Section { count: 2, index: 1 };
    states[2].interrupt_type = 2;
    states[2].interrupt_target = 1;
    let graph = Graph {
        states: &states,
        transitions: &[],
        indices: &[],
        current: 2,
    };
    let mut reached = Vec::new();
    let selected = graph
        .next(&mut |mask, expression, _| {
            reached.push((mask, expression));
            Ok(expression != 11)
        })
        .unwrap();
    assert_eq!(selected.state, 2);
    assert_eq!(
        reached,
        [
            (Mask::Sustain, 11),
            (Mask::Sustain, 10),
            (Mask::Pre, 11),
            (Mask::Sustain, 12)
        ]
    );
}

#[test]
fn target_precedes_shared_parent_sustain_and_transition_error_stops_search() {
    let mut states = [state(NULL_INDEX, 10), state(0, NULL_INDEX), state(0, 12)];
    states[1].transitions = Section { count: 1, index: 0 };
    let transitions = [transition(3, 2, 9)];
    let graph = Graph {
        states: &states,
        transitions: &transitions,
        indices: &[0],
        current: 1,
    };
    let mut reached = Vec::new();
    let selected = graph
        .next(&mut |mask, expression, parent| {
            reached.push((mask, expression, parent));
            Ok(true)
        })
        .unwrap();
    assert_eq!(selected.state, 2);
    assert_eq!(
        reached,
        [
            (Mask::Pre, 9, 1),
            (Mask::Pre, 12, 2),
            (Mask::Sustain, 10, 0),
            (Mask::Pre, 12, 2)
        ]
    );
    reached.clear();
    assert!(matches!(
        graph.next(&mut |mask, expression, parent| {
            reached.push((mask, expression, parent));
            Err(Error::PendingExpression(expression))
        }),
        Err(Error::PendingExpression(9))
    ));
    assert_eq!(reached, [(Mask::Pre, 9, 1)]);
}

#[test]
fn right_grab_swaps_intent_read_order_without_changing_mirror_gate() {
    for mask in [Mask::Pre, Mask::Sustain] {
        for left in [false, true] {
            for right in [false, true] {
                for mirrored in [false, true] {
                    let mut reads = Vec::new();
                    let actual = evaluate_expression(mask, 46, |index| {
                        reads.push(index);
                        Ok(match index {
                            38 => left,
                            39 => right,
                            42 => mirrored,
                            _ => panic!("unexpected condition"),
                        })
                    })
                    .unwrap();
                    let expected = if mask == Mask::Pre {
                        if mirrored { left } else { right }
                    } else {
                        left || right
                    };
                    assert_eq!(actual, expected);
                    assert_eq!(reads[0], 39);
                    if mask == Mask::Sustain {
                        assert_eq!(reads, if right { vec![39] } else { vec![39, 38] });
                    } else if right && !mirrored {
                        assert_eq!(reads, [39, 42]);
                    } else if right {
                        assert_eq!(
                            reads,
                            if left {
                                vec![39, 42, 38, 42]
                            } else {
                                vec![39, 42, 38]
                            }
                        );
                    } else {
                        assert_eq!(reads, if left { vec![39, 38, 42] } else { vec![39, 38] });
                    }
                }
            }
        }
        assert!(matches!(
            evaluate_expression(mask, 46, |_| Err(Error::PendingExpression(702))),
            Err(Error::PendingExpression(702))
        ));
    }
}

#[test]
fn directional_grab_pre_uses_intent_or_retained_state_but_sustain_reads_neither() {
    for (expression, intent, current) in [(47, 56, 57), (49, 62, 63)] {
        for (first, second, expected, order) in [
            (true, false, true, vec![intent]),
            (false, true, true, vec![intent, current]),
            (false, false, false, vec![intent, current]),
        ] {
            let mut reads = Vec::new();
            assert_eq!(
                evaluate_expression(Mask::Pre, expression, |index| {
                    reads.push(index);
                    Ok(if index == intent {
                        first
                    } else {
                        assert_eq!(index, current);
                        second
                    })
                })
                .unwrap(),
                expected
            );
            assert_eq!(reads, order);
        }
        assert!(
            evaluate_expression(Mask::Sustain, expression, |_| panic!(
                "directional Sustain reads no conditions"
            ))
            .unwrap()
        );
        assert!(matches!(
            evaluate_expression(Mask::Pre, expression, |index| if index == intent {
                Ok(false)
            } else {
                Err(Error::PendingExpression(703))
            }),
            Err(Error::PendingExpression(703))
        ));
    }
}

#[test]
fn action_current_state_reuses_parent_relative_lookup_for_actual_up_branch() {
    use crate::generated_motion_load::runtime::grounded_selection::state_matches;
    let states = &data::STATES;
    let up = u16::try_from(
        states
            .iter()
            .position(|state| state.parent == 39 && state.name == "Up")
            .unwrap(),
    )
    .unwrap();
    assert!(state_matches(states, up, up, "Up").unwrap());
    assert!(!state_matches(states, up, 94, "Up").unwrap());
    assert!(!state_matches(states, up, up, "AbsentNativeChild").unwrap());
}

#[test]
fn lateral_grabs_require_three_intents_but_sustain_only_the_first() {
    for (expression, conditions) in [(85, [61, 66, 67]), (86, [61, 68, 69])] {
        for bits in 0_u8..8 {
            for mask in [Mask::Pre, Mask::Sustain] {
                let mut reads = Vec::new();
                let result = evaluate_expression(mask, expression, |condition| {
                    reads.push(condition);
                    let slot = conditions.iter().position(|c| *c == condition).unwrap();
                    Ok(bits & (1 << slot) != 0)
                })
                .unwrap();
                let expected = if mask == Mask::Pre {
                    bits == 7
                } else {
                    bits & 1 != 0
                };
                assert_eq!(result, expected);
                let count = if mask == Mask::Sustain || bits & 1 == 0 {
                    1
                } else if bits & 2 == 0 {
                    2
                } else {
                    3
                };
                assert_eq!(reads, conditions[..count]);
            }
        }
        for failure in conditions {
            let mut reads = Vec::new();
            assert!(matches!(
                evaluate_expression(Mask::Pre, expression, |condition| {
                    reads.push(condition);
                    if condition == failure {
                        Err(Error::PendingExpression(704))
                    } else {
                        Ok(true)
                    }
                }),
                Err(Error::PendingExpression(704))
            ));
            assert_eq!(reads.last(), Some(&failure));
        }
    }
}

#[test]
fn new_one_foot_samples_intent_only_for_pre() {
    for value in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            evaluate_expression(Mask::Pre, 9, |condition| {
                reads.push(condition);
                Ok(value)
            })
            .unwrap(),
            value
        );
        assert_eq!(reads, [43]);
    }
    assert!(evaluate_expression(Mask::Sustain, 9, |_| panic!("no Sustain9 condition")).unwrap());
    assert!(matches!(
        evaluate_expression(Mask::Pre, 9, |_| Err(Error::PendingExpression(705))),
        Err(Error::PendingExpression(705))
    ));
}

#[test]
fn air_dismount_gate_keeps_intent_before_landing_time_and_constant_sustain() {
    for intent in [false, true] {
        for time in [false, true] {
            let mut reads = Vec::new();
            let value = evaluate_expression(Mask::Pre, 55, |i| {
                reads.push(i);
                Ok(match i {
                    70 => intent,
                    71 => time,
                    _ => panic!("unexpected condition"),
                })
            })
            .unwrap();
            assert_eq!(value, intent && time);
            assert_eq!(reads, if intent { vec![70, 71] } else { vec![70] });
        }
    }
    assert!(evaluate_expression(Mask::Sustain, 55, |_| panic!("constant Sustain")).unwrap());
    for fail in [70, 71] {
        let mut reads = Vec::new();
        assert!(matches!(
            evaluate_expression(Mask::Pre, 55, |i| {
                reads.push(i);
                if i == fail {
                    Err(Error::PendingExpression(705))
                } else {
                    Ok(true)
                }
            }),
            Err(Error::PendingExpression(705))
        ));
        assert_eq!(reads.last(), Some(&fail));
    }
}
#[test]
fn actual_idle_air_priority_chain_continues_after_absent_dismount_intent() {
    let graph = Graph {
        states: &data::STATES,
        transitions: &data::TRANSITIONS,
        indices: &data::INDICES,
        current: 43,
    };
    let mut expressions = Vec::new();
    let mut conditions = Vec::new();
    // Controlled false predicates check actual topology, not runtime physics.
    assert!(
        graph
            .search(&mut |mask, index, _| {
                expressions.push(index);
                evaluate_expression(mask, index, |condition| {
                    conditions.push(condition);
                    Ok(false)
                })
            })
            .unwrap()
            .is_none()
    );
    let air = expressions.iter().position(|i| *i == 55).unwrap();
    assert_eq!(&expressions[air..], &[55, 37, 38, 74]);
    assert!(conditions.contains(&70));
    assert!(!conditions.contains(&71));
}

#[test]
fn running_release_and_horizontal_adjust_transitions_preserve_lazy_reads() {
    for active in [false, true] {
        let mut calls = Vec::new();
        assert_eq!(
            evaluate_expression(Mask::Pre, 33, |index| {
                calls.push(index);
                Ok(active)
            })
            .unwrap(),
            !active
        );
        assert_eq!(calls, [139]);
    }
    for (expression, first, second) in [(53, 66, 67), (54, 68, 69)] {
        for first_value in [false, true] {
            let mut calls = Vec::new();
            assert_eq!(
                evaluate_expression(Mask::Pre, expression, |index| {
                    calls.push(index);
                    Ok(index != first || first_value)
                })
                .unwrap(),
                first_value
            );
            assert_eq!(
                calls,
                if first_value {
                    vec![first, second]
                } else {
                    vec![first]
                }
            );
        }
    }
}

#[test]
fn grab_expression48_preserves_entry_gate_mirroring_and_sustain_reads() {
    let mut reads = Vec::new();
    assert!(
        !evaluate_expression(Mask::Pre, 48, |i| {
            reads.push(i);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [58, 57]);
    reads.clear();
    assert!(
        evaluate_expression(Mask::Pre, 48, |i| {
            reads.push(i);
            Ok(matches!(i, 57 | 39 | 42))
        })
        .unwrap()
    );
    assert_eq!(reads, [58, 57, 38, 39, 42]);
    reads.clear();
    assert!(
        evaluate_expression(Mask::Sustain, 48, |i| {
            reads.push(i);
            Ok(i == 38)
        })
        .unwrap()
    );
    assert_eq!(reads, [38]);
}

#[test]
fn grab79_entry_checks_stance_but_sustain_only_requires_one_grab() {
    for mirrored in [false, true] {
        for left in [false, true] {
            for right in [false, true] {
                for mask in [Mask::Pre, Mask::Sustain] {
                    let mut reads = Vec::new();
                    let actual = evaluate_expression(mask, 79, |i| {
                        reads.push(i);
                        Ok(match i {
                            42 => mirrored,
                            44 => left,
                            45 => right,
                            _ => panic!("unexpected condition"),
                        })
                    })
                    .unwrap();
                    let expected = match mask {
                        Mask::Pre => {
                            if mirrored {
                                right && !left
                            } else {
                                left && !right
                            }
                        }
                        Mask::Sustain => left != right,
                    };
                    assert_eq!(actual, expected);
                    if matches!(mask, Mask::Sustain) {
                        assert!(!reads.contains(&42));
                    }
                }
            }
        }
    }
}

#[test]
fn late_flip_and_grab_switch_leaves_read_condition17_in_both_masks() {
    for mask in [Mask::Pre, Mask::Sustain] {
        for mirrored in [false, true] {
            for (expression, expected) in [(4, !mirrored), (5, mirrored)] {
                let mut reads = Vec::new();
                assert_eq!(
                    evaluate_expression(mask, expression, |index| {
                        reads.push(index);
                        Ok(mirrored)
                    })
                    .unwrap(),
                    expected
                );
                assert_eq!(reads, [17]);
            }
        }
    }
}

#[test]
fn not_both_air_grabs_reads_left_grab_only_while_right_is_held() {
    for (condition, attribute, intent) in [(64, 349, "RightAirGrab"), (65, 348, "LeftAirGrab")] {
        assert_eq!(data::CONDITIONS[condition].name, "HasAGIntent");
        assert_eq!(
            data::CONDITIONS[condition].attributes.index as usize,
            attribute
        );
        assert_eq!(data::ATTRIBUTES[attribute].value, intent);
    }
    for right in [false, true] {
        for left in [false, true] {
            let mut reads = Vec::new();
            let held = |index| if index == 64 { right } else { left };
            assert_eq!(
                evaluate_expression(Mask::Pre, 52, |index| {
                    reads.push(index);
                    Ok(held(index))
                })
                .unwrap(),
                !(right && left)
            );
            assert_eq!(reads, if right { vec![64, 65] } else { vec![64] });
        }
    }
    for expression in [49, 52, 53, 54, 55] {
        assert!(evaluate_expression(Mask::Sustain, expression, |_| unreachable!()).unwrap());
    }
}
