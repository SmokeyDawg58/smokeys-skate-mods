//! Controlled source inputs and fallible services; actual pool integration is
//! checked by the caller's corpus, not replaced by these kernel collaborators.

use crate::physics_state_ids::PHYSICS_STATE_TELEPORTING;
use super::*;
use std::{cell::RefCell, rc::Rc};

type Trace = Rc<RefCell<Vec<&'static str>>>;

#[test]
fn grounded_trajectory_tail_clears_only_next_transform_and_reset_prefix_preserves_it() {
    let mut general = SkeletonGeneralUpdate::new();
    assert_bits(general.next_trajectory_transform(), IDENTITY);
    let seeded = SkeletonRootMatrix::known(matrix(
        [2.0, 3.0, 4.0],
        [7.0, 8.0, 9.0, 10.0],
        [11.0, 12.0, 13.0],
    ));
    general.next_trajectory_transform = seeded;
    general.physics_skateboard = seeded;
    general.animation_world_velocity = SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]);
    general.add_skateboard_error = true;
    general.clear_add_skateboard_error();
    assert_bits(general.next_trajectory_transform(), known(&seeded));
    general.finish_grounded_trajectory();
    assert_bits(general.next_trajectory_transform(), IDENTITY);
    assert_bits(general.physics_skateboard(), known(&seeded));
    assert_eq!(
        general.animation_world_velocity().xyz.map(f32::to_bits),
        [1.0_f32, 2.0, 3.0].map(f32::to_bits)
    );
    assert_eq!(
        general.animation_world_velocity().w.map(f32::to_bits),
        Some(4.0_f32.to_bits())
    );
    assert!(!general.add_skateboard_error());
}

fn matrix(basis: [f32; 3], position: [f32; 4], w: [f32; 3]) -> AnimationMatrix {
    [
        basis[0],
        0.0,
        0.0,
        w[0],
        0.0,
        basis[1],
        0.0,
        w[1],
        0.0,
        0.0,
        basis[2],
        w[2],
        position[0],
        position[1],
        position[2],
        position[3],
    ]
}

fn translated(position: [f32; 4]) -> AnimationMatrix {
    matrix([1.0; 3], position, [0.0; 3])
}

fn roots_with(world: SkeletonRootMatrix) -> SkeletonRootTransforms {
    let mut roots = SkeletonRootTransforms::new();
    roots.seed_general_update_inputs(
        SkeletonRootMatrix::known(IDENTITY),
        world,
        SkeletonRootMatrix::known(IDENTITY),
        SkeletonRootVector::known([0.0; 4]),
    );
    roots
}

fn known(matrix: &SkeletonRootMatrix) -> AnimationMatrix {
    matrix.known_lanes().expect("test requests a known result")
}

fn assert_bits(actual: &SkeletonRootMatrix, expected: AnimationMatrix) {
    assert_eq!(known(actual).map(f32::to_bits), expected.map(f32::to_bits));
}

struct Board {
    transform: AnimationMatrix,
    fail: bool,
    calls: usize,
}
impl Board {
    fn new(transform: AnimationMatrix) -> Self {
        Self {
            transform,
            fail: false,
            calls: 0,
        }
    }
}
impl SkeletonRootBoard for Board {
    type Error = &'static str;
    fn transform(&mut self) -> Result<AnimationMatrix, Self::Error> {
        self.calls += 1;
        if self.fail {
            Err("board")
        } else {
            Ok(self.transform)
        }
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        panic!("grounded suffix must not resample board velocity")
    }
}

struct Hooks {
    matrices: [SkeletonRootMatrix; 2],
    trace: Trace,
    fail: Option<&'static str>,
}
impl Hooks {
    fn new(trace: &Trace) -> Self {
        Self {
            matrices: [SkeletonRootMatrix::known(IDENTITY); 2],
            trace: Rc::clone(trace),
            fail: None,
        }
    }
    fn event(&self, name: &'static str) -> Result<(), &'static str> {
        self.trace.borrow_mut().push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl SkeletonGeneralUpdateHooks for Hooks {
    type Error = &'static str;
    fn transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error> {
        assert_eq!(part, 0);
        self.event("get0")?;
        Ok(self.matrices[part])
    }
    fn set_transform(
        &mut self,
        part: usize,
        transform: &SkeletonRootMatrix,
    ) -> Result<(), Self::Error> {
        self.event(if part == 0 { "set0" } else { "set1" })?;
        self.matrices[part] = *transform;
        Ok(())
    }
}

struct Numeric {
    trace: Trace,
    fail: Option<&'static str>,
    seed: Option<f32>,
    squared: Option<f32>,
    last_delta: Option<[f32; 3]>,
    last_time_delta: Option<f32>,
}
impl Numeric {
    fn new(trace: &Trace) -> Self {
        Self {
            trace: Rc::clone(trace),
            fail: None,
            seed: None,
            squared: None,
            last_delta: None,
            last_time_delta: None,
        }
    }
    fn event(&self, name: &'static str) -> Result<(), &'static str> {
        self.trace.borrow_mut().push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        self.event("dot")?;
        assert_eq!(left[..3], right[..3]);
        self.last_delta = Some([left[0], left[1], left[2]]);
        Ok(self.squared.unwrap_or_else(|| {
            let x = left[0] * right[0];
            let y = left[1] * right[1];
            let z = left[2] * right[2];
            (x + y) + z
        }))
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("reciprocal")?;
        self.last_time_delta = Some(value);
        Ok(self.seed.unwrap_or_else(|| value.recip()))
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no hook/velocity rsqrt service")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no hook/velocity conversion service")
    }
}

#[test]
fn constructor_initializes_full_zero_histories_identity_matrices_and_false_flags() {
    let state = SkeletonGeneralUpdate::new();
    assert_bits(state.physics_skateboard(), IDENTITY);
    assert_bits(state.skate_root(), IDENTITY);
    assert_bits(state.animation_board_to_physics_board(), IDENTITY);
    assert_eq!(
        state.animation_world_velocity(),
        SkeletonRootVector::known([0.0; 4])
    );
    assert_eq!(
        state.last_animation_world_position(),
        SkeletonRootVector::known([0.0; 4])
    );
    assert!(!state.add_skateboard_error());
    assert!(!state.teleporting());
    assert!(!state.want_respawn());
}

#[test]
fn reset_flag_helpers_preserve_other_history_and_respawn_is_sticky() {
    let mut state = SkeletonGeneralUpdate::new();
    state.add_skateboard_error = true;
    state.teleporting = true;
    state.last_animation_world_position = SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]);
    state.clear_add_skateboard_error();
    assert!(!state.add_skateboard_error());
    assert!(state.teleporting());
    assert_eq!(state.last_animation_world_position().w, Some(4.0));
    for skipped in [0, 0x2be] {
        state.mark_want_respawn_after_reset(skipped);
        assert!(!state.want_respawn());
    }
    for eligible in [-1, 1, 2, 0x2bd, 0x2bf] {
        let mut state = SkeletonGeneralUpdate::new();
        state.mark_want_respawn_after_reset(eligible);
        assert!(state.want_respawn());
        state.mark_want_respawn_after_reset(0);
        state.mark_want_respawn_after_reset(PHYSICS_STATE_TELEPORTING);
        assert!(state.want_respawn());
    }
}

#[test]
fn grounded_recomposes_raw_board_space_and_preserves_unknown_predicted_w() {
    let mut roots = SkeletonRootTransforms::new();
    roots.seed_general_update_inputs(
        SkeletonRootMatrix::known(matrix(
            [2.0, 3.0, 4.0],
            [7.0, 8.0, 9.0, 10.0],
            [11.0, 12.0, 13.0],
        )),
        SkeletonRootMatrix::known(IDENTITY),
        SkeletonRootMatrix::known(IDENTITY),
        SkeletonRootVector {
            xyz: [100.0, 200.0, 300.0],
            w: None,
        },
    );
    let mut state = SkeletonGeneralUpdate::new();
    let mut output = SkeletonRootMatrix::known(IDENTITY);
    let mut flags = 0xffff_ffff;
    let physical = translated([30.0, 40.0, 50.0, 9.0]);
    let mut board = Board::new(physical);
    state
        .try_grounded_suffix(
            &roots,
            &translated([1.0, 2.0, 3.0, f32::NAN]),
            &mut output,
            &mut flags,
            &mut board,
        )
        .unwrap();
    assert_eq!(
        output.columns()[0],
        SkeletonRootVector::known([2.0, 0.0, 0.0, 0.0])
    );
    assert_eq!(
        output.columns()[1],
        SkeletonRootVector::known([0.0, 3.0, 0.0, 0.0])
    );
    assert_eq!(
        output.columns()[2],
        SkeletonRootVector::known([0.0, 0.0, 4.0, 0.0])
    );
    assert_eq!(
        output.columns()[3],
        SkeletonRootVector {
            xyz: [109.0, 214.0, 321.0],
            w: None
        }
    );
    assert_eq!(flags, 0xfff7_ffff);
    assert_eq!(board.calls, 1);
    assert_bits(state.physics_skateboard(), physical);
    assert_bits(state.skate_root(), physical);
}

#[test]
fn grounded_assertions_precede_output_flags_and_late_board_service() {
    use SkeletonGroundedSuffixAssertion as Assertion;
    for failure in [
        Assertion::PredictedBoardPosition,
        Assertion::AnimationToWorld,
        Assertion::SkateboardOrientation,
    ] {
        let mut predicted = SkeletonRootVector::known([0.0; 4]);
        let mut world = IDENTITY;
        let mut mapped = IDENTITY;
        match failure {
            Assertion::PredictedBoardPosition => {
                predicted.xyz[0] = f32::NAN;
                world[0] = f32::NAN;
                mapped[0] = f32::NAN;
            }
            Assertion::AnimationToWorld => {
                world[4] = f32::NAN;
                mapped[0] = f32::NAN;
            }
            Assertion::SkateboardOrientation => mapped[12] = f32::NAN,
        }
        let mut roots = SkeletonRootTransforms::new();
        roots.seed_general_update_inputs(
            SkeletonRootMatrix::known(IDENTITY),
            SkeletonRootMatrix::known(world),
            SkeletonRootMatrix::known(IDENTITY),
            predicted,
        );
        let mut state = SkeletonGeneralUpdate::new();
        let mut output = SkeletonRootMatrix::known(translated([8.0; 4]));
        let mut flags = 0x0008_0040;
        let mut board = Board::new(IDENTITY);
        assert_eq!(
            state.try_grounded_suffix(&roots, &mapped, &mut output, &mut flags, &mut board),
            Err(SkeletonGroundedSuffixError::Assertion(failure))
        );
        assert_bits(&output, translated([8.0; 4]));
        assert_eq!(flags, 0x0008_0040);
        assert_eq!(board.calls, 0);
        assert_bits(state.skate_root(), IDENTITY);
    }
}

#[test]
fn grounded_late_board_failure_preserves_publication_and_prior_physical_matrices() {
    let roots = roots_with(SkeletonRootMatrix::known(IDENTITY));
    let mut state = SkeletonGeneralUpdate::new();
    let previous = translated([9.0, 8.0, 7.0, 6.0]);
    state.physics_skateboard = SkeletonRootMatrix::known(previous);
    state.skate_root = SkeletonRootMatrix::known(previous);
    let mut output = SkeletonRootMatrix::known(IDENTITY);
    let mut flags = 0x80040;
    let mut board = Board::new(IDENTITY);
    board.fail = true;
    assert_eq!(
        state.try_grounded_suffix(
            &roots,
            &translated([1.0, 2.0, 3.0, 5.0]),
            &mut output,
            &mut flags,
            &mut board
        ),
        Err(SkeletonGroundedSuffixError::Board("board"))
    );
    assert_bits(&output, translated([1.0, 2.0, 3.0, 0.0]));
    assert_eq!(flags, 0x40);
    assert_bits(state.physics_skateboard(), previous);
    assert_bits(state.skate_root(), previous);
}

#[test]
fn grounded_physical_getter_copy_preserves_every_lane_without_normalizing_or_asserting() {
    let roots = roots_with(SkeletonRootMatrix::known(IDENTITY));
    let physical = matrix(
        [2.0, -3.0, 4.0],
        [5.0, f32::INFINITY, 7.0, f32::from_bits(0x7fc0_1234)],
        [-0.0, 9.0, -10.0],
    );
    let mut board = Board::new(physical);
    let mut state = SkeletonGeneralUpdate::new();
    state
        .try_grounded_suffix(
            &roots,
            &IDENTITY,
            &mut SkeletonRootMatrix::known(IDENTITY),
            &mut 0,
            &mut board,
        )
        .unwrap();
    assert_bits(state.physics_skateboard(), physical);
    assert_bits(state.skate_root(), physical);
}

#[test]
fn grounded_world_w_is_not_an_assertion_input() {
    let mut world = IDENTITY;
    for lane in [3, 7, 11, 15] {
        world[lane] = f32::NAN;
    }
    let roots = roots_with(SkeletonRootMatrix::known(world));
    let mut output = SkeletonRootMatrix::known(IDENTITY);
    SkeletonGeneralUpdate::new()
        .try_grounded_suffix(
            &roots,
            &IDENTITY,
            &mut output,
            &mut 0,
            &mut Board::new(IDENTITY),
        )
        .unwrap();
    assert_bits(&output, IDENTITY);
}

#[test]
fn grounded_world_xyz_infinity_passes_nan_only_assertion_and_is_not_composed() {
    let mut world = IDENTITY;
    world[0] = f32::INFINITY;
    world[13] = f32::NEG_INFINITY;
    let roots = roots_with(SkeletonRootMatrix::known(world));
    let mut output = SkeletonRootMatrix::known(IDENTITY);
    SkeletonGeneralUpdate::new()
        .try_grounded_suffix(
            &roots,
            &IDENTITY,
            &mut output,
            &mut 0,
            &mut Board::new(IDENTITY),
        )
        .unwrap();
    assert_bits(&output, IDENTITY);
}

#[test]
fn grounded_translation_composition_executes_zero_products_before_xyz_assertion() {
    let mut roots = SkeletonRootTransforms::new();
    roots.seed_general_update_inputs(
        SkeletonRootMatrix::known(translated([f32::INFINITY, 0.0, 0.0, 0.0])),
        SkeletonRootMatrix::known(IDENTITY),
        SkeletonRootMatrix::known(IDENTITY),
        SkeletonRootVector::known([0.0; 4]),
    );
    let mut state = SkeletonGeneralUpdate::new();
    let mut output = SkeletonRootMatrix::known(IDENTITY);
    let mut flags = 0x80000;
    let mut board = Board::new(IDENTITY);
    // The inner P.X stays infinity, but the outer translation's zero X.Y/Z
    // factors make those lanes NaN. Merely adding predicted XYZ would miss it.
    assert_eq!(
        state.try_grounded_suffix(&roots, &IDENTITY, &mut output, &mut flags, &mut board),
        Err(SkeletonGroundedSuffixError::Assertion(
            SkeletonGroundedSuffixAssertion::SkateboardOrientation
        ))
    );
    assert_bits(&output, IDENTITY);
    assert_eq!(flags, 0x80000);
    assert_eq!(board.calls, 0);
}

#[test]
fn velocity_uses_two_refinements_and_updates_full_four_lane_history() {
    let trace = Trace::default();
    let mut numeric = Numeric::new(&trace);
    numeric.seed = Some(0.25);
    let roots = roots_with(SkeletonRootMatrix::known(translated([
        12.0, 24.0, -12.0, 48.0,
    ])));
    let mut state = SkeletonGeneralUpdate::new();
    state
        .try_update_velocity(&roots, 3.0, &mut numeric)
        .unwrap();
    // Exact dyadic seed: r1=.3125, r2=.33203125. A single refinement or
    // direct division gives distinct values, with no oracle call to production.
    assert_eq!(
        state.animation_world_velocity(),
        SkeletonRootVector::known([3.984_375, 7.96875, -3.984_375, 15.9375])
    );
    assert_eq!(
        state.last_animation_world_position(),
        SkeletonRootVector::known([12.0, 24.0, -12.0, 48.0])
    );
    assert_eq!(numeric.last_time_delta, Some(3.0));
    assert_eq!(&*trace.borrow(), &["reciprocal"]);
}

#[test]
fn velocity_refusal_preserves_both_histories_then_retry_uses_last_successful_position() {
    let trace = Trace::default();
    let mut numeric = Numeric::new(&trace);
    let mut state = SkeletonGeneralUpdate::new();
    let first = roots_with(SkeletonRootMatrix::known(translated([
        4.0, 8.0, 12.0, 16.0,
    ])));
    state
        .try_update_velocity(&first, 2.0, &mut numeric)
        .unwrap();
    let next = roots_with(SkeletonRootMatrix::known(translated([
        10.0, 16.0, 22.0, 28.0,
    ])));
    numeric.fail = Some("reciprocal");
    assert_eq!(
        state.try_update_velocity(&next, 2.0, &mut numeric),
        Err("reciprocal")
    );
    assert_eq!(
        state.animation_world_velocity(),
        SkeletonRootVector::known([2.0, 4.0, 6.0, 8.0])
    );
    assert_eq!(
        state.last_animation_world_position(),
        first.animation_to_world().columns()[3]
    );
    numeric.fail = None;
    state.try_update_velocity(&next, 2.0, &mut numeric).unwrap();
    assert_eq!(
        state.animation_world_velocity(),
        SkeletonRootVector::known([3.0, 4.0, 5.0, 6.0])
    );
}

#[test]
fn velocity_unknown_w_recovers_only_after_two_consecutive_known_positions() {
    let trace = Trace::default();
    let mut numeric = Numeric::new(&trace);
    let mut state = SkeletonGeneralUpdate::new();
    let mut columns = *SkeletonRootMatrix::known(translated([1.0, 2.0, 3.0, 4.0])).columns();
    columns[3].w = None;
    let unknown = roots_with(SkeletonRootMatrix::from_columns(columns));
    state
        .try_update_velocity(&unknown, 1.0, &mut numeric)
        .unwrap();
    assert_eq!(state.animation_world_velocity().w, None);
    assert_eq!(state.last_animation_world_position().w, None);
    let current = roots_with(SkeletonRootMatrix::known(translated([2.0, 4.0, 6.0, 8.0])));
    state
        .try_update_velocity(&current, 1.0, &mut numeric)
        .unwrap();
    assert_eq!(state.animation_world_velocity().w, None);
    assert_eq!(state.last_animation_world_position().w, Some(8.0));
    state
        .try_update_velocity(&current, 1.0, &mut numeric)
        .unwrap();
    assert_eq!(state.animation_world_velocity().w, Some(0.0));
}

#[test]
fn velocity_zero_timestep_runs_numeric_graph_and_commits_nan_result() {
    let trace = Trace::default();
    let mut numeric = Numeric::new(&trace);
    let roots = roots_with(SkeletonRootMatrix::known(translated([1.0; 4])));
    let mut state = SkeletonGeneralUpdate::new();
    state
        .try_update_velocity(&roots, 0.0, &mut numeric)
        .unwrap();
    assert!(
        state
            .animation_world_velocity()
            .xyz
            .iter()
            .all(|lane| lane.is_nan())
    );
    assert!(state.animation_world_velocity().w.unwrap().is_nan());
    assert_eq!(
        state.last_animation_world_position(),
        SkeletonRootVector::known([1.0; 4])
    );
    assert_eq!(&*trace.borrow(), &["reciprocal"]);
}

#[test]
fn hooks_compose_full_four_lanes_and_use_physical_skate_root_for_hook_one() {
    let mut roots = SkeletonRootTransforms::new();
    roots.seed_general_update_inputs(
        SkeletonRootMatrix::known(IDENTITY),
        SkeletonRootMatrix::known(matrix(
            [2.0, 3.0, 4.0],
            [10.0, 20.0, 30.0, 8.0],
            [5.0, 6.0, 7.0],
        )),
        SkeletonRootMatrix::known(matrix(
            [0.5, 0.25, 2.0],
            [-2.0, -3.0, -4.0, 12.0],
            [9.0, 10.0, 11.0],
        )),
        SkeletonRootVector::known([0.0; 4]),
    );
    let trace = Trace::default();
    let mut numeric = Numeric::new(&trace);
    let mut hooks = Hooks::new(&trace);
    hooks.matrices[0] = SkeletonRootMatrix::known(translated([11.5, 26.25, 42.0, 999.0]));
    let mut state = SkeletonGeneralUpdate::new();
    let physical = matrix(
        [6.0, 7.0, 8.0],
        [70.0, 80.0, 90.0, 10.0],
        [11.0, 12.0, 13.0],
    );
    state.skate_root = SkeletonRootMatrix::known(physical);
    let mut proxy = None;
    assert!(
        state
            .try_update_hooks(
                &roots,
                &translated([1.0, 2.0, 3.0, 14.0]),
                &translated([-1.0, 1.0, 2.0, 19.0]),
                &mut proxy,
                &mut hooks,
                &mut numeric
            )
            .unwrap()
    );
    assert_bits(
        &hooks.matrices[0],
        matrix([2.0, 3.0, 4.0], [12.0, 26.0, 42.0, 46.0], [5.0, 6.0, 7.0]),
    );
    assert_bits(
        state.animation_board_to_physics_board(),
        matrix(
            [1.0, 0.75, 8.0],
            [2.0, 2.75, 72.0, 732.0],
            [18.0, 30.0, 44.0],
        ),
    );
    assert_bits(&hooks.matrices[1], physical);
    assert_eq!(
        proxy,
        Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            [70.0, 80.0, 90.0, 10.0]
        ))
    );
    assert_eq!(numeric.last_delta, Some([-0.5, 0.25, 0.0]));
    assert_eq!(&*trace.borrow(), &["get0", "set0", "set1", "dot"]);
}

#[test]
fn hooks_recover_cache_w_through_known_outer_inverse_but_retain_partial_hook_zero() {
    let mut columns = *SkeletonRootMatrix::known(translated([1.0, 2.0, 3.0, 4.0])).columns();
    for column in &mut columns {
        column.w = None;
    }
    let roots = roots_with(SkeletonRootMatrix::from_columns(columns));
    let trace = Trace::default();
    let mut hooks = Hooks::new(&trace);
    let mut state = SkeletonGeneralUpdate::new();
    state
        .try_update_hooks(
            &roots,
            &IDENTITY,
            &IDENTITY,
            &mut None,
            &mut hooks,
            &mut Numeric::new(&trace),
        )
        .unwrap();
    assert!(
        hooks.matrices[0]
            .columns()
            .iter()
            .all(|column| column.w.is_none())
    );
    assert_bits(
        state.animation_board_to_physics_board(),
        translated([1.0, 2.0, 3.0, 0.0]),
    );
}

#[test]
fn hooks_failure_prefixes_keep_cache_then_each_completed_hook_then_proxy() {
    let roots = roots_with(SkeletonRootMatrix::known(translated([1.0, 2.0, 3.0, 4.0])));
    for failure in ["get0", "set0", "set1", "dot"] {
        let trace = Trace::default();
        let mut hooks = Hooks::new(&trace);
        let mut numeric = Numeric::new(&trace);
        hooks.fail = Some(failure);
        numeric.fail = Some(failure);
        let mut state = SkeletonGeneralUpdate::new();
        let physical = translated([9.0, 8.0, 7.0, 6.0]);
        state.skate_root = SkeletonRootMatrix::known(physical);
        let mut proxy = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            [50.0; 4],
        ));
        let result = state.try_update_hooks(
            &roots,
            &IDENTITY,
            &IDENTITY,
            &mut proxy,
            &mut hooks,
            &mut numeric,
        );
        assert_eq!(
            result,
            Err(if failure == "dot" {
                SkeletonHookUpdateError::Numeric(failure)
            } else {
                SkeletonHookUpdateError::Hook(failure)
            })
        );
        assert_bits(
            state.animation_board_to_physics_board(),
            translated([1.0, 2.0, 3.0, 0.0]),
        );
        if matches!(failure, "get0" | "set0") {
            assert_bits(&hooks.matrices[0], IDENTITY);
        } else {
            assert_bits(&hooks.matrices[0], translated([1.0, 2.0, 3.0, 4.0]));
        }
        if failure == "dot" {
            assert_bits(&hooks.matrices[1], physical);
            assert_eq!(
                proxy,
                Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
                    [9.0, 8.0, 7.0, 6.0]
                ))
            );
        } else {
            assert_bits(&hooks.matrices[1], IDENTITY);
            assert_eq!(
                proxy,
                Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
                    [50.0; 4]
                ))
            );
        }
        let count = match failure {
            "get0" => 1,
            "set0" => 2,
            "set1" => 3,
            _ => 4,
        };
        assert_eq!(&*trace.borrow(), &["get0", "set0", "set1", "dot"][..count]);
    }
}

#[test]
fn hooks_distance_uses_ordered_greater_than_one_and_nan_does_not_reset() {
    for (squared, expected) in [
        (0.0, true),
        (1.0, true),
        (f32::from_bits(0x3f80_0001), false),
        (f32::INFINITY, false),
        (f32::NAN, true),
    ] {
        let trace = Trace::default();
        let mut numeric = Numeric::new(&trace);
        numeric.squared = Some(squared);
        let result = SkeletonGeneralUpdate::new()
            .try_update_hooks(
                &SkeletonRootTransforms::new(),
                &IDENTITY,
                &IDENTITY,
                &mut None,
                &mut Hooks::new(&trace),
                &mut numeric,
            )
            .unwrap();
        assert_eq!(result, expected);
    }
}

#[test]
fn teleporting_still_calls_dot_after_both_hooks_and_proxy_and_preserves_numeric_failure() {
    for fail in [false, true] {
        let trace = Trace::default();
        let mut state = SkeletonGeneralUpdate::new();
        state.teleporting = true;
        let mut numeric = Numeric::new(&trace);
        numeric.squared = Some(f32::NAN);
        numeric.fail = fail.then_some("dot");
        let mut proxy = None;
        let result = state.try_update_hooks(
            &SkeletonRootTransforms::new(),
            &IDENTITY,
            &IDENTITY,
            &mut proxy,
            &mut Hooks::new(&trace),
            &mut numeric,
        );
        assert_eq!(
            result,
            if fail {
                Err(SkeletonHookUpdateError::Numeric("dot"))
            } else {
                Ok(false)
            }
        );
        assert_eq!(
            proxy,
            Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4]
            ))
        );
        assert_eq!(&*trace.borrow(), &["get0", "set0", "set1", "dot"]);
    }
}

#[test]
fn hook_publication_preserves_unknown_physical_root_w_in_same_proxy_target() {
    let trace = Trace::default();
    let mut hooks = Hooks::new(&trace);
    let mut state = SkeletonGeneralUpdate::new();
    let target = crate::skeleton_root_transforms::SkeletonRootVector {
        xyz: [9.0, 8.0, 7.0],
        w: None,
    };
    state.skate_root.set_column(3, target);
    let mut proxy = None;
    state
        .try_update_hooks(
            &SkeletonRootTransforms::new(),
            &IDENTITY,
            &IDENTITY,
            &mut proxy,
            &mut hooks,
            &mut Numeric::new(&trace),
        )
        .unwrap();
    assert_eq!(
        proxy.unwrap().xyz.map(f32::to_bits),
        target.xyz.map(f32::to_bits)
    );
    assert_eq!(proxy.unwrap().w, None);
    assert_eq!(
        hooks.matrices[1].columns()[3].xyz.map(f32::to_bits),
        target.xyz.map(f32::to_bits)
    );
    assert_eq!(hooks.matrices[1].columns()[3].w, None);
    assert_eq!(&*trace.borrow(), &["get0", "set0", "set1", "dot"]);
}
