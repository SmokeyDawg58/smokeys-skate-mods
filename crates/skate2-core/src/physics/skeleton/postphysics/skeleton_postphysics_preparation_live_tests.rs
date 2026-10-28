use super::transform_point;
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
use crate::{
    attribute_runtime::tests::graph,
    dynamic_assembly_session::skeleton_collision_publication::tests::fixture,
    reckoning_numeric::HostReckoningNumeric,
    skateboard_assembly::SkateboardSimulationBackend,
    skeleton_attributes::SkeletonAttributeClass,
    skeleton_body_attributes::{DEFAULT_COLLECTION, SkeletonBodyAttributes},
};
use std::{convert::Infallible, sync::Arc};

#[test]
fn bad_data_reads_actual_attached_angular_velocity_and_ignores_packed_w() {
    let mut owner = fixture(None);
    let mut backend = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
    owner.attach(&mut backend).unwrap();
    let vault = Arc::new(graph(SkeletonAttributeClass::Skeleton.key(), vec![]));
    let mut attributes = SkeletonBodyAttributes::bind(&vault, DEFAULT_COLLECTION);
    assert!(
        !owner
            .try_postphysics_bad_data::<Infallible, _>(
                &backend,
                &mut attributes,
                &mut HostReckoningNumeric
            )
            .unwrap()
    );
    let body = owner
        .session
        .try_part_contact_spy_observation(&backend, 7)
        .unwrap();
    backend
        .try_set_part_angular_velocity(body, [0.0, f32::NAN, 0.0])
        .unwrap();
    assert!(
        owner
            .try_postphysics_bad_data::<Infallible, _>(
                &backend,
                &mut attributes,
                &mut HostReckoningNumeric
            )
            .unwrap()
    );
    owner.detach(&mut backend).unwrap();
}

#[test]
fn local_publication_retains_unknown_basis_w_even_with_zero_coefficients() {
    let mut matrix = SkeletonRootMatrix::known(crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM);
    matrix.set_column(
        0,
        SkeletonRootVector {
            xyz: [1.0, 0.0, 0.0],
            w: None,
        },
    );
    let actual = transform_point(&matrix, [0.0, 2.0, 3.0]);
    assert_eq!(
        actual.xyz.map(f32::to_bits),
        [0.0_f32, 2.0, 3.0].map(f32::to_bits)
    );
    assert!(actual.w.is_none());
}

#[test]
fn affine_suffix_includes_translation_and_full_w_but_not_position_w() {
    let matrix = SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([2.0, 0.0, 0.0, 3.0]),
        SkeletonRootVector::known([0.0, 4.0, 0.0, 5.0]),
        SkeletonRootVector::known([0.0, 0.0, 6.0, 7.0]),
        SkeletonRootVector::known([10.0, 20.0, 30.0, 9.0]),
    ]);
    assert_eq!(
        transform_point(&matrix, [1.0, 2.0, 3.0])
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [12.0_f32, 28.0, 48.0, 43.0].map(f32::to_bits)
    );
}

#[test]
fn affine_suffix_uses_three_fused_stages_in_original_order() {
    // Product differs when rounded before the cancellation with translation.
    let a = f32::from_bits(0x3f80_0001);
    let p = -f32::from_bits(0x3f80_0002);
    let matrix = SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([a; 4]),
        SkeletonRootVector::known([0.0; 4]),
        SkeletonRootVector::known([0.0; 4]),
        SkeletonRootVector::known([p; 4]),
    ]);
    let result = transform_point(&matrix, [a, 0.0, 0.0]);
    assert_eq!(result.xyz[0].to_bits(), 0x2880_0000);
    assert_eq!((a * a + p).to_bits(), 0);
}
