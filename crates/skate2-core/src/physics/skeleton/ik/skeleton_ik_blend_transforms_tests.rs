#![expect(
    clippy::float_cmp,
    reason = "exact finite fixture values; numerical discriminators use bit assertions"
)]
use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric, reckoning_sin_cos::HostReckoningSinCosNumeric,
};
use std::sync::Arc;

const IDENTITY: AnimationMatrix = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
#[derive(Default)]
struct Numeric {
    calls: Vec<(&'static str, u32)>,
    fail: Option<usize>,
}
impl Numeric {
    fn event(&mut self, name: &'static str, input: f32, value: f32) -> Result<f32, &'static str> {
        let index = self.calls.len();
        self.calls.push((name, input.to_bits()));
        if self.fail == Some(index) {
            Err("refused")
        } else {
            Ok(value)
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        let value = HostReckoningNumeric.dot3(a, b).unwrap();
        self.event("dot", value, value)
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.event("reciprocal", v, v.recip())
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.event("rsqrt", v, v.sqrt().recip())
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no conversion")
    }
}
fn transform(position: [f32; 4]) -> SkeletonRootMatrix {
    SkeletonRootMatrix::translation(SkeletonRootVector::known(position))
}
fn fixture() -> (
    SkeletonIkState,
    SkeletonIkTargets,
    SkeletonIkDataIn,
    [AnimationMatrix; 24],
) {
    let (hierarchy, parameters, pose) = crate::skeleton_data_tests::fixture();
    let data = Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap());
    let mut targets = SkeletonIkTargets::new(data);
    targets.initialize();
    let state = SkeletonIkState::new(&mut targets);
    let matrix = SkeletonRootMatrix::known(IDENTITY);
    let data = SkeletonIkDataIn {
        animation_to_world: matrix,
        world_to_animation: matrix,
        physics_skateboard: matrix,
        inverse_skateboard: matrix,
        hips: matrix,
        skateboard: matrix,
        toe_contact_normal: SkeletonRootVector::known([0.0; 4]),
        hips_offset: SkeletonRootVector::known([0.0; 4]),
    };
    (state, targets, data, [IDENTITY; 24])
}

#[test]
fn hybrid_board_uses_previous_deck_position_and_physical_basis_before_primary_parent_blend() {
    let (mut state, targets, mut data, mut previous) = fixture();
    state.modes[0] = 1;
    state.on_deck_blends[0] = 0.5;
    state.part_to_world[0] = transform([1.0, 2.0, 3.0, 4.0]);
    state.part_to_deck[0] = transform([7.0, 11.0, 13.0, 17.0]);
    state.parent_to_deck[0] = transform([-1.0, -2.0, -3.0, 19.0]);
    data.animation_to_world = transform([10.0, 20.0, 30.0, 23.0]);
    let mut columns = *data.animation_to_world.columns();
    columns[3].w = None;
    data.animation_to_world = SkeletonRootMatrix::from_columns(columns);
    data.physics_skateboard = transform([1000.0, 2000.0, 3000.0, 4000.0]);
    let mut physical = *data.physics_skateboard.columns();
    physical[0].xyz[0] = 2.0;
    physical[1].xyz[1] = 3.0;
    physical[2].xyz[2] = 4.0;
    data.physics_skateboard = SkeletonRootMatrix::from_columns(physical);
    previous[0][12..].copy_from_slice(&[2.0, 3.0, 5.0, 29.0]);
    state
        .try_blend_transforms(
            &data,
            &targets,
            &previous,
            &mut Numeric::default(),
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert_eq!(state.part_to_world[0].columns()[3].xyz, [13.5, 29.0, 45.0]);
    assert_eq!(state.parent_to_world[0].columns()[3].xyz, [5.0, 8.5, 11.5]);
    assert_eq!(state.part_to_world[0].columns()[3].w, None);
    assert_eq!(state.parent_to_world[0].columns()[3].w, None);
    assert_eq!(state.part_to_deck[0], transform([7.0, 11.0, 13.0, 17.0]));
}

#[test]
fn base_position_uses_candidate_fma_after_rounded_old_product() {
    let from = transform([0.0, 0.0, 0.0, f32::from_bits(0x40c8_cbc3)]);
    let to = transform([0.0, 0.0, 0.0, f32::from_bits(0xc0d3_7cc1)]);
    let result = try_base_blend(
        &from,
        &to,
        f32::from_bits(0x3ed8_0623),
        &mut Numeric::default(),
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap();
    // Independent Python/ucrt fmaf: source base3f56c1cd, generic affine
    // wrapper3f56c1ce, fully defused3f56c1cc. These paths are not interchangeable.
    assert_eq!(result.columns()[3].w.unwrap().to_bits(), 0x3f56_c1cd);
}

#[test]
fn both_normalizations_run_at_upper_endpoint_and_parent_failures_keep_primary_commit() {
    let (mut complete, targets, data, previous) = fixture();
    complete.modes[0] = 1;
    complete.on_deck_blends[0] = 1.0;
    complete.part_to_deck[0] = transform([2.0, 3.0, 5.0, 7.0]);
    complete.parent_to_deck[0] = transform([11.0, 13.0, 17.0, 19.0]);
    let mut baseline = Numeric::default();
    complete
        .try_blend_transforms(
            &data,
            &targets,
            &previous,
            &mut baseline,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert_eq!(
        baseline.calls.len(),
        36,
        "two classical normalizers per primary and parent, then raw upper endpoint"
    );
    for index in 0..36 {
        let (mut state, targets, data, previous) = fixture();
        state.modes[0] = 1;
        state.on_deck_blends[0] = 1.0;
        state.part_to_deck[0] = complete.part_to_deck[0];
        state.parent_to_deck[0] = complete.parent_to_deck[0];
        let old_primary = state.part_to_world[0];
        let old_parent = state.parent_to_world[0];
        let mut numeric = Numeric {
            fail: Some(index),
            ..Numeric::default()
        };
        assert_eq!(
            state.try_blend_transforms(
                &data,
                &targets,
                &previous,
                &mut numeric,
                &mut HostReckoningSinCosNumeric
            ),
            Err(SkeletonIkTransformBlendError::Affine(
                SkeletonAffineInterpolationError::Numeric("refused")
            ))
        );
        assert_eq!(numeric.calls, baseline.calls[..=index]);
        assert_eq!(
            state.part_to_world[0],
            if index < 18 {
                old_primary
            } else {
                complete.part_to_world[0]
            }
        );
        assert_eq!(state.parent_to_world[0], old_parent);
    }
}

#[test]
fn nan_external_blend_copies_both_matrices_for_hand_without_parent_and_no_services() {
    let (mut state, targets, data, previous) = fixture();
    assert!(targets.parents()[2].is_none());
    state.modes[2] = 2;
    state.external_blends[2] = f32::NAN;
    state.part_to_world_external[2] = transform([2.0, 3.0, 5.0, 7.0]);
    state.parent_to_world_external[2] = transform([11.0, 13.0, 17.0, 19.0]);
    let mut numeric = Numeric::default();
    state
        .try_blend_transforms(
            &data,
            &targets,
            &previous,
            &mut numeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert_eq!(state.part_to_world[2], state.part_to_world_external[2]);
    assert_eq!(state.parent_to_world[2], state.parent_to_world_external[2]);
    assert!(numeric.calls.is_empty());
}

#[test]
fn hand_external_parent_interpolation_failure_preserves_completed_primary() {
    let (mut state, targets, data, previous) = fixture();
    state.modes[2] = 3;
    state.external_blends[2] = 0.5;
    state.part_to_world_external[2] = transform([4.0, 6.0, 8.0, 10.0]);
    state.parent_to_world_external[2] = transform([12.0, 14.0, 16.0, 18.0]);
    let mut first = Numeric::default();
    try_interpolate_affine_typed(
        &state.part_to_world[2],
        &state.part_to_world_external[2],
        0.5,
        &mut first,
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap();
    let old_parent = state.parent_to_world[2];
    let mut numeric = Numeric {
        fail: Some(first.calls.len()),
        ..Numeric::default()
    };
    assert_eq!(
        state.try_blend_transforms(
            &data,
            &targets,
            &previous,
            &mut numeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonIkTransformBlendError::Affine(
            SkeletonAffineInterpolationError::Numeric("refused")
        ))
    );
    assert_eq!(
        state.part_to_world[2].columns()[3],
        SkeletonRootVector::known([2.0, 3.0, 4.0, 5.0])
    );
    assert_eq!(state.parent_to_world[2], old_parent);
    assert_eq!(&numeric.calls[..first.calls.len()], first.calls);
}

#[test]
fn disabled_invalid_and_nonpositive_base_modes_preserve_world_outputs() {
    let (mut state, targets, data, previous) = fixture();
    state.modes = [0, u32::MAX, 1, 1];
    state.on_deck_blends = [1.0, 1.0, f32::NAN, -1.0];
    state.part_to_world = [transform([2.0, 3.0, 5.0, 7.0]); 4];
    state.parent_to_world = [transform([11.0, 13.0, 17.0, 19.0]); 4];
    let mut numeric = Numeric::default();
    assert_eq!(
        state.try_blend_transforms(
            &data,
            &targets,
            &[],
            &mut numeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonIkTransformBlendError::PreviousDeckUnavailable)
    );
    state
        .try_blend_transforms(
            &data,
            &targets,
            &previous,
            &mut numeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert_eq!(state.part_to_world, [transform([2.0, 3.0, 5.0, 7.0]); 4]);
    assert_eq!(
        state.parent_to_world,
        [transform([11.0, 13.0, 17.0, 19.0]); 4]
    );
    assert!(numeric.calls.is_empty());
}
