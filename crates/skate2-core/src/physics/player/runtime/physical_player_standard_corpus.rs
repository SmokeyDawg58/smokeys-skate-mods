//! Original admitted board -> actual `SetStandard` -> SAME collision publication.
use super::PhysicalAssemblyWorld;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeRead, SkateboardAttributes,
    },
};
use std::sync::Arc;

pub(super) fn check(world: &mut PhysicalAssemblyWorld, board: &SimulationSkaterAssemblyHandle) {
    let owner = Arc::clone(world.physical_body_member(board).unwrap().owner());
    let mut oracle = SkateboardAttributes::bind(&owner.inputs().vault);
    let scalar = oracle
        .read_float(SkateboardAttributeClass::Deck, 0x8d30_65ce_d858_a7df)
        .unwrap();
    let expected_drag = scalar * f32::from_bits(0x426f_ffff);
    let materials = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .standard_material_owners()
        .unwrap();
    let deck = world
        .board(board)
        .unwrap()
        .try_part_contact_spy_observation(world.backend(), 6)
        .unwrap();
    let inertia = world.backend().try_capture_part_inertia(deck).unwrap();
    let before_inertia = inertia.try_read().unwrap();
    let collision = Arc::clone(world.board(board).unwrap().template().collision());
    let old_revision = collision.volume_flags.read().unwrap().revision();
    let frame = world
        .backend()
        .collision_body_pool()
        .frame_identity()
        .unwrap();

    world.try_set_board_standard(board).unwrap();
    let mut after_inertia = inertia.try_read().unwrap();
    assert_eq!(
        after_inertia.angular_drag.to_bits(),
        expected_drag.to_bits()
    );
    after_inertia.angular_drag = before_inertia.angular_drag;
    assert_eq!(after_inertia, before_inertia);
    assert!(
        !world
            .board_construction(board)
            .unwrap()
            .unwrap()
            .post_fields()
            .unwrap()
            .free_wheel_enabled
    );
    world.with_board_materials_for_test(board, |session, publication, _| {
        assert!(!publication.has_bounds_for_frame(frame));
        for part in 0..7 {
            let expected = &materials[if part == 6 {
                0
            } else if part < 4 {
                1
            } else {
                2
            }];
            assert!(std::ptr::eq(
                session.current_part_material(part).unwrap(),
                Arc::as_ref(expected)
            ));
        }
        assert!(Arc::ptr_eq(session.template().collision(), &collision));
    });
    let flags = collision.volume_flags.read().unwrap();
    assert_eq!(
        flags.revision() - old_revision,
        3 + u64::try_from(flags.deck_count()).unwrap()
    );
    for emission in 0..(6 + flags.deck_count()) {
        assert_ne!(flags.emission_flags(emission).unwrap() & 1, 0);
    }
    drop(flags);
    world.try_refresh_boards().unwrap();
    world.with_board_materials_for_test(board, |session, publication, _| {
        assert!(publication.has_bounds_for_frame(frame));
        assert!(publication.records().iter().all(|record| record.enabled));
        for record in publication.records() {
            assert_eq!(
                record.material,
                *session
                    .current_part_material(usize::from(record.rigid_body_index))
                    .unwrap()
            );
        }
    });
    // Hold calls EnableCollisionVolumes once more after SetStandard. The
    // already-set flags still invalidate the same published bounds.
    let before_hold_revision = collision.volume_flags.read().unwrap().revision();
    world.try_enable_board_collision_volumes(board).unwrap();
    let flags = collision.volume_flags.read().unwrap();
    assert_eq!(
        flags.revision() - before_hold_revision,
        3 + u64::try_from(flags.deck_count()).unwrap()
    );
    drop(flags);
    world.with_board_materials_for_test(board, |_, publication, _| {
        assert!(!publication.has_bounds_for_frame(frame));
    });
    println!(
        "actual SetStandard: runtime global deck damping {expected_drag:?}, exact constructor material owners, all shared/child volumes enabled and SAME publication refreshed; selected ground-entry stage only"
    );
}
