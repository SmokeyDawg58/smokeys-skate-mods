use super::{blend_and_publish, choose_blend, curve_get, with_elapsed_time};
use crate::generated_motion_load::construct::filter::BlendCurve;

#[test]
fn anticipation_ramp_clamps_and_interpolates_native_two_point_curve() {
    let curve = BlendCurve {
        scale: 1.0,
        count: 2,
        coordinates: [
            Some([0.0, 0.0]),
            Some([0.2, 1.0]),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ],
    };
    for (time, expected) in [
        (-0.1_f32, 0.0_f32),
        (0.0, 0.0),
        (0.1, 0.5),
        (0.2, 1.0),
        (0.4, 1.0),
    ] {
        assert_eq!(curve_get(&curve, time).to_bits(), expected.to_bits());
    }
}

#[test]
fn native_fmadds_blend_keeps_fused_result_before_delta_and_publication() {
    let mut data = [0.0, 0.0, f32::from_bits(0xc293_bf1c)];
    let target = f32::from_bits(0xc274_9bff);
    let blend = f32::from_bits(0x3f3f_e686);
    let mut published = None;
    blend_and_publish(&mut data, target, blend, None, None, |value| {
        published = Some(value.to_bits());
        Ok::<(), ()>(())
    })
    .unwrap();
    // Independently rounded PPC fmuls(0xc2375ca7), fsubs(0x3e8032f4),
    // fmadds(0xc280accf), then fsubs and fadds. An unfused multiply/add
    // produces delta 0x41189270 instead of the native 0x41189268.
    assert_eq!(data[1].to_bits(), 0x4118_9268);
    assert_eq!(data[2].to_bits(), 0xc280_accf);
    assert_eq!(published, Some(0xc280_accf));
}

#[test]
fn unordered_fsel_takes_upper_acceleration_endpoint() {
    let mut data = [0.0; 3];
    blend_and_publish(&mut data, f32::NAN, 0.25, Some(0.03), Some(0.2), |_| {
        Ok::<(), ()>(())
    })
    .unwrap();
    // Both acceleration comparisons are unordered: first retains NaN,
    // second selects the upper endpoint. Velocity clamp then keeps it.
    assert_eq!(data[1].to_bits(), 0.03_f32.to_bits());
    assert_eq!(data[2].to_bits(), 0.03_f32.to_bits());
}

#[test]
fn negative_zero_is_nonnegative_for_fsel_and_preserved_by_data_writes() {
    let mut data = [0.0, -0.0, -0.0];
    blend_and_publish(&mut data, 0.0, 0.0, Some(0.0), None, |_| Ok::<(), ()>(())).unwrap();
    assert_eq!(data[1].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(data[2].to_bits(), (-0.0_f32).to_bits());
}

#[test]
fn absent_source_uses_blend_out_even_when_target_is_falling() {
    assert_eq!(
        choose_blend(false, -10.0, 0.0, 0.25, 0.75, Some(0.08)),
        0.08
    );
    assert_eq!(choose_blend(true, -10.0, 0.0, 0.25, 0.75, Some(0.08)), 0.75);
}

#[test]
fn early_and_late_failures_retain_their_native_data_prefixes() {
    let mut data = [1.0, -0.5, 0.25];
    let missing_graph: Result<(), &str> = with_elapsed_time(&mut data, 0.125, |_| Err("graph"));
    assert_eq!(missing_graph, Err("graph"));
    assert_eq!(
        data.map(f32::to_bits),
        [
            1.125_f32.to_bits(),
            (-0.5_f32).to_bits(),
            0.25_f32.to_bits()
        ]
    );

    let mut observed = None;
    let missing_output = blend_and_publish(&mut data, 1.0, 0.25, None, None, |value| {
        observed = Some(value.to_bits());
        Err::<(), _>("output map")
    });
    assert_eq!(missing_output, Err("output map"));
    assert_eq!(data[0].to_bits(), 1.125_f32.to_bits());
    assert_eq!(data[1].to_bits(), 0.1875_f32.to_bits());
    assert_eq!(data[2].to_bits(), 0.4375_f32.to_bits());
    assert_eq!(observed, Some(0.4375_f32.to_bits()));
}
