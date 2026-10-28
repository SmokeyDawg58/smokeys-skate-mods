use super::*;

#[test]
fn hand_target_setters_feed_external_modes_without_overwriting_foot_or_local_targets() {
    let mut ik = state();
    let left = SkeletonRootVector {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    let right = SkeletonRootVector::known([4.0, 5.0, 6.0, -0.0]);
    ik.set_world_foot_target(true, [7.0, 8.0, 9.0, 0.0].into(), 0.75);
    ik.set_world_hand_target(true, left, 0.25);
    ik.set_world_hand_target(false, right, 0.5);
    assert_eq!(ik.external_target_positions_world[2], left);
    assert_eq!(ik.external_target_positions_world[3], right);
    assert_eq!(ik.external_target_positions_world[0].xyz, [7.0, 8.0, 9.0]);
    assert_eq!(ik.external_target_set, [true, false, true, true]);
    assert_eq!(ik.target_blends, [0.75, 0.0, 0.25, 0.5]);
    assert_eq!(ik.local_target_set, [false; 4]);
    assert_eq!(ik.external_target_normal_set, [false; 4]);
    ik.update_status(0);
    assert_eq!(ik.modes[2..], [2, 2]);
}
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarManager, InputCvarReferences},
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead},
    skeleton_data_tests::{build, fixture},
};
use std::sync::Arc;

const VECTOR_KEYS: [u64; 2] = [0xaa37_8f2e_bf7a_a313, 0x7ce3_2881_4f93_09ae];
const SCALAR_KEYS: [u64; 2] = [0x39fa_8c07_e835_6284, 0x094a_ce5d_e068_51fc];
fn targets() -> SkeletonIkTargets {
    let (hierarchy, parameters, pose) = fixture();
    let mut result = SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
    result.initialize();
    result
}
fn state() -> SkeletonIkState {
    SkeletonIkState::new(&mut targets())
}

#[test]
fn walking_plant_uses_world_target_and_releases_instead_of_refreshing_stale_local_target() {
    let mut state = state();
    let planted = SkeletonRootVector::known([120.0, 3.0, -80.0, 0.0]);
    let swinging = SkeletonRootVector::known([119.0, 3.0, -81.0, 0.0]);
    // A stale target from an earlier step must not become an active request.
    state.external_target_positions_world[1] = swinging;
    state.set_world_foot_target(true, planted, 1.0);
    state.update_status(0);
    assert_eq!(state.modes, [2, 1, 0, 0]);
    assert_eq!(state.external_blends, [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(state.external_target_positions_world[0], planted);
    assert_eq!(state.external_target_positions_world[1], swinging);
    assert_eq!(state.local_target_set, [false; 4]);
    // End-of-tick flags clear; on the next footfall only the right is set.
    state.external_target_set.fill(false);
    state.set_world_foot_target(false, swinging, 1.0);
    state.update_status(0);
    assert_eq!(state.modes, [3, 2, 0, 0]);
    assert_eq!(state.external_target_set, [false, true, false, false]);
    assert_eq!(state.local_target_set, [false; 4]);
    assert_eq!(state.target_blends[1], 1.0);
}

#[test]
fn local_foot_target_setters_keep_both_slots_and_unknown_w_separate() {
    let mut state = state();
    let mut left = SkeletonRootVector::known([2.0, 3.0, 4.0, 5.0]);
    left.w = None;
    state.set_local_foot_target(true, left, 0.4);
    assert_eq!(state.external_target_positions_local()[0], left);
    assert_eq!(state.local_target_set(), &[true, false, false, false]);
    assert_eq!(state.target_blends()[0], 0.4);

    let right = SkeletonRootVector::known([7.0, 8.0, 9.0, 10.0]);
    state.set_local_foot_target(false, right, 0.6);
    assert_eq!(state.external_target_positions_local()[0], left);
    assert_eq!(state.external_target_positions_local()[1], right);
    assert_eq!(state.local_target_set(), &[true, true, false, false]);
    assert_eq!(state.target_blends()[1], 0.6);
}

#[test]
fn target_normal_first_call_sets_flag_and_blend_without_prior_normal_clamp() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    assert_eq!(
        refs.try_max_normal_angle_delta().unwrap().to_bits(),
        0x3dcc_cccd
    );
    assert_eq!(
        refs.try_target_normal_blend_speed().unwrap().to_bits(),
        0x3e4c_cccd
    );
    let mut state = state();
    let mut incoming = SkeletonRootVector::known([0.0, 1.0, 0.0, 7.0]);
    incoming.w = None;
    state
        .try_set_foot_target_normal(true, incoming, &refs)
        .unwrap();
    assert_eq!(
        state.external_target_normal_set(),
        &[true, false, false, false]
    );
    assert_eq!(
        state.external_target_normals()[0],
        SkeletonRootVector::known([0.0; 4])
    );
    assert_eq!(
        state.external_target_normal_blends()[0].to_bits(),
        0x3e4c_cccd
    );
    assert_eq!(state.external_target_normal_blends()[1], 0.0);
}

#[test]
fn target_normal_prior_blend_requires_linked_angle_before_any_new_store() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut state = state();
    state.external_target_normal_blends[0] = 0.25;
    state.external_target_normals[0] = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
    let before = state.external_target_normals[0];
    assert!(matches!(
        state.try_set_foot_target_normal(true, before, &refs),
        Err(set_target_normal::SetTargetNormalError::Cvar(
            InputCvarError::Unlinked("kMaxNormalAngleDelta")
        ))
    ));
    assert_eq!(state.external_target_normals[0], before);
    assert!(!state.external_target_normal_set[0]);
    assert_eq!(state.external_target_normal_blends[0], 0.25);
}

#[test]
fn target_normal_unordered_prior_blend_enters_native_clamp_gate() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut state = state();
    state.external_target_normal_blends[0] = f32::NAN;
    assert!(matches!(
        state.try_set_foot_target_normal(
            true,
            SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            &refs,
        ),
        Err(set_target_normal::SetTargetNormalError::Cvar(
            InputCvarError::Unlinked("kMaxNormalAngleDelta")
        ))
    ));
    assert!(!state.external_target_normal_set[0]);
}

#[test]
fn target_normal_prior_blend_reuses_source_clamp_and_keeps_unknown_w() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut state = state();
    state.external_target_normal_blends[0] = 0.25;
    state.external_target_normals[0] = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
    let mut incoming = SkeletonRootVector::known([0.0, 1.0, 0.0, 7.0]);
    incoming.w = None;
    state
        .try_set_foot_target_normal(true, incoming, &refs)
        .unwrap();
    assert_eq!(state.external_target_normals[0], incoming);
    assert!(state.external_target_normal_set[0]);
    assert!((state.external_target_normal_blends[0] - 0.45).abs() < 1e-6);
}

#[test]
fn skeleton_init_resets_modes_and_two_blends_without_replaying_full_ik_reset() {
    let mut state = state();
    state.modes = [3, 2, 1, 4];
    state.on_deck_blends = [0.5; 4];
    state.external_blends = [0.75; 4];
    state.target_blends = [0.25; 4];
    state.external_target_normal_blends = [0.125; 4];
    state.contact_offsets = [1.0, 2.0, 3.0, 4.0];
    state.external_target_set = [true; 4];
    state.finish_skeleton_init();
    assert!(state.do_ik());
    assert_eq!(state.modes, [0; 4]);
    assert_eq!(state.on_deck_blends.map(f32::to_bits), [0; 4]);
    assert_eq!(state.external_blends.map(f32::to_bits), [0; 4]);
    assert_eq!(
        state.target_blends.map(f32::to_bits),
        [0.25_f32.to_bits(); 4]
    );
    assert_eq!(
        state.external_target_normal_blends.map(f32::to_bits),
        [0.125_f32.to_bits(); 4]
    );
    assert_eq!(
        state.contact_offsets.map(f32::to_bits),
        [1.0_f32, 2.0, 3.0, 4.0].map(f32::to_bits)
    );
    assert_eq!(state.external_target_set, [true; 4]);
}

#[test]
fn disable_clears_each_entrys_blends_and_mode_but_keeps_targets_and_contacts() {
    let mut state = state();
    state.modes = [1, 1, 2, 3];
    state.on_deck_blends = [1.0, 1.0, 0.5, 0.25];
    state.external_blends = [0.75; 4];
    state.target_blends = [0.25; 4];
    state.contact_offsets = [1.0, 2.0, 3.0, 4.0];
    state.contact_states = [7, 6, 5, 4];
    state.external_target_set = [true; 4];
    state.disable();
    assert!(!state.do_ik());
    assert_eq!(state.modes, [0; 4]);
    assert_eq!(state.on_deck_blends.map(f32::to_bits), [0; 4]);
    assert_eq!(state.external_blends.map(f32::to_bits), [0; 4]);
    assert_eq!(
        state.target_blends.map(f32::to_bits),
        [0.25_f32.to_bits(); 4]
    );
    assert_eq!(
        state.contact_offsets.map(f32::to_bits),
        [1.0_f32, 2.0, 3.0, 4.0].map(f32::to_bits)
    );
    assert_eq!(state.contact_states, [7, 6, 5, 4]);
    assert_eq!(state.external_target_set, [true; 4]);
}

#[test]
fn disabled_feet_leave_on_deck_ik_at_once() {
    let mut state = state();
    state.modes = [1, 1, 0, 0];
    state.on_deck_blends = [1.0, 1.0, 0.0, 0.0];
    state.disable();
    state.update_status(0);
    assert_eq!(state.modes, [0; 4]);
    assert_eq!(state.on_deck_blends.map(f32::to_bits), [0; 4]);
}

#[test]
fn enable_changes_only_retained_flag_preserving_contact_and_blend_histories() {
    let mut state = state();
    state.disable();
    state.modes = [3, 2, 1, 4];
    state.contact_offsets = [1.0, 2.0, 3.0, 4.0];
    state.on_deck_blends = [0.1, 0.2, 0.3, 0.4];
    state.contact_states = [7, 6, 5, 4];
    state.enable();
    assert!(state.do_ik());
    assert_eq!(state.modes, [3, 2, 1, 4]);
    assert_eq!(
        state.contact_offsets.map(f32::to_bits),
        [1.0_f32, 2.0, 3.0, 4.0].map(f32::to_bits)
    );
    assert_eq!(
        state.on_deck_blends.map(f32::to_bits),
        [0.1_f32, 0.2, 0.3, 0.4].map(f32::to_bits)
    );
    assert_eq!(state.contact_states, [7, 6, 5, 4]);
}

struct Deck {
    loads: usize,
    fail_at: Option<usize>,
}
impl SkateboardAttributeRead for Deck {
    type Pointer = u64;
    type Error = &'static str;
    fn lookup(&mut self, class: SkateboardAttributeClass, key: u64) -> Result<u64, Self::Error> {
        assert_eq!(class, SkateboardAttributeClass::Deck);
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        self.loads += 1;
        if self.fail_at == Some(self.loads) {
            return Err("deck");
        }
        Ok(match key {
            0xd76d_9db5_ee1f_d008 | 0xc94d_6509_1dcb_828b => 2.0,
            0x1d53_bc53_9321_90e7 => 0.0,
            0x8020_3de9_f9b7_32ca => 30.0,
            _ => panic!("unexpected geometry key"),
        })
    }
    fn byte(&mut self, _: &u64) -> Result<u8, Self::Error> {
        panic!("no byte")
    }
    fn integer(&mut self, _: &u64) -> Result<i32, Self::Error> {
        panic!("no integer")
    }
}
fn geometry() -> SkeletonIkGeometry {
    let mut geometry = SkeletonIkGeometry::new();
    geometry
        .try_construct(
            &mut Deck {
                loads: 0,
                fail_at: None,
            },
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    geometry
}
struct Attributes {
    events: Vec<(bool, u64)>,
    vectors: [[f32; 4]; 2],
    scalars: [f32; 2],
    fail_at: Option<usize>,
}
impl Default for Attributes {
    fn default() -> Self {
        Self {
            events: vec![],
            vectors: [[0.0, 1.0, 0.0, f32::NAN], [1.0, 1.0, 1.0, f32::NAN]],
            scalars: [0.25, 0.125],
            fail_at: None,
        }
    }
}
impl Attributes {
    fn record(&mut self, vector: bool, key: u64) -> Result<(), &'static str> {
        self.events.push((vector, key));
        if self.fail_at == Some(self.events.len()) {
            Err("attribute")
        } else {
            Ok(())
        }
    }
}
impl SkeletonIkAttributeRead for Attributes {
    type Error = &'static str;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        self.record(false, key)?;
        Ok(self.scalars[SCALAR_KEYS
            .iter()
            .position(|candidate| *candidate == key)
            .unwrap()])
    }
    fn vector(&mut self, key: u64) -> Result<[f32; 4], Self::Error> {
        self.record(true, key)?;
        Ok(self.vectors[VECTOR_KEYS
            .iter()
            .position(|candidate| *candidate == key)
            .unwrap()])
    }
}
fn vector_seed() -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [11.0, 12.0, 13.0],
        w: None,
    }
}
fn bits(values: &[f32; 4]) -> [u32; 4] {
    values.map(f32::to_bits)
}
fn assert_vector_zero(vector: SkeletonRootVector) {
    assert_eq!(vector.xyz.map(f32::to_bits), [0; 3]);
    assert_eq!(vector.w.map(f32::to_bits), Some(0));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one full Reset oracle seeds and checks every native retained field"
)]
fn full_reset_clears_every_history_and_reuses_existing_target_mapping_authority() {
    let mut targets = targets();
    let data = Arc::clone(targets.data());
    let bones = *targets.bone_ids();
    let target_bones = *targets.target_bone_ids();
    let mut state = SkeletonIkState::new(&mut targets);
    let mut offset = IDENTITY;
    offset[12] = 19.0;
    targets.add_offset_transform(&offset);
    state.do_ik = false;
    state.skateboard_dont_push = true;
    state.tell_anim_dont_push_or_brake = true;
    state.modes = [99; 4];
    state.contact_states = [99; 4];
    state.parts_on_deck = [99; 4];
    for values in [
        &mut state.on_deck_blends,
        &mut state.external_blends,
        &mut state.target_blends,
        &mut state.external_target_normal_blends,
        &mut state.contact_target_offsets,
        &mut state.contact_offsets,
    ] {
        values.fill(7.0);
    }
    for flags in [
        &mut state.external_target_normal_set,
        &mut state.external_target_set,
        &mut state.local_target_set,
    ] {
        flags.fill(true);
    }
    for vectors in [
        &mut state.external_target_positions_world,
        &mut state.external_target_positions_local,
        &mut state.external_target_local_deltas,
        &mut state.external_target_normals,
        &mut state.part_positions,
        &mut state.contact_positions,
    ] {
        vectors.fill(vector_seed());
    }
    let matrix = SkeletonRootMatrix::from_columns([vector_seed(); 4]);
    for matrices in [
        &mut state.parent_to_animation,
        &mut state.parent_to_world,
        &mut state.parent_to_world_external,
        &mut state.parent_to_deck,
        &mut state.part_to_world_external,
        &mut state.part_to_world,
        &mut state.part_to_deck,
    ] {
        matrices.fill(matrix);
    }
    state.last_left_foot_position_deck = vector_seed();
    state.last_right_foot_position_deck = vector_seed();
    state.reset(&mut targets);
    assert!(state.do_ik());
    assert!(!state.skateboard_dont_push());
    assert!(!state.tell_anim_dont_push_or_brake());
    assert_eq!(*state.modes(), [0; 4]);
    assert_eq!(*state.contact_states(), [0; 4]);
    assert_eq!(*state.parts_on_deck(), [0; 4]);
    for values in [
        state.on_deck_blends(),
        state.external_blends(),
        state.target_blends(),
        state.external_target_normal_blends(),
        state.contact_target_offsets(),
        state.contact_offsets(),
    ] {
        assert_eq!(bits(values), [0; 4]);
    }
    for flags in [
        state.external_target_normal_set(),
        state.external_target_set(),
        state.local_target_set(),
    ] {
        assert_eq!(*flags, [false; 4]);
    }
    for vectors in [
        state.external_target_positions_world(),
        state.external_target_positions_local(),
        state.external_target_local_deltas(),
        state.external_target_normals(),
        state.part_positions(),
        state.contact_positions(),
    ] {
        for vector in vectors {
            assert_vector_zero(*vector);
        }
    }
    for matrices in [
        state.parent_to_animation(),
        state.parent_to_world(),
        state.parent_to_world_external(),
        state.parent_to_deck(),
        state.part_to_world_external(),
        state.part_to_world(),
        state.part_to_deck(),
    ] {
        for matrix in matrices {
            assert_eq!(
                matrix.known_lanes().unwrap().map(f32::to_bits),
                IDENTITY.map(f32::to_bits)
            );
        }
    }
    assert_vector_zero(state.last_left_foot_position_deck());
    assert_vector_zero(state.last_right_foot_position_deck());
    for matrix in targets.targets() {
        assert_eq!(matrix.map(f32::to_bits), IDENTITY.map(f32::to_bits));
    }
    assert!(targets.initialized());
    assert!(Arc::ptr_eq(targets.data(), &data));
    assert_eq!(*targets.bone_ids(), bones);
    assert_eq!(*targets.target_bone_ids(), target_bones);
}

#[test]
fn desired_feet_follow_do_ik_while_hands_independently_follow_bits_nine_and_ten() {
    for (do_ik, flags, expected) in [
        (true, 0, [1, 1, 0, 0]),
        (false, 0x200, [0, 0, 1, 0]),
        (false, 0x400, [0, 0, 0, 1]),
        (true, 0xffff_ffff, [1; 4]),
    ] {
        let mut state = state();
        state.do_ik = do_ik;
        state.on_deck_blends = [0.7; 4];
        state.external_blends = [0.8; 4];
        state.update_status(flags);
        assert_eq!(*state.modes(), expected);
        assert_eq!(bits(state.on_deck_blends()), [0; 4]);
        assert_eq!(bits(state.external_blends()), [0; 4]);
    }
}

#[test]
fn disabled_external_priority_and_local_entry_have_distinct_delta_resets() {
    let mut state = state();
    state.external_target_set = [true, false, true, false];
    state.local_target_set = [false, true, true, false];
    state.target_blends = [0.25, 0.5, 0.75, 1.0];
    state.on_deck_blends = [0.8; 4];
    state.external_target_local_deltas = [vector_seed(); 4];
    state.update_status(0x400);
    assert_eq!(*state.modes(), [2, 3, 2, 1]);
    assert_eq!(
        bits(state.external_blends()),
        [0.25_f32.to_bits(), 0.5_f32.to_bits(), 0.75_f32.to_bits(), 0]
    );
    assert_eq!(bits(state.on_deck_blends()), [0; 4]);
    assert_vector_zero(state.external_target_local_deltas[1]);
    for index in [0, 2, 3] {
        assert_eq!(state.external_target_local_deltas[index].w, None);
    }
}

#[test]
fn on_deck_external_transitions_preserve_old_blend_and_ordered_zero_controls_exit() {
    let mut state = state();
    state.do_ik = false;
    state.modes = [1; 4];
    state.on_deck_blends = [0.25, 0.5, -0.0, f32::NAN];
    state.external_blends = [0.75; 4];
    state.target_blends = [0.125; 4];
    state.external_target_set[0] = true;
    state.local_target_set[1] = true;
    state.external_target_local_deltas = [vector_seed(); 4];
    state.update_status(0);
    assert_eq!(*state.modes(), [2, 3, 0, 1]);
    assert_eq!(
        bits(state.external_blends()),
        [
            0.125_f32.to_bits(),
            0.125_f32.to_bits(),
            0,
            0.75_f32.to_bits()
        ]
    );
    assert_eq!(state.on_deck_blends[0].to_bits(), 0.25_f32.to_bits());
    assert_eq!(state.on_deck_blends[1].to_bits(), 0.5_f32.to_bits());
    assert_eq!(state.on_deck_blends[2].to_bits(), 0x8000_0000);
    assert!(state.on_deck_blends[3].is_nan());
    assert_vector_zero(state.external_target_local_deltas[1]);
    assert_eq!(state.external_target_local_deltas[2].w, None);
}

#[test]
fn losing_external_target_only_changes_mode_without_clearing_histories() {
    let mut state = state();
    state.modes = [2; 4];
    state.external_target_set = [false, true, false, true];
    state.external_blends = [0.5; 4];
    state.on_deck_blends = [0.25; 4];
    state.external_target_local_deltas = [vector_seed(); 4];
    state.update_status(0);
    assert_eq!(*state.modes(), [3, 2, 3, 2]);
    assert_eq!(bits(state.external_blends()), [0.5_f32.to_bits(); 4]);
    assert_eq!(bits(state.on_deck_blends()), [0.25_f32.to_bits(); 4]);
    assert!(
        state
            .external_target_local_deltas
            .iter()
            .all(|value| value.w.is_none())
    );
}

#[test]
fn external_local_second_test_observes_new_external_blend_and_can_override_mode_again() {
    let mut state = state();
    state.modes = [3; 4];
    state.external_target_set = [true, true, true, false];
    state.local_target_set = [false, false, true, false];
    state.target_blends = [-0.0, 0.75, 0.0, 1.0];
    state.external_blends = [1.0, 0.0, 1.0, f32::NAN];
    state.external_target_local_deltas = [vector_seed(); 4];
    state.update_status(0);
    assert_eq!(*state.modes(), [1, 2, 2, 3]);
    assert_vector_zero(state.external_target_local_deltas[0]);
    assert_eq!(state.external_blends[0].to_bits(), 0x8000_0000);
    assert_eq!(state.external_blends[1].to_bits(), 0.75_f32.to_bits());
    for index in [1, 2, 3] {
        assert_eq!(state.external_target_local_deltas[index].w, None);
    }
}

#[test]
fn invalid_raw_modes_preserve_blends_and_flags_through_status_and_required_blend_reads() {
    let mut state = state();
    state.modes = [4, u32::MAX, 0x8000_0000, 100];
    state.external_target_set = [true; 4];
    state.local_target_set = [true; 4];
    state.on_deck_blends = [0.25; 4];
    state.external_blends = [0.75; 4];
    state.update_status(u32::MAX);
    let mut attrs = Attributes::default();
    state
        .try_update_blend_values(&geometry(), 0, &mut attrs)
        .unwrap();
    assert_eq!(*state.modes(), [4, u32::MAX, 0x8000_0000, 100]);
    assert_eq!(bits(state.on_deck_blends()), [0.25_f32.to_bits(); 4]);
    assert_eq!(bits(state.external_blends()), [0.75_f32.to_bits(); 4]);
    assert_eq!(
        attrs.events,
        [
            (true, VECTOR_KEYS[0]),
            (true, VECTOR_KEYS[1]),
            (false, SCALAR_KEYS[0]),
            (false, SCALAR_KEYS[1])
        ]
    );
    assert_eq!(*state.external_target_set(), [true; 4]);
    assert_eq!(*state.local_target_set(), [true; 4]);
}

#[test]
fn incomplete_geometry_refuses_at_its_actual_prefix_read_without_attributes() {
    for fail_at in [Some(1), Some(3)] {
        let mut geometry = SkeletonIkGeometry::new();
        let mut deck = Deck { loads: 0, fail_at };
        assert!(
            geometry
                .try_construct(
                    &mut deck,
                    &mut HostReckoningNumeric,
                    &mut HostReckoningSinCosNumeric
                )
                .is_err()
        );
        let mut state = state();
        let mut attrs = Attributes::default();
        assert_eq!(
            state.try_update_blend_values(&geometry, 0, &mut attrs),
            Err(SkeletonIkBlendError::GeometryUnavailable(
                if fail_at == Some(1) {
                    SkeletonIkGeometryProgress::BoxHalfWidth
                } else {
                    SkeletonIkGeometryProgress::TotalHalfLength
                }
            ))
        );
        assert!(attrs.events.is_empty());
    }
}

#[test]
fn all_four_attribute_failures_precede_any_retained_blend_write() {
    let geometry = geometry();
    for fail_at in 1..=4 {
        let mut state = state();
        state.modes = [3; 4];
        state.on_deck_blends = [0.5; 4];
        state.external_blends = [0.75; 4];
        state.local_target_set = [true; 4];
        let mut attrs = Attributes {
            fail_at: Some(fail_at),
            ..Attributes::default()
        };
        assert_eq!(
            state.try_update_blend_values(&geometry, 0, &mut attrs),
            Err(SkeletonIkBlendError::Attribute("attribute"))
        );
        assert_eq!(bits(state.on_deck_blends()), [0.5_f32.to_bits(); 4]);
        assert_eq!(bits(state.external_blends()), [0.75_f32.to_bits(); 4]);
        assert_eq!(attrs.events.len(), fail_at);
    }
}

#[test]
fn late_external_assertion_keeps_earlier_limbs_and_current_later_old_blends() {
    let mut state = state();
    state.modes = [1, 3, 2, 3];
    state.on_deck_blends = [0.5; 4];
    state.external_blends = [0.75; 4];
    state.local_target_set[1] = true;
    let mut attrs = Attributes::default();
    assert_eq!(
        state.try_update_blend_values(&geometry(), 0, &mut attrs),
        Err(SkeletonIkBlendError::ExternalTargetNotSet { entry: 2 })
    );
    assert_eq!(
        bits(state.on_deck_blends()),
        [
            0.75_f32.to_bits(),
            0.25_f32.to_bits(),
            0.5_f32.to_bits(),
            0.5_f32.to_bits()
        ]
    );
    assert_eq!(
        bits(state.external_blends()),
        [
            0.75_f32.to_bits(),
            1.0_f32.to_bits(),
            0.75_f32.to_bits(),
            0.75_f32.to_bits()
        ]
    );
    assert_eq!(attrs.events.len(), 4);
}

#[test]
fn external_and_local_modes_commit_their_distinct_blends_and_positive_zero_decay() {
    let mut state = state();
    state.modes = [2, 3, 3, 2];
    state.external_target_set = [true, false, false, true];
    state.local_target_set[1] = true;
    state.target_blends = [0.75; 4];
    state.on_deck_blends = [0.5, 0.125, -0.0, f32::NAN];
    state.external_blends = [0.5, 0.5, 0.0625, 0.5];
    state
        .try_update_blend_values(&geometry(), 0, &mut Attributes::default())
        .unwrap();
    assert_eq!(
        bits(state.external_blends()),
        [0.75_f32.to_bits(), 1.0_f32.to_bits(), 0, 0.75_f32.to_bits()]
    );
    assert_eq!(state.on_deck_blends[0].to_bits(), 0.25_f32.to_bits());
    assert_eq!(state.on_deck_blends[1].to_bits(), 0);
    assert_eq!(state.on_deck_blends[2].to_bits(), 0);
    assert!(state.on_deck_blends[3].is_nan());
}

#[test]
fn feet_use_flags_bit_twenty_eight_and_per_call_step_without_timestep() {
    let geometry = geometry();
    for (flags, expected) in [(0, 0.75_f32), (0x1000_0000, 0.25), (0xefff_ffff, 0.75)] {
        let mut state = state();
        state.modes = [1, 1, 0, 0];
        state.on_deck_blends = [0.5; 4];
        state
            .try_update_blend_values(&geometry, flags, &mut Attributes::default())
            .unwrap();
        assert_eq!(state.on_deck_blends[0].to_bits(), expected.to_bits());
        assert_eq!(state.on_deck_blends[1].to_bits(), expected.to_bits());
        assert_eq!(state.on_deck_blends[2].to_bits(), 0.5_f32.to_bits());
    }
}

#[test]
fn hands_consume_prior_positions_and_positive_blends_decay_after_leaving_box() {
    let mut state = state();
    state.modes = [0, 0, 1, 1];
    state.part_positions[2] = SkeletonRootVector {
        xyz: [-1.5, 0.0, 0.0],
        w: None,
    };
    state.part_positions[3] = SkeletonRootVector::known([0.0, 0.0, 0.0, f32::NAN]);
    let geometry = geometry();
    state
        .try_update_blend_values(&geometry, 0, &mut Attributes::default())
        .unwrap();
    assert_eq!(
        bits(state.on_deck_blends()),
        [0, 0, 0.25_f32.to_bits(), 0.25_f32.to_bits()]
    );
    assert_eq!(
        state.part_positions[2].xyz.map(f32::to_bits),
        [-1.5_f32, 0.0, 0.0].map(f32::to_bits)
    );
    state.part_positions[2].xyz = [3.0, 0.0, 0.0];
    state
        .try_update_blend_values(&geometry, 0, &mut Attributes::default())
        .unwrap();
    assert_eq!(state.on_deck_blends[2].to_bits(), 0);
    state.on_deck_blends[2] = -0.25;
    state
        .try_update_blend_values(&geometry, 0, &mut Attributes::default())
        .unwrap();
    assert_eq!(state.on_deck_blends[2].to_bits(), (-0.25_f32).to_bits());
}

#[test]
fn strict_box_edges_and_unordered_coordinates_reject_zero_blend_hands() {
    let geometry = geometry();
    for axis in 0..3 {
        for coordinate in [2.0, f32::NAN] {
            let mut state = state();
            state.modes[2] = 1;
            state.part_positions[2].xyz[axis] = coordinate;
            state
                .try_update_blend_values(&geometry, 0, &mut Attributes::default())
                .unwrap();
            assert_eq!(state.on_deck_blends[2].to_bits(), 0);
        }
    }
}

#[test]
fn positive_old_hand_blend_bypasses_nan_box_test_and_ordered_falloff_skips_nan_axis() {
    let mut state = state();
    state.modes[2] = 1;
    state.on_deck_blends[2] = 0.25;
    state.part_positions[2] = SkeletonRootVector {
        xyz: [f32::NAN, 0.0, 0.0],
        w: None,
    };
    state
        .try_update_blend_values(&geometry(), 0, &mut Attributes::default())
        .unwrap();
    assert_eq!(state.on_deck_blends[2].to_bits(), 0.5_f32.to_bits());
}

#[test]
fn unordered_z_candidate_retains_x_target_but_unordered_y_candidate_clamps_to_one() {
    for (axis, expected) in [(2, 0.5_f32), (1, 1.0)] {
        let mut state = state();
        state.modes[2] = 1;
        state.on_deck_blends[2] = 0.25;
        state.part_positions[2].xyz = [1.5, 0.0, 0.0];
        state.part_positions[2].xyz[axis] = 1.5;
        let mut attrs = Attributes::default();
        attrs.scalars[0] = 2.0;
        attrs.vectors[1][axis] = f32::NAN;
        state
            .try_update_blend_values(&geometry(), 0, &mut attrs)
            .unwrap();
        assert_eq!(state.on_deck_blends[2].to_bits(), expected.to_bits());
    }
}

#[test]
fn zero_range_and_negative_or_nan_steps_follow_source_selections_without_repair() {
    let geometry = geometry();
    let mut state = state();
    state.modes[2] = 1;
    state.on_deck_blends[2] = 0.5;
    state.part_positions[2].xyz[0] = 1.5;
    let mut attrs = Attributes::default();
    attrs.vectors[1][0] = 0.0;
    state
        .try_update_blend_values(&geometry, 0, &mut attrs)
        .unwrap();
    assert_eq!(state.on_deck_blends[2].to_bits(), 0.25_f32.to_bits());
    state.modes = [1, 1, 3, 3];
    state.on_deck_blends = [0.5; 4];
    state.external_blends = [0.5; 4];
    attrs.scalars = [-0.25, -0.125];
    state
        .try_update_blend_values(&geometry, 0, &mut attrs)
        .unwrap();
    assert_eq!(
        bits(state.on_deck_blends()),
        [
            0.25_f32.to_bits(),
            0.25_f32.to_bits(),
            0.75_f32.to_bits(),
            0.75_f32.to_bits()
        ]
    );
    assert_eq!(state.external_blends[2].to_bits(), 0.625_f32.to_bits());
    attrs.scalars = [f32::NAN; 2];
    state
        .try_update_blend_values(&geometry, 0, &mut attrs)
        .unwrap();
    assert!(state.on_deck_blends.iter().all(|value| value.is_nan()));
    assert!(state.external_blends[2].is_nan());
}

#[test]
fn live_blend_flag_is_read_per_active_foot_after_attributes_and_preserves_first_store() {
    use crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError;
    let mut state = state();
    state.modes = [1, 1, 0, 0];
    state.on_deck_blends = [0.0; 4];
    let mut attributes = Attributes::default();
    let mut reads = 0;
    let result = state.try_update_blend_values_read(
        &geometry(),
        || {
            reads += 1;
            if reads == 1 {
                Ok(false)
            } else {
                Err(GroundedInputError::Missing("blend flag"))
            }
        },
        &mut attributes,
    );
    assert!(matches!(
        result,
        Err(SkeletonIkBlendError::Input(GroundedInputError::Missing(
            "blend flag"
        )))
    ));
    assert_eq!(attributes.events.len(), 4);
    assert_eq!(reads, 2);
    assert_eq!(state.on_deck_blends[0].to_bits(), 0.25_f32.to_bits());
    assert_eq!(state.on_deck_blends[1].to_bits(), 0);
    state.modes = [0; 4];
    state
        .try_update_blend_values_read(
            &geometry(),
            || panic!("inactive feet do not read blend flag"),
            &mut attributes,
        )
        .unwrap();
}
