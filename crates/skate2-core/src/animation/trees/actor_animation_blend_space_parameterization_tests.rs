use super::*;
use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric};

fn tetra() -> AndaleBlendSpaceVolume {
    // Barycentric basis: origin, unit X, unit Y, unit Z. Native coefficient i
    // uses vertex (i+1)%4 as its plane anchor.
    let vertices = [
        [0., 0., 0., 1.],
        [1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., 0., 1., 1.],
    ];
    let normals = [
        [-1., -1., -1., 0.],
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
    ];
    let mut bytes = vec![0; 168];
    for (offset, values) in [(20, vertices), (88, normals)] {
        for (i, value) in values.into_iter().flatten().enumerate() {
            bytes[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&f32::to_be_bytes(value));
        }
    }
    for i in 0..4 {
        bytes[152 + i * 4..156 + i * 4].copy_from_slice(&1f32.to_be_bytes());
    }
    AndaleBlendSpaceVolume {
        animation_indexes_offset: 0,
        heights_offset: 152,
        normals_offset: 84,
        matrix_coordinates: vec![],
        serialized_data: bytes,
        serialized_following_word: None,
    }
}
#[test]
fn first_containing_volume_skips_live_slicing_and_preserves_query_w() {
    let mut projected = [-7.; 4];
    let mut weights = [-8.; 4];
    let mut found = None;
    let mut needs = true;
    let mut ready = false;
    compute_parameterization3(
        [0.25, 0.25, 0.25, 7.],
        &[tetra(), tetra()],
        ParameterizationOutput {
            projected: &mut projected,
            coefficients: &mut weights,
            found: &mut found,
            needs_projection: &mut needs,
            ready: &mut ready,
        },
        &mut HostReckoningNumeric,
        || panic!("inside volume must not sample slicing"),
    )
    .unwrap();
    assert_eq!(found, Some(0));
    assert!(!needs);
    assert!(ready);
    assert_eq!(
        projected.map(f32::to_bits),
        [0.25, 0.25, 0.25, 7.].map(f32::to_bits)
    );
    assert_eq!(weights.map(f32::to_bits), [0.25f32; 4].map(f32::to_bits));
}
#[test]
fn slicing_preserves_native_duplicate_z_probe_instead_of_intersecting_segment() {
    let geometry = geometry::<()>(&tetra()).unwrap();
    let candidate = sliced_candidate(
        &geometry.vertices,
        [0.25, 0.25, 0.25, 1.],
        &mut HostReckoningNumeric,
    )
    .unwrap();
    // Z endpoints are 0 and 0.5. Native duplicate probes force nearest fallback;
    // exact tie retains the first, whereas repaired geometry would return0.25.
    assert_eq!(
        candidate.map(f32::to_bits),
        [0.25, 0.25, 0., 1.].map(f32::to_bits)
    );
}
#[test]
fn tiny_crossing_inserts_both_endpoints_and_nearest_keeps_first_tie() {
    let points = [
        [0., 3., 4., 1.],
        [0.000_001, 8., 9., 1.],
        [1., 10., 11., 1.],
    ];
    let mut result = [[-1.; 4]; 2];
    assert!(slice(&points, 0.000_000_5, &mut result, &mut HostReckoningNumeric).unwrap());
    assert_eq!(
        result.map(|v| v.map(f32::to_bits)),
        [points[0], points[1]].map(|v| v.map(f32::to_bits))
    );
    assert_eq!(
        nearest(&[[0., 4., 0., 1.], [2., 9., 0., 1.]], 1.).map(f32::to_bits),
        [0., 4., 0., 1.].map(f32::to_bits)
    );
    assert_eq!(nearest(&[[f32::NAN; 4]], 1.).map(f32::to_bits), [0; 4]);
}
struct FailingMinimum;
impl ReckoningNumeric for FailingMinimum {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        unreachable!()
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        unreachable!()
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        Ok(1. / v)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        unreachable!()
    }
}
impl PresentationDecodeNumeric for FailingMinimum {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(HostReckoningNumeric.dot4(a, b).unwrap())
    }
}
impl BlendSpaceNumeric for FailingMinimum {
    fn maximum(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(HostReckoningNumeric.maximum(a, b).unwrap())
    }
    fn minimum(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Err("minimum")
    }
}
#[test]
fn final_numeric_failure_retains_native_projected_x_and_found_prefix() {
    let mut projected = [-7.; 4];
    let mut weights = [-8.; 4];
    let mut found = None;
    let mut needs = true;
    let mut ready = false;
    assert!(matches!(
        compute_parameterization3(
            [0.25, 0.25, 0.25, 1.],
            &[tetra()],
            ParameterizationOutput {
                projected: &mut projected,
                coefficients: &mut weights,
                found: &mut found,
                needs_projection: &mut needs,
                ready: &mut ready,
            },
            &mut FailingMinimum,
            || panic!("inside")
        ),
        Err(ParameterizationError::Numeric("minimum"))
    ));
    assert_eq!(found, Some(0));
    assert!(!needs);
    assert!(!ready);
    assert_eq!(
        projected.map(f32::to_bits),
        [0.25, -7., -7., -7.].map(f32::to_bits)
    );
    assert_eq!(weights.map(f32::to_bits), [-8.; 4].map(f32::to_bits));
}

#[test]
fn normal_w_reads_real_adjacent_word_and_refuses_unknown_tail() {
    let mut volume = tetra();
    volume.serialized_data.resize(180, 0);
    volume.normals_offset = 116;
    volume.heights_offset = 100;
    let bits = 0x7fc1_2345_u32;
    volume.serialized_following_word = Some(bits.to_be_bytes());
    let decoded = geometry::<()>(&volume).unwrap();
    assert_eq!(decoded.normals[3][3].to_bits(), bits);
    volume.serialized_following_word = None;
    assert!(matches!(
        geometry::<()>(&volume),
        Err(ParameterizationError::VolumeBounds)
    ));
}
