use super::*;
use crate::skeleton_data_tests::{build, fixture};
use std::cell::RefCell;

struct Board {
    reads: usize,
    fail: Option<usize>,
    transforms: [SkeletonRootMatrix; 2],
    events: RefCell<Vec<&'static str>>,
}
impl SkeletonProcessedContactBoard for Board {
    type Error = &'static str;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("mass");
        self.reads = 0;
        Ok(9.0)
    }
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.events.borrow_mut().push("transform");
        let index = self.reads;
        self.reads += 1;
        if self.fail == Some(index) {
            Err("transform")
        } else {
            Ok(self.transforms[index])
        }
    }
    fn flags_8c4(&self) -> u32 {
        assert_eq!(self.reads, 2);
        self.events.borrow_mut().push("flags");
        0x0010_0000
    }
}
fn board() -> Board {
    Board {
        reads: 0,
        fail: None,
        transforms: [
            SkeletonRootMatrix::known([1.0; 16]),
            SkeletonRootMatrix::known([2.0; 16]),
        ],
        events: RefCell::new(vec![]),
    }
}
fn raw(words: [u32; 5]) -> [u8; 0x40] {
    let mut raw = [0; 0x40];
    for (chunk, word) in raw[..20].chunks_exact_mut(4).zip(words) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    raw[0x34] = 4;
    raw[0x35] = 3;
    raw
}

#[test]
fn raw_hierarchy_id_uses_acceptance_first_match_and_last_accepted_record() {
    let (mut hierarchy, parameters, mut pose) = fixture();
    let first = hierarchy.packed_bone_names[2];
    // Duplicate outside the physics mapping keeps the actual SkeletonData valid.
    hierarchy.packed_bone_names.push(first);
    hierarchy.bone_names.push("duplicate".into());
    hierarchy.parents.push(-1);
    hierarchy.bone_metadata.push(0);
    pose.push(pose[0]);
    let data = build(hierarchy, parameters, &pose).unwrap();
    let mut input = SkeletonProcessedContact::new();
    assert_eq!(input.contact_bone_id, Some(0));
    assert!(input.ready(0).is_none());
    let mut accepted = raw(first.words);
    // Attribute name +20 has no bearing on the earlier hierarchy-ID store.
    accepted[0x20..0x34].fill(0xff);
    assert!(input.apply_raw_attribute(&data, &accepted));
    assert_eq!(input.contact_bone_id, Some(2));
    let mut rejected = raw([u32::MAX; 5]);
    rejected[0x35] = 0x83;
    assert!(!input.apply_raw_attribute(&data, &rejected));
    rejected[0x35] = 3;
    rejected[0x34] = 0xf3;
    assert!(!input.apply_raw_attribute(&data, &rejected));
    assert_eq!(input.contact_bone_id, Some(2));
    rejected[0x34] = 8;
    assert!(input.apply_raw_attribute(&data, &rejected));
    assert_eq!(input.contact_bone_id, None);
    input.reset();
    assert_eq!(input.contact_bone_id, Some(0));
}

#[test]
fn typed_contact_uses_bone_payload_and_hand_side_not_attribute_event_side() {
    let (hierarchy, parameters, pose) = fixture();
    let data = build(hierarchy, parameters, &pose).unwrap();
    let mut processed = SkeletonProcessedContact::new();
    processed.reset();
    let mut payload = [None; 24];
    let bone = crate::play_animation::FastString30::encode("righthand").unwrap();
    for (target, byte) in payload
        .iter_mut()
        .zip(bone.words.iter().flat_map(|word| word.to_be_bytes()))
    {
        *target = Some(byte);
    }
    // Native transition bytes are unused for a hand event; keep them unknown.
    let mut attribute = crate::actor_construction::anim_out_phys_in::PhysicsAnimationAttribute {
        payload,
        begin: 0.0,
        end: 1.0,
        name: crate::play_animation::FastString30::encode("left_hand_grab").unwrap(),
        flags: 4,
        kind: 3,
        sequence: None,
    };
    assert_eq!(
        processed
            .try_apply_animation_contact_prefix(&attribute, &data)
            .unwrap(),
        None
    );
    assert_eq!(processed.known_flags_8c8().unwrap() & 0x600, 0x400);
    assert_eq!(
        processed.contact_bone_id,
        data.hierarchy()
            .packed_bone_names
            .iter()
            .position(|name| *name == bone)
    );
    attribute.name = crate::play_animation::FastString30::encode("unrecognized_contact").unwrap();
    attribute.payload[..20].fill(Some(0xff));
    processed
        .try_apply_animation_contact_prefix(&attribute, &data)
        .unwrap();
    assert_eq!(processed.contact_bone_id, None);
    assert_eq!(processed.known_flags_8c8().unwrap() & 0x600, 0x400);
}

#[test]
fn second_transform_failure_retains_mass_deck_and_old_effective_but_not_readiness() {
    let mut input = SkeletonProcessedContact::new();
    let mut board = board();
    input.try_sample_toolkit_prefix(&mut board).unwrap();
    let old_effective = input.effective.unwrap();
    board.fail = Some(1);
    board.transforms[0] = SkeletonRootMatrix::known([3.0; 16]);
    board.events.borrow_mut().clear();
    assert_eq!(
        input.try_sample_toolkit_prefix(&mut board),
        Err("transform")
    );
    assert_eq!(input.physical_mass().unwrap().to_bits(), 9.0_f32.to_bits());
    assert_eq!(input.deck_transform(), Some(&board.transforms[0]));
    assert_eq!(input.effective, Some(old_effective));
    assert!(input.ready(0).is_none());
    assert_eq!(*board.events.borrow(), ["mass", "transform", "transform"]);
}

#[test]
fn physical_effective_uses_second_read_and_xors_all_known_xz_lanes() {
    let mut input = SkeletonProcessedContact::new();
    let mut board = board();
    let bits = [0_u32, 0x8000_0000, 0x7fc1_2345, 0xffc5_4321];
    let original =
        SkeletonRootMatrix::known(std::array::from_fn(|lane| f32::from_bits(bits[lane % 4])));
    board.transforms[1] = original;
    input.try_sample_toolkit_prefix(&mut board).unwrap();
    let ready = input.ready(0x4200_0000).unwrap();
    assert_eq!(ready.flags_8c4(), 0x4200_0000);
    let result = ready.effective_transform().known_lanes().unwrap();
    for lane in 0..16 {
        let sign = if lane / 4 == 0 || lane / 4 == 2 {
            0x8000_0000
        } else {
            0
        };
        assert_eq!(result[lane].to_bits(), bits[lane % 4] ^ sign);
    }
    assert_eq!(
        *board.events.borrow(),
        ["mass", "transform", "transform", "flags"]
    );
    let mut partial = *original.columns();
    partial[0].w = None;
    partial[2].w = None;
    board.transforms[1] = SkeletonRootMatrix::from_columns(partial);
    input.try_sample_toolkit_prefix(&mut board).unwrap();
    let ready = input.ready(0).unwrap();
    assert_eq!(ready.effective_transform().columns()[0].w, None);
    assert_eq!(ready.effective_transform().columns()[2].w, None);
    input.reset();
    assert!(input.ready(0).is_none());
}
