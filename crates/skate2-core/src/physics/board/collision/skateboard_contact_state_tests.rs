use super::*;

fn words(value: Option<SkeletonRootVector>) -> Option<[u32; 4]> {
    let value = value?;
    Some([
        value.xyz[0].to_bits(),
        value.xyz[1].to_bits(),
        value.xyz[2].to_bits(),
        value.w?.to_bits(),
    ])
}

#[test]
fn reset_establishes_source_arrays_but_not_relative_velocity_or_unknown_flags() {
    let mut state = SkateboardContactState::uninitialized();
    assert!(!state.reset_completed());
    assert!(state.normal.iter().all(Option::is_none));
    assert_eq!(state.surfaces.physics_type(0), None);
    assert_eq!(state.flags.read(u32::MAX), None);
    state.reset();
    assert!(state.reset_completed());
    let up = Some([0, 0x3f80_0000, 0, 0]);
    assert_eq!(words(state.average_normal), up);
    assert_eq!(words(state.last_valid_normal), up);
    for part in 0..7 {
        assert_eq!(words(state.normal[part]), up);
        assert_eq!(words(state.position[part]), Some([0; 4]));
        assert_eq!(words(state.velocity[part]), Some([0; 4]));
        assert_eq!(words(state.acceleration[part]), Some([0; 4]));
        assert_eq!(state.relative_velocity[part], None);
        assert_eq!(state.in_contact[part], Some(false));
        assert_eq!(state.surfaces.physics_type(part), Some(0));
        assert_eq!(state.surfaces.validity(part), Some(0));
        assert_eq!(state.audio_type[part], Some(0));
        assert_eq!(state.pattern_type[part], Some(0));
    }
    assert_eq!(state.flags.known_mask(), 0xe800_0000);
    assert_eq!(state.flags.read(0xe800_0000), Some(0));
    assert_eq!(state.flags.read(0x40), None);
}

#[test]
fn repeated_source_reset_preserves_relative_velocity_and_remaining_flag_bits() {
    let mut state = SkateboardContactState::uninitialized();
    state.reset();
    let relative = SkeletonRootVector {
        xyz: [
            f32::from_bits(0x8000_0000),
            f32::from_bits(0x7fc1_2345),
            3.0,
        ],
        w: None,
    };
    state.relative_velocity[6] = Some(relative);
    state.flags = SkateboardContactFlags::known(u32::MAX);
    state.reset();
    let retained = state.relative_velocity[6].unwrap();
    assert_eq!(
        retained.xyz.map(f32::to_bits),
        relative.xyz.map(f32::to_bits)
    );
    assert_eq!(retained.w, None);
    assert_eq!(state.flags.read(u32::MAX), Some(0x17ff_ffff));
    assert!(state.relative_velocity[..6].iter().all(Option::is_none));
}

#[test]
fn validity_and_first_physics_word_share_big_endian_bytes_in_both_directions() {
    let mut state = SkateboardContactState::uninitialized();
    state.surfaces.set_validity(4, 1);
    state.surfaces.set_validity(5, 1);
    state.surfaces.set_validity(6, 1);
    assert_eq!(state.surfaces.physics_type(0), None); // low byte still unknown
    state.reset();
    for part in 0..7 {
        state.surfaces.set_validity(part, 1);
    }
    assert_eq!(state.surfaces.physics_type(0), Some(0x0101_0100));
    assert_eq!(state.surfaces.physics_type(1), Some(0));
    state.surfaces.set_physics_type(0, 0x1234_56ab);
    assert_eq!(state.surfaces.validity(4), Some(0x12));
    assert_eq!(state.surfaces.validity(5), Some(0x34));
    assert_eq!(state.surfaces.validity(6), Some(0x56));
    assert_eq!(state.surfaces.validity(3), Some(1));
    state.surfaces.set_validity(5, 0);
    assert_eq!(state.surfaces.physics_type(0), Some(0x1200_56ab));
}

#[test]
fn flag_operations_establish_only_the_bits_the_source_writes() {
    let mut flags = SkateboardContactFlags::uninitialized();
    flags.or_mask(0x40);
    assert_eq!(flags.read(0x40), Some(0x40));
    assert_eq!(flags.read(0x41), None);
    flags.and_mask(0x17ff_ffff);
    assert_eq!(flags.read(0xe800_0040), Some(0x40));
    assert_eq!(flags.read(1), None);
}
