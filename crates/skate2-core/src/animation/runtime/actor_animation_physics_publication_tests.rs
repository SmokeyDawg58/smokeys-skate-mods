use super::*;
use crate::actor_graph_construction::ActorGraphIntents;

fn output() -> AnimOutPhysInOwner {
    let mut owner = AnimOutPhysInOwner::new_sim_allocation();
    owner.try_construct(0).unwrap();
    owner
}
fn scalar(name: &str, value: f32) -> PhysicsAnimationAttribute {
    PhysicsAnimationAttribute::motion(&ActorMgAttribute {
        name: FastString30::encode(name).unwrap(),
        value,
    })
}

#[test]
fn attribute_assignment_retains_unused_payload_lanes_and_unknown_sequence() {
    let mut owner = output();
    owner.append_attribute(scalar("Wipeout", -0.0)).unwrap();
    let first = owner.attribute(0).unwrap();
    assert_eq!(first.payload[..4], (-0.0_f32).to_be_bytes().map(Some));
    assert_eq!(first.payload[4..], [None; 20]);
    assert_eq!((first.kind, first.flags, first.sequence), (0, 6, Some(-1)));
    assert_eq!((first.begin, first.end), (-1.0, -1.0));
    let contact = AndaleRuntimeAttribute {
        sequence_id: -1,
        begin_seconds: 1.0,
        end_seconds: 2.0,
        name: "contact".into(),
        status: 9,
        value: AndaleAttributeValue::Bytes24([0xa7; 24]),
    };
    owner.attribute_count = 0;
    owner
        .append_attribute(PhysicsAnimationAttribute::animation(&contact).unwrap())
        .unwrap();
    assert_eq!(owner.attribute(0).unwrap().sequence, None);
    owner.attribute_count = 0;
    owner
        .append_attribute(scalar("Wipeout", f32::from_bits(0x7fc0_0042)))
        .unwrap();
    let scalar = owner.attribute(0).unwrap();
    assert_eq!(scalar.payload[..4], 0x7fc0_0042_u32.to_be_bytes().map(Some));
    assert_eq!(scalar.payload[4..], [Some(0xa7); 20]);
    assert_eq!(scalar.sequence, Some(-1));
    assert!(owner.attribute(1).is_none());
}

#[test]
fn vector_and_unknown_type_copy_only_source_assigned_payload_bytes() {
    let mut owner = output();
    let mut attribute = scalar("test", 0.0);
    attribute.kind = 3;
    attribute.payload = [Some(0x41); 24];
    owner.append_attribute(attribute).unwrap();
    owner.attribute_count = 0;
    let vector = AndaleRuntimeAttribute {
        sequence_id: -1,
        begin_seconds: 0.0,
        end_seconds: 0.0,
        name: "vec".into(),
        status: 1,
        value: AndaleAttributeValue::Vector([1.0, -0.0, 3.0, 4.0]),
    };
    owner
        .append_attribute(PhysicsAnimationAttribute::animation(&vector).unwrap())
        .unwrap();
    assert_eq!(owner.attribute(0).unwrap().payload[16..], [Some(0x41); 8]);
    let before = owner.attribute(0).unwrap().payload;
    owner.attribute_count = 0;
    let mut unknown = vector;
    unknown.value = AndaleAttributeValue::Unsupported {
        attribute_type: 255,
        bytes: vec![9; 24],
    };
    owner
        .append_attribute(PhysicsAnimationAttribute::animation(&unknown).unwrap())
        .unwrap();
    assert_eq!(owner.attribute(0).unwrap().kind, -1);
    assert_eq!(owner.attribute(0).unwrap().payload, before);
}

#[test]
fn capacity_failure_preserves_exact_successful_append_prefix() {
    let mut owner = output();
    for index in 0..120 {
        owner
            .append_attribute(scalar("Wipeout", f32::from_bits(index)))
            .unwrap();
    }
    assert!(matches!(
        owner.append_attribute(scalar("other", 1.0)),
        Err(AnimationPhysicsPublicationError::AttributeCapacity)
    ));
    assert_eq!(owner.attribute_count, 120);
    assert_eq!(
        owner.attribute(119).unwrap().payload[..4],
        119_u32.to_be_bytes().map(Some)
    );
    assert!(owner.attribute(120).is_none());
}

#[test]
fn actual_motion_owner_drains_payloads_and_reinserts_collision_chains() {
    let (actor, _route) = crate::actor_construction::tests::actor_with_listener();
    actor.try_construct_graphs().unwrap();
    let mut prefix = actor.prefix().unwrap();
    let motion = prefix.graphs.as_mut().unwrap().motion_mut().unwrap();
    let state = motion.state.as_mut().unwrap();
    state.trick_name = ActorGraphIntents::native_name(b"Kickflip");
    state.trick_gesture = ActorGraphIntents::native_name(b"Ollie");
    state.trick_bits = 0xf102_8345;
    state.trick_tweak = [-0.0, f32::from_bits(0x7fc0_1234)];
    state.gesture = 4;
    state.is_gesturing = true;
    state.wipeout = [-0.0, 0.5];
    // Distinct raw names with equal wrapping hash deliberately share a bucket.
    let a = crate::play_animation::FastString36 {
        words: [1, 0, 0, 0, 0, 0],
    };
    let b = crate::play_animation::FastString36 {
        words: [0, 1, 0, 0, 0, 0],
    };
    let intents = motion.intents.as_mut().unwrap();
    intents.insert_unique(a, 3.0).unwrap();
    intents.insert_unique(b, -0.0).unwrap();
    assert_eq!(
        intents.iter().map(|entry| entry.name).collect::<Vec<_>>(),
        [b, a]
    );
    let mut owner = output();
    owner.append_attribute(scalar("stale", 9.0)).unwrap();
    publish_motion(motion, &mut owner).unwrap();
    assert_eq!(
        owner.trick.name,
        ActorGraphIntents::native_name(b"Kickflip")
    );
    assert_eq!(owner.trick.bits, 0xf102_8345);
    assert_eq!(owner.trick.tweak_x.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(owner.trick.tweak_y.to_bits(), 0x7fc0_1234);
    assert_eq!((owner.gesture, owner.gesturing), (4, true));
    assert_eq!(
        owner.wipeout_gesture.map(f32::to_bits),
        [(-0.0_f32).to_bits(), 0.5_f32.to_bits()]
    );
    assert_eq!(
        owner
            .intents
            .as_ref()
            .unwrap()
            .iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>(),
        [a, b]
    );
    assert_eq!(
        owner.intents.as_ref().unwrap().get(&b).unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(owner.attribute_count, 0);
    let state = motion.state.as_ref().unwrap();
    assert_eq!(state.trick_bits, 0);
    assert_eq!((state.gesture, state.is_gesturing), (35, false));
    assert_eq!(state.wipeout.map(f32::to_bits), [0; 2]);
    publish_motion(motion, &mut owner).unwrap();
    assert_eq!(
        owner.trick,
        crate::physical_player_phys_out::trick::AnimOutTrickData::reset_values()
    );
    assert_eq!((owner.gesture, owner.gesturing), (35, false));
}
