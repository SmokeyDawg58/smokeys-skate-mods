use super::*;
use crate::{
    camera_attributes::CameraAttributes,
    camera_frame::{
        composition_math::{exceeds_epsilon, framing_basis, look_basis},
        reference::weighted_reference,
    },
    camera_subject::CameraSubject,
    reckoning_sin_cos::ReckoningSinCosNumeric,
};
use std::sync::{Arc, Mutex};

fn composer() -> CameraFrameComposer {
    let mut attributes = CameraAttributes::begin_init();
    attributes.parameters.fields_mut().max_pitch = Some(80.0);
    attributes.parameters.fields_mut().dutch_interpolation_speed = Some(2.0);
    let mut composer = CameraFrameComposer::constructed(Arc::new(Mutex::new(attributes)));
    let mut subject = CameraSubject::try_construct().unwrap();
    subject
        .reference_positions
        .fill(Vector::known([0.0, 0.0, 2.0, 0.0]));
    composer.subject = Some(Arc::new(Mutex::new(subject)));
    composer
}

#[test]
fn zero_weights_still_read_points_and_nan_arithmetic_is_not_removed() {
    let fallback = Vector::known([1.0, 2.0, 3.0, 4.0]);
    let mut visited = Vec::new();
    let result = weighted_reference(&[0.0, 0.0], fallback, |index| {
        visited.push(index);
        Ok(Vector::known([f32::NAN; 4]))
    })
    .unwrap();
    assert_eq!(visited, [0, 1]);
    assert_eq!(
        result.known_lanes().unwrap().map(f32::to_bits),
        fallback.known_lanes().unwrap().map(f32::to_bits)
    );
    assert!(matches!(
        weighted_reference(&[1.0, 0.0], fallback, |index| Ok(Vector::known(
            if index == 0 {
                [1.0; 4]
            } else {
                [f32::INFINITY; 4]
            }
        ))),
        Err(FrameCompositionError::ReferenceNaN(0))
    ));
}

#[test]
fn offset_uses_subject_up_for_nonboard_point_and_preserves_unknown_w() {
    let composer = composer();
    {
        let mut subject = composer.subject.as_ref().unwrap().lock().unwrap();
        subject.reference_positions[0] = Vector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        };
        subject.up = Vector::known([0.0, 2.0, 0.0, 7.0]);
    }
    let mut shot = CameraShot::constructed();
    shot.reference_weights = [0.0; 9];
    shot.reference_weights[0] = 1.0;
    shot.board_offset = 3.0;
    let reference = composer
        .reference_point(&shot, Vector::known([9.0; 4]))
        .unwrap();
    assert_eq!(
        reference.xyz.map(f32::to_bits),
        [2.0_f32, 9.0, 4.0].map(f32::to_bits)
    );
    assert!(reference.w.is_none());
}

#[test]
fn reference_assertion_ignores_w_but_rejects_unordered_weight() {
    let reference = weighted_reference(&[1.0], Vector::known([0.0; 4]), |_| {
        Ok(Vector::known([1.0, 2.0, 3.0, f32::NAN]))
    })
    .unwrap();
    assert!(reference.w.unwrap().is_nan());
    assert!(matches!(
        weighted_reference(&[f32::NAN], Vector::known([0.0; 4]), |_| Ok(Vector::known(
            [0.0; 4]
        ))),
        Err(FrameCompositionError::ReferenceNaN(_))
    ));
}

#[test]
fn zero_direction_bypasses_live_pitch_parameter_but_reads_all_trig_angles() {
    let mut composer = composer();
    composer.parameters.lock().unwrap().parameters = CameraParameterPublication::Unwritten;
    let shot = CameraShot::constructed();
    let point = Vector::known([0.0, 0.0, 2.0, 0.0]);
    assert!(composer.single(&shot, point, point, 1.0).is_ok());
    assert!(matches!(
        composer.single(&shot, point, Vector::known([0.0; 4]), 1.0),
        Err(FrameCompositionError::MissingParameter("max_pitch"))
    ));
    assert!(!exceeds_epsilon(Vector::known([
        f32::NAN,
        0.0,
        0.0,
        f32::INFINITY
    ])));
    assert!(exceeds_epsilon(Vector::known([f32::NAN, 0.0, 1.0, 0.0])));
    struct Third(usize);
    impl ReckoningSinCosNumeric for Third {
        type Error = ();
        fn round_nearest_even(&mut self, value: f32) -> Result<f32, ()> {
            self.0 += 1;
            if self.0 == 3 {
                Err(())
            } else {
                Ok(value.round_ties_even())
            }
        }
    }
    let mut trig = Third(0);
    assert!(framing_basis(0.1, 0.2, &mut trig).is_err());
    assert_eq!(trig.0, 3);
}

#[test]
fn matrix33_normalizes_second_cross_and_retains_supplied_z() {
    let input = Vector {
        xyz: [0.0, 0.0, 2.0],
        w: None,
    };
    let basis = look_basis(input, &mut HostReckoningNumeric).unwrap();
    assert!((basis[1].xyz[1] - 1.0).abs() < 0.00001);
    assert_eq!(basis[2].xyz.map(f32::to_bits), input.xyz.map(f32::to_bits));
    assert!(basis.iter().all(|column| column.w.is_none()));
}

#[test]
fn current_composition_precedes_previous_and_late_roll_failure_preserves_stores() {
    let mut composer = composer();
    let mut current = CameraShot::constructed();
    current.yaw = 0.25;
    let mut previous = CameraShot::constructed();
    previous.yaw = -0.5;
    previous.pitch = 0.125;
    composer
        .parameters
        .lock()
        .unwrap()
        .parameters
        .fields_mut()
        .dutch_interpolation_speed = None;
    let error = composer
        .compose(
            0.1,
            &previous,
            &current,
            0.3,
            Vector::known([0.0; 4]),
            Vector::known([0.0; 4]),
            -1.0,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        FrameCompositionError::MissingParameter("dutch_interpolation_speed")
    ));
    assert_eq!(composer.framing_x.to_bits(), 0.5_f32.to_bits());
    assert_eq!(composer.framing_y.to_bits(), (-0.125_f32).to_bits());
    assert_eq!(composer.last_roll.to_bits(), 0.0_f32.to_bits());
    // A failure during the first current reference prevents any previous store.
    current.reference_weights[0] = f32::NAN;
    previous.yaw = 8.0;
    assert!(matches!(
        composer.compose(
            0.0,
            &previous,
            &current,
            1.0,
            Vector::known([0.0; 4]),
            Vector::known([0.0; 4]),
            1.0
        ),
        Err(FrameCompositionError::ReferenceNaN(_))
    ));
    assert_eq!(composer.framing_x.to_bits(), 0.5_f32.to_bits());
}

#[test]
fn nonpositive_and_nan_dt_skip_dutch_parameter() {
    let mut composer = composer();
    composer.parameters.lock().unwrap().parameters = CameraParameterPublication::Unwritten;
    composer.update_roll(f32::NAN, -0.0).unwrap();
    assert_eq!(composer.last_roll.to_bits(), (-0.0_f32).to_bits());
    composer.update_roll(-1.0, 3.0).unwrap();
    assert_eq!(composer.last_roll.to_bits(), 3.0_f32.to_bits());
    assert_eq!(
        transition_amount(f32::NAN, 0.0).to_bits(),
        1.0_f32.to_bits()
    );
}

#[test]
fn shared_quaternion_core_does_not_repeat_transition_easing() {
    struct Count(usize);
    impl ReckoningSinCosNumeric for Count {
        type Error = std::convert::Infallible;
        fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
            self.0 += 1;
            Ok(value.round_ties_even())
        }
    }
    let first = [0.0, 0.0, 0.0, 1.0];
    let second = [0.0, 0.5, 0.0, 0.866_025_4];
    let amount = transition_amount(1.0, 0.2);
    let mut core = Count(0);
    let result =
        runtime_math::quaternion_core(first, second, amount, &mut HostReckoningNumeric, &mut core)
            .unwrap();
    let mut wrapper = Count(0);
    let expected = runtime_math::quaternion(
        first,
        second,
        0,
        0.2,
        &mut HostReckoningNumeric,
        &mut wrapper,
    )
    .unwrap();
    assert_eq!(result.map(f32::to_bits), expected.map(f32::to_bits));
    assert_eq!(wrapper.0, core.0 + 1);
}
