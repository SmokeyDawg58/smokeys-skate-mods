use super::*;
use crate::input_cvar_runtime::InputCvarManager;
use crate::reckoning_numeric::HostReckoningNumeric;

#[test]
fn fixed_foot_targets_keep_native_left_right_height_asymmetry() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let original = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
    let animation = SkeletonRootVector::known([0.1, 1.1, 0.0, 0.0]);
    let mut left_fixed = original;
    let mut right_fixed = original;
    let mut left = original;
    let mut right = original;
    calculate_fixed_foot_target(true, animation, &mut left_fixed, &mut left, &refs).unwrap();
    calculate_fixed_foot_target(false, animation, &mut right_fixed, &mut right, &refs).unwrap();
    // Source left clamp is horizontal; right uses full distance, then restores Y.
    assert!((left.xyz[0] - 0.02).abs() < 1e-6);
    assert!((right.xyz[0] - (0.1 - 0.08 / 2.0_f32.sqrt())).abs() < 1e-6);
    assert_eq!(left.xyz[1], 1.0);
    assert_eq!(right.xyz[1], 1.0);
    assert_eq!(left_fixed, left);
    assert_eq!(right_fixed, right);
}

#[test]
fn fixed_foot_gate_ignores_height_and_retains_anchor_below_range() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let original = SkeletonRootVector { xyz: [0.0, 1.0, 0.0], w: None };
    for left in [true, false] {
        let mut fixed = original;
        let mut target = SkeletonRootVector::known([99.0; 4]);
        calculate_fixed_foot_target(
            left,
            SkeletonRootVector::known([0.01, 20.0, 0.0, 0.0]),
            &mut fixed,
            &mut target,
            &refs,
        ).unwrap();
        assert_eq!(fixed, original);
        assert_eq!(target, original);
    }
}

#[test]
fn calculate_new_foot_y_reads_live_limits_in_native_order() {
    struct Source(Vec<&'static str>);
    impl NewFootYSource for Source {
        type Error = ();
        fn minimum_height(&mut self) -> Result<f32, Self::Error> {
            self.0.push("minimum");
            Ok(0.02)
        }
        fn maximum_lift(&mut self) -> Result<f32, Self::Error> {
            self.0.push("lift");
            Ok(0.08)
        }
        fn maximum_up_distance(&mut self) -> Result<f32, Self::Error> {
            self.0.push("distance");
            Ok(0.4)
        }
    }
    let mut source = Source(Vec::new());
    // A foot at the animation-space origin rests at least 0.02 above ground.
    assert_eq!(calculate_new_foot_y(0.0, 0.0, 0.0, &mut source), Ok(0.02));
    assert_eq!(source.0, ["minimum", "lift", "distance"]);

    // A world-space animation point above the lift ceiling is capped at 0.10.
    assert!((calculate_new_foot_y(0.0, 0.0, -1.0, &mut source).unwrap() - 0.1).abs() < 1e-6);
    // A correction over 0.4 leaves the original animation height unchanged.
    assert_eq!(calculate_new_foot_y(0.0, -0.5, -0.5, &mut source), Ok(-0.5));
}

#[test]
fn calculate_new_foot_y_cvars_are_linked_and_mutable() {
    let refs = InputCvarReferences::new_static_initialized();
    assert_eq!(
        refs.try_y_min_height_to_be_off_ground(),
        Err(InputCvarError::Unlinked("kYMinHeightToBeOffGround"))
    );
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    assert_eq!(
        refs.try_y_min_height_to_be_off_ground().unwrap().to_bits(),
        0x3ca3_d70a
    );
    assert_eq!(
        refs.try_ob_ik_max_y_lift_up().unwrap().to_bits(),
        0x3da3_d70a
    );
    assert_eq!(
        refs.try_ob_ik_max_y_dist_up().unwrap().to_bits(),
        0x3ecc_cccd
    );
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\nkYMinHeightToBeOffGround=0.05\r\nkOBIK_IKMaxYLiftUp=0.20\r\nkOBIK_IKMaxYDistUp=0.10\r\n",
        ))
        .unwrap();
    assert_eq!(
        calculate_new_foot_y(0.0, 0.0, 0.0, &mut RetainedNewFootYSource(&refs)),
        Ok(0.05)
    );
    assert_eq!(
        calculate_new_foot_y(0.0, -0.2, -0.2, &mut RetainedNewFootYSource(&refs)),
        Ok(-0.2)
    );
}

#[test]
fn apply_ik_gate_and_blend_are_live_linked_values() {
    let refs = InputCvarReferences::new_static_initialized();
    assert_eq!(
        refs.try_enable_off_board_feet_ik(),
        Err(InputCvarError::Unlinked("kEnableOffBoardFeetIK"))
    );
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    assert_eq!(refs.try_enable_off_board_feet_ik(), Ok(true));
    assert_eq!(refs.try_ob_ik_blend_val().unwrap().to_bits(), 0x3ecc_cccd);
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\nkEnableOffBoardFeetIK=false\r\nkOBIK_IKBlendVal=0.75\r\n",
        ))
        .unwrap();
    assert_eq!(refs.try_enable_off_board_feet_ik(), Ok(false));
    assert_eq!(refs.try_ob_ik_blend_val(), Ok(0.75));
}

#[test]
fn calculate_state2_target_preserves_unknown_w_and_surface_read_order() {
    struct Source(Vec<&'static str>);
    impl NewFootYSource for Source {
        type Error = ();
        fn minimum_height(&mut self) -> Result<f32, Self::Error> {
            self.0.push("minimum");
            Ok(0.02)
        }
        fn maximum_lift(&mut self) -> Result<f32, Self::Error> {
            self.0.push("lift");
            Ok(0.08)
        }
        fn maximum_up_distance(&mut self) -> Result<f32, Self::Error> {
            self.0.push("distance");
            Ok(0.4)
        }
    }
    impl FootTargetSource for Source {
        fn blend_out_time(&mut self) -> Result<f32, Self::Error> {
            self.0.push("blend");
            Ok(0.1)
        }
    }
    let mut source = Source(Vec::new());
    let mut raw = SkeletonRootVector::known([3.0, 0.0, 4.0, 5.0]);
    raw.w = None;
    let mut target = SkeletonRootVector::known([9.0, 9.0, 9.0, 9.0]);
    let mut timer = 0.0;
    calculate_state2_foot_target(
        raw,
        result(0.0, true),
        0.0,
        0.0,
        &mut target,
        &mut timer,
        &mut source,
    )
    .unwrap();
    assert_eq!(source.0, ["minimum", "lift", "distance"]);
    assert_eq!(target.xyz, [3.0, 0.02, 4.0]);
    assert_eq!(target.w, None);
    assert_eq!(timer, 0.0);

    // An invalid surface skips all height cvars. The positive timer alone
    // reads the fixed-foot blend cvar after its native decrement.
    source.0.clear();
    timer = 0.05;
    calculate_state2_foot_target(
        raw,
        result(0.0, false),
        0.0,
        0.0,
        &mut target,
        &mut timer,
        &mut source,
    )
    .unwrap();
    assert_eq!(source.0, ["blend"]);
    assert!((timer - (0.05 - f32::from_bits(0x3c88_8889))).abs() < 1e-7);
    assert_eq!(target.w, None);
}

#[test]
fn calculate_state2_target_keeps_partial_timer_write_on_cvar_error() {
    struct Source;
    impl NewFootYSource for Source {
        type Error = &'static str;
        fn minimum_height(&mut self) -> Result<f32, Self::Error> {
            Ok(0.02)
        }
        fn maximum_lift(&mut self) -> Result<f32, Self::Error> {
            Ok(0.08)
        }
        fn maximum_up_distance(&mut self) -> Result<f32, Self::Error> {
            Ok(0.4)
        }
    }
    impl FootTargetSource for Source {
        fn blend_out_time(&mut self) -> Result<f32, Self::Error> {
            Err("blend unavailable")
        }
    }
    let original = SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]);
    let mut target = original;
    let mut timer = 0.01;
    assert_eq!(
        calculate_state2_foot_target(
            original,
            result(0.0, false),
            0.0,
            0.0,
            &mut target,
            &mut timer,
            &mut Source,
        ),
        Err("blend unavailable")
    );
    assert_eq!(timer, 0.0);
    assert_eq!(target, original);
}

#[test]
fn clamp_foot_normal_uses_live_maximum_reads_and_horizontal_direction() {
    let mut reads = 0;
    let original = SkeletonRootVector::known([3.0, 0.0, 4.0, 5.0]);
    let clamped = try_clamp_foot_normal(
        original,
        || {
            reads += 1;
            Ok::<f32, ()>(match reads {
                1 => 0.5,
                2 => 0.6,
                _ => 0.7,
            })
        },
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(reads, 3);
    // Horizontal (3,4) is length 5; the second threshold selects radius 0.8.
    assert!((clamped.xyz[0] - 0.48).abs() < 1e-6);
    assert_eq!(clamped.xyz[1], 0.7);
    assert!((clamped.xyz[2] - 0.64).abs() < 1e-6);
    assert!((clamped.w.unwrap() - 0.8).abs() < 1e-6);
}

#[test]
fn clamp_foot_normal_skips_clamp_for_high_y_or_zero_horizontal_length() {
    for normal in [
        SkeletonRootVector::known([1.0, 0.9, 0.0, 9.0]),
        SkeletonRootVector::known([0.0, 0.0, 0.0, 9.0]),
    ] {
        let mut reads = 0;
        let actual = try_clamp_foot_normal(
            normal,
            || {
                reads += 1;
                Ok::<f32, ()>(0.9)
            },
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(actual, normal);
        assert_eq!(reads, 1);
    }
}

#[test]
fn feet_down_prefix_uses_local_part_order_and_two_distinct_height_reads() {
    struct Source {
        events: Vec<&'static str>,
    }
    impl FeetDownSource for Source {
        type Error = ();
        fn clamp_normal(
            &mut self,
            left: bool,
            normal: SkeletonRootVector,
        ) -> Result<SkeletonRootVector, Self::Error> {
            self.events.push(if left { "clamp L" } else { "clamp R" });
            Ok(normal)
        }
        fn raw_local_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
            self.events.push(match part {
                15 => "raw toe L",
                19 => "raw toe R",
                16 => "raw foot L",
                20 => "raw foot R",
                _ => unreachable!(),
            });
            let y = match part {
                15 | 16 => 0.05,
                _ => 0.15,
            };
            Ok(SkeletonRootVector::known([part as f32, y, 0.0, 0.0]))
        }
        fn foot_down_height(&mut self) -> Result<f32, Self::Error> {
            self.events.push("height");
            Ok(if self.events.len() == 7 { 0.09 } else { 0.2 })
        }
    }
    let mut source = Source { events: Vec::new() };
    let mut left_normal = SkeletonRootVector::known([0.0; 4]);
    let mut right_normal = left_normal;
    let mut left_down = false;
    let mut right_down = false;
    let raw = calculate_feet_down_prefix(
        &mut left_normal,
        &mut right_normal,
        &mut left_down,
        &mut right_down,
        &result(0.0, true),
        &result(0.0, true),
        &mut source,
    )
    .unwrap();
    assert_eq!(
        source.events,
        [
            "clamp L",
            "clamp R",
            "raw toe L",
            "raw toe R",
            "raw foot L",
            "raw foot R",
            "height",
            "height",
        ]
    );
    assert_eq!(
        raw.map(|position| position.xyz[0]),
        [15.0, 19.0, 16.0, 20.0]
    );
    assert!(left_down && right_down);
}

#[test]
fn feet_down_prefix_uses_strict_toe_and_foot_thresholds() {
    struct Source {
        positions: [f32; 4],
        next: usize,
    }
    impl FeetDownSource for Source {
        type Error = ();
        fn clamp_normal(
            &mut self,
            _: bool,
            normal: SkeletonRootVector,
        ) -> Result<SkeletonRootVector, Self::Error> {
            Ok(normal)
        }
        fn raw_local_position(&mut self, _: usize) -> Result<SkeletonRootVector, Self::Error> {
            let y = self.positions[self.next];
            self.next += 1;
            Ok(SkeletonRootVector::known([0.0, y, 0.0, 0.0]))
        }
        fn foot_down_height(&mut self) -> Result<f32, Self::Error> {
            Ok(0.09)
        }
    }
    let mut left_normal = SkeletonRootVector::known([0.0; 4]);
    let mut right_normal = left_normal;
    let mut left_down = true;
    let mut right_down = true;
    calculate_feet_down_prefix(
        &mut left_normal,
        &mut right_normal,
        &mut left_down,
        &mut right_down,
        &result(0.0, true),
        &result(0.0, true),
        &mut Source {
            // L toe exactly equals threshold; R foot exactly equals threshold.
            positions: [0.09, 0.08, 0.08, 0.09],
            next: 0,
        },
    )
    .unwrap();
    assert!(!left_down && !right_down);
}

#[test]
fn feet_down_part_queries_keep_raw_positions_and_read_four_fresh_transforms() {
    struct Source(Vec<usize>);
    impl PartTransformSource for Source {
        type Error = ();
        fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
            self.0.push(part);
            Ok(SkeletonRootVector {
                xyz: [part as f32, 2.0, 3.0],
                w: None,
            })
        }
    }
    let raw = [15.0, 19.0, 16.0, 20.0].map(|part| SkeletonRootVector::known([part, 1.0, 0.0, 0.0]));
    let mut source = Source(Vec::new());
    let actual = query_feet_down_part_positions(raw, &mut source).unwrap();
    assert_eq!(source.0, [15, 19, 16, 20]);
    assert_eq!(actual.raw_local, raw);
    assert_eq!(
        actual.part_world.map(|position| position.xyz[0]),
        [15.0, 19.0, 16.0, 20.0]
    );
    assert!(
        actual
            .part_world
            .iter()
            .all(|position| position.w.is_none())
    );
}

#[test]
fn surface_foot_down_uses_farthest_projected_part_and_strict_height() {
    let surface = result(0.0, true);
    let normal = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
    let toe = SkeletonRootVector::known([0.0, 0.05, 0.0, 0.0]);
    let foot = SkeletonRootVector::known([0.0, 0.1, 0.0, 0.0]);
    let mut reads = 0;
    let down = resolve_surface_foot_down(surface, normal, toe, foot, true, || {
        reads += 1;
        Ok::<f32, ()>(0.09)
    })
    .unwrap();
    assert!(!down);
    assert_eq!(reads, 1);
    let down = resolve_surface_foot_down(surface, normal, toe, foot, false, || Ok::<f32, ()>(0.1))
        .unwrap();
    assert!(!down); // equality is not foot-down
    let down = resolve_surface_foot_down(surface, normal, toe, foot, false, || Ok::<f32, ()>(0.11))
        .unwrap();
    assert!(down);
}

#[test]
fn invalid_surface_copies_animation_flag_without_height_read_or_w_guess() {
    let mut surface = result(0.0, false);
    surface.position.w = None;
    let mut toe = SkeletonRootVector::known([0.0, 0.0, 0.0, 0.0]);
    toe.w = None;
    let mut reads = 0;
    let actual = resolve_surface_foot_down(
        surface,
        SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
        toe,
        toe,
        true,
        || {
            reads += 1;
            Ok::<f32, ()>(0.09)
        },
    )
    .unwrap();
    assert!(actual);
    assert_eq!(reads, 0);
}

#[test]
fn processed_foot_fix_masks_force_the_native_priority_and_reload_order() {
    struct Source {
        word: u32,
        gates: Vec<u32>,
        reads: usize,
    }
    impl ProcessedFootFlagsSource for Source {
        type Error = ();
        fn flags_8d0(&mut self) -> Result<u32, Self::Error> {
            Ok(self.word)
        }
        fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
            let gate = self.gates[self.reads];
            self.reads += 1;
            Ok(gate)
        }
    }
    let mut left = false;
    let mut right = true;
    let mut source = Source {
        word: 0x300,
        gates: vec![0],
        reads: 0,
    };
    override_processed_foot_flags(&mut left, &mut right, &mut source).unwrap();
    assert_eq!((left, right, source.reads), (true, false, 1));

    let mut source = Source {
        word: 0x300,
        gates: vec![8, 8],
        reads: 0,
    };
    override_processed_foot_flags(&mut left, &mut right, &mut source).unwrap();
    assert_eq!((left, right, source.reads), (true, false, 2));

    let mut source = Source {
        word: 0x100,
        gates: vec![0, 0],
        reads: 0,
    };
    override_processed_foot_flags(&mut left, &mut right, &mut source).unwrap();
    assert_eq!((left, right, source.reads), (false, true, 2));
}

#[test]
fn processed_foot_fix_skips_mirror_word_when_no_fix_bit_is_set() {
    struct Source;
    impl ProcessedFootFlagsSource for Source {
        type Error = ();
        fn flags_8d0(&mut self) -> Result<u32, Self::Error> {
            Ok(0)
        }
        fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
            panic!("no source read when both foot-fix bits are clear")
        }
    }
    let mut left = true;
    let mut right = false;
    override_processed_foot_flags(&mut left, &mut right, &mut Source).unwrap();
    assert!(left && !right);
}

#[test]
fn feet_ik_update_state_reuses_retained_position_until_a_real_state_change() {
    struct Source {
        events: Vec<&'static str>,
    }
    impl FeetIkUpdateStateSource for Source {
        type Error = ();
        fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
            self.events.push("flags");
            Ok(0)
        }
        fn current_physics_state(&mut self) -> Result<i32, Self::Error> {
            self.events.push("physics");
            Ok(500)
        }
        fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
            assert_eq!(part, 15);
            self.events.push("part15");
            Ok(SkeletonRootVector {
                xyz: [1.0, 2.0, 3.0],
                w: None,
            })
        }
    }
    let mut source = Source { events: Vec::new() };
    let mut state = 2;
    let mut timer = 0.25;
    let mut position = SkeletonRootVector::known([5.0; 4]);
    update_feet_ik_state(
        &mut state,
        &mut timer,
        &mut position,
        false,
        false,
        false,
        &mut source,
    )
    .unwrap();
    assert_eq!(state, 2);
    assert_eq!(
        timer.to_bits(),
        (0.25 + f32::from_bits(0x3c88_8889)).to_bits()
    );
    assert_eq!(position.known_lanes(), Some([5.0; 4]));
    assert_eq!(source.events, ["flags", "physics"]);

    source.events.clear();
    update_feet_ik_state(
        &mut state,
        &mut timer,
        &mut position,
        true,
        true,
        false,
        &mut source,
    )
    .unwrap();
    assert_eq!(state, 0);
    assert_eq!(timer.to_bits(), 0.0f32.to_bits());
    assert_eq!(position.xyz, [1.0, 2.0, 3.0]);
    assert_eq!(position.w, None);
    assert_eq!(source.events, ["flags", "physics", "part15"]);
}

#[test]
fn feet_ik_update_state_locomotion_skips_physics_read_and_zeros_position() {
    struct Source;
    impl FeetIkUpdateStateSource for Source {
        type Error = ();
        fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
            Ok(0x40)
        }
        fn current_physics_state(&mut self) -> Result<i32, Self::Error> {
            panic!("locomotion state skips current physics read")
        }
        fn part_position(&mut self, _: usize) -> Result<SkeletonRootVector, Self::Error> {
            panic!("locomotion state writes zero without a part query")
        }
    }
    let mut state = 0;
    let mut timer = 1.0;
    let mut position = SkeletonRootVector {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    update_feet_ik_state(
        &mut state,
        &mut timer,
        &mut position,
        true,
        true,
        false,
        &mut Source,
    )
    .unwrap();
    assert_eq!(state, 3);
    assert_eq!(timer.to_bits(), 0.0f32.to_bits());
    assert_eq!(position.known_lanes(), Some([0.0; 4]));
}

fn result(y: f32, valid: bool) -> SurfaceResult {
    SurfaceResult {
        position: SkeletonRootVector::known([0.0, y, 0.0, 0.0]),
        normal: SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
        surface_id: 7,
        valid,
    }
}

#[test]
fn update_copies_effective_animation_translation_with_unknown_w() {
    let mut effective = SkeletonRootMatrix::known([
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 7.0, 8.0, 9.0, 1.0,
    ]);
    let mut translation = effective.columns()[3];
    translation.w = None;
    effective.set_column(3, translation);
    let mut last = SkeletonRootVector::known([2.0; 4]);
    let mut valid = false;
    copy_last_good_animation_position(0, Some(effective), &mut last, &mut valid).unwrap();
    assert_eq!(last.xyz, [7.0, 8.0, 9.0]);
    assert_eq!(last.w, None);
    assert!(valid);
}

#[test]
fn update_skips_processed_position_when_bad_correction_is_active() {
    let mut last = SkeletonRootVector::known([2.0, 3.0, 4.0, 5.0]);
    let mut valid = false;
    copy_last_good_animation_position(1, None, &mut last, &mut valid).unwrap();
    assert_eq!(last.known_lanes(), Some([2.0, 3.0, 4.0, 5.0]));
    assert!(!valid);
    assert_eq!(
        copy_last_good_animation_position(0, None, &mut last, &mut valid),
        Err(OffboardFeetIkPrefixError::MissingEffectiveAnimation)
    );
}

#[test]
fn init_update_copies_both_prior_surfaces_before_raw_toe_reads() {
    struct Source {
        events: Vec<&'static str>,
    }
    impl PrefixSource for Source {
        type Error = ();
        fn surface(&mut self, left: bool) -> Result<SurfaceResult, Self::Error> {
            self.events
                .push(if left { "surface L" } else { "surface R" });
            Ok(result(if left { 1.0 } else { 2.0 }, true))
        }
        fn raw_toe_world_position(
            &mut self,
            part: usize,
        ) -> Result<SkeletonRootVector, Self::Error> {
            self.events.push(if part == 15 { "toe L" } else { "toe R" });
            Ok(SkeletonRootVector::known([part as f32; 4]))
        }
    }
    let mut source = Source { events: Vec::new() };
    let mut left = result(0.0, false);
    let mut right = result(0.0, false);
    let mut left_pos = SkeletonRootVector::known([0.0; 4]);
    let mut right_pos = SkeletonRootVector::known([0.0; 4]);
    load_prefix(
        &mut left,
        &mut right,
        &mut left_pos,
        &mut right_pos,
        &mut source,
    )
    .unwrap();
    assert_eq!(source.events, ["surface L", "surface R", "toe L", "toe R"]);
    assert_eq!((left.position.xyz[1], right.position.xyz[1]), (1.0, 2.0));
    assert_eq!((left_pos.xyz[0], right_pos.xyz[0]), (15.0, 19.0));
}

#[test]
fn full_vector_copies_retain_unknown_w_while_y_condition_still_runs() {
    struct Source;
    impl PrefixSource for Source {
        type Error = ();
        fn surface(&mut self, left: bool) -> Result<SurfaceResult, Self::Error> {
            let mut result = result(if left { 0.8 } else { 0.0 }, true);
            result.position.w = None;
            result.normal.w = None;
            Ok(result)
        }
        fn raw_toe_world_position(
            &mut self,
            part: usize,
        ) -> Result<SkeletonRootVector, Self::Error> {
            let mut position = SkeletonRootVector::known([0.0, 0.0, part as f32, 0.0]);
            position.w = None;
            Ok(position)
        }
    }
    let mut left = result(0.0, false);
    let mut right = result(0.0, false);
    let mut left_toe = SkeletonRootVector::known([0.0; 4]);
    let mut right_toe = SkeletonRootVector::known([0.0; 4]);
    load_prefix(
        &mut left,
        &mut right,
        &mut left_toe,
        &mut right_toe,
        &mut Source,
    )
    .unwrap();
    assert_eq!(
        (left.position.w, left.normal.w, left_toe.w, right_toe.w),
        (None, None, None, None)
    );
    condition_ground_results(
        &mut left,
        &mut right,
        left_toe.xyz[1],
        right_toe.xyz[1],
        || Ok::<_, ()>(0.3),
    )
    .unwrap();
    assert!(!left.valid && right.valid);
}

#[test]
fn conditioning_preserves_ties_and_original_validity() {
    let mut left = result(0.3, true);
    let mut right = result(0.0, true);
    condition_ground_results(&mut left, &mut right, 0.0, 0.0, || Ok::<_, ()>(0.3)).unwrap();
    assert!(left.valid && right.valid); // strict outer comparison

    left = result(0.6, true);
    right = result(-0.6, true);
    condition_ground_results(&mut left, &mut right, 0.0, 0.0, || Ok::<_, ()>(0.3)).unwrap();
    assert!(!left.valid && !right.valid); // both see original validity

    left = result(0.6, true);
    right = result(0.0, false);
    condition_ground_results(&mut left, &mut right, 0.0, 0.0, || Ok::<_, ()>(0.3)).unwrap();
    assert!(left.valid && !right.valid); // a missing opposite hit cannot clear L
}

#[test]
fn conditioner_reads_threshold_again_only_after_outer_test_passes() {
    let mut left = result(0.6, true);
    let mut right = result(-0.6, true);
    let mut reads = 0;
    condition_ground_results(&mut left, &mut right, 0.0, 0.0, || {
        reads += 1;
        Ok::<_, ()>(if reads == 1 { 0.3 } else { 0.7 })
    })
    .unwrap();
    assert_eq!(reads, 2);
    assert!(!left.valid && right.valid);

    let mut left = result(0.0, true);
    let mut right = result(0.0, true);
    condition_ground_results(&mut left, &mut right, 0.0, 0.0, || {
        reads += 1;
        Ok::<_, ()>(0.3)
    })
    .unwrap();
    assert_eq!(reads, 3);
}

#[test]
fn conditioning_uses_linked_live_maximum() {
    let refs = InputCvarReferences::new_static_initialized();
    assert_eq!(
        refs.try_ob_ik_max_feet_y_dist(),
        Err(InputCvarError::Unlinked("kOB_IK_MaxFeetYDist"))
    );
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    assert_eq!(
        refs.try_ob_ik_max_feet_y_dist().unwrap().to_bits(),
        0x3e99_999a
    );
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkOB_IK_MaxFeetYDist=0.75\r\n"))
        .unwrap();
    assert_eq!(refs.try_ob_ik_max_feet_y_dist().unwrap(), 0.75);
    let mut left = result(0.5, true);
    let mut right = result(0.0, true);
    condition_ground_results(&mut left, &mut right, 0.0, 0.0, || {
        refs.try_ob_ik_max_feet_y_dist()
    })
    .unwrap();
    assert!(left.valid);
}

#[test]
fn air_height_skips_invalid_surfaces_and_preserves_slope_then_uses_native_fsel() {
    let x = SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]);
    let z = SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]);
    let mut slope = 7.0;
    let mut calls = 0;
    let mut scalar = || {
        calls += 1;
        Ok(0.2)
    };
    assert_eq!(
        air_surface_height(result(3.0, false), 4.0, x, z, 10.0, &mut slope, &mut scalar).unwrap(),
        10.0
    );
    assert_eq!(slope, 7.0);
    let mut surface = result(3.0, true);
    surface.normal = z;
    let height = air_surface_height(surface, 4.0, x, z, 10.0, &mut slope, &mut scalar).unwrap();
    assert_eq!(height, 4.0 - (0.2 + 3.0));
    assert_eq!(slope, 0.2);
    assert_eq!(
        air_surface_height(surface, f32::NAN, x, z, height, &mut slope, &mut scalar).unwrap(),
        height
    );
    assert_eq!(calls, 2);
}
