use super::*;

fn call_bits(calls: &[(&'static str, f32)]) -> Vec<(&'static str, u32)> {
    calls
        .iter()
        .map(|(name, value)| (*name, value.to_bits()))
        .collect()
}

fn partial(matrix: AnimationMatrix, unknown: &[usize]) -> SkeletonRootMatrix {
    let mut columns = *SkeletonRootMatrix::known(matrix).columns();
    for index in unknown {
        columns[*index].w = None;
    }
    SkeletonRootMatrix::from_columns(columns)
}

#[test]
fn typed_endpoints_preserve_selected_w_and_lower_endpoint_still_executes_query() {
    let from = partial(IDENTITY, &[0, 3]);
    let to = partial(QUARTER_TURN, &[1, 2]);
    let mut numeric = Numeric::default();
    let mut trig = Trigonometry::default();
    let (upper, angle) = try_slerp_typed(&from, &to, 1.0, &mut numeric, &mut trig).unwrap();
    assert_eq!(upper, to);
    assert_eq!(angle.to_bits(), 0);
    assert!(numeric.calls.is_empty() && trig.calls.is_empty());
    let (lower, angle) = try_slerp_typed(&from, &to, -0.5, &mut numeric, &mut trig).unwrap();
    assert_eq!(lower, from);
    assert!(angle > 1.5 && angle < 1.6);
    assert_eq!(numeric.calls.last().unwrap().0, "reciprocal");
    assert!(trig.calls.is_empty());
}

#[test]
fn typed_linear_w_depends_on_both_endpoints_per_column() {
    let mut from = IDENTITY;
    let mut to = IDENTITY;
    from[7] = 4.0;
    to[7] = 12.0;
    from[15] = -2.0;
    to[15] = 6.0;
    let (result, angle) = try_slerp_typed(
        &partial(from, &[0]),
        &partial(to, &[2]),
        0.25,
        &mut Numeric::default(),
        &mut Trigonometry::default(),
    )
    .unwrap();
    assert_eq!(result.columns()[0].w, None);
    assert_eq!(result.columns()[2].w, None);
    assert_eq!(result.columns()[1].w.unwrap().to_bits(), 6.0_f32.to_bits());
    assert_eq!(result.columns()[3].w.unwrap().to_bits(), 0);
    assert_eq!(angle.to_bits(), 0);
}

#[test]
fn typed_rodrigues_recovers_basis_w_from_xyz_but_keeps_unknown_position_w() {
    let from = partial(IDENTITY, &[0, 1, 2, 3]);
    let to = partial(QUARTER_TURN, &[0, 1, 2, 3]);
    let mut numeric = Numeric::default();
    let mut trig = Trigonometry::default();
    let (result, angle) = try_slerp_typed(&from, &to, 0.5, &mut numeric, &mut trig).unwrap();
    for column in &result.columns()[..3] {
        assert_eq!(
            column.w.unwrap().to_bits(),
            column.xyz[0].to_bits(),
            "identity From selects source Rodrigues columns whose W is X"
        );
    }
    assert_eq!(result.columns()[3].w, None);
    assert!(angle > 0.78 && angle < 0.79);
    assert_eq!(trig.calls.len(), 1);
    let mut known_numeric = Numeric::default();
    let mut known_trig = Trigonometry::default();
    let known = try_interpolate_affine(
        &IDENTITY,
        &QUARTER_TURN,
        0.5,
        &mut known_numeric,
        &mut known_trig,
    )
    .unwrap();
    for (index, column) in result.columns().iter().enumerate() {
        assert_eq!(
            column.xyz.map(f32::to_bits),
            std::array::from_fn(|lane| known[index * 4 + lane].to_bits())
        );
    }
    assert_eq!(call_bits(&numeric.calls), call_bits(&known_numeric.calls));
    assert_eq!(trig.calls, known_trig.calls);
}

#[test]
fn weighted_affine_position_does_not_inherit_rotational_endpoint_w_knowledge() {
    let from = partial(IDENTITY, &[0, 3]);
    let mut to = QUARTER_TURN;
    to[12..].copy_from_slice(&[2.0, 4.0, 6.0, 8.0]);
    let to = SkeletonRootMatrix::known(to);
    for blend in [1.0, 2.0] {
        let mut numeric = Numeric::default();
        let result = try_interpolate_affine_typed(
            &from,
            &to,
            blend,
            &mut numeric,
            &mut Trigonometry::default(),
        )
        .unwrap();
        assert_eq!(&result.columns()[..3], &to.columns()[..3]);
        assert_eq!(
            result.columns()[3].xyz,
            [2.0 * blend, 4.0 * blend, 6.0 * blend]
        );
        assert_eq!(
            result.columns()[3].w,
            None,
            "unknown times zero is not a known zero"
        );
        assert!(numeric.calls.is_empty());
    }
}

#[test]
fn typed_query_and_trigonometry_failures_retain_the_exact_existing_callback_prefix() {
    let from = partial(IDENTITY, &[0, 3]);
    let to = partial(QUARTER_TURN, &[2]);
    let mut baseline = Numeric::default();
    try_slerp_typed(&from, &to, 0.5, &mut baseline, &mut Trigonometry::default()).unwrap();
    for index in 0..baseline.calls.len() {
        let mut numeric = Numeric {
            fail_at: Some(index),
            ..Numeric::default()
        };
        let mut trig = Trigonometry::default();
        assert_eq!(
            try_slerp_typed(&from, &to, 0.5, &mut numeric, &mut trig),
            Err(SkeletonAffineInterpolationError::Numeric("numeric refusal"))
        );
        assert_eq!(
            call_bits(&numeric.calls),
            call_bits(&baseline.calls[..=index])
        );
        assert!(trig.calls.is_empty());
    }
    let mut numeric = Numeric::default();
    let mut trig = Trigonometry {
        fail: true,
        ..Trigonometry::default()
    };
    assert_eq!(
        try_slerp_typed(&from, &to, 0.5, &mut numeric, &mut trig),
        Err(SkeletonAffineInterpolationError::Trigonometry(
            "trig refusal"
        ))
    );
    assert_eq!(call_bits(&numeric.calls), call_bits(&baseline.calls));
}

#[test]
fn classical_normalizer_preserves_p_and_full_w_without_homogeneous_repair() {
    let input = SkeletonRootMatrix::known([
        4.0, 0.0, 0.0, 8.0, 0.0, 3.0, 0.0, 9.0, 0.0, 0.0, 2.0, 8.0, 11.0, 13.0, 17.0, -0.0,
    ]);
    let mut numeric = Numeric::default();
    let output = try_normalize_rotation(&input, &mut numeric).unwrap();
    let actual = output.known_lanes().unwrap();
    for (axis, expected) in [2.0_f32, 3.0, 4.0].into_iter().enumerate() {
        assert!((actual[axis * 4 + axis] - 1.0).abs() < 0.000_001);
        assert!((actual[axis * 4 + 3] - expected).abs() < 0.000_001);
    }
    assert_eq!(output.columns()[3], input.columns()[3]);
    assert_eq!(
        output.columns()[3].w.unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        numeric.calls.iter().map(|c| c.0).collect::<Vec<_>>(),
        [
            "dot", "rsqrt", "dot", "dot", "rsqrt", "dot", "dot", "dot", "rsqrt"
        ]
    );
}

#[test]
fn classical_normalizer_propagates_unknown_w_through_zero_projection_products() {
    for missing in [0, 1, 2] {
        let input = partial(IDENTITY, &[missing]);
        let output = try_normalize_rotation(&input, &mut Numeric::default()).unwrap();
        for axis in 0..3 {
            assert_eq!(output.columns()[axis].w.is_none(), axis <= missing);
        }
        assert_eq!(output.columns()[3], input.columns()[3]);
    }
}

#[test]
fn classical_normalizer_failure_prefix_uses_original_column_projection() {
    let input = SkeletonRootMatrix::known([
        3.0, 2.0, 1.0, 5.0, 1.0, 4.0, 2.0, 7.0, 2.0, 1.0, 3.0, 11.0, 13.0, 17.0, 19.0, 23.0,
    ]);
    let mut baseline = Numeric::default();
    let output = try_normalize_rotation(&input, &mut baseline).unwrap();
    let expected_projection = HostReckoningNumeric
        .dot3(
            [
                output.columns()[1].xyz[0],
                output.columns()[1].xyz[1],
                output.columns()[1].xyz[2],
                0.0,
            ],
            [3.0, 2.0, 1.0, 5.0],
        )
        .unwrap();
    assert_eq!(baseline.calls[6], ("dot", expected_projection));
    assert_eq!(baseline.calls.len(), 9);
    for index in 0..9 {
        let mut numeric = Numeric {
            fail_at: Some(index),
            ..Numeric::default()
        };
        assert_eq!(
            try_normalize_rotation(&input, &mut numeric),
            Err("numeric refusal")
        );
        assert_eq!(numeric.calls, baseline.calls[..=index]);
    }
}
