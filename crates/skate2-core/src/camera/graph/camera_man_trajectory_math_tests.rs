use super::*;
use crate::{
    camera_attributes::CameraAttributes, camera_shake::ShakeKeyframes, camera_subject::ZERO,
    reckoning_numeric::HostReckoningNumeric, skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::{Arc, Mutex};

#[test]
fn flat_guard_ignores_w_and_unordered_x_does_not_force_normalization() {
    let mut subject = CameraSubject::try_construct().unwrap();
    let mut columns = *subject.board_effective_transform.columns();
    columns[2] = Vector {
        xyz: [f32::NAN, 9.0, 0.0],
        w: Some(f32::INFINITY),
    };
    subject.board_effective_transform = SkeletonRootMatrix::from_columns(columns);
    let basis = flat_basis(&subject, &mut HostReckoningNumeric).unwrap();
    assert_eq!(
        basis,
        [identity().columns()[0], UP, identity().columns()[2]]
    );
    columns[2] = Vector {
        xyz: [0.0, 9.0, 2.0],
        w: None,
    };
    subject.board_effective_transform = SkeletonRootMatrix::from_columns(columns);
    let basis = flat_basis(&subject, &mut HostReckoningNumeric).unwrap();
    assert_eq!(basis[2].xyz.map(f32::to_bits), [0, 0, 1.0_f32.to_bits()]);
    assert!(basis[2].w.is_none());
    assert!(basis[0].w.is_none());
}

#[test]
fn landing_uses_full_inverse_instead_of_transpose() {
    let basis = [
        Vector::known([2.0, 0.0, 0.0, 0.0]),
        UP,
        Vector::known([0.0, 0.0, 4.0, 0.0]),
    ];
    let result = landing_incline(
        basis,
        Vector::known([0.0, 0.0, 2.0, 0.0]),
        &mut HostReckoningNumeric,
    )
    .unwrap();
    let expected = -try_asin(0.5, &mut HostReckoningNumeric).unwrap();
    assert_eq!(result.to_bits(), expected.to_bits());
}

fn shake() -> CameraShake {
    CameraShake {
        parameters: Arc::new(Mutex::new(CameraAttributes::begin_init())),
        keyframes: ShakeKeyframes::constructed(),
        offset: ZERO,
        time: 0.0,
        one_shot_effect: 0.0,
        multiplier: 1.0,
        wait_for_drop: false,
    }
}

#[test]
fn stationary_tremor_does_not_read_unpublished_parameters() {
    let mut shake = shake();
    assert!(
        !shake
            .try_tremor(UP, ZERO, &mut HostReckoningNumeric)
            .unwrap()
    );
    assert!(matches!(
        shake.parameters.lock().unwrap().parameters,
        CameraParameterPublication::Unwritten
    ));
}

#[test]
fn impact_requires_actual_parameters_and_only_increases_retained_effect() {
    let mut shake = shake();
    let velocity = Vector::known([0.0, -2.0, 0.0, 0.0]);
    assert!(matches!(
        shake.try_tremor(UP, velocity, &mut HostReckoningNumeric),
        Err(TremorError::UnwrittenParameter(
            "shake_one_shot_min_velocity"
        ))
    ));
    {
        let mut owner = shake.parameters.lock().unwrap();
        let p = owner.parameters.fields_mut();
        p.shake_one_shot_min_velocity = Some(0.0);
        p.shake_one_shot_max_velocity = Some(2.0);
        p.shake_one_shot_magnitude = Some(3.0);
    }
    assert!(
        shake
            .try_tremor(UP, velocity, &mut HostReckoningNumeric)
            .unwrap()
    );
    assert!(shake.one_shot_effect > 2.9);
    shake.one_shot_effect = 4.0;
    assert!(
        !shake
            .try_tremor(UP, velocity, &mut HostReckoningNumeric)
            .unwrap()
    );
    assert_eq!(shake.one_shot_effect.to_bits(), 4.0_f32.to_bits());
}
