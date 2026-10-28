//! Synthetic source-graph discriminators, not captured Xenon results.
//! Frozen rotation scalars were independently evaluated with CRT fmaf and
//! explicit f32 rounding; mounted authored geometry is a separate root test.

#![expect(
    clippy::float_cmp,
    reason = "exact arithmetic and untouched-field discriminators, not approximate numerical comparisons"
)]

use super::*;

#[derive(Debug, PartialEq)]
enum NumericEvent {
    Reciprocal(SimulationVolumeReciprocalSite, u32),
    Sqrt(u32),
    Divide(u32, u32),
}

#[derive(Default)]
struct Numeric {
    events: Vec<NumericEvent>,
    final_volume_seed: Option<f32>,
    fail_at: Option<SimulationVolumeReciprocalSite>,
}

impl SerialisedAssemblyBBoxMinMaxNumeric for Numeric {
    type Error = u8;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(HostSerialisedAssemblyBBoxNumeric
            .minimum(left, right)
            .unwrap())
    }
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(HostSerialisedAssemblyBBoxNumeric
            .maximum(left, right)
            .unwrap())
    }
}

impl SerialisedAssemblyPartBBoxNumeric for Numeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(HostSerialisedAssemblyBBoxNumeric
            .sum_three_products(left, right)
            .unwrap())
    }
    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(value.sqrt().recip())
    }
}

impl SimulationVolumeIntegralNumeric for Numeric {
    fn reciprocal_estimate(
        &mut self,
        site: SimulationVolumeReciprocalSite,
        value: f32,
    ) -> Result<f32, Self::Error> {
        self.events
            .push(NumericEvent::Reciprocal(site, value.to_bits()));
        if self.fail_at == Some(site) {
            return Err(7);
        }
        Ok(if site == SimulationVolumeReciprocalSite::FinalVolume {
            self.final_volume_seed.unwrap_or_else(|| value.recip())
        } else {
            value.recip()
        })
    }
    fn scalar_sqrt(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.events.push(NumericEvent::Sqrt(value.to_bits()));
        Ok(value.sqrt())
    }
    fn scalar_divide(&mut self, numerator: f32, denominator: f32) -> Result<f32, Self::Error> {
        self.events.push(NumericEvent::Divide(
            numerator.to_bits(),
            denominator.to_bits(),
        ));
        Ok(numerator / denominator)
    }
}

fn primitive(kind: VolumeType) -> Volume {
    Volume {
        transform: flatten(IDENTITY),
        vtable_offset: kind as u32,
        shape_words: [0; 3],
        radius: 0.0,
        group_id: 0,
        surface_id: 0,
        flags: 1,
    }
}

fn box_volume(half_extents: [f32; 3]) -> Volume {
    let mut volume = primitive(VolumeType::Box);
    volume.shape_words = half_extents.map(f32::to_bits);
    volume
}

#[test]
fn cylinder_outer_radius_expands_both_dimensions_with_no_clamp() {
    // Independent simple-input scalar evaluation: R=3, H=4, numerator=91,
    // length product=36. Frozen f32 values are host predictions, not traces.
    let (diagonal, volume) = cylinder_unit_mass(2.0, 1.0, 3.0);
    assert_eq!(
        diagonal.map(f32::to_bits),
        [0x40f2_aaab, 0x40f2_aaab, 0x4090_0000]
    );
    assert_eq!(volume.to_bits(), 0x4362_31d6);
    let (without_outer, smaller_volume) = cylinder_unit_mass(2.0, 0.0, 3.0);
    assert_eq!(without_outer, [4.0, 4.0, 2.0]);
    assert!(smaller_volume < volume);
    let (_, negative_volume) = cylinder_unit_mass(2.0, 0.0, -3.0);
    assert_eq!(
        negative_volume.to_bits(),
        smaller_volume.to_bits() ^ 0x8000_0000
    );
    let (nan_diagonal, nan_volume) = cylinder_unit_mass(f32::NAN, 0.0, 3.0);
    assert!(nan_diagonal.iter().all(|value| value.is_nan()));
    assert!(nan_volume.is_nan());
}

#[test]
fn cylinder_reads_native_fields_and_transforms_disabled_child_exactly_once() {
    let mut cylinder = primitive(VolumeType::Cylinder);
    // Native +0x44 half-height, +0x48 inner radius, +0x50 outer radius.
    cylinder.shape_words = [3.0_f32.to_bits(), 2.0_f32.to_bits(), 0x7fc1_2345];
    cylinder.radius = 1.0;
    cylinder.flags = 0;
    cylinder.transform = flatten([
        [1.0, 0.0, 0.0, f32::NAN],
        [0.0, 1.0, 0.0, f32::NAN],
        [0.0, 0.0, 1.0, f32::NAN],
        [2.0, -4.0, 8.0, f32::NAN],
    ]);
    let mut numeric = Numeric::default();
    let result = try_compute_mapped_array_integral(&[cylinder], None, &mut numeric).unwrap();
    assert!(result.any_child_succeeded);
    assert!(
        numeric.events.is_empty(),
        "Cylinder uses its simple kernel, not a bbox/estimate fallback"
    );
    let volume = f32::from_bits(0x4362_31d6);
    assert_eq!(
        result.integral.columns[3],
        [2.0 * volume, -4.0 * volume, 8.0 * volume, volume]
    );
    for lane in 0..3 {
        assert_eq!(
            result.integral.columns[lane][3],
            result.integral.columns[3][lane]
        );
    }
    assert!(
        result
            .integral
            .columns
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
    // The X first moment above is 2*volume: a second child transform would
    // produce 4*volume. Native flags and unused union words do not filter it.
}

#[test]
fn construct_integral_is_homogeneous_second_moment_and_multiplies_volume_first() {
    let integral = construct_integral([2.0, 3.0, 4.0], 8.0);
    assert_eq!(
        integral.columns,
        [
            [20.0, 0.0, 0.0, 0.0],
            [0.0, 12.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [0.0, 0.0, 0.0, 8.0]
        ]
    );
    let overflow = construct_integral([1.0e20; 3], 4.0e18);
    assert!(overflow.columns[0][0].is_infinite());
    let half_first = (1.0e20_f32 * 0.5) * 4.0e18;
    assert!(
        half_first.is_finite(),
        "moving half before volume changes source overflow"
    );
}

#[test]
fn transform_uses_two_complete_products_and_ignores_incoming_w_lanes() {
    let mut integral = construct_integral([2.0, 3.0, 4.0], 8.0);
    let original = integral.clone();
    transform_integral(&mut integral, None);
    assert_eq!(integral, original);
    let frame = flatten([
        [0.0, 1.0, 0.0, f32::NAN],
        [-1.0, 0.0, 0.0, f32::NAN],
        [0.0, 0.0, 1.0, f32::NAN],
        [4.0, -2.0, 3.0, f32::NAN],
    ]);
    transform_integral(&mut integral, Some(&frame));
    assert_eq!(
        integral.columns,
        [
            [140.0, -64.0, 96.0, 32.0],
            [-64.0, 52.0, -48.0, -16.0],
            [96.0, -48.0, 76.0, 24.0],
            [32.0, -16.0, 24.0, 8.0]
        ]
    );
    let result = try_diagonalise_volume_integral(&mut integral, &mut Numeric::default()).unwrap();
    assert_eq!(
        result.diagonal,
        [3.0, 2.0, 4.0],
        "principal values are not sorted"
    );
    assert_eq!(&result.body_to_part[12..], &[4.0, -2.0, 3.0, 4.0]);
    assert_eq!(
        [
            result.body_to_part[3],
            result.body_to_part[7],
            result.body_to_part[11]
        ],
        [0.0, 1.0, 0.0]
    );
    assert_eq!(
        integral.columns[3],
        [0.0, 0.0, 0.0, 8.0],
        "input is centered in-place"
    );
}

#[test]
fn triangle_is_its_bbox_contribution_with_source_thickness_floor() {
    let mut triangle = primitive(VolumeType::Triangle);
    triangle.transform = flatten([
        [0.0; 4],
        [2.0, 0.0, 0.0, 0.0],
        [0.0, 4.0, 0.0, 0.0],
        [f32::NAN; 4],
    ]);
    let result =
        try_compute_mapped_array_integral(&[triangle], None, &mut Numeric::default()).unwrap();
    assert!(result.any_child_succeeded);
    assert_eq!(
        result.integral.columns[3][3].to_bits(),
        0x3fcc_cccd,
        "thin triangle bbox gets half-thickness0.1, not zero volume"
    );
    let mut degenerate = primitive(VolumeType::Triangle);
    degenerate.transform = [0.0; 16];
    let degenerate =
        try_compute_mapped_array_integral(&[degenerate], None, &mut Numeric::default()).unwrap();
    assert!(
        degenerate.integral.columns[3][3] > 0.0,
        "source epsilon floor retains even a point triangle"
    );
}

#[test]
fn overlapping_and_disabled_children_are_added_in_definition_order() {
    let large = box_volume([128.0; 3]); // exact volume2^24
    let mut small = box_volume([0.5; 3]); // exact volume1
    small.flags = 0;
    let first = try_compute_mapped_array_integral(
        &[large.clone(), small.clone(), small.clone()],
        None,
        &mut Numeric::default(),
    )
    .unwrap();
    let reordered = try_compute_mapped_array_integral(
        &[small.clone(), small, large],
        None,
        &mut Numeric::default(),
    )
    .unwrap();
    assert_eq!(first.integral.columns[3][3], 16_777_216.0);
    assert_eq!(
        reordered.integral.columns[3][3], 16_777_218.0,
        "separate ascending f32 additions, no pairwise reduction or overlap subtraction"
    );
}

#[test]
fn empty_and_all_null_aggregate_succeed_without_transform_or_zero_volume_guard() {
    let poisoned_frame = [f32::NAN; 16];
    for children in [vec![], vec![primitive(VolumeType::Null)]] {
        let mut result = try_compute_mapped_array_integral(
            &children,
            Some(&poisoned_frame),
            &mut Numeric::default(),
        )
        .unwrap();
        assert!(!result.any_child_succeeded);
        assert_eq!(
            result
                .integral
                .columns
                .map(|column| column.map(f32::to_bits)),
            [[0; 4]; 4]
        );
        let mut numeric = Numeric::default();
        let principal =
            try_diagonalise_volume_integral(&mut result.integral, &mut numeric).unwrap();
        assert_eq!(principal.volume.to_bits(), 0);
        assert!(principal.diagonal.iter().all(|value| value.is_nan()));
        assert_eq!(
            numeric.events,
            vec![
                NumericEvent::Reciprocal(SimulationVolumeReciprocalSite::CenterVolume, 0),
                NumericEvent::Reciprocal(SimulationVolumeReciprocalSite::FinalVolume, 0)
            ]
        );
    }
}

#[test]
fn aggregate_own_frame_applies_after_children_and_unsupported_types_are_errors() {
    let child = box_volume([0.5; 3]);
    let mut frame = IDENTITY;
    frame[3] = [3.0, 4.0, 5.0, f32::NAN];
    let result =
        try_compute_mapped_array_integral(&[child], Some(&flatten(frame)), &mut Numeric::default())
            .unwrap();
    assert_eq!(result.integral.columns[3], [3.0, 4.0, 5.0, 1.0]);
    assert!(matches!(
        try_compute_mapped_array_integral(
            &[
                primitive(VolumeType::Null),
                primitive(VolumeType::Aggregate)
            ],
            None,
            &mut Numeric::default()
        ),
        Err(SimulationVolumeIntegralError::UnsupportedVolume {
            ordinal: 1,
            volume_type: Some(VolumeType::Aggregate)
        })
    ));
}

#[test]
fn sphere_and_capsule_reuse_the_source_unit_mass_helpers() {
    for kind in [VolumeType::Sphere, VolumeType::Capsule] {
        let mut volume = primitive(kind);
        volume.radius = f32::from_bits(0x3cfd_f3b6);
        volume.shape_words[0] = 0.25_f32.to_bits();
        let expected_volume = if kind == VolumeType::Sphere {
            0x3902_d997
        } else {
            0x3ad6_36cc
        };
        let result =
            try_compute_mapped_array_integral(&[volume], None, &mut Numeric::default()).unwrap();
        assert_eq!(result.integral.columns[3][3].to_bits(), expected_volume);
    }
}

#[test]
fn equal_diagonal_schur_uses_negative_tau_zero_root_and_zero_cross_skips_numeric() {
    let mut matrix = [[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
    let mut numeric = Numeric::default();
    let (cosine, sine) = try_schur(&matrix, 1, 0, &mut numeric).unwrap();
    assert_eq!(cosine.to_bits(), 0x3f35_04f3);
    assert_eq!(sine.to_bits(), 0xbf35_04f3);
    assert!(
        matches!(numeric.events[2], NumericEvent::Divide(0xbf80_0000, _)),
        "tau==0 divides minusone by root-tau"
    );
    matrix[1][0] = -0.0;
    numeric.events.clear();
    assert_eq!(try_schur(&matrix, 1, 0, &mut numeric).unwrap(), (1.0, 0.0));
    assert!(numeric.events.is_empty());
}

#[test]
fn jacobi_uses_minimum_diagonal_scale_and_source_pair_order() {
    let mut matrix = [[1000.0, 0.1, 0.0], [0.1, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut numeric = Numeric::default();
    let (_, remaining) = try_jacobi(&mut matrix, 1, 0.01, &mut numeric).unwrap();
    assert_eq!(
        remaining, 0,
        "maximum diagonal scale would wrongly skip this sweep"
    );
    assert!(matches!(
        numeric.events.first(),
        Some(NumericEvent::Reciprocal(
            SimulationVolumeReciprocalSite::Schur { p: 1, q: 0 },
            _
        ))
    ));
    let mut matrix = [[3.0, 0.4, 0.5], [0.4, 2.0, 0.6], [0.5, 0.6, 1.0]];
    numeric.events.clear();
    try_jacobi(&mut matrix, 1, 0.0, &mut numeric).unwrap();
    let pairs = numeric
        .events
        .iter()
        .filter_map(|event| match event {
            NumericEvent::Reciprocal(SimulationVolumeReciprocalSite::Schur { p, q }, _) => {
                Some((*p, *q))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(pairs, [(1, 0), (2, 0), (2, 1)]);
}

#[test]
fn jacobi_stops_on_unordered_off_norm_and_returns_remaining_not_used_count() {
    let mut numeric = Numeric::default();
    let mut diagonal = [[3.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(
        try_jacobi(&mut diagonal, 10, 0.0, &mut numeric).unwrap().1,
        10
    );
    diagonal[2][1] = f32::NAN;
    assert_eq!(
        try_jacobi(&mut diagonal, 10, 0.0, &mut numeric).unwrap().1,
        10
    );
    assert!(numeric.events.is_empty());
}

#[test]
fn rotation_p_arm_is_unfused_and_q_arm_is_fused_with_old_pairs() {
    let [u, v, cosine, sine] =
        [0x3fe0_5dee, 0xbf77_0592, 0xbfde_83d9, 0x3f1a_9cb3].map(f32::from_bits);
    let mut columns = [[v, u, 7.0], [v, u, 8.0], [v, u, 9.0]];
    rotate_columns(&mut columns, 1, 0, cosine, sine);
    for (row, untouched) in columns.iter().zip([7.0, 8.0, 9.0]) {
        assert_eq!(row[1].to_bits(), 0xc01d_b8e0);
        assert_eq!(row[0].to_bits(), 0x402f_1bf3);
        assert_eq!(row[2], untouched);
    }
    let mut rows = [[v; 3], [u; 3], [7.0, 8.0, 9.0]];
    rotate_rows(&mut rows, 1, 0, cosine, sine);
    assert_eq!(rows[1].map(f32::to_bits), [0xc01d_b8e0; 3]);
    assert_eq!(rows[0].map(f32::to_bits), [0x402f_1bf3; 3]);
    assert_eq!(rows[2], [7.0, 8.0, 9.0]);
    assert_eq!(
        u.mul_add(cosine, -(v * sine)).to_bits(),
        0xc01d_b8df,
        "forbidden fused p mutant differs"
    );
    assert_eq!(
        ((u * sine) + (v * cosine)).to_bits(),
        0x402f_1bf2,
        "forbidden split q mutant differs"
    );
}

#[test]
fn volume_reciprocals_are_independent_and_each_refines_exactly_twice() {
    let mut integral = construct_integral([2.0, 3.0, 4.0], 8.0);
    let mut numeric = Numeric {
        final_volume_seed: Some(0.0625),
        ..Numeric::default()
    };
    let result = try_diagonalise_volume_integral(&mut integral, &mut numeric).unwrap();
    assert_eq!(
        numeric.events,
        [
            NumericEvent::Reciprocal(
                SimulationVolumeReciprocalSite::CenterVolume,
                8.0_f32.to_bits()
            ),
            NumericEvent::Reciprocal(
                SimulationVolumeReciprocalSite::FinalVolume,
                8.0_f32.to_bits()
            )
        ]
    );
    assert_eq!(
        result.diagonal,
        [1.875, 2.8125, 3.75],
        "finalseed half reciprocal refines .0625→.09375→.1171875; reusing centerseed differs"
    );
    assert_eq!(result.volume, 8.0);
}

#[test]
fn estimate_errors_propagate_at_their_actual_stage_without_fake_rollback() {
    let original = construct_integral([2.0, 3.0, 4.0], 8.0);
    let mut integral = original.clone();
    let mut numeric = Numeric {
        fail_at: Some(SimulationVolumeReciprocalSite::CenterVolume),
        ..Numeric::default()
    };
    assert_eq!(
        try_diagonalise_volume_integral(&mut integral, &mut numeric),
        Err(7)
    );
    assert_eq!(integral, original);
    let mut frame = IDENTITY;
    frame[3][0] = 4.0;
    transform_integral(&mut integral, Some(&flatten(frame)));
    numeric.fail_at = Some(SimulationVolumeReciprocalSite::FinalVolume);
    assert_eq!(
        try_diagonalise_volume_integral(&mut integral, &mut numeric),
        Err(7)
    );
    assert_eq!(
        integral, original,
        "centering occurred before final-estimate refusal"
    );
}
