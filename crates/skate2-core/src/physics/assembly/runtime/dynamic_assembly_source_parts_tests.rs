use super::*;
use crate::{
    dynamic_assembly_session::DynamicAssemblyBodyOnlyPartAttachError,
    simulation_assembly_backend::{
        SimulationAssemblyBackendAddFailure,
        source::tests::{Backend, IDENTITY, configured, move_body, source_template},
    },
};

fn session(
    backend: &Backend,
    template: &SerialisedAssemblyTemplate,
) -> DynamicAssemblyBodyOnlyPartSession<Backend> {
    let attachment = backend
        .try_bind_serialised_part_attachment_template(template)
        .unwrap();
    DynamicAssemblyBodyOnlyPartSession::try_new(attachment, vec![IDENTITY; template.parts().len()])
        .unwrap()
}

fn bits(matrix: [f32; 16]) -> [u32; 16] {
    matrix.map(f32::to_bits)
}

#[test]
fn attached_getter_reads_actual_packed_lanes_and_detach_copies_them_back() {
    let template = source_template(&[0x10], None);
    let mut backend = configured(1, template.owner());
    let mut parts = session(&backend, &template);
    assert_eq!(
        bits(parts.try_native_part_transform(&backend, 0).unwrap()),
        bits(IDENTITY)
    );
    parts.attach(&mut backend, 4).unwrap();
    let body = parts.transaction.parts[0].body.unwrap();
    let mut moved = IDENTITY;
    moved[12..15].copy_from_slice(&[3.0, 5.0, 7.0]);
    move_body(&mut backend, body, moved);
    let live = parts.try_native_part_transform(&backend, 0).unwrap();
    assert_eq!(&live[12..15], &[3.0, 5.0, 7.0]);
    assert_eq!(live[7].to_bits(), 0x2010);
    assert_ne!(live[3].to_bits(), 0);
    assert_ne!(live[15].to_bits(), 1.0f32.to_bits());
    assert_eq!(
        bits(*parts.part_transforms().next().unwrap()),
        bits(IDENTITY)
    );
    parts.detach(&mut backend).unwrap();
    assert_eq!(
        bits(parts.try_native_part_transform(&backend, 0).unwrap()),
        bits(live)
    );
    assert_eq!(backend.rigid_body_snapshot().counts.free, 1);
    assert!(
        backend
            .try_source_body_pose(body, template.owner())
            .is_err()
    );
}

#[test]
fn inverse_getter_and_copyback_use_all_four_lanes_in_the_same_order() {
    let mut inverse = IDENTITY;
    inverse[0] = 2.0;
    inverse[5] = 3.0;
    inverse[10] = 4.0;
    inverse[12] = 5.0;
    inverse[13] = 7.0;
    inverse[14] = 11.0;
    inverse[3] = f32::NAN;
    inverse[7] = f32::INFINITY;
    inverse[11] = -13.0;
    inverse[15] = 19.0;
    let template = source_template(&[0x10], Some(inverse));
    let mut backend = configured(1, template.owner());
    let mut parts = session(&backend, &template);
    parts.attach(&mut backend, 4).unwrap();
    let body = parts.transaction.parts[0].body.unwrap();
    move_body(&mut backend, body, IDENTITY);
    let raw = backend
        .try_source_body_pose(body, template.owner())
        .unwrap();
    // Independent source-form scalar oracle: inner W never participates.
    let expected = std::array::from_fn(|i| {
        let lane = i % 4;
        let c = i / 4 * 4;
        let first = if c == 12 {
            inverse[c].mul_add(raw[lane], raw[12 + lane])
        } else {
            inverse[c] * raw[lane]
        };
        let second = inverse[c + 1].mul_add(raw[4 + lane], first);
        inverse[c + 2].mul_add(raw[8 + lane], second)
    });
    let live = parts.try_native_part_transform(&backend, 0).unwrap();
    assert_eq!(bits(live), bits(expected));
    assert_ne!(live[7].to_bits(), 0);
    parts.detach(&mut backend).unwrap();
    assert_eq!(
        bits(*parts.part_transforms().next().unwrap()),
        bits(expected)
    );
}

#[test]
fn create_capture_uses_retained_cache_and_explicit_user_words_not_live_getter() {
    let template = source_template(&[0x10], None);
    let mut backend = configured(1, template.owner());
    let mut parts = session(&backend, &template);
    parts.attach(&mut backend, 4).unwrap();
    let body = parts.transaction.parts[0].body.unwrap();
    let mut moved = IDENTITY;
    moved[12] = 29.0;
    move_body(&mut backend, body, moved);
    let capture = parts
        .try_source_create_parts(&backend, &template, &[0xa5a5_1234])
        .unwrap();
    assert_eq!(capture[0].body, body.observation());
    assert_eq!(bits(capture[0].part_transform), bits(IDENTITY));
    assert_eq!(capture[0].user_data, 0xa5a5_1234);
    assert_eq!(capture[0].material, template.parts()[0].material);
    assert_eq!(
        parts.try_native_part_transform(&backend, 0).unwrap()[12].to_bits(),
        29.0_f32.to_bits()
    );
    parts.detach(&mut backend).unwrap();
}

#[test]
fn capture_rejects_foreign_backend_template_and_user_count() {
    let template = source_template(&[0x10], None);
    let foreign = source_template(&[0x10], None);
    let mut backend = configured(1, template.owner());
    let other = configured(1, template.owner());
    let mut parts = session(&backend, &template);
    assert!(matches!(
        parts.try_source_create_parts(&backend, &template, &[0]),
        Err(DynamicAssemblySourcePartError::NotAttached)
    ));
    parts.attach(&mut backend, 4).unwrap();
    assert!(matches!(
        parts.try_native_part_transform(&other, 0),
        Err(DynamicAssemblySourcePartError::ForeignBackend)
    ));
    assert!(matches!(
        parts.try_source_create_parts(&other, &template, &[0]),
        Err(DynamicAssemblySourcePartError::ForeignBackend)
    ));
    assert!(matches!(
        parts.try_source_create_parts(&backend, &template, &[]),
        Err(DynamicAssemblySourcePartError::UserDataCount {
            parts: 1,
            values: 0
        })
    ));
    assert!(matches!(
        parts.try_source_create_parts(&backend, &foreign, &[0]),
        Err(DynamicAssemblySourcePartError::SourceMismatch { index: 0 })
    ));
    assert!(matches!(
        parts.try_native_part_transform(&backend, 1),
        Err(DynamicAssemblySourcePartError::PartIndex { index: 1, count: 1 })
    ));
    parts.detach(&mut backend).unwrap();
}

#[test]
fn late_add_failure_rolls_back_source_outcomes_and_preserves_original_diagnostic() {
    let template = source_template(&[0x10, 0x11], Some(IDENTITY));
    let mut backend = configured(2, template.owner());
    let mut parts = session(&backend, &template);
    assert_eq!(
        parts.attach(&mut backend, 4),
        Err(DynamicAssemblyBodyOnlyPartAttachError::PartUnavailable { ordinal: 1 })
    );
    assert!(!parts.is_attached());
    assert_eq!(backend.rigid_body_snapshot().counts.free, 2);
    assert!(matches!(
        backend.last_add_failure(),
        Some(SimulationAssemblyBackendAddFailure::SourceRigidBody(_))
    ));
    let cache = *parts.part_transforms().next().unwrap();
    assert_eq!(cache[7].to_bits(), 0x2010);
    assert_ne!(cache[3].to_bits(), 0);
    assert_eq!(
        bits(*parts.part_transforms().nth(1).unwrap()),
        bits(IDENTITY)
    );
    // Both Static attaches bypass the invalid address; rollback retired route
    // and outcome records, so the same slots are now usable again.
    parts.attach(&mut backend, 1).unwrap();
    assert_eq!(backend.last_add_failure(), None);
    parts.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.free, 2);
}

#[test]
fn capacity_failure_rolls_back_without_fabricating_a_source_error() {
    let template = source_template(&[0x10, 0x40], None);
    let mut backend = configured(1, template.owner());
    let mut parts = session(&backend, &template);
    assert_eq!(
        parts.attach(&mut backend, 4),
        Err(DynamicAssemblyBodyOnlyPartAttachError::PartUnavailable { ordinal: 1 })
    );
    assert_eq!(backend.last_add_failure(), None);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 1);
    assert_eq!(parts.part_transforms().next().unwrap()[7].to_bits(), 0x2010);
}
