use super::*;
use crate::{
    attribute_notifications::AttributeNotifications,
    attribute_runtime::tests::{
        add_fixture_class, collection, definition, graph, node, set_dependency_bytes,
    },
    camera_attributes::CameraParameterPublication,
};
use std::sync::Arc;

const DURATION: u64 = 0x80cc_1cba_de8e_f0fb;
const POSITIONER_DISTANCE: u64 = 0x5da7_a3ab_955a_a369;

fn fixture(duration: f32, late_bad_node: bool) -> CameraAttributes {
    let (class, collection_id, _) = BINDINGS[1];
    let mut vault = graph(class, vec![definition(DURATION, 4, 4)]);
    let mut layout = vec![0; 0x2b0];
    layout[149 * 4..150 * 4].copy_from_slice(&9.0_f32.to_be_bytes());
    set_dependency_bytes(&mut vault, 1, layout);
    collection(
        &mut vault,
        class,
        collection_id,
        0,
        Some(0),
        vec![node(DURATION, 0x40, duration.to_bits())],
    );
    if late_bad_node {
        let (class, collection_id, _) = BINDINGS[17];
        add_fixture_class(
            &mut vault,
            class,
            vec![definition(POSITIONER_DISTANCE, 4, 1)],
        );
        collection(
            &mut vault,
            class,
            collection_id,
            0,
            None,
            vec![node(POSITIONER_DISTANCE, 0x40, 1)],
        );
    }
    let mut attributes = CameraAttributes::begin_init();
    attributes
        .initialize(&Arc::new(vault), &AttributeNotifications::constructed())
        .unwrap();
    attributes
}
fn published(attributes: &CameraAttributes) -> &CameraParameters {
    let CameraParameterPublication::Fields(fields) = &attributes.parameters else {
        panic!("read prefix not published");
    };
    fields
}

#[test]
fn nonpositive_and_nan_duration_keep_prefix_and_changed_flag() {
    for duration in [0.0, -1.0, f32::from_bits(0x7fc1_2345)] {
        let mut attributes = fixture(duration, false);
        assert!(
            matches!(attributes.try_read_if_changed(), Err(CameraAttributeReadError::NearClipDurationAssertion { bits }) if bits == duration.to_bits())
        );
        let fields = published(&attributes);
        assert_eq!(
            fields.heading_change_response_speed.unwrap().to_bits(),
            9.0_f32.to_bits()
        );
        assert!(fields.max_pitch.is_some());
        assert!(fields.near_clipping_distance_min.is_none());
        assert!(fields.drop_elevation_offset.is_none());
        assert!(!fields.read_completed);
        assert!(attributes.changed.load(Ordering::Relaxed));
    }
}

#[test]
fn malformed_late_value_preserves_completed_earlier_sections() {
    let mut attributes = fixture(1.0, true);
    assert!(matches!(
        attributes.try_read_if_changed(),
        Err(CameraAttributeReadError::Runtime(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
    let fields = published(&attributes);
    assert!(fields.near_clipping_distance_step.is_some());
    assert!(fields.compass_deadzone_size_smoothing.is_some());
    assert!(fields.positioner_distance_smoothing.is_none());
    assert!(fields.free_cam_heading_speed.is_none());
    assert!(attributes.changed.load(Ordering::Relaxed));
}

#[test]
fn successful_read_clears_changed_and_unchanged_path_is_lazy() {
    let mut attributes = fixture(1.0, false);
    assert!(attributes.try_read_if_changed().unwrap());
    assert!(published(&attributes).read_completed);
    assert!(published(&attributes).free_cam_heading_speed.is_some());
    assert!(!attributes.changed.load(Ordering::Relaxed));
    attributes.bindings.clear(); // A second read would now fail at its first owner.
    assert!(!attributes.try_read_if_changed().unwrap());
}
