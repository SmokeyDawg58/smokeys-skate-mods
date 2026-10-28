use super::*;
use crate::attribute_runtime::tests::{collection, graph, set_dependency_bytes};
use std::sync::Arc;

fn database(bytes: Vec<u8>) -> CameraShotDatabase {
    let mut vault = graph(CAMERA_SHOTS_CLASS, vec![]);
    collection(
        &mut vault,
        CAMERA_SHOTS_CLASS,
        string_hash64("alpha"),
        0,
        Some(0),
        vec![],
    );
    set_dependency_bytes(&mut vault, 1, bytes);
    CameraShotDatabase::constructed(Arc::new(vault))
}

#[test]
fn absent_collection_preserves_every_observed_destination_field() {
    let database = database(vec![0; 0xe8]);
    let mut shot = CameraShot::constructed();
    shot.name[0] = Some(b'z');
    shot.shot_type = 17;
    shot.distance = -4.0;
    database.try_write_to_shot(&mut shot, "missing").unwrap();
    assert_eq!(shot.name[0], Some(b'z'));
    assert_eq!(shot.shot_type, 17);
    assert_eq!(shot.distance.to_bits(), (-4.0_f32).to_bits());
}

#[test]
fn live_layout_order_case_conversion_and_unknown_unit_bits_survive() {
    let mut bytes = vec![0; 0xe8];
    bytes[0x50..0x54].copy_from_slice(&99_i32.to_be_bytes());
    bytes[0x6c..0x70].copy_from_slice(&1_i32.to_be_bytes());
    bytes[0x98..0x9c].copy_from_slice(&90.0_f32.to_be_bytes());
    bytes[0xe4..0xe8].copy_from_slice(&3.0_f32.to_be_bytes());
    bytes[0xe0..0xe4].copy_from_slice(&2.0_f32.to_be_bytes());
    bytes[0xdc..0xe0].copy_from_slice(&1.0_f32.to_be_bytes());
    let database = database(bytes);
    let mut shot = CameraShot::constructed();
    shot.name[20] = Some(0xaa);
    shot.blend_names[0][7] = Some(0xbb);
    database.try_write_to_shot(&mut shot, "ALPHA").unwrap();
    assert_eq!(
        &shot.name[..6],
        &[
            Some(b'a'),
            Some(b'l'),
            Some(b'p'),
            Some(b'h'),
            Some(b'a'),
            Some(0)
        ]
    );
    assert_eq!(shot.name[20], Some(0xaa));
    assert_eq!(shot.name_hash, state_graph_string_hash32(b"alpha"));
    assert_eq!(shot.transition_units.native_value(), 99);
    assert_eq!(
        shot.heading.to_bits(),
        (90.0 * DEGREES_TO_RADIANS).to_bits()
    );
    assert_eq!(shot.blend_names[0][0], Some(0));
    assert_eq!(shot.blend_names[0][7], Some(0xbb));
    assert_eq!(
        shot.blend_points.map(f32::to_bits),
        [3.0_f32.to_bits(), 2.0_f32.to_bits(), 1.0_f32.to_bits()]
    );
}

#[test]
fn late_source_failure_retains_prior_publications() {
    let mut bytes = vec![0; 0x70];
    bytes[0x6c..0x70].copy_from_slice(&8_i32.to_be_bytes());
    bytes[0x54..0x58].copy_from_slice(&7.0_f32.to_be_bytes());
    let database = database(bytes);
    let mut shot = CameraShot::constructed();
    assert!(matches!(
        database.try_write_to_shot(&mut shot, "alpha"),
        Err(ShotAttributeError::Runtime(_))
    ));
    assert_eq!(shot.shot_type, 8);
    assert_eq!(shot.transition_time.to_bits(), 7.0_f32.to_bits());
    assert_eq!(shot.distance.to_bits(), 2.0_f32.to_bits());
    assert_eq!(shot.name_hash, state_graph_string_hash32(b"alpha"));
}

#[test]
fn copy_name_stops_at_nul_and_retains_unwritten_tail() {
    let mut name = [None; 32];
    name[9] = Some(0xff);
    copy_name(&mut name, Some(b"a\0ignored")).unwrap();
    assert_eq!(name[0], Some(b'a'));
    assert_eq!(name[1], Some(0));
    assert_eq!(name[9], Some(0xff));
}
