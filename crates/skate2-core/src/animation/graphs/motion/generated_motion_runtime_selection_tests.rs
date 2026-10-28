use super::*;

#[test]
fn expression341_preserves_ordered_entry_and_distinct_sustain_intents() {
    for first in [false, true] {
        for second in [false, true] {
            let mut reads = Vec::new();
            let mut condition = |index| {
                reads.push(index);
                Ok(match index {
                    290 => first,
                    291 => second,
                    _ => panic!("unexpected condition {index}"),
                })
            };
            assert_eq!(expression(341, &mut condition).unwrap(), !first && !second);
            assert_eq!(reads, if first { vec![290] } else { vec![290, 291] });
            reads.clear();
            assert_eq!(
                super::super::sustain::expression(341, &mut |index| {
                    reads.push(index);
                    assert_eq!(index, 291);
                    Ok(second)
                }).unwrap(),
                !second
            );
            assert_eq!(reads, [291]);
        }
    }
}

#[test]
fn left_slide_exit_and_nollie_pop_shuvit_sustain_follow_native_switches() {
    for value in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(232, &mut |index| {
                reads.push(index);
                Ok(value)
            })
            .unwrap(),
            value
        );
        assert_eq!(reads, [320]);
    }
    assert!(matches!(
        expression(232, &mut |_| Err(MotionUpdateError::PendingCondition(320))),
        Err(MotionUpdateError::PendingCondition(320))
    ));
    for id in [110, 232] {
        assert!(
            super::super::sustain::expression(id, &mut |_| panic!("native constant Sustain"))
                .unwrap()
        );
    }
}

#[test]
fn expression234_reuses_should_leave_slide_and_sustain_reads_nothing() {
    for value in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(234, &mut |index| {
                reads.push(index);
                Ok(value)
            })
            .unwrap(),
            value
        );
        assert_eq!(reads, [341]);
    }
    assert!(matches!(
        expression(234, &mut |_| Err(MotionUpdateError::PendingCondition(341))),
        Err(MotionUpdateError::PendingCondition(341))
    ));
    assert!(
        super::super::sustain::expression(234, &mut |_| panic!("native constant Sustain")).unwrap()
    );
}

#[test]
fn expression406_short_circuits_direct_intent_before_action_pair() {
    let mut reads = Vec::new();
    assert!(
        expression(406, &mut |c| {
            reads.push(c);
            Ok(true)
        })
        .unwrap()
    );
    assert_eq!(reads, [226]);
    reads.clear();
    assert!(
        !expression(406, &mut |c| {
            reads.push(c);
            Ok(c == 238)
        })
        .unwrap()
    );
    assert_eq!(reads, [226, 238, 239]);
    for id in [4, 115, 116, 398, 406] {
        assert!(super::super::sustain::expression(id, &mut |_| panic!("native default")).unwrap());
    }
}

#[test]
fn grab_contact_exit_checks_filtered_air_only_without_truck_or_deck_contact() {
    let mut reads = Vec::new();
    assert!(
        expression(304, &mut |c| {
            reads.push(c);
            Ok(true)
        })
        .unwrap()
    );
    assert_eq!(reads, [227]);
    reads.clear();
    assert!(
        !expression(304, &mut |c| {
            reads.push(c);
            Ok(c == 85)
        })
        .unwrap()
    );
    assert_eq!(reads, [227, 85]);
    assert!(super::super::sustain::expression(304, &mut |_| panic!("native default")).unwrap());
    assert!(super::super::sustain::expression(114, &mut |_| panic!("native default")).unwrap());
}

#[test]
fn push_exit_443_requires_expiry_before_any_interruption_gate() {
    let mut reads = Vec::new();
    assert!(
        !expression(443, &mut |c| {
            reads.push(c);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [214]);
    reads.clear();
    assert!(
        expression(443, &mut |c| {
            reads.push(c);
            Ok(matches!(c, 214 | 343))
        })
        .unwrap()
    );
    assert_eq!(reads, [214, 18, 288, 343]);
    assert!(
        super::super::sustain::expression(443, &mut |_| panic!("native default has no reads"))
            .unwrap()
    );
    assert!(
        super::super::sustain::expression(113, &mut |_| panic!("native default has no reads"))
            .unwrap()
    );
}

#[test]
fn air_mount_candidates_preserve_lazy_primary_and_current_state_fallback() {
    for (id, foot, state) in [(431, 61, 74), (432, 62, 75)] {
        let mut reads = Vec::new();
        assert!(
            !expression(id, &mut |c| {
                reads.push(c);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [72, state]);
        reads.clear();
        assert!(
            expression(id, &mut |c| {
                reads.push(c);
                Ok(c == 72 || c == foot || c == 73)
            })
            .unwrap()
        );
        assert_eq!(reads, [72, foot, 73, 67, 68]);
        reads.clear();
        assert!(
            expression(id, &mut |c| {
                reads.push(c);
                Ok(c == state || c == 26)
            })
            .unwrap()
        );
        assert_eq!(reads, [72, state, 26]);
    }
}

#[test]
fn offboard_mount_checks_do_not_read_feet_or_retrieval_without_mount_intent() {
    // Source expressions all start with HasIntent(OB_Mount). A walk/jump
    // must not evaluate otherwise unrelated mount callbacks.
    for id in [374, 375, 378, 379, 421, 422] {
        let mut reads = Vec::new();
        assert!(
            !expression(id, &mut |condition| {
                reads.push(condition);
                assert_eq!(condition, 26);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [26]);
    }
}

#[test]
fn sprint_mount_requires_intent_sprint_and_either_foot_down_or_matching_state() {
    for (id, foot, state) in [(421, 64, 65), (422, 69, 70)] {
        for foot_down in [false, true] {
            let mut reads = Vec::new();
            assert!(
                expression(id, &mut |condition| {
                    reads.push(condition);
                    Ok(match condition {
                        26 | 66 => true,
                        n if n == foot => foot_down,
                        n if n == state => true,
                        67 | 68 => false,
                        n => panic!("unexpected condition {n}"),
                    })
                })
                .unwrap()
            );
            let expected = if foot_down {
                vec![26, 66, foot, 67, 68]
            } else {
                vec![26, 66, foot, state, 67, 68]
            };
            assert_eq!(reads, expected);
        }
    }
}

#[test]
fn held_kickflip_cycle_exits_keep_native_lazy_condition_order() {
    for (expression_id, landing_condition) in [(389, 169), (390, 170)] {
        for (landing, flipping, held, expected, expected_reads) in [
            (true, false, true, true, vec![landing_condition]),
            (false, true, true, true, vec![landing_condition, 160]),
            (false, false, true, false, vec![landing_condition, 160, 158]),
            (false, false, false, true, vec![landing_condition, 160, 158]),
        ] {
            let mut reads = Vec::new();
            let result = expression(expression_id, &mut |condition| {
                reads.push(condition);
                Ok(match condition {
                    x if x == landing_condition => landing,
                    160 => flipping,
                    158 => held,
                    _ => panic!("unexpected Kickflip exit condition {condition}"),
                })
            })
            .unwrap();
            assert_eq!(result, expected);
            assert_eq!(reads, expected_reads);
        }
    }
    for (flipping, held, expected, expected_reads) in [
        (true, true, true, vec![160]),
        (false, true, false, vec![160, 158]),
        (false, false, true, vec![160, 158]),
    ] {
        let mut reads = Vec::new();
        let result = expression(296, &mut |condition| {
            reads.push(condition);
            Ok(match condition {
                160 => flipping,
                158 => held,
                _ => panic!("unexpected Kickflip Out3 condition {condition}"),
            })
        })
        .unwrap();
        assert_eq!(result, expected);
        assert_eq!(reads, expected_reads);
    }
}

#[test]
fn debug_animations_pre_reads_the_native_cvar_condition_once() {
    for enabled in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(272, &mut |condition| {
                reads.push(condition);
                Ok(enabled)
            })
            .unwrap(),
            enabled
        );
        assert_eq!(reads, [405]);
    }
}

#[test]
fn spawn_dismount_pre_reads_only_its_authored_intent() {
    for present in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(1, &mut |condition| {
                reads.push(condition);
                Ok(present)
            })
            .unwrap(),
            present
        );
        assert_eq!(reads, [7]);
    }
}

#[test]
fn idle_dismount_pre_preserves_velocity_and_exclusion_order() {
    for (values, expected, reads) in [
        ([(16, false), (17, false), (18, false)], false, vec![16]),
        ([(16, true), (17, true), (18, false)], false, vec![16, 17]),
        (
            [(16, true), (17, false), (18, true)],
            false,
            vec![16, 17, 18],
        ),
        (
            [(16, true), (17, false), (18, false)],
            true,
            vec![16, 17, 18],
        ),
    ] {
        let mut seen = Vec::new();
        assert_eq!(
            expression(274, &mut |index| {
                seen.push(index);
                Ok(values
                    .into_iter()
                    .find(|(condition, _)| *condition == index)
                    .unwrap()
                    .1)
            })
            .unwrap(),
            expected
        );
        assert_eq!(seen, reads);
    }
}

#[test]
fn offboard_mount_pre_checks_intent_then_retrieving_then_dropping() {
    for (values, expected, reads) in [
        ([(26, false), (67, false), (68, false)], false, vec![26]),
        ([(26, true), (67, true), (68, false)], false, vec![26, 67]),
        (
            [(26, true), (67, false), (68, true)],
            false,
            vec![26, 67, 68],
        ),
        (
            [(26, true), (67, false), (68, false)],
            true,
            vec![26, 67, 68],
        ),
    ] {
        let mut seen = Vec::new();
        assert_eq!(
            expression(379, &mut |index| {
                seen.push(index);
                Ok(values
                    .into_iter()
                    .find(|(condition, _)| *condition == index)
                    .unwrap()
                    .1)
            })
            .unwrap(),
            expected
        );
        assert_eq!(seen, reads);
    }
}

#[test]
fn push_pre438_preserves_native_lazy_condition_order() {
    let ordered = [18, 288, 343, 287, 225];
    for (stop_index, stop) in ordered.iter().enumerate() {
        let mut reads = Vec::new();
        assert!(
            expression(438, &mut |condition| {
                reads.push(condition);
                Ok(condition == *stop)
            })
            .unwrap()
        );
        assert_eq!(reads, ordered[..=stop_index]);
    }
    let mut reads = Vec::new();
    assert!(
        !expression(438, &mut |condition| {
            reads.push(condition);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, ordered);
}

#[test]
fn push_gesture_pre357_358_keep_goofy_branch_order() {
    let mut reads = Vec::new();
    assert!(
        !expression(357, &mut |condition| {
            reads.push(condition);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [218]);

    reads.clear();
    assert!(
        expression(357, &mut |condition| {
            reads.push(condition);
            Ok(true)
        })
        .unwrap()
    );
    assert_eq!(reads, [218, 366]);

    reads.clear();
    assert!(
        !expression(358, &mut |condition| {
            reads.push(condition);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [366]);

    reads.clear();
    assert!(
        expression(358, &mut |condition| {
            reads.push(condition);
            Ok(condition == 366)
        })
        .unwrap()
    );
    assert_eq!(reads, [366, 218]);
}

#[test]
fn mongo_push_pre415_lazily_checks_foot_stance_and_state() {
    let mut reads = Vec::new();
    assert!(
        !expression(415, &mut |condition| {
            reads.push(condition);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [358]);

    reads.clear();
    assert!(
        expression(415, &mut |condition| {
            reads.push(condition);
            Ok(matches!(condition, 358 | 360))
        })
        .unwrap()
    );
    assert_eq!(reads, [358, 360, 359]);

    reads.clear();
    assert!(
        expression(415, &mut |condition| {
            reads.push(condition);
            Ok(condition == 358)
        })
        .unwrap()
    );
    assert_eq!(reads, [358, 360, 361, 360]);
}

#[test]
fn mongo_push_pre356_negates_foot_distance_before_shared_stance_tail() {
    let mut reads = Vec::new();
    assert!(
        !expression(356, &mut |condition| {
            reads.push(condition);
            Ok(true)
        })
        .unwrap()
    );
    assert_eq!(reads, [358]);

    reads.clear();
    assert!(
        expression(356, &mut |condition| {
            reads.push(condition);
            Ok(condition == 360)
        })
        .unwrap()
    );
    assert_eq!(reads, [358, 360, 359]);
}

#[test]
fn specific_grind_pre446_preserves_native_order_and_lazy_failure() {
    for stop in [144, 149, 155] {
        let mut reads = Vec::new();
        assert!(
            expression(446, &mut |condition| {
                reads.push(condition);
                Ok(condition == stop)
            })
            .unwrap()
        );
        assert_eq!(reads, (144..=stop).collect::<Vec<_>>());
    }
    let mut reads = Vec::new();
    assert!(matches!(
        expression(446, &mut |condition| {
            reads.push(condition);
            if condition == 150 {
                Err(MotionUpdateError::MissingOwner("grind output"))
            } else {
                Ok(false)
            }
        }),
        Err(MotionUpdateError::MissingOwner("grind output"))
    ));
    assert_eq!(reads, (144..=150).collect::<Vec<_>>());
}

#[test]
fn manual_history_pre294_skips_timer_on_matching_last_state() {
    let mut reads = Vec::new();
    assert!(
        expression(294, &mut |condition| {
            reads.push(condition);
            Ok(condition == 161)
        })
        .unwrap()
    );
    assert_eq!(reads, [161]);

    reads.clear();
    assert!(
        expression(294, &mut |condition| {
            reads.push(condition);
            Ok(condition == 162)
        })
        .unwrap()
    );
    assert_eq!(reads, [161, 162]);

    reads.clear();
    assert!(
        !expression(294, &mut |condition| {
            reads.push(condition);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [161, 162]);
    assert!(matches!(
        expression(294, &mut |condition| {
            if condition == 162 {
                Err(MotionUpdateError::MissingOwner("Motion timer"))
            } else {
                Ok(false)
            }
        }),
        Err(MotionUpdateError::MissingOwner("Motion timer"))
    ));
}

#[test]
fn trick_runout_transition387_keeps_native_lazy_physics_order() {
    for (present, expected, reads) in [
        (vec![], false, vec![87]),
        (vec![87, 89], true, vec![87, 89]),
        (vec![87, 13], true, vec![87, 89, 13]),
        (vec![87], false, vec![87, 89, 13]),
    ] {
        let mut observed = Vec::new();
        assert_eq!(
            expression(387, &mut |condition| {
                observed.push(condition);
                Ok(present.contains(&condition))
            })
            .unwrap(),
            expected
        );
        assert_eq!(observed, reads);
    }
}

#[test]
fn root_teleport_transition_skips_current_state_unless_physics_matches() {
    let mut calls = Vec::new();
    assert!(
        !expression(273, &mut |condition| {
            calls.push(condition);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(calls, [0]);
    calls.clear();
    assert!(
        expression(273, &mut |condition| {
            calls.push(condition);
            Ok(condition == 0)
        })
        .unwrap()
    );
    assert_eq!(calls, [0, 1]);
}

#[test]
fn original_initial_fallback_keeps_offboard_before_onboard_and_parent_indices() {
    let mut calls = Vec::new();
    let selected = evaluate(
        &crate::generated_motion_load::data::STATES,
        0,
        &mut |condition, parent| {
            calls.push((condition, parent));
            Ok(false)
        },
    )
    .unwrap();
    assert_eq!(selected, 6);
    assert_eq!(
        calls,
        [
            (6, 8),
            (76, 36),
            (76, 37),
            (26, 37),
            (76, 38),
            (87, 17),
            (94, 18),
            (139, 19),
            (212, 75),
            (222, 76),
            (223, 21),
            (202, 24),
            (394, 90),
            (402, 5)
        ]
    );
}

#[test]
fn stride_expression_keeps_repeated_operands_and_secondary_fallback() {
    let mut calls = Vec::new();
    assert!(
        expression(433, &mut |condition| {
            calls.push(condition);
            Ok(matches!(condition, 76 | 77 | 48 | 26 | 81))
        })
        .unwrap()
    );
    assert_eq!(calls, [76, 77, 48, 78, 78, 26, 79, 80, 81]);
    // 8291b2cc: IsMirrored, LocoStartingFoot, IsMirrored, LocoStartingFoot;
    // both arms return true only without LocoStartingFoot.
    for (mirrored, starting_foot, expected, reads) in [
        (false, true, false, &[76, 77, 48, 48, 78][..]),
        (false, false, true, &[76, 77, 48, 48, 78][..]),
        (true, false, true, &[76, 77, 48, 78][..]),
        (true, true, false, &[76, 77, 48, 78, 48][..]),
    ] {
        calls.clear();
        assert_eq!(
            expression(423, &mut |condition| {
                calls.push(condition);
                Ok(match condition {
                    76 | 77 => true,
                    48 => mirrored,
                    78 => starting_foot,
                    _ => false,
                })
            })
            .unwrap(),
            expected
        );
        assert_eq!(calls, reads);
    }
}

#[test]
fn unavailable_reached_condition_does_not_fall_through_to_default() {
    let mut calls = Vec::new();
    let error = evaluate(
        &crate::generated_motion_load::data::STATES,
        0,
        &mut |condition, parent| {
            calls.push((condition, parent));
            if condition == 87 {
                Err(MotionUpdateError::MissingOwner("physical"))
            } else {
                Ok(false)
            }
        },
    )
    .unwrap_err();
    assert!(matches!(error, MotionUpdateError::MissingOwner("physical")));
    assert_eq!(calls.last(), Some(&(87, 17)));
    assert!(!calls.iter().any(|&(condition, _)| condition == 94));
}

#[test]
fn successful_offboard_parent_selects_neutral_standing_leaf() {
    let mut calls = Vec::new();
    let selected = evaluate(
        &crate::generated_motion_load::data::STATES,
        0,
        &mut |condition, parent| {
            calls.push((condition, parent));
            Ok(condition == 6)
        },
    )
    .unwrap();
    assert_eq!(selected, 752); // Standing / Still in the original declarations.
    // Spawn and dismount are checked first, before the neutral standing path.
    assert!(calls.starts_with(&[(6, 8), (7, 31), (9, 32)]));
}

#[test]
fn fakie_pre_samples_actual_shared_condition_and_propagates_failure() {
    for value in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(3, &mut |condition| {
                reads.push(condition);
                Ok(value)
            })
            .unwrap(),
            value
        );
        assert_eq!(reads, [14]);
    }
    assert!(matches!(
        expression(3, &mut |condition| {
            assert_eq!(condition, 14);
            Err(MotionUpdateError::MissingOwner("actual animation flag"))
        }),
        Err(MotionUpdateError::MissingOwner("actual animation flag"))
    ));
}

#[test]
fn push_selection_reads_only_reached_intents_and_current_state_names() {
    for (index, first) in [(416, 362), (417, 363)] {
        for stop in [Some(first), Some(364), Some(365), None] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(index, &mut |condition| {
                    reads.push(condition);
                    Ok(Some(condition) == stop)
                })
                .unwrap(),
                stop.is_some()
            );
            let expected = if stop == Some(first) {
                vec![first]
            } else if stop == Some(364) {
                vec![first, 364]
            } else {
                vec![first, 364, 365]
            };
            assert_eq!(reads, expected);
        }
        for failed in [first, 364, 365] {
            let mut reads = Vec::new();
            assert!(matches!(
                expression(index, &mut |condition| {
                    reads.push(condition);
                    if condition == failed {
                        Err(MotionUpdateError::MissingOwner("push owner"))
                    } else {
                        Ok(false)
                    }
                }),
                Err(MotionUpdateError::MissingOwner("push owner"))
            ));
            assert_eq!(reads.last(), Some(&failed));
        }
    }
    for (index, target) in [(254, 362), (255, 363)] {
        let mut reads = Vec::new();
        assert!(
            !expression(index, &mut |condition| {
                reads.push(condition);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [target]);
    }
}

#[test]
fn anticipation_pre441_stops_at_first_intent_or_failed_read() {
    for stop in 371..=376 {
        let mut reads = Vec::new();
        let result = expression(441, &mut |c| {
            reads.push(c);
            Ok(c == stop)
        })
        .unwrap();
        assert_eq!(result, stop != 376);
        assert_eq!(reads, (371..=stop.min(375)).collect::<Vec<_>>());
    }
    let mut reads = Vec::new();
    assert!(matches!(
        expression(441, &mut |c| {
            reads.push(c);
            if c == 373 {
                Err(MotionUpdateError::MissingOwner("intent"))
            } else {
                Ok(false)
            }
        }),
        Err(MotionUpdateError::MissingOwner("intent"))
    ));
    assert_eq!(reads, [371, 372, 373]);
}

#[test]
fn antic_out_tail_pre439_negates_the_same_ordered_five_intents() {
    for stop in 371..=376 {
        let mut reads = Vec::new();
        let result = expression(439, &mut |condition| {
            reads.push(condition);
            Ok(condition == stop)
        })
        .unwrap();
        assert_eq!(result, stop == 376);
        assert_eq!(reads, (371..=stop.min(375)).collect::<Vec<_>>());
    }
    let mut reads = Vec::new();
    assert!(matches!(
        expression(439, &mut |condition| {
            reads.push(condition);
            if condition == 373 {
                Err(MotionUpdateError::MissingOwner("intent"))
            } else {
                Ok(false)
            }
        }),
        Err(MotionUpdateError::MissingOwner("intent"))
    ));
    assert_eq!(reads, [371, 372, 373]);
}

#[test]
fn nollie_anticipation_pre442_preserves_lazy_reads_and_failures() {
    for stop in 376..=381 {
        let mut reads = Vec::new();
        let result = expression(442, &mut |c| {
            reads.push(c);
            Ok(c == stop)
        })
        .unwrap();
        assert_eq!(result, stop != 381);
        assert_eq!(reads, (376..=stop.min(380)).collect::<Vec<_>>());
    }
    let mut reads = Vec::new();
    assert!(matches!(
        expression(442, &mut |c| {
            reads.push(c);
            if c == 378 {
                Err(MotionUpdateError::MissingOwner("intent"))
            } else {
                Ok(false)
            }
        }),
        Err(MotionUpdateError::MissingOwner("intent"))
    ));
    assert_eq!(reads, [376, 377, 378]);
}

#[test]
fn grounded_grab_admission_leaves_and_lazy_double_grab() {
    for (index, condition) in [(199, 288), (237, 343), (5, 19), (270, 404), (142, 232)] {
        for expected in [false, true] {
            assert_eq!(
                expression(index, &mut |c| {
                    assert_eq!(c, condition);
                    Ok(expected)
                })
                .unwrap(),
                expected
            );
        }
        assert!(matches!(
            expression(index, &mut |_| Err(MotionUpdateError::MissingOwner(
                "intent"
            ))),
            Err(MotionUpdateError::MissingOwner("intent"))
        ));
    }
    for (first, second) in [(false, false), (false, true), (true, false)] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(355, &mut |c| {
                reads.push(c);
                Ok(if c == 287 { first } else { second })
            })
            .unwrap(),
            first || second
        );
        assert_eq!(reads, if first { vec![287] } else { vec![287, 225] });
    }
    let mut reads = Vec::new();
    assert!(
        expression(355, &mut |c| {
            reads.push(c);
            Err(MotionUpdateError::MissingOwner("intent"))
        })
        .is_err()
    );
    assert_eq!(reads, [287]);
}

#[test]
fn compound_conditions_preserve_lazy_second_read_and_failure() {
    for (expression_index, first, second) in [(344, 308, 309), (345, 19, 14)] {
        for (bump, slope) in [(false, false), (false, true), (true, false), (true, true)] {
            let mut reads = Vec::new();
            let result = expression(expression_index, &mut |c| {
                reads.push(c);
                Ok(match c {
                    index if index == first => bump,
                    index if index == second => slope,
                    _ => panic!("unexpected condition"),
                })
            })
            .unwrap();
            assert_eq!(result, bump && slope);
            assert_eq!(
                reads,
                if bump {
                    vec![first, second]
                } else {
                    vec![first]
                }
            );
        }
        let mut reads = Vec::new();
        assert!(
            expression(expression_index, &mut |c| {
                reads.push(c);
                Err(MotionUpdateError::MissingOwner("bump source"))
            })
            .is_err()
        );
        assert_eq!(reads, [first]);
    }
}

#[test]
fn into405_pre_preserves_order_short_circuit_and_reached_failure() {
    for values in 0_u8..8 {
        let mut reads = Vec::new();
        let result = expression(405, &mut |index| {
            reads.push(index);
            Ok(values & (1 << (index - 224)) != 0)
        })
        .unwrap();
        assert_eq!(result, values != 0);
        let expected: &[u16] = if values & 1 != 0 {
            &[224]
        } else if values & 2 != 0 {
            &[224, 225]
        } else {
            &[224, 225, 226]
        };
        assert_eq!(reads, expected);
    }
    for failure in 224..=226 {
        let mut reads = Vec::new();
        assert!(matches!(
            expression(405, &mut |index| {
                reads.push(index);
                if index == failure {
                    Err(MotionUpdateError::MissingOwner("reached intent"))
                } else {
                    Ok(false)
                }
            }),
            Err(MotionUpdateError::MissingOwner("reached intent"))
        ));
        assert_eq!(reads, (224..=failure).collect::<Vec<_>>());
    }
}

#[test]
fn push_end_pre444_reuses_438_tail_after_breakout_and_stops_at_first_true() {
    let order = [368, 18, 288, 343, 287, 225];
    for stop in 0..=order.len() {
        let mut reads = Vec::new();
        let result = expression(444, &mut |index| {
            reads.push(index);
            Ok(order.get(stop).copied() == Some(index))
        })
        .unwrap();
        assert_eq!(result, stop < order.len());
        assert_eq!(reads, order[..(stop + 1).min(order.len())]);
    }
    let mut reads = Vec::new();
    assert!(matches!(
        expression(444, &mut |index| {
            reads.push(index);
            Err(MotionUpdateError::MissingOwner("breakout owner"))
        }),
        Err(MotionUpdateError::MissingOwner("breakout owner"))
    ));
    assert_eq!(reads, [368]);
}

#[test]
fn ordered_air_entry_leaves_use_actual_condition_indices() {
    for (expression_index, condition_index) in [
        (157, 245),
        (161, 233),
        (164, 256),
        (166, 258),
        (167, 261),
        (168, 262),
        (175, 268),
        (184, 275),
        (185, 276),
        (186, 277),
        (187, 278),
        (188, 279),
        (189, 280),
        (190, 281),
    ] {
        for expected in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(expression_index, &mut |i| {
                    reads.push(i);
                    Ok(expected)
                })
                .unwrap(),
                expected
            );
            assert_eq!(reads, [condition_index]);
        }
        assert!(matches!(
            expression(expression_index, &mut |_| Err(
                MotionUpdateError::MissingOwner("Air condition")
            )),
            Err(MotionUpdateError::MissingOwner("Air condition"))
        ));
    }
}
#[test]
fn air_sibling_search_reaches_actual_default_after_ordered_rejections() {
    let mut reads = Vec::new();
    let result = evaluate(
        &crate::generated_motion_load::data::STATES,
        21,
        &mut |c, p| {
            reads.push((c, p));
            Ok(c == 223)
        },
    )
    .unwrap();
    assert_eq!(result, 691);
    assert_eq!(
        reads,
        [
            (223, 21),
            (224, 642),
            (225, 642),
            (226, 642),
            (232, 647),
            (245, 658),
            (233, 669),
            (256, 676),
            (223, 78),
            (258, 248),
            (261, 249),
            (262, 685),
            (268, 687),
            (223, 79),
            (275, 690)
        ]
    );
}
#[test]
fn adjacent_air_transition_operands_remain_lazy() {
    for (index, first, second) in [(292, 9, 85), (335, 223, 139)] {
        let mut reads = Vec::new();
        assert!(
            !expression(index, &mut |i| {
                reads.push(i);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [first]);
        reads.clear();
        assert!(
            expression(index, &mut |i| {
                reads.push(i);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(reads, [first, second]);
    }
}

#[test]
fn air_default_callback_slots_match_shared_native_thunk() {
    use crate::generated_motion_load::data::{BEHAVIOURS, STATES};
    for (leaf, animation) in [(690, 1359), (691, 1360)] {
        let mut parents = Vec::new();
        let mut current = leaf;
        while current != u16::MAX {
            parents.push(current);
            current = STATES[usize::from(current)].parent;
        }
        let indices = BEHAVIOURS
            .iter()
            .enumerate()
            .filter(|(_, b)| parents.contains(&b.parent))
            .map(|(i, _)| u16::try_from(i).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            indices,
            [
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                650, 651, 652, animation
            ]
        );
        assert_eq!(STATES[usize::from(leaf)].update_address, 0x828f_1898);
        assert_eq!(BEHAVIOURS[650].name, "ControlAirLegExtension");
    }
}

#[test]
fn connected_air_transition_checks_keep_source_short_circuit_and_negation() {
    for (index, first, second, negate) in [
        (336, 240, 159, false),
        (337, 244, 159, false),
        (338, 160, 87, false),
        (339, 89, 282, true),
    ] {
        for a in [false, true] {
            for b in [false, true] {
                let mut reads = Vec::new();
                let actual = expression(index, &mut |i| {
                    reads.push(i);
                    Ok(if i == first {
                        a
                    } else {
                        assert_eq!(i, second);
                        b
                    })
                })
                .unwrap();
                assert_eq!(actual, a && (b != negate));
                assert_eq!(reads, if a { vec![first, second] } else { vec![first] });
            }
        }
        let mut reads = Vec::new();
        assert!(
            !expression(index, &mut |i| {
                reads.push(i);
                if i == first {
                    Ok(false)
                } else {
                    Err(MotionUpdateError::MissingOwner("late owner"))
                }
            })
            .unwrap()
        );
        assert_eq!(reads, [first]);
        assert!(matches!(
            expression(index, &mut |i| if i == first {
                Ok(true)
            } else {
                Err(MotionUpdateError::MissingOwner("late owner"))
            }),
            Err(MotionUpdateError::MissingOwner("late owner"))
        ));
    }
}
#[test]
fn air_stance_and_direct_manual_target_gates_use_existing_intents() {
    for (index, condition, negate) in [
        (191, 218, false),
        (192, 218, true),
        (242, 350, false),
        (243, 351, true),
    ] {
        for answer in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(index, &mut |i| {
                    reads.push(i);
                    Ok(answer)
                })
                .unwrap(),
                answer != negate
            );
            assert_eq!(reads, [condition]);
        }
    }
}

#[test]
fn landing_normal_children_try_spin_then_sketchy_before_straight() {
    let states = &crate::generated_motion_load::data::STATES;
    for (spin, sketchy, expected, reads) in [
        (true, false, 261, vec![283]),
        (false, true, 262, vec![283, 284]),
        (false, false, 263, vec![283, 284]),
    ] {
        let mut actual = Vec::new();
        assert_eq!(
            evaluate(states, 82, &mut |condition, _| {
                actual.push(condition);
                Ok(match condition {
                    283 => spin,
                    284 => sketchy,
                    _ => panic!("unexpected condition"),
                })
            })
            .unwrap(),
            expected
        );
        assert_eq!(actual, reads);
    }
    for index in [193, 194] {
        assert!(
            super::super::sustain::expression(index, &mut |_| panic!("constant Sustain")).unwrap()
        );
        assert!(matches!(
            expression(index, &mut |_| Err(MotionUpdateError::MissingOwner(
                "landing physical"
            ))),
            Err(MotionUpdateError::MissingOwner("landing physical"))
        ));
    }
}

#[test]
fn landing_selected_thunks_expand_shared_parent_once() {
    use crate::generated_motion_load::data::{BEHAVIOURS, STATES};
    for (leaf, address, tail) in [
        (261, 0x828f_0068, vec![683]),
        (262, 0x828f_0008, vec![684, 685]),
        (263, 0x828f_0068, vec![686]),
    ] {
        let mut parents = Vec::new();
        let mut current = leaf;
        while current != u16::MAX {
            parents.push(current);
            current = STATES[usize::from(current)].parent;
        }
        assert_eq!(parents, [leaf, 82, 22, 3, 0]);
        let actual = BEHAVIOURS
            .iter()
            .enumerate()
            .filter(|(_, b)| parents.contains(&b.parent))
            .map(|(i, _)| u16::try_from(i).unwrap())
            .collect::<Vec<_>>();
        let mut expected = (0..4).chain(17..33).chain(77..85).collect::<Vec<u16>>();
        expected.extend(tail);
        assert_eq!(actual, expected);
        assert_eq!(STATES[usize::from(leaf)].update_address, address);
    }
    assert_eq!(BEHAVIOURS[684].name, "ChooseRandomSketchyLanding");
    for i in [683, 685, 686] {
        assert_eq!(BEHAVIOURS[i].name, "PlayAnimation");
    }
}

#[test]
fn handplant_selection_defers_animation_gate_until_actual_state_is_true() {
    for (index, handplant) in [(351, 326), (352, 328)] {
        for active in [false, true] {
            let mut reads = Vec::new();
            let result = expression(index, &mut |condition| {
                reads.push(condition);
                if condition == handplant {
                    Ok(active)
                } else {
                    Err(MotionUpdateError::PendingCondition(condition))
                }
            });
            if active {
                assert!(matches!(
                    result,
                    Err(MotionUpdateError::PendingCondition(216))
                ));
                assert_eq!(reads, [handplant, 216]);
            } else {
                assert!(matches!(result, Ok(false)));
                assert_eq!(reads, [handplant]);
            }
        }
    }
}

#[test]
fn grabbing_into_411_preserves_native_pre_order_and_sustain_constant() {
    for (answers, expected_reads, expected) in [
        (
            [(268, false), (271, true), (269, true)].as_slice(),
            vec![268],
            false,
        ),
        (
            [(268, true), (271, true), (269, false)].as_slice(),
            vec![268, 271],
            true,
        ),
        (
            [(268, true), (271, false), (269, true)].as_slice(),
            vec![268, 271, 269],
            true,
        ),
        (
            [(268, true), (271, false), (269, false)].as_slice(),
            vec![268, 271, 269],
            false,
        ),
    ] {
        let mut reads = Vec::new();
        let actual = expression(411, &mut |index| {
            reads.push(index);
            Ok(answers.iter().find(|(i, _)| *i == index).unwrap().1)
        })
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(reads, expected_reads);
    }
    for failure in [268, 271, 269] {
        let mut reads = Vec::new();
        assert!(matches!(
            expression(411, &mut |index| {
                reads.push(index);
                if index == failure {
                    Err(MotionUpdateError::PendingCondition(index))
                } else {
                    Ok(index == 268)
                }
            }),
            Err(MotionUpdateError::PendingCondition(index)) if index == failure
        ));
        assert_eq!(
            reads,
            match failure {
                268 => vec![268],
                271 => vec![268, 271],
                _ => vec![268, 271, 269],
            }
        );
    }
    assert!(super::super::sustain::expression(411, &mut |_| panic!("constant Sustain")).unwrap());
}

#[test]
fn crail_grab_into_180_reads_only_its_own_cached_condition() {
    for expected in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(180, &mut |index| {
                reads.push(index);
                Ok(expected)
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, [273]);
    }
    assert!(matches!(
        expression(180, &mut |index| {
            assert_eq!(index, 273);
            Err(MotionUpdateError::PendingCondition(index))
        }),
        Err(MotionUpdateError::PendingCondition(273))
    ));
    assert!(super::super::sustain::expression(180, &mut |_| panic!("constant Sustain")).unwrap());
}

#[test]
fn filtered_board_adjust_transition_410_is_lazy_and_negates_down() {
    for (answers, expected_reads, expected) in [
        ([(259, false), (260, true), (268, false)], vec![259], false),
        (
            [(259, true), (260, false), (268, false)],
            vec![259, 260],
            false,
        ),
        (
            [(259, true), (260, true), (268, false)],
            vec![259, 260, 268],
            true,
        ),
        (
            [(259, true), (260, true), (268, true)],
            vec![259, 260, 268],
            false,
        ),
    ] {
        let mut reads = Vec::new();
        let actual = expression(410, &mut |index| {
            reads.push(index);
            Ok(answers.iter().find(|(i, _)| *i == index).unwrap().1)
        })
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(reads, expected_reads);
    }
    for failure in [259, 260, 268] {
        let mut reads = Vec::new();
        assert!(matches!(
            expression(410, &mut |index| {
                reads.push(index);
                if index == failure {
                    Err(MotionUpdateError::PendingCondition(index))
                } else {
                    Ok(index != 268)
                }
            }),
            Err(MotionUpdateError::PendingCondition(index)) if index == failure
        ));
        assert_eq!(
            reads,
            match failure {
                259 => vec![259],
                260 => vec![259, 260],
                _ => vec![259, 260, 268],
            }
        );
    }
    assert!(super::super::sustain::expression(410, &mut |_| panic!("constant Sustain")).unwrap());
}

#[test]
fn fs_grab_cycle_427_keeps_native_lazy_condition_order() {
    for (answers, expected_reads, expected) in [
        (
            [(232, true), (235, false), (233, false), (234, false)],
            vec![232],
            true,
        ),
        (
            [(232, false), (235, true), (233, false), (234, false)],
            vec![232, 235],
            true,
        ),
        (
            [(232, false), (235, false), (233, false), (234, true)],
            vec![232, 235, 233],
            false,
        ),
        (
            [(232, false), (235, false), (233, true), (234, false)],
            vec![232, 235, 233, 234],
            false,
        ),
        (
            [(232, false), (235, false), (233, true), (234, true)],
            vec![232, 235, 233, 234],
            true,
        ),
    ] {
        let mut reads = Vec::new();
        let actual = expression(427, &mut |index| {
            reads.push(index);
            Ok(answers
                .iter()
                .find(|(candidate, _)| *candidate == index)
                .unwrap()
                .1)
        })
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(reads, expected_reads);
    }
    assert!(super::super::sustain::expression(427, &mut |_| panic!("constant Sustain")).unwrap());
}

#[test]
fn fs_grab_cycle_exit_435_short_circuits_tweak_and_contact_checks() {
    for (answers, expected_reads, expected) in [
        (
            [
                (4, true),
                (87, false),
                (236, false),
                (227, false),
                (85, true),
            ],
            vec![4],
            true,
        ),
        (
            [
                (4, false),
                (87, true),
                (236, false),
                (227, false),
                (85, true),
            ],
            vec![4, 87],
            true,
        ),
        (
            [
                (4, false),
                (87, false),
                (236, false),
                (227, false),
                (85, true),
            ],
            vec![4, 87, 236],
            false,
        ),
        (
            [
                (4, false),
                (87, false),
                (236, true),
                (227, true),
                (85, true),
            ],
            vec![4, 87, 236, 227],
            true,
        ),
        (
            [
                (4, false),
                (87, false),
                (236, true),
                (227, false),
                (85, false),
            ],
            vec![4, 87, 236, 227, 85],
            true,
        ),
        (
            [
                (4, false),
                (87, false),
                (236, true),
                (227, false),
                (85, true),
            ],
            vec![4, 87, 236, 227, 85],
            false,
        ),
    ] {
        let mut reads = Vec::new();
        let actual = expression(435, &mut |index| {
            reads.push(index);
            Ok(answers
                .iter()
                .find(|(candidate, _)| *candidate == index)
                .unwrap()
                .1)
        })
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(reads, expected_reads);
    }
    assert!(super::super::sustain::expression(435, &mut |_| panic!("constant Sustain")).unwrap());
}
#[test]
fn ground_fs_grab_235_reads_its_single_condition_in_both_masks() {
    for expected in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(235, &mut |index| {
                reads.push(index);
                Ok(expected)
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, [342]);
        let mut reads = Vec::new();
        assert_eq!(
            super::super::sustain::expression(235, &mut |index| {
                reads.push(index);
                Ok(expected)
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, [342]);
    }
}

#[test]
fn to_dbl_grab_312_reads_coffin_only_after_missing_dbl_grab() {
    for (dbl, coffin, expected_reads, expected) in [
        (true, false, vec![249], true),
        (false, true, vec![249, 250], true),
        (false, false, vec![249, 250], false),
    ] {
        for sustain in [false, true] {
            let mut reads = Vec::new();
            let mut condition = |index| {
                reads.push(index);
                Ok(if index == 249 { dbl } else { coffin })
            };
            let actual = if sustain {
                super::super::sustain::expression(312, &mut condition)
            } else {
                expression(312, &mut condition)
            }
            .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(reads, expected_reads);
        }
    }
}

#[test]
fn fs_christ_140_pre_reads_intent_and_sustain_is_constant() {
    for expected in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(140, &mut |index| {
                reads.push(index);
                Ok(expected)
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, [230]);
    }
    assert!(super::super::sustain::expression(140, &mut |_| panic!("constant Sustain")).unwrap());
}

#[test]
fn one_foot_air_305_through_308_keep_distinct_lazy_intent_pairs() {
    for (expression_index, first, second) in [
        (305, 232, 240),
        (306, 232, 244),
        (307, 245, 240),
        (308, 245, 244),
    ] {
        for (first_result, second_result, expected_reads, expected) in [
            (false, true, vec![first], false),
            (true, false, vec![first, second], false),
            (true, true, vec![first, second], true),
        ] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(expression_index, &mut |index| {
                    reads.push(index);
                    Ok(if index == first {
                        first_result
                    } else {
                        second_result
                    })
                })
                .unwrap(),
                expected
            );
            assert_eq!(reads, expected_reads);
        }
        assert!(
            super::super::sustain::expression(expression_index, &mut |_| panic!(
                "OneFootAir Sustain is constant"
            ))
            .unwrap()
        );
    }
}

#[test]
fn fs_grab_finger_flip_candidate_gates_keep_native_reads() {
    for (expression_index, condition_index) in [(150, 246), (151, 248)] {
        for expected in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(expression_index, &mut |index| {
                    reads.push(index);
                    Ok(expected)
                })
                .unwrap(),
                expected
            );
            assert_eq!(reads, [condition_index]);
        }
    }
    for (shuv, grab, expected_reads, expected) in [
        (false, true, vec![234], false),
        (true, false, vec![234, 232], false),
        (true, true, vec![234, 232], true),
    ] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(311, &mut |index| {
                reads.push(index);
                Ok(if index == 234 { shuv } else { grab })
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, expected_reads);
    }
    for expression_index in [150, 151, 311] {
        assert!(
            super::super::sustain::expression(expression_index, &mut |_| panic!(
                "FingerFlip candidate Sustain is constant"
            ))
            .unwrap()
        );
    }
}

#[test]
fn fs_grab_tweak_exit_136_reads_condition_229_once() {
    for expected in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(136, &mut |index| {
                reads.push(index);
                Ok(expected)
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, [229]);
    }
    assert!(super::super::sustain::expression(136, &mut |_| panic!("constant Sustain")).unwrap());
}

#[test]
fn fs_grab_out_152_negates_the_cached_fs_grab_intent() {
    for present in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(152, &mut |index| {
                reads.push(index);
                Ok(present)
            })
            .unwrap(),
            !present
        );
        assert_eq!(reads, [232]);
    }
    assert!(super::super::sustain::expression(152, &mut |_| panic!("constant Sustain")).unwrap());
}

#[test]
fn brake_out_217_negates_the_cached_brake_intent() {
    for braking in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(217, &mut |index| {
                reads.push(index);
                Ok(braking)
            })
            .unwrap(),
            !braking
        );
        assert_eq!(reads, [311]);
    }
    assert!(super::super::sustain::expression(217, &mut |_| panic!("constant Sustain")).unwrap());
}

#[test]
fn flip_family_release_preserves_urgency_body_and_hold_order() {
    for (first, second, third, hold) in [
        (391, 392, 297, 172),
        (393, 394, 298, 187),
        (395, 396, 299, 189),
    ] {
        for (expr, urgency) in [(first, Some(169)), (second, Some(170)), (third, None)] {
            for (urgent, flipping, held) in [
                (true, false, true),
                (false, true, true),
                (false, false, true),
                (false, false, false),
            ] {
                let mut reads = Vec::new();
                let result = expression(expr, &mut |index| {
                    reads.push(index);
                    Ok(if Some(index) == urgency {
                        urgent
                    } else if index == 160 {
                        flipping
                    } else {
                        assert_eq!(index, hold);
                        held
                    })
                })
                .unwrap();
                let mut expected = Vec::new();
                if let Some(index) = urgency {
                    expected.push(index);
                }
                if urgency.is_none() || !urgent {
                    expected.push(160);
                    if !flipping {
                        expected.push(hold);
                    }
                }
                assert_eq!(reads, expected);
                assert_eq!(result, (urgency.is_some() && urgent) || flipping || !held);
            }
            assert!(
                super::super::sustain::expression(expr, &mut |_| panic!("constant source Sustain"))
                    .unwrap()
            );
        }
    }
    for (expr, intent) in [(94, 171), (108, 186), (109, 188)] {
        for present in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(expr, &mut |index| {
                    reads.push(index);
                    Ok(present)
                })
                .unwrap(),
                present
            );
            assert_eq!(reads, [intent]);
        }
        assert!(
            super::super::sustain::expression(expr, &mut |_| panic!("constant source Sustain"))
                .unwrap()
        );
    }
}

#[test]
fn bs_grab_selection_preserves_source_short_circuit() {
    for (values, expected, expected_reads) in [
        ([true, false, false, false], true, vec![245]),
        ([false, true, false, false], true, vec![245, 235]),
        ([false, false, false, true], false, vec![245, 235, 233]),
        ([false, false, true, true], true, vec![245, 235, 233, 234]),
        ([false, false, true, false], false, vec![245, 235, 233, 234]),
    ] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(428, &mut |i| {
                reads.push(i);
                Ok(values[[245, 235, 233, 234].iter().position(|v| *v == i).unwrap()])
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, expected_reads);
    }
    for present in [false, true] {
        assert_eq!(
            expression(159, &mut |i| {
                assert_eq!(i, 245);
                Ok(present)
            })
            .unwrap(),
            !present
        );
    }
    for expr in [141, 159, 315, 428] {
        assert!(
            super::super::sustain::expression(expr, &mut |_| panic!("constant native Sustain"))
                .unwrap()
        );
    }
}

#[test]
fn nollie_preceding_candidates_read_their_authored_intent_once() {
    for (expr, condition) in (95..=107).zip(173..=185) {
        for present in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(expr, &mut |index| {
                    reads.push(index);
                    Ok(present)
                })
                .unwrap(),
                present
            );
            assert_eq!(reads, [condition]);
        }
        assert!(
            super::super::sustain::expression(expr, &mut |_| panic!(
                "native Sustain is constant true"
            ))
            .unwrap()
        );
    }
}

#[test]
fn anticipation_and_force_brake_pre_and_sustain_read_real_conditions() {
    for (index, source) in [(42, 93), (260, 382), (261, 387)] {
        for sustain in [false, true] {
            for value in [false, true] {
                let mut reads = Vec::new();
                let mut read = |i| {
                    reads.push(i);
                    Ok(value)
                };
                let result = if sustain {
                    super::super::sustain::expression(index, &mut read)
                } else {
                    expression(index, &mut read)
                };
                assert_eq!(result.unwrap(), value);
                assert_eq!(reads, [source]);
            }
            let mut fail = |i| {
                assert_eq!(i, source);
                Err(MotionUpdateError::MissingOwner("intent owner"))
            };
            let result = if sustain {
                super::super::sustain::expression(index, &mut fail)
            } else {
                expression(index, &mut fail)
            };
            assert!(matches!(
                result,
                Err(MotionUpdateError::MissingOwner("intent owner"))
            ));
        }
    }
}

#[test]
fn anticipation_shuvit_siblings_preserve_lazy_pre_and_sustain_reads() {
    for (index, a, b) in [
        (359, 383, 384),
        (360, 385, 386),
        (361, 388, 389),
        (362, 390, 391),
    ] {
        for sustain in [false, true] {
            for (first, second) in [(false, false), (false, true), (true, false), (true, true)] {
                let mut reads = Vec::new();
                let mut read = |i| {
                    reads.push(i);
                    Ok(if i == a {
                        first
                    } else {
                        assert_eq!(i, b);
                        second
                    })
                };
                let result = if sustain {
                    super::super::sustain::expression(index, &mut read)
                } else {
                    expression(index, &mut read)
                };
                assert_eq!(result.unwrap(), first || second);
                assert_eq!(reads, if first { vec![a] } else { vec![a, b] });
            }
        }
    }
    for index in [258, 259, 381, 439, 440] {
        assert!(
            super::super::sustain::expression(index, &mut |_| panic!("native constant Sustain"))
                .unwrap()
        );
    }
}

#[test]
fn manual_exit250_negates_only_the_loaded_intent_condition() {
    for answer in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(250, &mut |i| {
                reads.push(i);
                Ok(answer)
            })
            .unwrap(),
            !answer
        );
        assert_eq!(reads, [356]);
    }
}

#[test]
fn expression398_preserves_velocity_then_two_physical_exclusions() {
    for (values, expected, count) in [
        ([true, true, true], true, 1),
        ([false, true, false], false, 2),
        ([false, false, true], false, 3),
        ([false, false, false], true, 3),
    ] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(398, &mut |id| {
                reads.push(id);
                Ok(values[match id {
                    207 => 0,
                    89 => 1,
                    85 => 2,
                    _ => panic!("unexpected condition"),
                }])
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, [207, 89, 85][..count]);
    }
}

#[test]
fn mount_batch_expressions_preserve_order_and_native_sustain_defaults() {
    for (id, condition) in [(117, 197), (143, 237)] {
        let mut reads = Vec::new();
        assert!(
            expression(id, &mut |c| {
                reads.push(c);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(reads, [condition]);
    }
    for (id, first, second, negate) in [(300, 89, 85, true), (322, 262, 263, false)] {
        for a in [false, true] {
            for b in [false, true] {
                let mut reads = Vec::new();
                let result = expression(id, &mut |c| {
                    reads.push(c);
                    Ok(if c == first {
                        a
                    } else {
                        assert_eq!(c, second);
                        b
                    })
                })
                .unwrap();
                let expected = if negate { !a && !b } else { a && b };
                assert_eq!(result, expected);
                assert_eq!(
                    reads,
                    if a != negate {
                        vec![first, second]
                    } else {
                        vec![first]
                    }
                );
            }
        }
    }
    for id in [117, 143, 300, 322] {
        assert!(
            super::super::sustain::expression(id, &mut |_| panic!("native Sustain default"))
                .unwrap()
        );
    }
}

#[test]
fn boneless_batch_intent_expressions_keep_their_independent_sustain_masks() {
    for (id, condition_id) in [(118, 198), (171, 266), (181, 274), (198, 287)] {
        for value in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(id, &mut |c| {
                    reads.push(c);
                    Ok(value)
                })
                .unwrap(),
                value
            );
            assert_eq!(reads, [condition_id]);
        }
    }
    for id in [118, 171, 198] {
        assert!(super::super::sustain::expression(id, &mut |_| panic!("native default")).unwrap());
    }
    for value in [false, true] {
        let mut reads = Vec::new();
        assert_eq!(
            super::super::sustain::expression(181, &mut |c| {
                reads.push(c);
                Ok(value)
            })
            .unwrap(),
            value
        );
        assert_eq!(reads, [274]);
    }
}

#[test]
fn tweak409_checks_filtered_pair_before_excluding_raw_intent() {
    for (values, expected, count) in [
        ([false, true, false], false, 1),
        ([true, false, false], false, 2),
        ([true, true, true], false, 3),
        ([true, true, false], true, 3),
    ] {
        let mut reads = Vec::new();
        assert_eq!(
            expression(409, &mut |id| {
                reads.push(id);
                Ok(values[match id {
                    259 => 0,
                    260 => 1,
                    262 => 2,
                    _ => panic!("unexpected condition"),
                }])
            })
            .unwrap(),
            expected
        );
        assert_eq!(reads, [259, 260, 262][..count]);
    }
    assert!(super::super::sustain::expression(409, &mut |_| panic!("native default")).unwrap());
}

#[test]
fn superman_and_one_foot_tail_guards_keep_native_reads() {
    for (id, condition, negated) in [
        (138, 226, false),
        (139, 226, true),
        (145, 241, false),
        (146, 240, true),
        (147, 242, false),
        (148, 243, false),
        (149, 244, true),
        (170, 265, true),
        (174, 263, true),
    ] {
        for value in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(id, &mut |c| {
                    reads.push(c);
                    Ok(value)
                })
                .unwrap(),
                value != negated
            );
            assert_eq!(reads, [condition]);
        }
    }
    for (id, foot) in [(323, 265), (324, 244), (325, 240)] {
        let mut reads = Vec::new();
        assert!(
            !expression(id, &mut |c| {
                reads.push(c);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [263]);
        reads.clear();
        assert!(
            expression(id, &mut |c| {
                reads.push(c);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(reads, [263, foot]);
    }
    for id in [138, 139, 145, 146, 147, 148, 149, 170, 174, 323, 324, 325] {
        assert!(super::super::sustain::expression(id, &mut |_| panic!("native default")).unwrap());
    }
}

#[test]
fn hippy_nose_rocket_and_runout_guards_keep_native_reads() {
    for (id, condition, negated) in [
        (38, 86, false),
        (179, 272, false),
        (176, 269, false),
        (177, 270, false),
        (178, 269, true),
        (183, 271, true),
        (264, 397, false),
        (265, 399, false),
        (266, 89, false),
    ] {
        for value in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(id, &mut |c| {
                    reads.push(c);
                    Ok(value)
                })
                .unwrap(),
                value != negated
            );
            assert_eq!(reads, [condition]);
        }
    }
    for (id, foot) in [(329, 265), (330, 244), (331, 240)] {
        let mut reads = Vec::new();
        assert!(
            !expression(id, &mut |c| {
                reads.push(c);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [271]);
        reads.clear();
        assert!(
            expression(id, &mut |c| {
                reads.push(c);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(reads, [271, foot]);
    }
    let mut reads = Vec::new();
    assert!(
        expression(364, &mut |c| {
            reads.push(c);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [362]);
    reads.clear();
    assert!(
        !expression(364, &mut |c| {
            reads.push(c);
            Ok(true)
        })
        .unwrap()
    );
    assert_eq!(reads, [362, 363]);
    reads.clear();
    assert!(
        !expression(365, &mut |c| {
            reads.push(c);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [19]);
    reads.clear();
    assert!(
        expression(365, &mut |c| {
            reads.push(c);
            Ok(c == 19)
        })
        .unwrap()
    );
    assert_eq!(reads, [19, 202, 398]);
    reads.clear();
    assert!(
        super::super::sustain::expression(365, &mut |c| {
            reads.push(c);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [202, 398]);
    for (id, condition) in [(179, 272), (177, 270)] {
        let mut reads = Vec::new();
        assert!(
            super::super::sustain::expression(id, &mut |c| {
                reads.push(c);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(reads, [condition]);
    }
    for id in [1, 38, 176, 178, 183, 264, 265, 266, 329, 330, 331, 364] {
        assert!(super::super::sustain::expression(id, &mut |_| panic!("native default")).unwrap());
    }
}

#[test]
fn mute_stale_fingerflip_and_planting_foot_guards_keep_native_reads() {
    for (id, grab) in [(318, 233), (319, 256)] {
        let mut reads = Vec::new();
        assert!(
            !expression(id, &mut |c| {
                reads.push(c);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [234]);
        reads.clear();
        assert!(
            expression(id, &mut |c| {
                reads.push(c);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(reads, [234, grab]);
    }
    for (id, condition, negated) in [
        (162, 254, false),
        (163, 233, true),
        (165, 256, true),
        (123, 205, false),
        (124, 206, false),
    ] {
        for value in [false, true] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(id, &mut |c| {
                    reads.push(c);
                    Ok(value)
                })
                .unwrap(),
                value != negated
            );
            assert_eq!(reads, [condition]);
        }
    }
    for (id, intent) in [(397, 203), (399, 208), (400, 209), (401, 210), (402, 211)] {
        let mut reads = Vec::new();
        assert!(
            !expression(id, &mut |c| {
                reads.push(c);
                Ok(c == 202)
            })
            .unwrap()
        );
        assert_eq!(reads, [202, intent]);
        reads.clear();
        assert!(
            expression(id, &mut |c| {
                reads.push(c);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(reads, [202, intent, 204]);
        reads.clear();
        assert!(
            !super::super::sustain::expression(id, &mut |c| {
                reads.push(c);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [202]);
    }
    for id in [123, 124, 163, 165, 318, 319] {
        assert!(super::super::sustain::expression(id, &mut |_| panic!("native default")).unwrap());
    }
}
#[test]
fn skitch_selection_preserves_lazy_order_and_independent_sustain() {
    for (id, condition) in [
        (204, 294),
        (205, 295),
        (206, 297),
        (207, 298),
        (209, 299),
        (210, 300),
    ] {
        let mut reads = Vec::new();
        assert!(
            expression(id, &mut |c| {
                reads.push(c);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(reads, [condition]);
    }
    let mut reads = Vec::new();
    assert!(
        !expression(208, &mut |c| {
            reads.push(c);
            Ok(true)
        })
        .unwrap()
    );
    assert_eq!(reads, [298]);
    reads.clear();
    assert!(
        expression(208, &mut |c| {
            reads.push(c);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [298, 297]);
    reads.clear();
    assert!(
        !expression(342, &mut |c| {
            reads.push(c);
            Ok(false)
        })
        .unwrap()
    );
    assert_eq!(reads, [296]);
    reads.clear();
    assert!(
        expression(342, &mut |c| {
            reads.push(c);
            Ok(c == 296)
        })
        .unwrap()
    );
    assert_eq!(reads, [296, 295]);
    for id in [202, 204, 205, 206, 207, 208, 209, 210, 342] {
        assert!(
            super::super::sustain::expression(id, &mut |_| panic!(
                "source default has no condition reads"
            ))
            .unwrap()
        );
    }
}
