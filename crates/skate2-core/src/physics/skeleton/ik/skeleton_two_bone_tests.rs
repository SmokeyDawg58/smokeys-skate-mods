//! Frozen values were independently calculated from the audited semantic graph
//! with explicit f32 rounding and ucrtbase.fmaf, including original polynomial
//! tables. They are host-graph fixtures, not a Xenon runtime trace.

use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
use std::{cell::RefCell, rc::Rc};

type Trace = Rc<RefCell<Vec<(&'static str, u32)>>>;

#[derive(Clone, Default)]
struct Services {
    trace: Trace,
    fail_at: Option<usize>,
}
impl Services {
    fn event(&mut self, name: &'static str, input: f32, output: f32) -> Result<f32, &'static str> {
        let index = self.trace.borrow().len();
        self.trace.borrow_mut().push((name, input.to_bits()));
        if self.fail_at == Some(index) {
            Err("refused")
        } else {
            Ok(output)
        }
    }
}
impl ReckoningNumeric for Services {
    type Error = &'static str;
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        let result = HostReckoningNumeric.dot3(left, right).unwrap();
        self.event("dot", result, result)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("rsqrt", value, 1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("reciprocal", value, 1.0 / value)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("the solver never truncates to an integer")
    }
}
impl ReckoningSinCosNumeric for Services {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("round", value, value.round_ties_even())
    }
}

fn point(value: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector::known(value)
}
fn chain() -> TwoBoneChain {
    TwoBoneChain {
        root: point([0.0; 4]),
        middle: point([1.0, 1.0, 0.0, 0.0]),
        end: point([2.0, 0.0, 0.0, 0.0]),
    }
}
fn limits(minimum: f32, maximum: f32) -> TwoBoneLimits {
    TwoBoneLimits {
        minimum_degrees: minimum,
        maximum_degrees: maximum,
        override_maximum_with_original: false,
    }
}
fn sentinel() -> SkeletonRootVector {
    point([31.0, 32.0, 33.0, 34.0])
}
fn assert_bits(actual: SkeletonRootVector, expected: [u32; 4]) {
    assert_eq!(actual.xyz.map(f32::to_bits), expected[..3]);
    assert_eq!(actual.w.map(f32::to_bits), Some(expected[3]));
}
fn assert_same(actual: SkeletonRootVector, expected: SkeletonRootVector) {
    assert_eq!(actual.xyz.map(f32::to_bits), expected.xyz.map(f32::to_bits));
    assert_eq!(actual.w.map(f32::to_bits), expected.w.map(f32::to_bits));
}

struct Run {
    result: SolveResult<TwoBoneIkOutcome, &'static str, &'static str>,
    middle: SkeletonRootVector,
    end: SkeletonRootVector,
    trace: Trace,
}
fn run(
    chain: &TwoBoneChain,
    target: SkeletonRootVector,
    limits: TwoBoneLimits,
    remaining: u32,
    fail_at: Option<usize>,
) -> Run {
    let mut numeric = Services {
        fail_at,
        ..Services::default()
    };
    let mut rounding = numeric.clone();
    let mut middle = sentinel();
    let mut end = target;
    let result = try_solve_two_bone(
        chain,
        &mut middle,
        &mut end,
        limits,
        remaining,
        &mut numeric,
        &mut rounding,
    );
    Run {
        result,
        middle,
        end,
        trace: numeric.trace,
    }
}

#[test]
fn minimum_assertion_precedes_budget_and_output_stores() {
    let target = point([1.5, 0.0, 0.0, 8.0]);
    for minimum in [f32::from_bits(90.0_f32.to_bits() + 1), f32::NAN] {
        let result = run(&chain(), target, limits(minimum, 180.0), 0, None);
        assert_eq!(
            result.result,
            Err(TwoBoneIkError::MinimumAngleAssertion {
                bits: minimum.to_bits()
            })
        );
        assert_same(result.middle, sentinel());
        assert_same(result.end, target);
        assert!(result.trace.borrow().is_empty());
    }
}

#[test]
fn zero_budget_restores_both_originals_without_numeric_calls() {
    let mut original = chain();
    original.middle.w = None;
    original.end.w = Some(f32::from_bits(0x7fc1_2345));
    for minimum in [90.0, 0.0, -f32::INFINITY] {
        let result = run(&original, sentinel(), limits(minimum, 180.0), 0, None);
        assert_eq!(result.result, Ok(TwoBoneIkOutcome::Invalid));
        assert_same(result.middle, original.middle);
        assert_same(result.end, original.end);
        assert!(result.trace.borrow().is_empty());
    }
}

#[test]
fn either_short_segment_copies_only_middle_after_both_length_services() {
    let target = point([4.0, 5.0, 6.0, 7.0]);
    for lower_is_short in [false, true] {
        let mut original = chain();
        if lower_is_short {
            original.end = original.middle;
        } else {
            original.middle = point([EPSILON * 0.5, 0.0, 0.0, 9.0]);
        }
        let result = run(&original, target, limits(0.0, 180.0), 2, None);
        assert_eq!(result.result, Ok(TwoBoneIkOutcome::Invalid));
        assert_same(result.middle, original.middle);
        assert_same(result.end, target);
        assert_eq!(
            result
                .trace
                .borrow()
                .iter()
                .map(|event| event.0)
                .collect::<Vec<_>>(),
            ["dot", "dot", "rsqrt", "rsqrt"]
        );
    }
}

#[test]
fn epsilon_length_equality_enters_geometry_without_copying_middle() {
    let original = TwoBoneChain {
        root: point([0.0; 4]),
        middle: point([EPSILON, 0.0, 0.0, 0.0]),
        end: point([EPSILON, 1.0, 0.0, 0.0]),
    };
    let target = point([1.0, 1.0, 0.0, 0.0]);
    let result = run(&original, target, limits(0.0, 180.0), 2, Some(4));
    assert_eq!(result.result, Err(TwoBoneIkError::Numeric("refused")));
    assert_same(result.middle, sentinel());
    assert_same(result.end, target);
}

#[test]
fn reachable_bent_chain_has_frozen_source_polynomial_result() {
    let target = point([1.5, 0.0, 0.0, 0.0]);
    let result = run(&chain(), target, limits(0.0, 180.0), 2, None);
    assert_eq!(result.result, Ok(TwoBoneIkOutcome::Reachable));
    assert_bits(result.middle, [0x3f40_000d, 0x3f99_776f, 0, 0]);
    assert_same(result.end, target);
    // Source acos(1) is 0x35000000, not a repaired zero. Its addition inside
    // the next acos input contributes to the frozen middle displacement.
    assert_eq!(result.trace.borrow()[25], ("rsqrt", 0x3400_0000));
}

#[test]
fn full_kernel_angle_fma_cancellation_differs_from_defused_arithmetic() {
    let target = point([f32::from_bits(0x3f34_ed91), 0.0, 0.0, 0.0]);
    let result = run(&chain(), target, limits(0.0, 180.0), 2, None);
    assert_eq!(result.result, Ok(TwoBoneIkOutcome::Reachable));
    // Independent fmaf oracle: angle_input3e8009a9; defused3e8009a8.
    // Defusing ONLY that FMA gives middle[3eb5129a,3faf448b,0,0].
    assert_bits(result.middle, [0x3eb5_1294, 0x3faf_448d, 0, 0]);
    assert_same(result.end, target);
}

#[test]
fn oblique_target_uses_recomputed_normal_and_cofactor_frame() {
    let target = point([1.5, 0.25, 1.0, 0.0]);
    let result = run(&chain(), target, limits(0.0, 180.0), 2, None);
    assert_eq!(result.result, Ok(TwoBoneIkOutcome::Reachable));
    assert_bits(result.middle, [0x3f19_eef1, 0x3f82_3310, 0x3eff_fff4, 0]);
    assert_same(result.end, target);
    let plane = try_plane_coordinates(&chain(), target, &mut Services::default()).unwrap();
    let normal = plane.target_frame.columns()[2];
    assert!(normal.xyz[0] > 0.0 && normal.xyz[1] > 0.0 && normal.xyz[2] > -1.0);
    assert_eq!(
        plane.target.w.map(f32::to_bits),
        Some(plane.target.xyz[1].to_bits())
    );
}

#[test]
fn adjugate_inverse_of_sheared_frame_is_not_a_rigid_transpose() {
    let frame = SkeletonRootMatrix::from_columns([
        SkeletonRootVector {
            xyz: [2.0, 0.0, 0.0],
            w: None,
        },
        SkeletonRootVector {
            xyz: [1.0, 4.0, 0.0],
            w: None,
        },
        SkeletonRootVector {
            xyz: [0.0, 1.0, 8.0],
            w: None,
        },
        SkeletonRootVector {
            xyz: [3.0, 5.0, 7.0],
            w: None,
        },
    ]);
    let inverse = try_cofactor_inverse(&frame, &mut Services::default()).unwrap();
    let expected: [f32; 16] = [
        0.5, -0.0, -0.0, -0.0, -0.125, 0.25, -0.0, 0.25, 0.015_625, -0.03125, 0.125, -0.03125,
        -0.984_375, -1.03125, -0.875, -1.03125,
    ];
    assert_eq!(
        inverse.known_lanes().unwrap().map(f32::to_bits),
        expected.map(f32::to_bits)
    );
    let local = transform_point(&inverse, point([8.0, 10.0, 15.0, f32::NAN]));
    assert_bits(
        local,
        [
            2.0_f32.to_bits(),
            1.0_f32.to_bits(),
            1.0_f32.to_bits(),
            1.0_f32.to_bits(),
        ],
    );
}

#[test]
fn translated_noncanonical_w_is_computed_in_the_world_output() {
    let original = TwoBoneChain {
        root: point([3.0, -2.0, 1.0, 5.0]),
        middle: point([4.0, -1.0, 1.0, 7.0]),
        end: point([5.0, -2.0, 1.0, 11.0]),
    };
    let target = point([4.5, -2.0, 1.0, 13.0]);
    let result = run(&original, target, limits(0.0, 180.0), 2, None);
    assert_eq!(result.result, Ok(TwoBoneIkOutcome::Reachable));
    assert_bits(
        result.middle,
        [0x4070_0003, 0xbf4d_1122, 0x3f80_0000, 0x4110_0004],
    );
    assert_same(result.end, target);
}

#[test]
fn known_nan_w_and_unknown_w_leave_xyz_unchanged_without_canonicalization() {
    for w in [Some(f32::NAN), None] {
        let mut original = chain();
        original.middle.w = w;
        let target = point([1.5, 0.0, 0.0, 0.0]);
        let result = run(&original, target, limits(0.0, 180.0), 2, None);
        assert_eq!(result.result, Ok(TwoBoneIkOutcome::Reachable));
        assert_eq!(
            result.middle.xyz.map(f32::to_bits),
            [0x3f40_000d, 0x3f99_776f, 0]
        );
        if w.is_some() {
            assert!(result.middle.w.unwrap().is_nan());
        } else {
            assert!(result.middle.w.is_none());
        }
        let local = try_plane_coordinates(&original, target, &mut Services::default()).unwrap();
        for point in [local.root, local.middle, local.end, local.target] {
            assert_eq!(point.w.map(f32::to_bits), Some(point.xyz[1].to_bits()));
        }
    }
}

#[test]
fn reachable_service_order_batches_geometry_and_runs_both_candidates() {
    let result = run(
        &chain(),
        point([1.5, 0.0, 0.0, 0.0]),
        limits(0.0, 180.0),
        2,
        None,
    );
    assert_eq!(result.result, Ok(TwoBoneIkOutcome::Reachable));
    assert_eq!(
        result
            .trace
            .borrow()
            .iter()
            .map(|event| event.0)
            .collect::<Vec<_>>(),
        [
            "dot",
            "dot",
            "rsqrt",
            "rsqrt",
            "dot",
            "dot",
            "rsqrt",
            "dot",
            "rsqrt",
            "rsqrt",
            "dot",
            "reciprocal",
            "dot",
            "reciprocal",
            "dot",
            "rsqrt",
            "reciprocal",
            "rsqrt",
            "round",
            "rsqrt",
            "round",
            "rsqrt",
            "reciprocal",
            "rsqrt",
            "reciprocal",
            "rsqrt",
            "reciprocal",
            "rsqrt",
            "round",
            "round",
            "dot",
            "rsqrt",
            "dot",
            "rsqrt",
        ]
    );
}

#[test]
fn every_reachable_service_refusal_preserves_both_outputs() {
    let target = point([1.5, 0.0, 0.0, 0.0]);
    for index in 0..34 {
        let result = run(&chain(), target, limits(0.0, 180.0), 2, Some(index));
        let expected = if [18, 20, 28, 29].contains(&index) {
            TwoBoneIkError::Trigonometry("refused")
        } else {
            TwoBoneIkError::Numeric("refused")
        };
        assert_eq!(result.result, Err(expected), "callback {index}");
        assert_eq!(result.trace.borrow().len(), index + 1);
        assert_same(result.middle, sentinel());
        assert_same(result.end, target);
    }
}

#[test]
fn direct_unreachable_at_exact_180_writes_middle_and_end() {
    let result = run(
        &chain(),
        point([5.0, 0.0, 0.0, 0.0]),
        limits(0.0, 180.0),
        2,
        None,
    );
    assert_eq!(result.result, Ok(TwoBoneIkOutcome::Unreachable));
    assert_bits(result.middle, [0x3fb5_04f3, 0, 0, 0]);
    assert_bits(result.end, [0x4035_04f3, 0, 0, 0]);
    assert_eq!(result.trace.borrow().len(), 24);
    let below_180 = f32::from_bits(180.0_f32.to_bits() - 1);
    let result = run(
        &chain(),
        point([5.0, 0.0, 0.0, 0.0]),
        limits(0.0, below_180),
        1,
        None,
    );
    assert_eq!(result.result, Ok(TwoBoneIkOutcome::Invalid));
    assert_same(result.middle, chain().middle);
    assert_same(result.end, chain().end);
}

#[test]
fn every_direct_unreachable_refusal_precedes_both_output_commits() {
    let target = point([5.0, 0.0, 0.0, 0.0]);
    for index in 0..24 {
        let result = run(&chain(), target, limits(0.0, 180.0), 2, Some(index));
        assert!(result.result.is_err(), "callback {index}");
        assert_same(result.middle, sentinel());
        assert_same(result.end, target);
    }
}

#[test]
fn recursive_clamps_commit_before_next_attempt_and_exhaustion_restores_originals() {
    let cases = [
        (
            point([0.125, 0.0, 0.0, 0.0]),
            limits(45.0, 90.0),
            24,
            0x3f8a_9c36,
        ),
        (
            point([5.0, 0.0, 0.0, 0.0]),
            limits(0.0, 90.0),
            24,
            0x3fff_ef9e,
        ),
        (
            point([2.0, 0.0, 0.0, 0.0]),
            limits(90.0, 180.0),
            26,
            0x4000_0831,
        ),
    ];
    for (target, limits, next_callback, clamped_x) in cases {
        let result = run(&chain(), target, limits, 2, Some(next_callback));
        assert_eq!(result.result, Err(TwoBoneIkError::Numeric("refused")));
        assert_same(result.middle, sentinel());
        assert_bits(result.end, [clamped_x, 0, 0, 0]);
        let exhausted = run(&chain(), target, limits, 1, None);
        assert_eq!(exhausted.result, Ok(TwoBoneIkOutcome::Invalid));
        assert_same(exhausted.middle, chain().middle);
        assert_same(exhausted.end, chain().end);
        assert_eq!(exhausted.trace.borrow().len(), next_callback);
    }
}

#[test]
fn too_close_still_reads_maximum_cosine_and_length_before_retry() {
    let target = point([0.125, 0.0, 0.0, 0.0]);
    for index in [20, 21, 22, 23] {
        let result = run(&chain(), target, limits(45.0, 90.0), 2, Some(index));
        assert!(result.result.is_err());
        assert_same(result.middle, sentinel());
        assert_same(result.end, target);
        assert_eq!(result.trace.borrow()[20].0, "round");
    }
}

#[test]
fn original_angle_override_changes_the_limit_without_mutating_input_tuning() {
    let target = point([1.5, 0.0, 0.0, 0.0]);
    let unchanged_limits = limits(0.0, 30.0);
    let clamped = run(&chain(), target, unchanged_limits, 2, None);
    assert_eq!(clamped.result, Ok(TwoBoneIkOutcome::Reachable));
    assert_bits(clamped.end, [0x3f3b_46ea, 0, 0, 0]);
    assert_bits(clamped.middle, [0x3ebb_4700, 0x3fae_dc1a, 0, 0]);
    let overridden = run(
        &chain(),
        target,
        TwoBoneLimits {
            override_maximum_with_original: true,
            ..unchanged_limits
        },
        2,
        None,
    );
    assert_eq!(overridden.result, Ok(TwoBoneIkOutcome::Reachable));
    assert_same(overridden.end, target);
    assert_bits(overridden.middle, [0x3f40_000d, 0x3f99_776f, 0, 0]);
    assert_eq!(
        unchanged_limits.maximum_degrees.to_bits(),
        30.0_f32.to_bits()
    );
    // Both sign reversals in the original-angle graph yield an acos input0.
    assert_eq!(overridden.trace.borrow()[14], ("dot", 0));
    assert_eq!(overridden.trace.borrow()[17], ("rsqrt", 0x3f80_0001));
}

#[test]
fn original_angle_both_sign_reversals_control_the_override_range() {
    let mut original = chain();
    original.middle = point([0.5, 1.0, 0.0, 0.0]);
    let target = point([2.2, 0.0, 0.0, 0.0]);
    let tuning = limits(0.0, 90.0);
    let without = run(&original, target, tuning, 1, None);
    assert_eq!(without.result, Ok(TwoBoneIkOutcome::Invalid));
    assert_same(without.end, original.end);
    let with = run(
        &original,
        target,
        TwoBoneLimits {
            override_maximum_with_original: true,
            ..tuning
        },
        1,
        None,
    );
    assert_eq!(with.result, Ok(TwoBoneIkOutcome::Reachable));
    assert_bits(with.middle, [0x3f25_3c8b, 0x3f69_b3de, 0, 0]);
    assert_same(with.end, target);
    // dot(u,-v)=+0.25; denominator a*(-b)<0. Source original angle exceeds90.
    assert_eq!(with.trace.borrow()[14], ("dot", 0.25_f32.to_bits()));
    assert!(f32::from_bits(with.trace.borrow()[16].1) < 0.0);
}

fn planar_fixture(middle_y: f32, target: [f32; 4]) -> PlaneCoordinates {
    PlaneCoordinates {
        target_frame: SkeletonRootMatrix::from_columns([
            point([1.0, 0.0, 0.0, 0.0]),
            point([0.0, 1.0, 0.0, 0.0]),
            point([0.0, 0.0, 1.0, 0.0]),
            point([0.0; 4]),
        ]),
        root: point([0.0; 4]),
        middle: point([1.0, middle_y, 0.0, middle_y]),
        end: point([2.0, 0.0, 0.0, 0.0]),
        target: point(target),
    }
}

#[test]
fn orientation_addition_is_inside_acos_before_clamping() {
    let plane = planar_fixture(1.0, [1.0, 1.0, 0.0, 1.0]);
    let lengths = try_chain_lengths(&chain(), &mut Services::default()).unwrap();
    let mut numeric = Services::default();
    let mut rounding = numeric.clone();
    let result = try_reachable_middle(
        &plane,
        &lengths,
        2.0_f32.sqrt(),
        2.0,
        &mut numeric,
        &mut rounding,
    )
    .unwrap();
    // 0.5 + pi/4 is clamped to1 INSIDE acos. A conventional angle addition
    // outside acos points the middle into a different quadrant.
    assert_bits(result, [0x3fb5_04f3, 0x3535_04f3, 0, 0]);
    assert_eq!(numeric.trace.borrow()[3], ("rsqrt", 0x3400_0000));
}

#[test]
fn candidate_strict_comparison_and_tie_preserve_both_trig_evaluations() {
    let lengths = try_chain_lengths(&chain(), &mut Services::default()).unwrap();
    for (middle_y, expected) in [
        (-1.0, [0x3f40_0009, 0xbf99_7771, 0, 0]),
        (0.0, [0x3f40_000d, 0x3f99_776f, 0, 0]),
    ] {
        let plane = planar_fixture(middle_y, [1.5, 0.0, 0.0, 0.0]);
        let mut numeric = Services::default();
        let mut rounding = numeric.clone();
        let result =
            try_reachable_middle(&plane, &lengths, 1.5, 2.25, &mut numeric, &mut rounding).unwrap();
        assert_bits(result, expected);
        assert_eq!(numeric.trace.borrow().len(), 10);
        assert_eq!(numeric.trace.borrow()[4].0, "round");
        assert_eq!(numeric.trace.borrow()[5].0, "round");
    }
}

#[test]
fn unsafe_degenerate_geometry_runs_the_graph_instead_of_an_invented_invalid_exit() {
    let mut collinear = chain();
    collinear.middle = point([1.0, 0.0, 0.0, 0.0]);
    for (original, target) in [
        (collinear, point([1.5, 0.0, 0.0, 0.0])),
        (chain(), point([0.0; 4])),
    ] {
        let result = run(&original, target, limits(0.0, 180.0), 2, None);
        assert_eq!(result.result, Ok(TwoBoneIkOutcome::Reachable));
        assert!(result.middle.xyz.iter().all(|lane| lane.is_nan()));
        assert_same(result.end, target);
        assert_eq!(result.trace.borrow().len(), 34);
    }
}

#[test]
fn unordered_lengths_do_not_take_short_chain_exit() {
    for value in [f32::NAN, f32::INFINITY] {
        let mut original = chain();
        original.middle.xyz[0] = value;
        let target = point([1.5, 0.0, 0.0, 0.0]);
        let result = run(&original, target, limits(0.0, 180.0), 2, Some(4));
        assert_eq!(result.result, Err(TwoBoneIkError::Numeric("refused")));
        assert_same(result.middle, sentinel());
        assert_same(result.end, target);
    }
}
