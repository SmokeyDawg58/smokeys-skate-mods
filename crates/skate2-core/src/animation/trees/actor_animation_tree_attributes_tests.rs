use super::*;

fn scalar(name: &str, value: f32) -> AndaleRuntimeAttribute {
    AndaleRuntimeAttribute {
        sequence_id: -1,
        begin_seconds: -1.0,
        end_seconds: -1.0,
        name: name.into(),
        status: 6,
        value: AndaleAttributeValue::Float(value),
    }
}

#[test]
fn live_clip_window_keeps_speed_scaled_length_and_previous_eval_boundary() {
    let mut clip = ClipNode::new();
    clip.length = 2.0;
    clip.old_time = 0.5;
    clip.time = 1.0;
    let attribute = AndaleAttribute {
        begin: 0.25,
        end: 0.5,
        name: "contact".into(),
        attribute_type: 0,
        value: AndaleAttributeValue::Float(1.0),
    };
    assert_eq!(clip_attribute_status(&clip, &attribute).unwrap(), 5);
    clip.time = 1.25;
    assert_eq!(clip_attribute_status(&clip, &attribute).unwrap(), 9);
    clip.old_time = 1.0;
    assert_eq!(clip_attribute_status(&clip, &attribute).unwrap(), 17);
    clip.loops = 1;
    clip.old_time = 1.75;
    clip.time = 0.75;
    assert_eq!(clip_attribute_status(&clip, &attribute).unwrap(), 5);
    clip.loops = 2;
    assert!(matches!(
        clip_attribute_status(&clip, &attribute),
        Err(TreeAttributeError::InvalidLoop)
    ));
}

#[test]
fn attribute_blend_intersects_names_preserving_left_status_and_contact_bone() {
    let mut left = vec![scalar("a", 2.0), scalar("b", 9.0), scalar("c", 4.0)];
    left.sort_by_key(|attribute| packed_name(attribute).unwrap().words);
    let mut right = vec![scalar("a", 6.0), scalar("c", 8.0)];
    right.sort_by_key(|attribute| packed_name(attribute).unwrap().words);
    let mut output = Vec::new();
    blend_attributes(left, &right, 0.25, &mut output).unwrap();
    assert_eq!(output.len(), 2);
    assert_eq!(
        output[0].scalar_value().unwrap().to_bits(),
        3.0_f32.to_bits()
    );
    assert_eq!(
        output[1].scalar_value().unwrap().to_bits(),
        5.0_f32.to_bits()
    );
    assert_eq!(output[0].begin_seconds.to_bits(), (-1.0_f32).to_bits());
    let mut left = scalar("contact", 0.0);
    let mut right = left.clone();
    let mut left_bytes = [0x11; 24];
    let mut right_bytes = [0x22; 24];
    left_bytes[20..].copy_from_slice(&2.0_f32.to_be_bytes());
    right_bytes[20..].copy_from_slice(&6.0_f32.to_be_bytes());
    left.value = AndaleAttributeValue::Bytes24(left_bytes);
    right.value = AndaleAttributeValue::Bytes24(right_bytes);
    blend_attribute(&mut left, &right, 0.25).unwrap();
    let AndaleAttributeValue::Bytes24(bytes) = left.value else {
        unreachable!()
    };
    assert_eq!(&bytes[..20], &left_bytes[..20]);
    assert_eq!(&bytes[20..], &3.0_f32.to_be_bytes());
}

#[test]
fn mirror_vector_x_flips_bits_without_quieting_nan_or_touching_other_lanes() {
    let mut attribute = scalar("vector", 0.0);
    attribute.value = AndaleAttributeValue::Vector([f32::from_bits(0x7f80_0001), -0.0, 2.0, 7.0]);
    mirror_attribute(&mut attribute, None).unwrap();
    let AndaleAttributeValue::Vector(value) = attribute.value else {
        unreachable!()
    };
    assert_eq!(
        value.map(f32::to_bits),
        [
            0xff80_0001,
            0x8000_0000,
            2.0_f32.to_bits(),
            7.0_f32.to_bits()
        ]
    );
}

#[test]
fn scratch_exact_end_rejects_without_consuming_an_element() {
    let mut pool = AnimationAttributePool {
        creation_thread: None,
        memory: vec![std::mem::MaybeUninit::uninit(); 7712 * 2],
        elements: vec![std::mem::MaybeUninit::uninit(); 2],
        element_count: 0,
    };
    push_scratch(&mut pool).unwrap();
    assert!(matches!(
        push_scratch(&mut pool),
        Err(TreeAttributeError::Capacity)
    ));
    assert_eq!(pool.element_count, 1);
}

#[test]
fn phase_attribute_cache_commits_before_missing_child_header() {
    let mut pool = AnimationTreePool::new(1024, 16);
    let name = FastString30::encode("speed").unwrap();
    let phase = pool.push_phase_blend(name).unwrap();
    let first = pool.push_clip().unwrap();
    let second = pool.push_clip().unwrap();
    pool.add_phase_child(&phase, &first).unwrap();
    pool.add_phase_child(&phase, &second).unwrap();
    assert!(matches!(
        pool.set_attributes(
            &phase,
            &[SettableTreeAttribute {
                name,
                value: 0.25,
                normalized: false,
                sequence_id: -1,
            }]
        ),
        Err(TreeAttributeError::Missing("Clip header"))
    ));
    let AnimationNodeKind::PhaseBlend(state) = &pool.node(&phase).unwrap().kind else {
        unreachable!()
    };
    assert_eq!(state.last_attribute.to_bits(), 0.25_f32.to_bits());
    assert!(!state.last_attribute_normalized);
    assert_eq!(state.child1.as_ref(), Some(&first));
    assert_eq!(state.child2.as_ref(), Some(&second));
}

#[test]
fn live_clip_status_uses_raw_unordered_branch_predicates() {
    let mut clip = ClipNode::new();
    clip.length = 1.0;
    clip.time = 0.5;
    clip.old_time = 0.25;
    let mut attribute = AndaleAttribute {
        begin: 0.0,
        end: f32::NAN,
        name: "window".into(),
        attribute_type: 0,
        value: AndaleAttributeValue::Float(1.0),
    };
    // No-loop NOTGT rejects unordered end versus previous time.
    assert_eq!(clip_attribute_status(&clip, &attribute).unwrap(), 17);
    attribute.end = 0.75;
    clip.old_time = f32::NAN;
    assert_eq!(clip_attribute_status(&clip, &attribute).unwrap(), 17);
    // Loop GT branch treats an unordered begin as crossed, before the loop check.
    clip.loops = 1;
    clip.old_time = 0.9;
    attribute.begin = f32::NAN;
    assert_eq!(clip_attribute_status(&clip, &attribute).unwrap(), 5);
    // Only ordered LT can skip the loop assertion.
    clip.time = f32::NAN;
    assert!(matches!(
        clip_attribute_status(&clip, &attribute),
        Err(TreeAttributeError::InvalidLoop)
    ));
}
