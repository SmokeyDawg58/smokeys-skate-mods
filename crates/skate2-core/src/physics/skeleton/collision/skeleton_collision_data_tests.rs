#![expect(
    clippy::float_cmp,
    reason = "copied fields and exactly representable synthetic products"
)]
use super::*;

#[test]
fn collision_vectors_distinguish_unknown_w_from_all_known_bit_patterns() {
    let xyz = [-0.0, f32::from_bits(0x7fc1_2345), f32::INFINITY];
    let projected = SkeletonCollisionVector::xyz_only(xyz);
    assert_eq!(projected.xyz().map(f32::to_bits), xyz.map(f32::to_bits));
    assert!(projected.known_lanes().is_none());
    for w in [0, 0x8000_0000, 0x7fc1_2345, 0x7f80_0000, 1] {
        let lanes = [xyz[0], xyz[1], xyz[2], f32::from_bits(w)];
        let known = SkeletonCollisionVector::known(lanes);
        assert_eq!(
            known.known_lanes().unwrap().map(f32::to_bits),
            lanes.map(f32::to_bits)
        );
        assert_eq!(known.xyz().map(f32::to_bits), xyz.map(f32::to_bits));
    }
}

#[test]
fn selective_reset_preserves_unknown_normal_info_but_restores_written_vectors() {
    let mut data = SkeletonCollisionData::try_new(&mut Spy::new()).unwrap();
    let unknown = SkeletonCollisionVector::xyz_only([1.0, -0.0, 3.0]);
    data.normal_info[4].normal = unknown;
    data.contact_normal = [unknown; 8];
    data.most_vertical_contact_normal = unknown;
    data.most_vertical_toe_contact_normal = unknown;
    for bone in &mut data.bone_data {
        bone.contact_normal = unknown;
        bone.sub_part_contact_normal = unknown;
        bone.slide_vector = unknown;
        bone.sub_part_slide_vector = unknown;
        bone.contact_position = unknown;
    }
    data.reset();
    assert!(data.normal_info[4].normal.known_lanes().is_none());
    assert_eq!(
        data.normal_info[4].normal.xyz().map(f32::to_bits),
        [0x3f80_0000, 0x8000_0000, 0x4040_0000]
    );
    for bone in data.bone_data {
        for vector in [
            bone.contact_normal,
            bone.sub_part_contact_normal,
            bone.slide_vector,
            bone.sub_part_slide_vector,
            bone.contact_position,
        ] {
            assert_eq!(vector.known_lanes().unwrap().map(f32::to_bits), [0; 4]);
        }
    }
    assert!(
        data.contact_normal
            .iter()
            .all(|v| v.known_lanes().unwrap().map(f32::to_bits) == [0; 4])
    );
    for vector in [
        data.most_vertical_contact_normal,
        data.most_vertical_toe_contact_normal,
    ] {
        assert_eq!(
            vector.known_lanes().unwrap().map(f32::to_bits),
            [0, 0x3f80_0000, 0, 0]
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Offset(SkeletonCollisionOffset),
    Lookup(u64, usize),
    Load(usize),
}

struct Spy {
    events: Vec<Event>,
    pointers: usize,
    fail_at: Option<usize>,
    values: [f32; 4],
}
impl Spy {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            pointers: 0,
            fail_at: None,
            values: [2.0, 3.0, 5.0, 7.0],
        }
    }
    fn record(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail_at == Some(self.events.len() - 1) {
            Err("source read unavailable")
        } else {
            Ok(())
        }
    }
}
impl SkeletonCollisionAttributeRead for Spy {
    type Pointer = usize;
    type Error = &'static str;
    fn cached_offset(&mut self, slot: SkeletonCollisionOffset) -> Result<[f32; 4], Self::Error> {
        self.record(Event::Offset(slot))?;
        Ok(match slot {
            SkeletonCollisionOffset::Groin => [1.0, 2.0, 3.0, -0.0],
            SkeletonCollisionOffset::Face => [4.0, 5.0, 6.0, 17.0],
        })
    }
    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        let id = self.pointers;
        self.record(Event::Lookup(key, id))?;
        self.pointers += 1;
        Ok(id)
    }
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        self.record(Event::Load(*pointer))?;
        Ok(self.values[*pointer])
    }
}

fn expected_events() -> Vec<Event> {
    vec![
        Event::Offset(SkeletonCollisionOffset::Groin),
        Event::Offset(SkeletonCollisionOffset::Face),
        Event::Lookup(0xa945_308c_de00_2e9c, 0),
        Event::Lookup(0xa945_308c_de00_2e9c, 1),
        Event::Load(1),
        Event::Load(0),
        Event::Lookup(0xa8ed_92d2_01be_e7dd, 2),
        Event::Lookup(0xa8ed_92d2_01be_e7dd, 3),
        Event::Load(3),
        Event::Load(2),
    ]
}

#[test]
fn constructor_samples_cached_offsets_then_duplicate_delayed_radius_pointers() {
    let mut reader = Spy::new();
    let data = SkeletonCollisionData::try_new(&mut reader).unwrap();
    assert_eq!(reader.events, expected_events());
    assert_eq!(
        data.specific_parts.parts,
        [SkeletonPartIndex::Hips, SkeletonPartIndex::Neck1]
    );
    assert_eq!(data.specific_parts.radius_squared, [6.0, 35.0]);
    assert_eq!(
        data.specific_parts.offsets[0].map(f32::to_bits),
        [0x3f80_0000, 0x4000_0000, 0x4040_0000, 0x8000_0000]
    );
    assert_eq!(data.specific_parts.offsets[1], [4.0, 5.0, 6.0, 17.0]);
    assert_eq!(data.specific_parts.world_positions, [[0.0; 4]; 2]);
    assert_eq!(data.specific_parts.contact, [None; 2]);
    assert_eq!(data.specific_parts.recent_contact, [None; 2]);
    assert_eq!(data.is_compliant, [0; 24]);
    for info in data.normal_info {
        assert_eq!(info.part, SkeletonPartIndex::SkateboardRoot);
        assert_eq!(info.normal.known_lanes().unwrap().map(f32::to_bits), [0; 4]);
    }
    assert_eq!(data.contact_bone, [None; 8]);
    assert_eq!(
        data.most_vertical_contact_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0, 0x3f80_0000, 0, 0]
    );
    assert_eq!(
        data.most_vertical_toe_contact_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0, 0x3f80_0000, 0, 0]
    );
}

#[test]
fn each_read_failure_stops_at_exact_prefix_without_publishing_partial_data() {
    let expected = expected_events();
    for failed in 0..expected.len() {
        let mut reader = Spy::new();
        reader.fail_at = Some(failed);
        assert_eq!(
            SkeletonCollisionData::try_new(&mut reader).unwrap_err(),
            "source read unavailable"
        );
        assert_eq!(reader.events, expected[..=failed]);
    }
}

#[test]
fn raw_radius_arithmetic_does_not_sanitize_or_replace_repeated_values() {
    let mut reader = Spy::new();
    reader.values = [-0.0, 3.0, f32::INFINITY, 0.0];
    let data = SkeletonCollisionData::try_new(&mut reader).unwrap();
    assert_eq!(data.specific_parts.radius_squared[0].to_bits(), 0x8000_0000);
    assert!(data.specific_parts.radius_squared[1].is_nan());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "every selective reset field is independently poisoned and checked"
)]
fn reset_clears_all_touched_fields_but_preserves_compliance_normals_and_specific_parts() {
    let mut data = SkeletonCollisionData::try_new(&mut Spy::new()).unwrap();
    let poison = f32::from_bits(0x7fc1_2345);
    data.normal_info[5] = SkeletonCollisionNormalInfo {
        normal: SkeletonCollisionVector::known([poison, -0.0, 7.0, 23.0]),
        part: SkeletonPartIndex::RightHand,
    };
    data.set_compliance(SkeletonPartIndex::SkateboardRoot, 0x80);
    data.set_compliance(SkeletonPartIndex::Neck1, 0xff);
    data.specific_parts.contact = [Some(0x81), None];
    data.specific_parts.recent_contact = [Some(0xff), Some(0x72)];
    data.specific_parts.world_positions = [[8.0, 9.0, 10.0, -0.0]; 2];
    let specific = data.specific_parts.clone();
    data.time_since_collision = [poison; 24];
    data.priority = [poison; 24];
    data.set_priority(SkeletonPartIndex::Hips, -0.0);
    data.just_collided = [0x83; 24];
    data.contact_force = [poison; 8];
    data.contact_friction_force = [poison; 8];
    data.slide_speed = [poison; 8];
    data.audio_surface_type = [u32::MAX; 8];
    data.contact_normal = [SkeletonCollisionVector::known([poison; 4]); 8];
    data.contact_bone = [Some(SkeletonPartIndex::Neck); 8];
    data.most_vertical_contact_normal = SkeletonCollisionVector::known([poison; 4]);
    data.most_vertical_toe_contact_normal = SkeletonCollisionVector::known([poison; 4]);
    data.bone_data = [SkeletonCollisionBoneData {
        contact_normal: SkeletonCollisionVector::known([poison; 4]),
        sub_part_contact_normal: SkeletonCollisionVector::known([poison; 4]),
        slide_vector: SkeletonCollisionVector::known([poison; 4]),
        sub_part_slide_vector: SkeletonCollisionVector::known([poison; 4]),
        contact_position: SkeletonCollisionVector::known([poison; 4]),
        contact_force: poison,
        sub_part_contact_force: poison,
        contact_surface_id: 101,
        sub_part_contact_surface_id: 102,
        hit_vehicle: 0x84,
        hit_pedestrian: 0x85,
        hit_skater: 0x86,
        hit_world: 0x87,
    }; 24];
    data.collision_timer = poison;
    data.drive_strength = poison;
    data.collision_priority = poison;
    data.contact_force_total_during_wipeout = poison;
    data.slide_force_total_during_wipeout = poison;
    data.time_in_wipeout = poison;
    data.skater_contact = poison;
    data.vehicle_contact = poison;
    data.pedestrian_contact = poison;
    data.unrideable_contact = 0x88;
    data.non_compliant_just_collided = 0x89;
    data.compliant_just_collided = 0x8a;
    data.compliant_active = 0x8b;
    data.hit_vehicle = 0x8c;
    data.hit_world = 0x8d;
    data.is_wiping_out = 0x8e;
    data.opposing_contacts = 0x8f;
    data.opposing_contacts_on_plane = 0x90;
    data.is_proxy_in_collision = 0x91;
    data.is_hook_to_hips_valid = 0x92;
    data.foot_hit_skateboard = 0x93;
    data.num_normals = -123;
    data.reset();
    assert_eq!(
        data.normal_info[5]
            .normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0x7fc1_2345, 0x8000_0000, 0x40e0_0000, 0x41b8_0000]
    );
    assert_eq!(data.normal_info[5].part, SkeletonPartIndex::RightHand);
    assert_eq!(&data.is_compliant[..2], &[0x80, 0xff]);
    assert_eq!(&data.is_compliant[2..], &[0; 22]);
    assert_eq!(data.specific_parts, specific);
    assert_eq!(
        data.specific_parts.world_positions[0][3].to_bits(),
        0x8000_0000
    );
    assert_eq!(data.specific_parts.offsets[0][3].to_bits(), 0x8000_0000);
    assert_eq!(data.time_since_collision.map(f32::to_bits), [0; 24]);
    assert_eq!(data.priority.map(f32::to_bits), [0; 24]);
    assert_eq!(data.just_collided, [0; 24]);
    assert_eq!(data.contact_force.map(f32::to_bits), [0; 8]);
    assert_eq!(data.contact_friction_force.map(f32::to_bits), [0; 8]);
    assert_eq!(data.slide_speed.map(f32::to_bits), [0; 8]);
    assert_eq!(data.audio_surface_type, [0; 8]);
    assert_eq!(
        data.contact_normal
            .map(|vector| vector.known_lanes().unwrap().map(f32::to_bits)),
        [[0; 4]; 8]
    );
    assert_eq!(data.contact_bone, [None; 8]);
    for bone in data.bone_data {
        assert_eq!(
            bone.contact_normal.known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            bone.sub_part_contact_normal
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            bone.slide_vector.known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            bone.sub_part_slide_vector
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            bone.contact_position
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            [bone.contact_force, bone.sub_part_contact_force].map(f32::to_bits),
            [0; 2]
        );
        assert_eq!(
            [bone.contact_surface_id, bone.sub_part_contact_surface_id],
            [0; 2]
        );
        assert_eq!(
            [
                bone.hit_vehicle,
                bone.hit_pedestrian,
                bone.hit_skater,
                bone.hit_world
            ],
            [0; 4]
        );
    }
    assert_eq!(
        [
            data.collision_timer,
            data.drive_strength,
            data.collision_priority,
            data.contact_force_total_during_wipeout,
            data.slide_force_total_during_wipeout,
            data.time_in_wipeout,
            data.skater_contact,
            data.vehicle_contact,
            data.pedestrian_contact,
        ]
        .map(f32::to_bits),
        [0; 9]
    );
    assert_eq!(
        [
            data.unrideable_contact,
            data.non_compliant_just_collided,
            data.compliant_just_collided,
            data.compliant_active,
            data.hit_vehicle,
            data.hit_world,
            data.is_wiping_out,
            data.opposing_contacts,
            data.opposing_contacts_on_plane,
            data.is_proxy_in_collision,
            data.is_hook_to_hips_valid,
            data.foot_hit_skateboard,
        ],
        [0; 12]
    );
    assert_eq!(data.num_normals, 0);
    assert_eq!(
        data.most_vertical_contact_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0, 0x3f80_0000, 0, 0]
    );
    assert_eq!(
        data.most_vertical_toe_contact_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0, 0x3f80_0000, 0, 0]
    );
}
