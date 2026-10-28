//! Controlled source-produced bindings; not original runtime/reference traces.
use super::*;
use crate::{
    skeleton_assembly::try_create_pre_normal_skeleton_assembly,
    skeleton_collision_attributes::{SkeletonCollisionAttributeRead, SkeletonCollisionOffset},
    skeleton_culling::{SkeletonCullingInventory, SkeletonCullingKey},
};

pub(crate) struct CollisionReader;
impl SkeletonCollisionAttributeRead for CollisionReader {
    type Pointer = ();
    type Error = std::convert::Infallible;
    fn cached_offset(&mut self, _slot: SkeletonCollisionOffset) -> Result<[f32; 4], Self::Error> {
        Ok([1.0, 2.0, 3.0, -0.0])
    }
    fn lookup(&mut self, _key: u64) -> Result<Self::Pointer, Self::Error> {
        Ok(())
    }
    fn float(&mut self, _pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        Ok(2.0)
    }
}

fn fixture() -> SkeletonRuntimeState {
    let (body, joints) = crate::skeleton_assembly::tests::fixture();
    let normal = SkeletonCullingInventory::default()
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    let session = try_create_pre_normal_skeleton_assembly(body, joints, normal).unwrap();
    SkeletonRuntimeState::new(
        Arc::clone(session.template().collision()),
        SkeletonCollisionData::try_new(&mut CollisionReader).unwrap(),
    )
}

#[test]
fn dmo_collision_disables_share_countdown_without_changing_groups_or_other_parts() {
    let mut state = fixture();
    state.disable_counts = [Some(0); 24];
    for header in &mut state.volume_headers {
        header.flags = 0xa501;
        header.group_id = 17;
    }
    state.disable_hand_collisions(5);
    state.disable_lower_leg_collisions(5);
    let parts = [7, 3, 8, 4, 1, 19, 15, 20, 16, 17, 21];
    for part in 0..24 {
        assert_eq!(
            state.disable_counts[part],
            Some(if parts.contains(&part) { 5 } else { 0 })
        );
        assert_eq!(
            state.volume_headers[part].flags,
            if parts.contains(&part) {
                0xa500
            } else {
                0xa501
            }
        );
        assert_eq!(state.volume_headers[part].group_id, 17);
    }
    for _ in 0..4 {
        state.try_update_volume_states().unwrap();
    }
    assert_eq!(state.disable_counts[7], Some(1));
    assert_eq!(state.volume_headers[7].flags, 0xa500);
    state.try_update_volume_states().unwrap();
    assert!(
        state
            .volume_headers
            .iter()
            .all(|h| h.flags == 0xa501 && h.group_id == 17)
    );
    assert_eq!(state.volume_disable_count_active, Some(false));
}

#[test]
fn volume_state_inactive_skips_unknown_counters_and_unknown_activity_refuses() {
    let mut state = fixture();
    let original = state.volume_headers;
    assert_eq!(
        state.try_update_volume_states(),
        Err(SkeletonRuntimeStateError::UnknownDisableCountActive)
    );
    assert_eq!(state.volume_headers, original);
    assert_eq!(state.disable_counts, [None; 24]);
    state.volume_disable_count_active = Some(false);
    state.try_update_volume_states().unwrap();
    assert_eq!(state.disable_counts, [None; 24]);
    assert_eq!(state.volume_headers, original);
}

#[test]
fn volume_state_countdown_changes_enabled_bit_only_for_positive_entries() {
    let mut state = fixture();
    state.disable_counts = [Some(-1); 24];
    state.disable_counts[..4].copy_from_slice(&[Some(1), Some(2), Some(0), Some(i32::MIN)]);
    state.volume_disable_count_active = Some(true);
    for (part, header) in state.volume_headers.iter_mut().enumerate() {
        header.flags = if part == 0 { 0xa500 } else { 0xa501 };
        header.group_id = 17;
    }
    let before = state.volume_headers;
    state.try_update_volume_states().unwrap();
    assert_eq!(
        &state.disable_counts[..4],
        &[Some(0), Some(1), Some(0), Some(i32::MIN)]
    );
    assert_eq!(state.volume_headers[0].flags, 0xa501);
    assert_eq!(state.volume_headers[1].flags, 0xa500);
    assert_eq!(&state.volume_headers[2..], &before[2..]);
    assert!(
        state
            .volume_headers
            .iter()
            .all(|header| header.group_id == 17)
    );
    assert_eq!(state.volume_disable_count_active, Some(true));
    state.try_update_volume_states().unwrap();
    assert_eq!(state.disable_counts[1], Some(0));
    assert_eq!(state.volume_headers[1].flags, 0xa501);
    assert_eq!(state.volume_disable_count_active, Some(false));
}

#[test]
fn volume_state_unknown_later_count_keeps_updated_prefix_and_old_activity() {
    let mut state = fixture();
    state.volume_disable_count_active = Some(true);
    state.disable_counts[0] = Some(1);
    state.volume_headers[0].flags &= !1;
    let before = state.volume_headers;
    assert_eq!(
        state.try_update_volume_states(),
        Err(SkeletonRuntimeStateError::UnknownDisableCount { part: 1 })
    );
    assert_eq!(state.disable_counts[0], Some(0));
    assert_eq!(state.volume_headers[0].flags, before[0].flags | 1);
    assert_eq!(&state.volume_headers[1..], &before[1..]);
    assert_eq!(state.volume_disable_count_active, Some(true));
}

#[test]
fn initial_stores_material_aliases_and_unknown_fields_are_distinct() {
    let mut state = fixture();
    assert_eq!(state.assembly_group, 0);
    assert_eq!(state.part_user_data, [0; 24]);
    assert_eq!(state.disable_counts, [None; 24]);
    assert_eq!(state.volume_disable_count_active, None);
    assert!(!state.is_ragdoll);
    assert!(Arc::ptr_eq(
        &state.current_culling,
        state.binding.initial_culling()
    ));
    for part in 0..24 {
        assert!(std::ptr::eq(
            state.material(part),
            &raw const state.binding.body().parts[part].material
        ));
    }
    let definition = state.binding.body().parts[1].material;
    state.set_group_and_userdata(1);
    state.select_root_zero_material();
    state.select_normal_material(1);
    state.select_normal_material(2);
    assert_eq!(state.assembly_group, 1);
    assert_eq!(state.part_user_data, [1; 24]);
    assert_eq!(
        state.material_selection[0],
        SkeletonPartMaterialSelection::Zero
    );
    let root = state.material(0);
    assert_eq!(
        [
            root.static_friction,
            root.dynamic_friction,
            root.restitution
        ]
        .map(f32::to_bits),
        [0; 3]
    );
    assert!(std::ptr::eq(state.material(1), state.material(2)));
    assert_eq!(
        state.material(1).static_friction.to_bits(),
        state
            .binding
            .body()
            .attributes
            .normal
            .static_friction
            .to_bits()
    );
    assert_ne!(
        state.material(1).static_friction.to_bits(),
        definition.static_friction.to_bits()
    );
    assert_eq!(state.binding.body().parts[1].material, definition);
    state.select_normal_culling();
    assert!(Arc::ptr_eq(
        &state.current_culling,
        state.binding.initial_culling()
    ));
}

#[test]
fn normal_collision_preserves_nonzero_counters_other_flag_bits_and_collision_data() {
    let mut state = fixture();
    state.volume_headers[0] = SkeletonVolumeHeader {
        flags: 0x105,
        group_id: 99,
    };
    state.volume_headers[1] = SkeletonVolumeHeader {
        flags: 0xa4,
        group_id: 7,
    };
    state.volume_headers[2] = SkeletonVolumeHeader {
        flags: 0xa4,
        group_id: 9,
    };
    state.volume_headers[3] = SkeletonVolumeHeader {
        flags: 0xff,
        group_id: 11,
    };
    state.disable_counts[1] = Some(0);
    state.disable_counts[2] = Some(-7);
    state.disable_counts[3] = Some(2);
    state.collision.priority[1] = 13.0;
    state.is_ragdoll = true;
    let collision_before = state.collision.clone();
    state.try_set_up_normal_collision().unwrap();
    assert_eq!(
        state.volume_headers[0],
        SkeletonVolumeHeader {
            flags: 0x104,
            group_id: 4
        }
    );
    assert_eq!(
        state.volume_headers[1],
        SkeletonVolumeHeader {
            flags: 0xa5,
            group_id: 0
        }
    );
    assert_eq!(
        state.volume_headers[2],
        SkeletonVolumeHeader {
            flags: 0xa4,
            group_id: 9
        }
    );
    assert_eq!(
        state.volume_headers[3],
        SkeletonVolumeHeader {
            flags: 0xff,
            group_id: 11
        }
    );
    assert_eq!(
        &state.disable_counts[..4],
        &[Some(0), Some(0), Some(-7), Some(2)]
    );
    assert!(state.disable_counts[4..].iter().all(Option::is_none));
    assert_eq!(state.part_user_data, [6; 24]);
    assert_eq!(state.assembly_group, 6);
    assert_eq!(state.collision, collision_before);
    assert_eq!(state.volume_disable_count_active, None);
    assert!(state.is_ragdoll);
}

#[test]
fn unknown_counter_join_keeps_unknown_and_refusal_preserves_completed_prefix() {
    let mut state = fixture();
    state.try_set_up_normal_collision().unwrap();
    assert_eq!(state.disable_counts[0], Some(0));
    assert!(state.disable_counts[1..].iter().all(Option::is_none));
    state.disable_counts[1] = Some(0);
    state.volume_headers[1] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 8,
    };
    state.volume_headers[4] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 0,
    };
    assert_eq!(
        state.try_set_up_normal_collision(),
        Err(SkeletonRuntimeStateError::UnknownDisableCount { part: 4 })
    );
    assert_eq!(
        state.volume_headers[1],
        SkeletonVolumeHeader {
            flags: 0x81,
            group_id: 0
        }
    );
    assert_eq!(
        state.volume_headers[4],
        SkeletonVolumeHeader {
            flags: 0x80,
            group_id: 0
        }
    );
    assert_eq!(state.part_user_data, [6; 24]);
    assert_eq!(state.disable_counts[4], None);
    // Already enabled is insufficient: group0 must also agree across branches.
    state.volume_headers[4] = SkeletonVolumeHeader {
        flags: 0x81,
        group_id: 9,
    };
    assert_eq!(
        state.try_set_up_normal_collision(),
        Err(SkeletonRuntimeStateError::UnknownDisableCount { part: 4 })
    );
}

#[test]
fn no_collision_preserves_groups_and_optional_counter_reset_is_exact() {
    let mut state = fixture();
    state.set_group_and_userdata(17);
    state.volume_headers[1] = SkeletonVolumeHeader {
        flags: 0xa5,
        group_id: 27,
    };
    state.disable_counts[1] = Some(-9);
    state.volume_disable_count_active = Some(true);
    state.collision.priority[1] = 2.0;
    state.set_up_no_collision(false);
    assert_eq!(
        state.volume_headers[1],
        SkeletonVolumeHeader {
            flags: 0xa4,
            group_id: 27
        }
    );
    assert_eq!(state.disable_counts[0], Some(0));
    assert_eq!(state.disable_counts[1], Some(-9));
    assert_eq!(state.disable_counts[2], None);
    assert_eq!(state.assembly_group, 17);
    assert_eq!(state.part_user_data, [17; 24]);
    assert_eq!(state.volume_disable_count_active, Some(true));
    assert_eq!(state.collision.priority[1].to_bits(), 2.0_f32.to_bits());
    state.set_up_no_collision(true);
    assert_eq!(state.disable_counts, [Some(0); 24]);
    assert_eq!(state.volume_headers[1].group_id, 27);
    assert_eq!(state.volume_disable_count_active, Some(true));
}

#[test]
fn world_only_late_unknown_retains_earlier_headers_without_group_or_root_changes() {
    let mut state = fixture();
    state.set_group_and_userdata(17);
    state.volume_headers[0] = SkeletonVolumeHeader {
        flags: 0x81,
        group_id: 99,
    };
    state.disable_counts[0] = Some(7);
    state.volume_headers[1] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 8,
    };
    state.disable_counts[1] = Some(0);
    state.volume_headers[2] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 9,
    };
    state.disable_counts[2] = Some(-2);
    state.volume_headers[4] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 0,
    };
    state.disable_counts[4] = None;
    assert_eq!(
        state.try_collide_with_world_only(),
        Err(SkeletonRuntimeStateError::UnknownDisableCount { part: 4 })
    );
    assert_eq!(
        state.volume_headers[1],
        SkeletonVolumeHeader {
            flags: 0x81,
            group_id: 0
        }
    );
    assert_eq!(
        state.volume_headers[2],
        SkeletonVolumeHeader {
            flags: 0x80,
            group_id: 9
        }
    );
    assert_eq!(
        state.volume_headers[0],
        SkeletonVolumeHeader {
            flags: 0x81,
            group_id: 99
        }
    );
    assert_eq!(state.disable_counts[0], Some(7));
    assert_eq!(state.assembly_group, 17);
    assert_eq!(state.part_user_data, [17; 24]);
    // The admitted unknown join never manufactures a zero counter.
    assert_eq!(state.disable_counts[3], None);
}

#[test]
fn full_collision_failure_keeps_root_and_group_before_partial_part_enables() {
    let mut state = fixture();
    state.set_group_and_userdata(17);
    state.volume_headers[0] = SkeletonVolumeHeader {
        flags: 0x81,
        group_id: 99,
    };
    state.disable_counts[0] = Some(7);
    state.volume_headers[1] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 8,
    };
    state.disable_counts[1] = Some(0);
    state.volume_headers[2] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 9,
    };
    state.disable_counts[2] = None;
    assert_eq!(
        state.try_set_up_full_collision(),
        Err(SkeletonRuntimeStateError::UnknownDisableCount { part: 2 })
    );
    assert_eq!(
        state.volume_headers[0],
        SkeletonVolumeHeader {
            flags: 0x80,
            group_id: 4
        }
    );
    assert_eq!(state.disable_counts[0], Some(0));
    assert_eq!(state.assembly_group, 6);
    assert_eq!(state.part_user_data, [6; 24]);
    assert_eq!(
        state.volume_headers[1],
        SkeletonVolumeHeader {
            flags: 0x81,
            group_id: 0
        }
    );
    assert_eq!(
        state.volume_headers[2],
        SkeletonVolumeHeader {
            flags: 0x80,
            group_id: 9
        }
    );
}

#[test]
fn ragdoll_collision_publishes_group_seven_and_never_touches_root() {
    let mut state = fixture();
    state.set_group_and_userdata(17);
    state.volume_headers[0] = SkeletonVolumeHeader {
        flags: 0x81,
        group_id: 99,
    };
    state.disable_counts[0] = Some(7);
    state.volume_headers[1] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 8,
    };
    state.disable_counts[1] = Some(0);
    state.volume_headers[2] = SkeletonVolumeHeader {
        flags: 0x80,
        group_id: 9,
    };
    state.disable_counts[2] = None;
    assert_eq!(
        state.try_set_up_ragdoll_collision(),
        Err(SkeletonRuntimeStateError::UnknownDisableCount { part: 2 })
    );
    assert_eq!(
        state.volume_headers[0],
        SkeletonVolumeHeader {
            flags: 0x81,
            group_id: 99
        }
    );
    assert_eq!(state.disable_counts[0], Some(7));
    assert_eq!(state.assembly_group, 7);
    assert_eq!(state.part_user_data, [7; 24]);
    assert_eq!(
        state.volume_headers[1],
        SkeletonVolumeHeader {
            flags: 0x81,
            group_id: 0
        }
    );
}

#[test]
fn world_only_completes_headers_then_group_and_outer_root_suffix_is_narrow() {
    let mut state = fixture();
    state.set_group_and_userdata(17);
    state.volume_headers[0] = SkeletonVolumeHeader {
        flags: 0x85,
        group_id: 99,
    };
    state.disable_counts[0] = Some(7);
    state.volume_headers[23] = SkeletonVolumeHeader {
        flags: 0x40,
        group_id: 8,
    };
    state.disable_counts[23] = Some(0);
    state.try_collide_with_world_only().unwrap();
    assert_eq!(
        state.volume_headers[23],
        SkeletonVolumeHeader {
            flags: 0x41,
            group_id: 0
        }
    );
    assert_eq!(state.assembly_group, 3);
    assert_eq!(state.part_user_data, [3; 24]);
    assert_eq!(
        state.volume_headers[0],
        SkeletonVolumeHeader {
            flags: 0x85,
            group_id: 99
        }
    );
    assert_eq!(state.disable_counts[0], Some(7));
    let non_root = state.volume_headers[1..].to_vec();
    state.disable_root_collision();
    assert_eq!(
        state.volume_headers[0],
        SkeletonVolumeHeader {
            flags: 0x84,
            group_id: 99
        }
    );
    assert_eq!(state.disable_counts[0], Some(0));
    assert_eq!(state.volume_headers[1..], non_root);
    assert_eq!(state.assembly_group, 3);
    assert_eq!(state.part_user_data, [3; 24]);
}

#[test]
fn teleport_reset_clears_priority_not_compliance_or_volume_material_state() {
    let mut state = fixture();
    state.select_root_zero_material();
    state.select_normal_material(1);
    state.set_group_and_userdata(6);
    state.set_up_no_collision(false);
    state.disable_counts[3] = Some(-12);
    state.is_ragdoll = true;
    state.volume_disable_count_active = Some(true);
    state.collision.priority[1] = 99.0;
    state.collision.is_compliant[1] = 0xfe;
    state.collision.normal_info[0].normal =
        crate::skeleton_collision_data::SkeletonCollisionVector::known([7.0; 4]);
    let specific = state.collision.specific_parts.clone();
    let headers = state.volume_headers;
    let materials = state.material_selection;
    state.teleport_reset();
    assert!(!state.is_ragdoll);
    assert_eq!(state.disable_counts, [Some(0); 24]);
    assert_eq!(state.volume_disable_count_active, Some(false));
    assert_eq!(state.collision.priority.map(f32::to_bits), [0; 24]);
    assert_eq!(state.collision.is_compliant[1], 0xfe);
    assert_eq!(
        state.collision.normal_info[0]
            .normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [7.0_f32.to_bits(); 4]
    );
    assert_eq!(state.collision.specific_parts, specific);
    assert_eq!(state.volume_headers, headers);
    assert_eq!(state.material_selection, materials);
    assert_eq!(state.part_user_data, [6; 24]);
}
