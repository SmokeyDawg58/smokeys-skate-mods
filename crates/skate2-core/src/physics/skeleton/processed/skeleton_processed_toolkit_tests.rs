use super::*;
use crate::skeleton_processed_contact::SkeletonProcessedContactBoard;

struct Board {
    deck: SkeletonRootMatrix,
    calls: Vec<&'static str>,
    flags: u32,
}
impl SkeletonProcessedContactBoard for Board {
    type Error = &'static str;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("mass");
        Ok(7.0)
    }
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.calls.push("deck");
        Ok(self.deck)
    }
    fn flags_8c4(&self) -> u32 {
        self.flags
    }
}
struct Input {
    flags: u32,
    speeds: [f32; 2],
    reads: usize,
    fail_flags: bool,
    fail_speed: Option<usize>,
}
impl SkeletonProcessedToolkitInputRead for Input {
    type Error = &'static str;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        if self.fail_flags {
            Err("flags")
        } else {
            Ok(self.flags)
        }
    }
    fn z_speed(&mut self) -> Result<f32, Self::Error> {
        self.reads += 1;
        if self.fail_speed == Some(self.reads) {
            return Err("speed");
        }
        Ok(self.speeds[self.reads - 1])
    }
}
#[derive(Default)]
struct Numeric {
    dots: usize,
    roots: usize,
    fail_dot: Option<usize>,
    fail_root: Option<usize>,
    first_dot: Option<f32>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.dots += 1;
        if self.fail_dot == Some(self.dots) {
            return Err("dot");
        }
        if self.dots == 1 {
            if let Some(value) = self.first_dot {
                return Ok(value);
            }
        }
        Ok(crate::reckoning_numeric::host_dot3(a, b))
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.roots += 1;
        if self.fail_root == Some(self.roots) {
            return Err("root");
        }
        Ok(1.0 / x.sqrt())
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no reciprocal in direction continuation")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no integer conversion in direction continuation")
    }
}
fn vec4(x: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector::known(x)
}
fn fixture() -> (SkeletonProcessedContact, Board, Input, Numeric) {
    let mut board = Board {
        deck: SkeletonRootMatrix::from_columns([
            vec4([0.0, 0.0, -2.0, 4.0]),
            vec4([0.0, 2.0, 0.0, 4.0]),
            vec4([2.0, 1.0, 0.0, 6.0]),
            SkeletonRootVector {
                xyz: [3.0, 4.0, 5.0],
                w: None,
            },
        ]),
        calls: vec![],
        flags: 0x0010_0000,
    };
    let mut owner = SkeletonProcessedContact::new();
    owner.try_sample_toolkit_prefix(&mut board).unwrap();
    (
        owner,
        board,
        Input {
            flags: 0x0010_0000,
            speeds: [3.0, -2.0],
            reads: 0,
            fail_flags: false,
            fail_speed: None,
        },
        Numeric::default(),
    )
}
fn xyz_bits(vector: Option<SkeletonRootVector>) -> [u32; 3] {
    vector.unwrap().xyz.map(f32::to_bits)
}

#[test]
fn same_owner_uses_physical_axes_and_existing_effective_inverse_without_body_reread() {
    let (mut owner, board, mut input, mut numeric) = fixture();
    owner
        .try_continue_toolkit_directions(&mut input, &mut numeric)
        .unwrap();
    assert_eq!(board.calls, ["mass", "deck", "deck"]);
    let state = owner.toolkit_state();
    assert!(state.directions_complete());
    assert_eq!(state.body_x(), Some(vec4([-0.0, -0.0, 2.0, -4.0])));
    assert_eq!(state.body_y(), Some(vec4([0.0, 2.0, 0.0, 4.0])));
    assert_eq!(state.body_z(), Some(vec4([-2.0, -1.0, -0.0, -6.0])));
    assert_eq!(
        state.body_z_direction(),
        Some(vec4([-1.0, 0.0, -0.0, -3.0]))
    );
    assert_eq!(state.body_y_direction(), Some(vec4([0.0, 1.0, 0.0, 2.0])));
    assert_eq!(
        state.z_axis_velocity(),
        Some(vec4([-6.0, -3.0, -0.0, -18.0]))
    );
    assert_eq!(
        state.z_axis_velocity_dir(),
        Some(vec4([2.0, 1.0, 0.0, 6.0]))
    );
    assert_eq!(state.abs_body_speed().unwrap().to_bits(), 2.0_f32.to_bits());
    let inverse = state.inverse_effective().unwrap();
    assert_eq!(
        inverse.columns()[3].xyz.map(f32::to_bits),
        [-10.0_f32, -8.0, 10.0].map(f32::to_bits)
    );
    for column in &inverse.columns()[..3] {
        assert_eq!(column.w.unwrap().to_bits(), 0);
    }
    assert_eq!(input.reads, 2);
    assert_eq!((numeric.dots, numeric.roots), (4, 2));
}

#[test]
fn late_toolkit_flip_sample_can_differ_from_completed_effective_prefix() {
    let (mut owner, _, mut input, mut numeric) = fixture();
    input.flags = 0;
    owner
        .try_continue_toolkit_directions(&mut input, &mut numeric)
        .unwrap();
    let state = owner.toolkit_state();
    assert_eq!(state.body_x(), Some(vec4([0.0, 0.0, -2.0, 4.0])));
    assert_eq!(state.body_z(), Some(vec4([2.0, 1.0, 0.0, 6.0])));
    assert_eq!(
        state.inverse_effective().unwrap().columns()[2].xyz[0].to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        state.flipped_controls_scalar().unwrap().to_bits(),
        1.0_f32.to_bits()
    );
}

#[test]
fn missing_prefix_and_late_flag_failure_keep_distinct_store_prefixes() {
    let (_, _, mut input, mut numeric) = fixture();
    let mut owner = SkeletonProcessedContact::new();
    assert!(matches!(
        owner.try_continue_toolkit_directions(&mut input, &mut numeric),
        Err(SkeletonProcessedToolkitError::MissingPrefix)
    ));
    assert!(owner.toolkit_state().inverse_effective().is_none());
    assert_eq!(numeric.dots, 0);
    let (mut owner, _, mut input, mut numeric) = fixture();
    input.fail_flags = true;
    assert!(
        owner
            .try_continue_toolkit_directions(&mut input, &mut numeric)
            .is_err()
    );
    let state = owner.toolkit_state();
    assert!(state.inverse_effective().is_some());
    assert_eq!(state.body_z(), Some(vec4([2.0, 1.0, 0.0, 6.0])));
    assert!(state.flipped_controls_scalar().is_none());
    assert!(!state.directions_complete());
}

#[test]
fn first_and_second_dot_failures_distinguish_direction_store_boundary() {
    for failed in [1, 2] {
        let (mut owner, _, mut input, mut numeric) = fixture();
        numeric.fail_dot = Some(failed);
        assert!(
            owner
                .try_continue_toolkit_directions(&mut input, &mut numeric)
                .is_err()
        );
        let state = owner.toolkit_state();
        assert!(state.body_z().is_some());
        if failed == 1 {
            assert!(state.body_z_direction().is_none());
        } else {
            assert_eq!(
                state.body_z_direction(),
                Some(vec4([-2.0, 0.0, -0.0, -6.0]))
            );
        }
        assert!(state.body_y_direction().is_none());
        assert_eq!(input.reads, 0);
        assert!(!state.directions_complete());
    }
}

#[test]
fn raw_body_y_store_survives_dot_or_estimate_failure() {
    for fail_dot in [true, false] {
        let (mut owner, _, mut input, mut numeric) = fixture();
        if fail_dot {
            numeric.fail_dot = Some(4);
        } else {
            numeric.fail_root = Some(2);
        }
        assert!(
            owner
                .try_continue_toolkit_directions(&mut input, &mut numeric)
                .is_err()
        );
        assert_eq!(
            owner.toolkit_state().body_y_direction(),
            Some(vec4([0.0, 2.0, 0.0, 4.0]))
        );
        assert_eq!(input.reads, 0);
        assert!(!owner.toolkit_state().directions_complete());
    }
}

#[test]
fn exact_threshold_and_unordered_first_dot_skip_second_dot_and_z_normalization() {
    for first in [f32::from_bits(0x3727_c5ac), f32::NAN, 0.0] {
        let (mut owner, _, mut input, mut numeric) = fixture();
        numeric.first_dot = Some(first);
        owner
            .try_continue_toolkit_directions(&mut input, &mut numeric)
            .unwrap();
        assert_eq!(
            owner.toolkit_state().body_z_direction(),
            Some(vec4([-2.0, 0.0, -0.0, -6.0]))
        );
        assert_eq!((numeric.dots, numeric.roots), (3, 1));
    }
}

#[test]
fn degenerate_body_y_normalization_is_unconditional_and_has_no_zero_fallback() {
    let (mut owner, mut board, mut input, mut numeric) = fixture();
    let mut columns = *board.deck.columns();
    columns[1] = vec4([0.0; 4]);
    board.deck = SkeletonRootMatrix::from_columns(columns);
    owner.try_sample_toolkit_prefix(&mut board).unwrap();
    owner
        .try_continue_toolkit_directions(&mut input, &mut numeric)
        .unwrap();
    assert!(
        owner
            .toolkit_state()
            .body_y_direction()
            .unwrap()
            .xyz
            .into_iter()
            .all(f32::is_nan)
    );
    assert!(owner.toolkit_state().directions_complete());
    assert_eq!(numeric.roots, 2);
}

#[test]
fn repeated_speed_load_failure_retains_first_velocity_without_abs_or_direction() {
    let (mut owner, _, mut input, mut numeric) = fixture();
    input.fail_speed = Some(2);
    assert!(
        owner
            .try_continue_toolkit_directions(&mut input, &mut numeric)
            .is_err()
    );
    let state = owner.toolkit_state();
    assert_eq!(
        state.z_axis_velocity(),
        Some(vec4([-6.0, -3.0, -0.0, -18.0]))
    );
    assert!(state.abs_body_speed().is_none());
    assert!(state.z_axis_velocity_dir().is_none());
    assert!(!state.directions_complete());
}

#[test]
fn speed_sign_preserves_signed_zero_and_nan_gt_not_set_selection() {
    for (speed, abs_bits) in [(0.0, Some(0x8000_0000)), (-0.0, Some(0)), (f32::NAN, None)] {
        let (mut owner, _, mut input, mut numeric) = fixture();
        input.speeds = [1.0, speed];
        owner
            .try_continue_toolkit_directions(&mut input, &mut numeric)
            .unwrap();
        let state = owner.toolkit_state();
        if let Some(bits) = abs_bits {
            assert_eq!(state.abs_body_speed().unwrap().to_bits(), bits);
        } else {
            assert!(state.abs_body_speed().unwrap().is_nan());
        }
        assert_eq!(
            xyz_bits(state.z_axis_velocity_dir()),
            [2.0_f32, 1.0, 0.0].map(f32::to_bits)
        );
    }
}

#[test]
fn unknown_axis_w_propagates_and_reset_invalidates_without_fake_zero_stores() {
    let (mut owner, mut board, mut input, mut numeric) = fixture();
    let mut columns = *board.deck.columns();
    columns[2].w = None;
    columns[1].w = None;
    board.deck = SkeletonRootMatrix::from_columns(columns);
    owner.try_sample_toolkit_prefix(&mut board).unwrap();
    owner
        .try_continue_toolkit_directions(&mut input, &mut numeric)
        .unwrap();
    assert_eq!(owner.toolkit_state().body_z_direction().unwrap().w, None);
    assert_eq!(owner.toolkit_state().body_y_direction().unwrap().w, None);
    assert_eq!(owner.toolkit_state().z_axis_velocity().unwrap().w, None);
    let prior = owner.toolkit_state().body_z_direction();
    owner.reset();
    assert!(!owner.toolkit_state().directions_complete());
    assert_eq!(owner.toolkit_state().body_z_direction(), prior);
}

#[derive(Clone, Default)]
struct JumpLog(std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>);
impl JumpLog {
    fn push(&self, event: &'static str) {
        self.0.borrow_mut().push(event);
    }
    fn events(&self) -> Vec<&'static str> {
        self.0.borrow().clone()
    }
}
struct JumpInput {
    log: JumpLog,
    ground: SkeletonRootVector,
}
impl SkeletonProcessedToolkitJumpInputRead for JumpInput {
    type Error = &'static str;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.log.push("ground");
        Ok(self.ground)
    }
}
struct JumpBoard {
    log: JumpLog,
    normal: SkeletonRootVector,
    reads: usize,
    second_read: Option<SkeletonRootVector>,
    fail_second: bool,
}
impl SkeletonProcessedToolkitJumpBoard for JumpBoard {
    type Error = &'static str;
    fn jump_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.log.push("board_read");
        self.reads += 1;
        if self.reads == 2 {
            if self.fail_second {
                return Err("final board read");
            }
            if let Some(normal) = self.second_read {
                return Ok(normal);
            }
        }
        Ok(self.normal)
    }
    fn set_jump_normal(&mut self, normal: SkeletonRootVector) -> Result<(), Self::Error> {
        self.log.push("board_write");
        self.normal = normal;
        Ok(())
    }
}
struct JumpCvars {
    log: JumpLog,
    maximum: f32,
    blend: f32,
    draw: bool,
    fail: Option<&'static str>,
}
impl JumpCvars {
    fn event(&self, event: &'static str) -> Result<(), &'static str> {
        self.log.push(event);
        if self.fail == Some(event) {
            Err(event)
        } else {
            Ok(())
        }
    }
}
impl SkeletonProcessedToolkitJumpCvars for JumpCvars {
    type FloatTarget = ToolkitJumpFloat;
    type BoolTarget = ();
    type Error = &'static str;
    fn capture_float(&mut self, field: ToolkitJumpFloat) -> Result<Self::FloatTarget, Self::Error> {
        self.event(match field {
            ToolkitJumpFloat::MaxCosAngleDelta => "max_capture",
            ToolkitJumpFloat::ProportionalBlend => "blend_capture",
        })?;
        Ok(field)
    }
    fn float(&mut self, target: &Self::FloatTarget) -> Result<f32, Self::Error> {
        self.event(match target {
            ToolkitJumpFloat::MaxCosAngleDelta => "max_value",
            ToolkitJumpFloat::ProportionalBlend => "blend_value",
        })?;
        Ok(match target {
            ToolkitJumpFloat::MaxCosAngleDelta => self.maximum,
            ToolkitJumpFloat::ProportionalBlend => self.blend,
        })
    }
    fn capture_draw_jump(&mut self) -> Result<(), Self::Error> {
        self.event("draw_capture")
    }
    fn draw_jump(&mut self, (): &()) -> Result<bool, Self::Error> {
        self.event("draw_value")?;
        Ok(self.draw)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.event("overlay")
    }
}
struct JumpNumeric {
    log: JumpLog,
    inner: Numeric,
    dot_inputs: Vec<([f32; 4], [f32; 4])>,
}
impl ReckoningNumeric for JumpNumeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.log.push("dot");
        self.dot_inputs.push((a, b));
        self.inner.dot3(a, b)
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.log.push("root");
        self.inner.reciprocal_sqrt_estimate(x)
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("jump tail has no reciprocal")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("jump tail has no truncate")
    }
}
fn jump_fixture() -> (
    SkeletonProcessedContact,
    JumpLog,
    JumpInput,
    JumpBoard,
    JumpCvars,
    JumpNumeric,
) {
    let (mut owner, _, mut input, mut numeric) = fixture();
    owner
        .try_continue_toolkit_directions(&mut input, &mut numeric)
        .unwrap();
    let log = JumpLog::default();
    (
        owner,
        log.clone(),
        JumpInput {
            log: log.clone(),
            ground: vec4([1.0, 0.0, 0.0, 2.0]),
        },
        JumpBoard {
            log: log.clone(),
            normal: vec4([0.0, 1.0, 0.0, 0.0]),
            reads: 0,
            second_read: None,
            fail_second: false,
        },
        JumpCvars {
            log: log.clone(),
            maximum: 0.9,
            blend: 0.5,
            draw: false,
            fail: None,
        },
        JumpNumeric {
            log,
            inner: Numeric::default(),
            dot_inputs: vec![],
        },
    )
}

#[test]
fn jump_tail_blends_and_normalizes_with_staged_live_reads_and_full_w() {
    let (mut owner, log, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
    owner
        .try_continue_toolkit_jump_normal(&mut input, &mut board, &mut cvars, &mut numeric)
        .unwrap();
    assert_eq!(
        log.events(),
        [
            "max_capture",
            "ground",
            "board_read",
            "dot",
            "max_value",
            "blend_capture",
            "blend_value",
            "dot",
            "root",
            "board_write",
            "draw_capture",
            "draw_value",
            "board_read"
        ]
    );
    assert!(owner.toolkit_state().jump_tail_complete());
    assert_eq!(
        owner.toolkit_state().processed_jump_normal(),
        Some(board.normal)
    );
    assert_eq!(board.normal.xyz[0].to_bits(), board.normal.xyz[1].to_bits());
    assert_eq!(
        board.normal.w.unwrap().to_bits(),
        (board.normal.xyz[0] * 2.0).to_bits()
    );
}

#[test]
fn jump_tail_ordered_gt_copies_ground_without_blend_or_normalization() {
    let (mut owner, log, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
    input.ground = vec4([0.0, 2.0, 0.0, 7.0]);
    owner
        .try_continue_toolkit_jump_normal(&mut input, &mut board, &mut cvars, &mut numeric)
        .unwrap();
    assert_eq!(board.normal, input.ground);
    assert!(!log.events().contains(&"blend_capture"));
    assert_eq!(numeric.inner.dots, 1);
    assert_eq!(numeric.inner.roots, 0);
}

#[test]
fn jump_tail_requires_directions_and_first_dot_failure_never_reads_max_value() {
    let (mut owner, log, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
    owner.toolkit.invalidate();
    assert!(matches!(
        owner.try_continue_toolkit_jump_normal(&mut input, &mut board, &mut cvars, &mut numeric),
        Err(SkeletonProcessedToolkitJumpError::MissingDirections)
    ));
    assert!(log.events().is_empty());
    let (mut owner, log, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
    numeric.inner.fail_dot = Some(1);
    let before = board.normal;
    assert!(
        owner
            .try_continue_toolkit_jump_normal(&mut input, &mut board, &mut cvars, &mut numeric)
            .is_err()
    );
    assert_eq!(log.events(), ["max_capture", "ground", "board_read", "dot"]);
    assert_eq!(board.normal, before);
    assert!(owner.toolkit_state().processed_jump_normal().is_none());
}

#[test]
fn jump_tail_normalization_failure_keeps_old_board_and_processed_normal() {
    let (mut owner, log, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
    numeric.inner.fail_root = Some(1);
    let before = board.normal;
    assert!(
        owner
            .try_continue_toolkit_jump_normal(&mut input, &mut board, &mut cvars, &mut numeric)
            .is_err()
    );
    assert_eq!(board.normal, before);
    assert!(!log.events().contains(&"board_write"));
    assert!(!owner.toolkit_state().jump_tail_complete());
}

#[test]
fn jump_tail_overlay_or_draw_failure_keeps_new_board_but_prior_processed_store() {
    for failure in ["draw_capture", "draw_value", "overlay"] {
        let (mut owner, log, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
        let prior = vec4([13.0; 4]);
        owner.toolkit.processed_jump_normal = Some(prior);
        cvars.draw = true;
        cvars.fail = Some(failure);
        let before = board.normal;
        assert!(
            owner
                .try_continue_toolkit_jump_normal(&mut input, &mut board, &mut cvars, &mut numeric)
                .is_err()
        );
        assert_ne!(board.normal, before);
        assert_eq!(owner.toolkit_state().processed_jump_normal(), Some(prior));
        assert!(!owner.toolkit_state().jump_tail_complete());
        assert_eq!(
            log.events()
                .iter()
                .filter(|&&event| event == "board_read")
                .count(),
            1
        );
    }
}

#[test]
fn jump_tail_final_board_reread_is_authoritative_and_can_fail_after_overlay() {
    for fail in [false, true] {
        let (mut owner, log, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
        cvars.draw = true;
        board.second_read = Some(vec4([9.0, 8.0, 7.0, 6.0]));
        board.fail_second = fail;
        let result = owner.try_continue_toolkit_jump_normal(
            &mut input,
            &mut board,
            &mut cvars,
            &mut numeric,
        );
        assert!(log.events().contains(&"overlay"));
        if fail {
            assert!(result.is_err());
            assert!(owner.toolkit_state().processed_jump_normal().is_none());
            assert!(!owner.toolkit_state().jump_tail_complete());
        } else {
            result.unwrap();
            assert_eq!(
                owner.toolkit_state().processed_jump_normal(),
                board.second_read
            );
            assert_ne!(
                owner.toolkit_state().processed_jump_normal(),
                Some(board.normal)
            );
        }
    }
}

#[test]
fn jump_tail_nan_comparison_blends_and_zero_fallback_recovers_only_proven_w() {
    for zero in [false, true] {
        let (mut owner, log, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
        cvars.maximum = f32::NAN;
        board.normal.w = None;
        if zero {
            board.normal = SkeletonRootVector {
                xyz: [-1.0, 0.0, 0.0],
                w: None,
            };
            input.ground = vec4([1.0, 0.0, 0.0, 9.0]);
        }
        owner
            .try_continue_toolkit_jump_normal(&mut input, &mut board, &mut cvars, &mut numeric)
            .unwrap();
        assert!(log.events().contains(&"blend_capture"));
        if zero {
            assert_eq!(board.normal, vec4([0.0; 4]));
            assert_eq!(numeric.inner.roots, 0);
        } else {
            assert_eq!(board.normal.w, None);
            assert_eq!(
                owner.toolkit_state().processed_jump_normal().unwrap().w,
                None
            );
        }
    }
}

#[test]
fn jump_blend_preserves_fused_old_product_after_rounded_current_product() {
    let (mut owner, _, mut input, mut board, mut cvars, mut numeric) = jump_fixture();
    let epsilon = f32::from_bits(0x3400_0000);
    board.normal = vec4([f32::from_bits(0x3f80_0001), 1.0, 0.0, 0.0]);
    input.ground = vec4([8_388_610.0, 0.0, 0.0, 0.0]);
    cvars.blend = -epsilon;
    cvars.maximum = f32::INFINITY;
    owner
        .try_continue_toolkit_jump_normal(&mut input, &mut board, &mut cvars, &mut numeric)
        .unwrap();
    assert_eq!(numeric.dot_inputs[1].0[0].to_bits(), 0x2880_0000);
}
