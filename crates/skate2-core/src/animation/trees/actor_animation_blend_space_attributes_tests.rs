use super::*;
use skate2_formats::andale::AndaleAttributeValue;
fn scalar(name: &str, value: f32) -> AndaleRuntimeAttribute {
    AndaleRuntimeAttribute {
        sequence_id: -1,
        name: name.into(),
        status: 6,
        value: AndaleAttributeValue::Float(value),
        begin_seconds: -1.0,
        end_seconds: -1.0,
    }
}
#[test]
fn repeated_mult_add_does_not_rescale_accumulated_attribute() {
    let mut value = scalar("a", 8.0);
    multiply_attribute(&mut value, 0.25);
    multiply_add_attribute(&mut value, &scalar("a", 4.0), 0.25).unwrap();
    multiply_add_attribute(&mut value, &scalar("a", 2.0), 0.5).unwrap();
    assert_eq!(value.scalar_value().unwrap().to_bits(), 4_f32.to_bits());
    assert_eq!(value.begin_seconds.to_bits(), (-1_f32).to_bits());
}
#[test]
fn mult_add_publishes_time_prefix_before_type_assertion() {
    let mut left = scalar("a", 7.0);
    left.status = 5;
    left.begin_seconds = 2.;
    left.end_seconds = 4.;
    let mut right = scalar("a", 0.);
    right.begin_seconds = 6.;
    right.end_seconds = 8.;
    right.value = AndaleAttributeValue::Vector([1.; 4]);
    assert!(matches!(
        multiply_add_attribute(&mut left, &right, 0.5),
        Err(TreeAttributeError::TypeMismatch)
    ));
    assert_eq!(left.begin_seconds.to_bits(), 5_f32.to_bits());
    assert_eq!(left.end_seconds.to_bits(), 8_f32.to_bits());
    assert_eq!(left.scalar_value().unwrap().to_bits(), 7_f32.to_bits());
    left.status = 6;
    left.begin_seconds = -0.;
    left.end_seconds = 3.;
    assert!(multiply_add_attribute(&mut left, &right, f32::NAN).is_err());
    assert_eq!(left.begin_seconds.to_bits(), 0_f32.to_bits());
    assert_eq!(left.end_seconds.to_bits(), 3_f32.to_bits());
}
#[test]
fn exhausted_scan_marks_current_missing_then_rejects_unknown_next_scratch() {
    let mut attributes = [scalar("a", 1.), scalar("b", 2.), scalar("c", 3.)];
    attributes.sort_by_key(|a| packed_name(a).unwrap().words);
    let right = [attributes[0].clone()];
    let mut left = attributes.into_iter().map(Some).collect::<Vec<_>>();
    assert!(matches!(
        intersect_volume_attributes(&mut left, &right, || Ok(1.)),
        Err(TreeAttributeError::Missing(
            "BlendSpace unwritten attribute scratch"
        ))
    ));
    assert_eq!(
        left[0].as_ref().unwrap().scalar_value().unwrap().to_bits(),
        2_f32.to_bits()
    );
    assert!(left[1].is_none());
    assert!(left[2].is_some());
}
#[test]
fn empty_second_list_requires_unknown_scratch_only_for_nonempty_first() {
    let mut empty = [];
    intersect_volume_attributes(&mut empty, &[], || panic!("no coefficient read")).unwrap();
    let mut left = [Some(scalar("a", 1.))];
    assert!(intersect_volume_attributes(&mut left, &[], || panic!("no coefficient read")).is_err());
    assert!(left[0].is_some());
}
