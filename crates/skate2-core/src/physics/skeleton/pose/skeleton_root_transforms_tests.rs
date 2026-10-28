//! Controlled source-kernel inputs, not a fabricated live board or Actor.
use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric, skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
};
use std::{cell::RefCell, rc::Rc};

type Trace = Rc<RefCell<Vec<&'static str>>>;

struct Board {
    matrix: AnimationMatrix,
    velocity: SkeletonRootVector,
    fail: Option<&'static str>,
    trace: Trace,
}
impl Board {
    fn new(trace: &Trace) -> Self {
        Self {
            matrix: IDENTITY,
            velocity: SkeletonRootVector::known([0.0; 4]),
            fail: None,
            trace: Rc::clone(trace),
        }
    }
}
impl SkeletonRootBoard for Board {
    type Error = &'static str;
    fn transform(&mut self) -> Result<AnimationMatrix, Self::Error> {
        self.trace.borrow_mut().push("board");
        if self.fail == Some("board") {
            Err("board")
        } else {
            Ok(self.matrix)
        }
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.trace.borrow_mut().push("velocity");
        if self.fail == Some("velocity") {
            Err("velocity")
        } else {
            Ok(self.velocity)
        }
    }
}

struct Numeric {
    trace: Trace,
    fail: Option<&'static str>,
    dots: Vec<([f32; 4], [f32; 4])>,
}
impl Numeric {
    fn new(trace: &Trace) -> Self {
        Self {
            trace: Rc::clone(trace),
            fail: None,
            dots: vec![],
        }
    }
    fn event(&self, event: &'static str) -> Result<(), &'static str> {
        self.trace.borrow_mut().push(event);
        if self.fail == Some(event) {
            Err(event)
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.dots.push((a, b));
        self.event("dot")?;
        HostReckoningNumeric.dot3(a, b).map_err(|_| "host")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("rsqrt")?;
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(value)
            .map_err(|_| "host")
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("reciprocal")?;
        HostReckoningNumeric
            .reciprocal_estimate(value)
            .map_err(|_| "host")
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        self.event("truncate")?;
        HostReckoningNumeric.truncate_i32(value).map_err(|_| "host")
    }
}
struct SinCos {
    trace: Trace,
    fail: bool,
}
impl ReckoningSinCosNumeric for SinCos {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.trace.borrow_mut().push("sin_cos_round");
        if self.fail {
            Err("sin_cos")
        } else {
            Ok(value.round_ties_even())
        }
    }
}

fn input() -> SkeletonRootInput {
    SkeletonRootInput {
        time_delta: 0.5,
        skate_animation: IDENTITY,
    }
}
fn vector_bits(v: SkeletonRootVector) -> ([u32; 3], Option<u32>) {
    (v.xyz.map(f32::to_bits), v.w.map(f32::to_bits))
}
fn matrix_bits(m: &SkeletonRootMatrix) -> [([u32; 3], Option<u32>); 4] {
    m.columns.map(vector_bits)
}
fn assert_matrix(a: &SkeletonRootMatrix, b: &SkeletonRootMatrix) {
    assert_eq!(matrix_bits(a), matrix_bits(b));
}
fn run(
    state: &mut SkeletonRootTransforms,
    input: &SkeletonRootInput,
    reckoning: &AnimationMatrix,
    board: &mut Board,
    numeric: &mut Numeric,
    sin: &mut SinCos,
) -> Result<(), SkeletonRootError<&'static str, &'static str, &'static str>> {
    state.try_update(
        input,
        reckoning,
        board,
        numeric,
        sin,
        &mut HostSkateboardSpawnMathNumeric,
    )
}

#[test]
fn constructor_has_native_zero_w_identity_and_distinct_initial_flags() {
    let state = SkeletonRootTransforms::new();
    for matrix in [
        &state.board,
        &state.board_inverse,
        &state.animation_angle,
        &state.animation_to_board,
        &state.animation_to_world,
        &state.world_to_animation,
    ] {
        // Frozen native ctor words, not the production identity constant.
        assert_eq!(
            matrix_bits(matrix),
            [
                ([0x3f80_0000, 0, 0], Some(0)),
                ([0, 0x3f80_0000, 0], Some(0)),
                ([0, 0, 0x3f80_0000], Some(0)),
                ([0, 0, 0], Some(0)),
            ]
        );
    }
    assert_eq!(vector_bits(state.last_board_position), ([0; 3], Some(0)));
    assert_eq!(
        vector_bits(state.predicted_board_position),
        ([0; 3], Some(0))
    );
    assert!(!state.prediction_valid);
    assert!(state.update_heading);
}

#[test]
fn repeated_updates_retain_previous_board_position_and_predict_fresh_each_time() {
    let trace = Trace::default();
    let mut board = Board::new(&trace);
    let mut numeric = Numeric::new(&trace);
    let mut sin = SinCos {
        trace: Rc::clone(&trace),
        fail: false,
    };
    let mut state = SkeletonRootTransforms::new();
    state.update_heading = false;
    board.matrix[12..16].copy_from_slice(&[10.0, 20.0, 30.0, -0.0]);
    board.velocity = SkeletonRootVector::known([2.0, -4.0, 6.0, 8.0]);
    run(
        &mut state,
        &input(),
        &IDENTITY,
        &mut board,
        &mut numeric,
        &mut sin,
    )
    .unwrap();
    assert_eq!(vector_bits(state.last_board_position), ([0; 3], Some(0)));
    assert_eq!(
        vector_bits(state.predicted_board_position),
        (
            [11.0_f32, 18.0, 33.0].map(f32::to_bits),
            Some(4.0_f32.to_bits())
        )
    );
    assert!(!state.prediction_valid);
    board.matrix[12..16].copy_from_slice(&[-1.0, 5.0, 7.0, 13.0]);
    board.velocity = SkeletonRootVector::known([-2.0, 2.0, 0.0, -4.0]);
    run(
        &mut state,
        &input(),
        &IDENTITY,
        &mut board,
        &mut numeric,
        &mut sin,
    )
    .unwrap();
    assert_eq!(
        vector_bits(state.last_board_position),
        ([10.0_f32, 20.0, 30.0].map(f32::to_bits), Some(0x8000_0000))
    );
    assert_eq!(
        vector_bits(state.predicted_board_position),
        (
            [-2.0_f32, 6.0, 7.0].map(f32::to_bits),
            Some(11.0_f32.to_bits())
        )
    );
    assert_eq!(
        &*trace.borrow(),
        &["board", "velocity", "board", "velocity"]
    );
}

#[test]
fn prediction_valid_and_heading_false_skip_their_actual_services_then_clear_valid() {
    let trace = Trace::default();
    let mut board = Board::new(&trace);
    board.fail = Some("velocity");
    let mut numeric = Numeric::new(&trace);
    numeric.fail = Some("dot");
    let mut sin = SinCos {
        trace: Rc::clone(&trace),
        fail: true,
    };
    let mut state = SkeletonRootTransforms::new();
    state.prediction_valid = true;
    state.update_heading = false;
    state.predicted_board_position = SkeletonRootVector {
        xyz: [4.0, 5.0, 6.0],
        w: None,
    };
    let predicted = state.predicted_board_position;
    run(
        &mut state,
        &input(),
        &IDENTITY,
        &mut board,
        &mut numeric,
        &mut sin,
    )
    .unwrap();
    assert_eq!(
        vector_bits(state.predicted_board_position),
        vector_bits(predicted)
    );
    assert!(!state.prediction_valid);
    assert_eq!(&*trace.borrow(), &["board"]);
    assert!(matches!(
        run(
            &mut state,
            &input(),
            &IDENTITY,
            &mut board,
            &mut numeric,
            &mut sin
        ),
        Err(SkeletonRootError::Board("velocity"))
    ));
}

#[test]
fn heading_samples_current_animation_z_flattened_y_not_old_world_z() {
    let trace = Trace::default();
    let mut board = Board::new(&trace);
    let mut numeric = Numeric::new(&trace);
    numeric.fail = Some("dot");
    let mut sin = SinCos {
        trace: Rc::clone(&trace),
        fail: false,
    };
    let mut state = SkeletonRootTransforms::new();
    state.animation_to_world.columns[2] = SkeletonRootVector::known([11.0, 12.0, 13.0, 14.0]);
    let previous = state.animation_to_world;
    let mut supplied = input();
    supplied.skate_animation[8..12].copy_from_slice(&[3.0, f32::NAN, 4.0, -0.0]);
    assert!(matches!(
        run(
            &mut state,
            &supplied,
            &IDENTITY,
            &mut board,
            &mut numeric,
            &mut sin
        ),
        Err(SkeletonRootError::Numeric("dot"))
    ));
    let expected = [3.0_f32, 0.0, 4.0, -0.0].map(f32::to_bits);
    assert_eq!(numeric.dots[0].0.map(f32::to_bits), expected);
    assert_eq!(numeric.dots[0].1.map(f32::to_bits), expected);
    assert_eq!(&*trace.borrow(), &["board", "velocity", "dot"]);
    assert_matrix(&state.animation_to_world, &previous);
    assert!(state.update_heading);
    assert!(!state.prediction_valid);
}

#[test]
fn board_and_velocity_failures_preserve_different_completed_state_prefixes() {
    for failure in ["board", "velocity"] {
        let trace = Trace::default();
        let mut board = Board::new(&trace);
        board.fail = Some(failure);
        board.matrix[12..16].copy_from_slice(&[10.0, 20.0, 30.0, 40.0]);
        let mut numeric = Numeric::new(&trace);
        let mut sin = SinCos {
            trace: Rc::clone(&trace),
            fail: false,
        };
        let mut state = SkeletonRootTransforms::new();
        state.board.columns[3] = SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]);
        state.predicted_board_position = SkeletonRootVector::known([7.0, 8.0, 9.0, 10.0]);
        let old_board = state.board;
        let old_prediction = state.predicted_board_position;
        assert!(
            matches!(run(&mut state,&input(),&IDENTITY,&mut board,&mut numeric,&mut sin),Err(SkeletonRootError::Board(value)) if value==failure)
        );
        assert_eq!(
            vector_bits(state.last_board_position),
            vector_bits(old_board.columns[3])
        );
        assert_eq!(
            vector_bits(state.predicted_board_position),
            vector_bits(old_prediction)
        );
        if failure == "board" {
            assert_matrix(&state.board, &old_board);
        } else {
            assert_matrix(&state.board, &SkeletonRootMatrix::known(board.matrix));
            assert_eq!(
                state.board_inverse.columns[3].xyz.map(f32::to_bits),
                [-10.0_f32, -20.0, -30.0].map(f32::to_bits)
            );
        }
        assert!(state.update_heading);
        assert!(numeric.dots.is_empty());
    }
}

#[test]
fn angle_estimate_and_sincos_failures_leave_yaw_uncommitted_after_prediction() {
    for failure in ["rsqrt", "sin_cos"] {
        let trace = Trace::default();
        let mut board = Board::new(&trace);
        board.matrix[12] = 6.0;
        board.velocity.xyz[0] = 2.0;
        let mut numeric = Numeric::new(&trace);
        if failure == "rsqrt" {
            numeric.fail = Some("rsqrt");
        }
        let mut sin = SinCos {
            trace: Rc::clone(&trace),
            fail: failure == "sin_cos",
        };
        let mut state = SkeletonRootTransforms::new();
        let yaw = state.animation_angle;
        let result = run(
            &mut state,
            &input(),
            &IDENTITY,
            &mut board,
            &mut numeric,
            &mut sin,
        );
        if failure == "rsqrt" {
            assert!(matches!(result, Err(SkeletonRootError::Numeric("rsqrt"))));
        } else {
            assert!(matches!(result, Err(SkeletonRootError::SinCos("sin_cos"))));
        }
        assert_eq!(
            state.predicted_board_position.xyz[0].to_bits(),
            7.0_f32.to_bits()
        );
        assert_matrix(&state.animation_angle, &yaw);
        assert_matrix(
            &state.animation_to_board,
            &SkeletonRootMatrix::known(IDENTITY),
        );
        assert!(state.update_heading);
        assert!(!state.prediction_valid);
        let events = trace.borrow();
        assert_eq!(&events[..2], &["board", "velocity"]);
        assert_eq!(
            events.last().copied(),
            Some(if failure == "rsqrt" {
                "rsqrt"
            } else {
                "sin_cos_round"
            })
        );
    }
}

#[test]
fn successful_yaw_retains_source_noncanonical_lanes_and_clears_heading_once() {
    let trace = Trace::default();
    let mut board = Board::new(&trace);
    let mut numeric = Numeric::new(&trace);
    let mut sin = SinCos {
        trace: Rc::clone(&trace),
        fail: false,
    };
    let mut state = SkeletonRootTransforms::new();
    let mut supplied = input();
    supplied.skate_animation[8..12].copy_from_slice(&[1.0, 99.0, 0.0, 23.0]);
    run(
        &mut state,
        &supplied,
        &IDENTITY,
        &mut board,
        &mut numeric,
        &mut sin,
    )
    .unwrap();
    let yaw = state.animation_angle;
    assert_eq!(
        yaw.columns[0].w.map(f32::to_bits),
        Some(yaw.columns[0].xyz[0].to_bits())
    );
    assert_eq!(
        yaw.columns[2].w.map(f32::to_bits),
        Some(yaw.columns[2].xyz[0].to_bits())
    );
    assert_eq!(
        yaw.columns[0].xyz[2].to_bits(),
        yaw.columns[2].xyz[0].to_bits() ^ 0x8000_0000
    );
    assert_eq!(
        vector_bits(yaw.columns[1]),
        ([0, 1.0_f32.to_bits(), 0], Some(0))
    );
    assert!(!state.update_heading);
    trace.borrow_mut().clear();
    numeric.fail = Some("dot");
    sin.fail = true;
    run(
        &mut state,
        &supplied,
        &IDENTITY,
        &mut board,
        &mut numeric,
        &mut sin,
    )
    .unwrap();
    assert_matrix(&state.animation_angle, &yaw);
    assert_eq!(&*trace.borrow(), &["board", "velocity"]);
}

#[test]
fn unknown_velocity_w_survives_zero_dt_and_world_position_but_inverse_recovers_known_w() {
    let trace = Trace::default();
    let mut board = Board::new(&trace);
    board.velocity.w = None;
    board.matrix[12..16].copy_from_slice(&[2.0, 3.0, 4.0, 17.0]);
    let mut numeric = Numeric::new(&trace);
    let mut sin = SinCos {
        trace: Rc::clone(&trace),
        fail: false,
    };
    let mut state = SkeletonRootTransforms::new();
    state.update_heading = false;
    let mut supplied = input();
    supplied.time_delta = 0.0;
    run(
        &mut state,
        &supplied,
        &IDENTITY,
        &mut board,
        &mut numeric,
        &mut sin,
    )
    .unwrap();
    assert_eq!(state.predicted_board_position.w, None);
    assert_eq!(state.animation_to_world.columns[3].w, None);
    assert!(
        state.animation_to_world.columns[..3]
            .iter()
            .all(|column| column.w.map(f32::to_bits) == Some(0))
    );
    assert!(
        state
            .world_to_animation
            .columns
            .iter()
            .all(|column| column.w.is_some())
    );
    assert_eq!(
        state.world_to_animation.columns[3].xyz.map(f32::to_bits),
        [-2.0_f32, -3.0, -4.0].map(f32::to_bits)
    );
}

#[test]
fn unknown_outer_axis_w_is_not_elided_by_zero_coefficients_but_inner_w_is_ignored() {
    let identity = SkeletonRootMatrix::known(IDENTITY);
    let mut outer = identity;
    outer.columns[0].w = None;
    assert!(
        outer
            .compose(&identity)
            .columns
            .iter()
            .all(|column| column.w.is_none())
    );
    let mut inner = identity;
    for column in &mut inner.columns {
        column.w = None;
    }
    assert_matrix(&identity.compose(&inner), &identity);
    let mut translation = identity;
    translation.columns[3].w = None;
    let output = translation.compose(&identity);
    assert!(output.columns[..3].iter().all(|column| column.w.is_some()));
    assert_eq!(output.columns[3].w, None);
    assert!(
        translation
            .inverse()
            .columns
            .iter()
            .all(|column| column.w.is_some())
    );
}

#[test]
fn original_prediction_and_composition_fmas_keep_independent_unit_residuals() {
    let trace = Trace::default();
    let mut board = Board::new(&trace);
    board.matrix[12] = -16_785_408.0;
    board.velocity.xyz[0] = 4097.0;
    let mut numeric = Numeric::new(&trace);
    let mut sin = SinCos {
        trace: Rc::clone(&trace),
        fail: false,
    };
    let mut state = SkeletonRootTransforms::new();
    state.update_heading = false;
    let mut supplied = input();
    supplied.time_delta = 4097.0;
    supplied.skate_animation[12] = -4097.0;
    let mut reckoning = IDENTITY;
    reckoning[0] = 4097.0;
    reckoning[12] = -16_785_408.0;
    run(
        &mut state,
        &supplied,
        &reckoning,
        &mut board,
        &mut numeric,
        &mut sin,
    )
    .unwrap();
    // 4097^2 = 16_785_409 exactly in integers; a rounded standalone product
    // would be16_785_408 and lose both independent source FMA residuals.
    assert_eq!(
        state.predicted_board_position.xyz[0].to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(
        state.animation_to_board.columns[3].xyz[0].to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(
        state.animation_to_world.columns[3].xyz[0].to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        state.animation_to_world.columns[0].xyz[0].to_bits(),
        1.0_f32.to_bits()
    );
}

#[test]
fn assertion_stops_retain_each_completed_matrix_and_leave_inverse_old() {
    for stage in [
        SkeletonRootAssertion::AnimationToBoard,
        SkeletonRootAssertion::PredictedBoardPosition,
        SkeletonRootAssertion::AnimationToWorld,
        SkeletonRootAssertion::NormalizedAnimationToWorld,
    ] {
        let trace = Trace::default();
        let mut board = Board::new(&trace);
        let mut numeric = Numeric::new(&trace);
        let mut sin = SinCos {
            trace: Rc::clone(&trace),
            fail: false,
        };
        let mut state = SkeletonRootTransforms::new();
        state.update_heading = false;
        let old_world = state.animation_to_world;
        let old_inverse = state.world_to_animation;
        let mut reckoning = IDENTITY;
        match stage {
            SkeletonRootAssertion::AnimationToBoard => reckoning[0] = f32::NAN,
            SkeletonRootAssertion::PredictedBoardPosition => board.velocity.xyz[0] = f32::NAN,
            SkeletonRootAssertion::AnimationToWorld => reckoning[12] = f32::INFINITY,
            SkeletonRootAssertion::NormalizedAnimationToWorld => reckoning[8..12].fill(0.0),
        }
        assert!(
            matches!(run(&mut state,&input(),&reckoning,&mut board,&mut numeric,&mut sin),Err(SkeletonRootError::Assertion(actual)) if actual==stage)
        );
        assert_matrix(&state.world_to_animation, &old_inverse);
        match stage {
            SkeletonRootAssertion::AnimationToBoard => {
                assert!(state.animation_to_board.has_nan_xyz());
                assert_matrix(&state.animation_to_world, &old_world);
            }
            SkeletonRootAssertion::PredictedBoardPosition => {
                assert!(!state.animation_to_board.has_nan_xyz());
                assert!(state.predicted_board_position.xyz[0].is_nan());
                assert_matrix(&state.animation_to_world, &old_world);
            }
            SkeletonRootAssertion::AnimationToWorld => {
                assert!(!state.animation_to_board.has_nan_xyz());
                assert!(state.animation_to_world.has_nan_xyz());
            }
            SkeletonRootAssertion::NormalizedAnimationToWorld => {
                assert!(!state.animation_to_board.has_nan_xyz());
                assert!(state.animation_to_world.has_nan_xyz());
                assert_eq!(
                    vector_bits(state.animation_to_world.columns[3]),
                    ([0; 3], Some(0))
                );
            }
        }
        assert_eq!(&*trace.borrow(), &["board", "velocity"]);
    }
}
