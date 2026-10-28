#![expect(
    clippy::float_cmp,
    reason = "exact finite fixture values; numerical discriminators use bit assertions"
)]
use super::super::initial::tests::{Services, data, matrix, state, targets};
use super::*;

type Error = SkeletonIkExternalError<&'static str, &'static str, &'static str>;

#[test]
fn local_foot_setter_transform_rotates_world_point_into_animation_space() {
    let matrix = SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([0.0, 0.0, -1.0, 0.0]),
        SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
        SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
        SkeletonRootVector::known([10.0, 20.0, 30.0, 1.0]),
    ]);
    // Rotated basis: (x,y,z)=(2,3,4) maps to (14,23,28) after translation.
    let mut world = SkeletonRootVector::known([2.0, 3.0, 4.0, 99.0]);
    world.w = None; // native setter ignores input W
    assert_eq!(transform_point(&matrix, world).xyz, [14.0, 23.0, 28.0]);
    let mut unknown_matrix = matrix;
    let mut axis = unknown_matrix.columns()[0];
    axis.w = None;
    unknown_matrix.set_column(0, axis);
    assert_eq!(transform_point(&unknown_matrix, world).w, None);
}
fn run(
    state: &mut SkeletonIkState,
    entry: usize,
    data: &SkeletonIkDataIn,
    targets: &SkeletonIkTargets,
    rows: &[AnimationMatrix],
    services: &Services,
) -> Result<(), Error> {
    state.try_calculate_external_offset(
        entry,
        data,
        targets,
        rows,
        &mut services.clone(),
        &mut services.clone(),
        &mut services.clone(),
    )
}

#[test]
fn local_target_uses_unclamped_blend_while_delta_accumulates_clamped_step() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[2] = 3;
    state.target_blends[2] = 0.5;
    state.external_target_local_deltas[2] = SkeletonRootVector::known([2.0, 0.0, 0.0, 4.0]);
    state.external_target_positions_local[2] = SkeletonRootVector::known([10.0, 0.0, 0.0, 12.0]);
    let mut rows = [matrix([0.0; 4]); 24];
    rows[3] = matrix([2.0, 0.0, 0.0, 2.0]);
    let services = Services {
        local_maximum: 0.75,
        fail_at: Some(5),
        ..Services::default()
    };
    assert_eq!(
        run(&mut state, 2, &data(), &targets, &rows, &services),
        Err(Error::Cvar("world"))
    );
    assert_eq!(
        state.external_target_positions_local[2],
        SkeletonRootVector::known([7.0, 0.0, 0.0, 9.0])
    );
    assert_eq!(
        state.external_target_local_deltas[2],
        SkeletonRootVector::known([2.75, 0.0, 0.0, 4.75])
    );
    assert_eq!(
        state.external_target_positions_world[2],
        SkeletonRootVector::known([7.0, 0.0, 0.0, 0.0])
    );
    assert_eq!(
        *services.events.borrow(),
        ["local", "dot", "rsqrt", "reciprocal", "world"]
    );
}

#[test]
fn failed_local_clamp_does_not_publish_any_local_position_or_delta() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[2] = 3;
    state.target_blends[2] = 1.0;
    let value = SkeletonRootVector::known([4.0, 0.0, 0.0, 8.0]);
    state.external_target_positions_local[2] = value;
    let services = Services {
        fail_at: Some(3),
        ..Services::default()
    };
    assert_eq!(
        run(
            &mut state,
            2,
            &data(),
            &targets,
            &[matrix([0.0; 4]); 24],
            &services
        ),
        Err(Error::Clamp(ReckoningClampError::Numeric("rsqrt")))
    );
    assert_eq!(state.external_target_positions_local[2], value);
    assert_eq!(
        state.external_target_local_deltas[2],
        SkeletonRootVector::known([0.0; 4])
    );
}

#[test]
fn world_mode_restamps_local_before_negative_world_limit_assertion() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[2] = 2;
    state.external_target_positions_world[2] = SkeletonRootVector::known([8.0, 2.0, 3.0, 9.0]);
    let mut data = data();
    data.world_to_animation = SkeletonRootMatrix::known(matrix([-3.0, 0.0, 0.0, 5.0]));
    let services = Services {
        world_maximum: -1.0,
        ..Services::default()
    };
    assert_eq!(
        run(
            &mut state,
            2,
            &data,
            &targets,
            &[matrix([0.0; 4]); 24],
            &services
        ),
        Err(Error::Clamp(ReckoningClampError::NegativeMaximum {
            bits: (-1.0_f32).to_bits()
        }))
    );
    assert_eq!(
        state.external_target_positions_local[2],
        SkeletonRootVector::known([5.0, 2.0, 3.0, 5.0])
    );
    assert_eq!(*services.events.borrow(), ["world"]);
}

#[test]
fn normal_decay_precedes_invalid_mode_and_keeps_nan_subtraction() {
    for decay in [0.25, f32::NAN] {
        let mut targets = targets();
        let mut state = state(&mut targets);
        state.external_target_normal_blends[2] = 0.5;
        let services = Services {
            decay,
            ..Services::default()
        };
        assert_eq!(
            run(
                &mut state,
                2,
                &data(),
                &targets,
                &[matrix([0.0; 4]); 24],
                &services
            ),
            Err(Error::InvalidMode { entry: 2, mode: 0 })
        );
        if decay.is_nan() {
            assert!(state.external_target_normal_blends[2].is_nan());
        } else {
            assert_eq!(
                state.external_target_normal_blends[2].to_bits(),
                0.25_f32.to_bits()
            );
        }
        assert_eq!(*services.events.borrow(), ["dot", "rsqrt", "decay"]);
    }
}

#[test]
fn normal_nan_blend_skips_every_normal_service() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[2] = 2;
    state.external_target_normal_blends[2] = f32::from_bits(0x7fc0_0042);
    let services = Services {
        fail_at: Some(1),
        ..Services::default()
    };
    assert_eq!(
        run(
            &mut state,
            2,
            &data(),
            &targets,
            &[matrix([0.0; 4]); 24],
            &services
        ),
        Err(Error::Cvar("world"))
    );
    assert_eq!(
        state.external_target_normal_blends[2].to_bits(),
        0x7fc0_0042
    );
    assert_eq!(*services.events.borrow(), ["world"]);
}

#[test]
fn normal_rotation_services_and_failures_follow_source_order() {
    let expected = [
        "dot",
        "rsqrt",
        "rsqrt",
        "reciprocal",
        "angle",
        "round",
        "decay",
        "world",
    ];
    for failure in 1..=8 {
        let mut targets = targets();
        let mut state = state(&mut targets);
        state.modes[2] = 2;
        state.external_target_normal_blends[2] = 0.5;
        state.external_target_normals[2] = SkeletonRootVector::known([0.0, 0.0, 0.5, 0.0]);
        let services = Services {
            fail_at: Some(failure),
            ..Services::default()
        };
        assert!(
            run(
                &mut state,
                2,
                &data(),
                &targets,
                &[matrix([0.0; 4]); 24],
                &services
            )
            .is_err()
        );
        assert_eq!(services.events.borrow().as_slice(), &expected[..failure]);
        let blend = if failure <= 7 { 0.5_f32 } else { 0.5_f32 - 0.2 };
        assert_eq!(
            state.external_target_normal_blends[2].to_bits(),
            blend.to_bits()
        );
        assert_eq!(
            state.part_to_world_external[2],
            SkeletonRootMatrix::known(super::super::IDENTITY)
        );
    }
}

#[test]
fn normal_set_preserves_blend_and_omits_decay_read_after_small_axis() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[2] = 2;
    state.external_target_normal_blends[2] = 0.5;
    state.external_target_normal_set[2] = true;
    let services = Services::default();
    run(
        &mut state,
        2,
        &data(),
        &targets,
        &[matrix([0.0; 4]); 24],
        &services,
    )
    .unwrap();
    assert_eq!(
        *services.events.borrow(),
        ["dot", "rsqrt", "world", "dot", "rsqrt"]
    );
    assert_eq!(
        state.external_target_normal_blends[2].to_bits(),
        0.5_f32.to_bits()
    );
}

#[test]
fn reconstructed_axis_length_tie_and_nan_skip_rotation_but_still_decay() {
    // The previous float below .05 reconstructs to exactly 3d4ccccd under
    // these declared host estimate seeds and two source refinements.
    for component in [f32::from_bits(0x3d4c_cccc), f32::NAN] {
        let mut targets = targets();
        let mut state = state(&mut targets);
        state.modes[2] = 2;
        state.external_target_normal_blends[2] = 0.5;
        state.external_target_normals[2] = SkeletonRootVector::known([0.0, 0.0, component, 0.0]);
        let services = Services {
            fail_at: Some(4),
            ..Services::default()
        };
        assert_eq!(
            run(
                &mut state,
                2,
                &data(),
                &targets,
                &[matrix([0.0; 4]); 24],
                &services
            ),
            Err(Error::Cvar("world"))
        );
        assert_eq!(
            *services.events.borrow(),
            ["dot", "rsqrt", "decay", "world"]
        );
        assert_eq!(
            state.external_target_normal_blends[2].to_bits(),
            (0.5_f32 - 0.2).to_bits()
        );
    }
}

#[test]
fn primary_external_commit_survives_late_parent_record_refusal() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[0] = 2;
    state.external_target_positions_world[0] = SkeletonRootVector::known([4.0, 0.0, 0.0, 0.0]);
    let services = Services {
        world_maximum: 2.0,
        ..Services::default()
    };
    assert_eq!(
        run(
            &mut state,
            0,
            &data(),
            &targets,
            &[matrix([0.0; 4]); 16],
            &services
        ),
        Err(Error::RecordUnavailable(SkeletonIkRecordRead::Parent {
            entry: 0,
            part: 16
        }))
    );
    assert_eq!(
        state.part_to_world_external[0].columns()[3],
        SkeletonRootVector::known([2.0, 0.0, 0.0, 0.0])
    );
    assert_eq!(
        state.parent_to_world_external[0],
        SkeletonRootMatrix::known(super::super::IDENTITY)
    );
}

#[test]
fn external_parent_uses_previous_primary_relative_frame() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[0] = 2;
    state.external_target_positions_world[0] = SkeletonRootVector::known([6.0, 0.0, 0.0, 0.0]);
    let mut rows = [matrix([0.0; 4]); 24];
    rows[15] = matrix([2.0, 0.0, 0.0, 7.0]);
    rows[16] = matrix([3.0, 4.0, 0.0, 9.0]);
    run(
        &mut state,
        0,
        &data(),
        &targets,
        &rows,
        &Services {
            world_maximum: 2.0,
            ..Services::default()
        },
    )
    .unwrap();
    assert_eq!(
        state.part_to_world_external[0].columns()[3].xyz,
        [4.0, 0.0, 0.0]
    );
    assert_eq!(
        state.parent_to_world_external[0].columns()[3].xyz,
        [5.0, 4.0, 0.0]
    );
}

#[test]
fn zero_angle_runs_rotation_and_full_pivot_recovers_basis_w() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[2] = 2;
    state.external_target_normal_blends[2] = 1.0;
    state.external_target_normal_set[2] = true;
    state.external_target_normals[2] = SkeletonRootVector {
        xyz: [0.0, 0.0, 0.5],
        w: None,
    };
    let mut data = data();
    let mut columns = *data.animation_to_world.columns();
    columns[0].w = None;
    data.animation_to_world = SkeletonRootMatrix::from_columns(columns);
    run(
        &mut state,
        2,
        &data,
        &targets,
        &[matrix([0.0; 4]); 24],
        &Services {
            maximum_angle: 0.0,
            ..Services::default()
        },
    )
    .unwrap();
    let result = state.part_to_world_external[2].columns();
    assert_eq!(result[0], SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]));
    assert_eq!(result[1], SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]));
    assert_eq!(result[2], SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]));
    assert!(result[3].w.is_none());
    // Rodrigues itself has W=X. The next outer translation composition
    // consumes only its XYZ coefficients and reconstructs known basis W=0.
    assert_eq!(
        normal_rotation([1.0, 0.0, 0.0], 0.0, 1.0).columns()[0].w,
        Some(1.0)
    );
}

#[test]
fn full_rotation_pivot_has_frozen_rounding_that_a_position_replacement_loses() {
    // Independent source-form oracle used ucrtbase.fmaf and rounded f32
    // arithmetic for ASin, the combined polynomial and all three compositions.
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[2] = 2;
    state.external_target_normal_blends[2] = 0.75;
    state.external_target_normal_set[2] = true;
    state.external_target_normals[2] = SkeletonRootVector::known([0.3, 0.8, 0.4, 0.0]);
    let desired = [100_000.625, -3000.0625, 7.5, 0.0];
    state.external_target_positions_world[2] = SkeletonRootVector::known(desired);
    let mut data = data();
    data.animation_to_world =
        SkeletonRootMatrix::known(matrix([100_000.125, -3000.0625, 7.5, 0.0]));
    let services = Services {
        maximum_angle: 0.3,
        world_maximum: 1.0,
        ..Services::default()
    };
    run(
        &mut state,
        2,
        &data,
        &targets,
        &[matrix([0.0; 4]); 24],
        &services,
    )
    .unwrap();
    let result = state.part_to_world_external[2].known_lanes().unwrap();
    assert_eq!(
        result.map(f32::to_bits),
        [
            0x3f7d_ad52,
            0xbe09_1399,
            0xbc46_3a16,
            0,
            0x3e09_1399,
            0x3f79_8c1c,
            0x3e36_c4cc,
            0,
            0xbc46_3a15,
            0xbe36_c4cc,
            0x3f7b_deca,
            0,
            0x47c3_5050,
            0xc53b_8101,
            0x40ef_ff46,
            0,
        ]
    );
    assert_ne!(result[13].to_bits(), desired[1].to_bits());
    assert_ne!(result[14].to_bits(), desired[2].to_bits());
    assert_eq!(
        *services.events.borrow(),
        [
            "dot",
            "rsqrt",
            "rsqrt",
            "reciprocal",
            "angle",
            "round",
            "world",
            "dot",
            "rsqrt",
            "reciprocal"
        ]
    );
}

#[test]
fn clamp_tiny_vectors_preserves_full_input_even_with_zero_maximum() {
    let value = SkeletonRootVector::known([f32::from_bits(0x3680_0000), 0.0, 0.0, 7.0]);
    let mut services = Services::default();
    assert_eq!(clamp(value, 0.0, &mut services).unwrap(), value);
    assert_eq!(*services.events.borrow(), ["dot", "rsqrt"]);
}

#[test]
fn clamp_nan_maximum_is_accepted_and_nan_length_takes_reciprocal_path() {
    for (vector, maximum) in [
        ([1.0, 0.0, 0.0, 4.0], f32::NAN),
        ([f32::NAN, 0.0, 0.0, 4.0], 1.0),
    ] {
        let mut services = Services::default();
        let result = clamp(SkeletonRootVector::known(vector), maximum, &mut services).unwrap();
        assert!(result.xyz.iter().all(|component| component.is_nan()));
        assert_eq!(*services.events.borrow(), ["dot", "rsqrt", "reciprocal"]);
    }
}
