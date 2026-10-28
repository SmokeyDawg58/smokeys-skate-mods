use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
use std::sync::{Arc, Mutex};

fn detector() -> CameraGrindDetector {
    let mut parameters = CameraAttributes::begin_init();
    let fields = parameters.parameters.fields_mut();
    fields.grind_min_lookahead_distance = Some(1.0);
    fields.grind_max_lookahead_distance = Some(10.0);
    fields.grind_lookahead_time = Some(10.0);
    fields.grind_frustum_angle_horizontal = Some(60.0);
    fields.grind_frustum_angle_vertical = Some(20.0);
    fields.grind_max_relative_angle = Some(30.0);
    fields.grind_skateboard_offset = Some(0.0);
    fields.grind_max_relative_height = Some(2.0);
    fields.grind_fade_in_speed = Some(2.0);
    fields.grind_fade_out_speed = Some(1.0);
    let mut detector = CameraGrindDetector::constructed(Arc::new(Mutex::new(parameters)));
    detector.active = true;
    detector
}

/// From the origin along +Z.
const FRUSTUM: [V4; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

const AHEAD: [V4; 2] = [[0.5, 0.0, 4.0, 1.0], [0.5, 0.0, 8.0, 1.0]];

/// One update at speed 1 (a lookahead of 10) against `rails`; the boxes it queried.
fn search(detector: &mut CameraGrindDetector, rails: &[[V4; 2]]) -> Vec<OctreeBounds> {
    let mut queries = Vec::new();
    let mut box_test = |bounds: OctreeBounds| -> Result<Vec<[V4; 2]>, CameraGrindUpdateError> {
        queries.push(bounds);
        Ok(rails.to_vec())
    };
    detector
        .try_update(
            FRUSTUM,
            1.0,
            false,
            &mut box_test,
            &mut HostReckoningNumeric,
        )
        .unwrap();
    queries
}

#[test]
fn the_nearest_rail_ahead_is_the_candidate() {
    let mut detector = detector();
    let further = [[-0.5, 0.0, 6.0, 1.0], [-0.5, 0.0, 9.0, 1.0]];
    let queries = search(&mut detector, &[further, AHEAD]);
    // Rays 30 degrees either side out to 10 and 20 degrees up, held 2 tall.
    let [bounds] = queries[..] else {
        panic!("one query: {queries:?}");
    };
    assert!((bounds.minimum[0] + 5.0).abs() < 1e-4);
    assert!((bounds.maximum[0] - 5.0).abs() < 1e-4);
    assert!(bounds.minimum[1].abs() < 1e-6);
    assert!((bounds.maximum[1] - 2.0).abs() < 1e-6);
    assert!(bounds.minimum[2].abs() < 1e-6);
    assert!((bounds.maximum[2] - 10.0).abs() < 1e-4);
    assert!(detector.position_valid);
    assert_eq!(
        detector.nearest_position.xyz.map(f32::to_bits),
        [0.5_f32, 0.0, 4.0].map(f32::to_bits)
    );
    assert!(detector.heading.abs() < 1e-6);
    assert!((detector.position_angle + 0.5_f32.atan2(4.0)).abs() < 1e-5);
}

#[test]
fn rails_across_behind_too_near_or_too_high_are_not_candidates() {
    let across = [[-1.0, 0.0, 5.0, 1.0], [1.0, 0.0, 5.0, 1.0]];
    let behind = [[0.0, 0.0, -4.0, 1.0], [0.0, 0.0, -8.0, 1.0]];
    let near = [[0.0, 0.0, 0.5, 1.0], [0.0, 0.0, 3.0, 1.0]];
    let high = [[0.0, 2.5, 4.0, 1.0], [0.0, 2.5, 8.0, 1.0]];
    for rail in [across, behind, near, high] {
        let mut detector = detector();
        search(&mut detector, &[rail]);
        assert!(!detector.position_valid, "{rail:?}");
        assert_eq!(detector.position_angle.to_bits(), PI.to_bits());
    }
}

#[test]
fn no_search_while_grinding_or_inactive() {
    let mut detector = detector();
    detector.position_valid = true;
    detector.position_angle = -1.0;
    let mut box_test =
        |_: OctreeBounds| -> Result<Vec<[V4; 2]>, CameraGrindUpdateError> { panic!("no query") };
    detector
        .try_update(FRUSTUM, 1.0, true, &mut box_test, &mut HostReckoningNumeric)
        .unwrap();
    assert!(!detector.position_valid);
    assert_eq!(detector.position_angle.to_bits(), PI.to_bits());
    detector.active = false;
    detector
        .try_update(
            FRUSTUM,
            1.0,
            false,
            &mut box_test,
            &mut HostReckoningNumeric,
        )
        .unwrap();
    assert!(!detector.position_valid);
}

#[test]
fn anticipation_rises_only_while_the_hips_block_the_rail() {
    let mut detector = detector();
    search(&mut detector, &[AHEAD]);
    let camera = [0.0, 1.5, -5.0, 1.0];
    detector
        .try_update_anticipation(
            0.25,
            camera,
            [0.1, 1.0, -1.0, 1.0],
            &mut HostReckoningNumeric,
        )
        .unwrap();
    assert_eq!(detector.anticipation.to_bits(), 0.5_f32.to_bits());
    assert_eq!(detector.shaped_anticipation.to_bits(), 0.5_f32.to_bits());
    detector
        .try_update_anticipation(
            0.25,
            camera,
            [2.0, 1.0, -1.0, 1.0],
            &mut HostReckoningNumeric,
        )
        .unwrap();
    assert_eq!(detector.anticipation.to_bits(), 0.25_f32.to_bits());
}

#[test]
fn without_a_rail_the_anticipation_falls_after_both_speeds_load() {
    let mut detector = detector();
    detector.anticipation = 0.75;
    detector
        .try_update_anticipation(0.25, [0.0; 4], [0.0; 4], &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(detector.anticipation.to_bits(), 0.5_f32.to_bits());
    assert_eq!(detector.shaped_anticipation.to_bits(), 0.5_f32.to_bits());
    detector
        .parameters
        .lock()
        .unwrap()
        .parameters
        .fields_mut()
        .grind_fade_in_speed = None;
    assert!(matches!(
        detector.try_update_anticipation(0.25, [0.0; 4], [0.0; 4], &mut HostReckoningNumeric),
        Err(E::Grind(CameraGrindUpdateError::Unwritten(
            "grind_fade_in_speed"
        )))
    ));
}
