use super::*;
use crate::{
    actor_animation_tree::attributes::SettableTreeAttribute,
    actor_animation_tree::{AnimationClipRef, ClipNode},
    actor_face_playback::FaceAnimationManagers,
    play_animation::{FastString30, FastString36},
};
fn clip(pool: &mut AnimationTreePool, length: f32) -> H {
    let database = FaceAnimationManagers::new()
        .database
        .lock()
        .unwrap()
        .allocate()
        .unwrap();
    let mut c = ClipNode::new();
    c.header = Some(AnimationClipRef {
        database,
        record_index: 0,
    });
    c.length = length;
    c.speed = 1.0;
    c.frames = length * 30.0 + 1.0;
    pool.push(80, Kind::Clip(c)).unwrap()
}
fn sequence() -> (AnimationTreePool, H, Vec<H>) {
    let mut pool = AnimationTreePool::new(16384, 100);
    let seq = pool.push_sequence(false, 3).unwrap();
    let mut clips = vec![];
    for i in 0..3 {
        let c = clip(&mut pool, 0.2);
        pool.add_sequence_child(&seq, c.clone(), i).unwrap();
        clips.push(c);
    }
    (pool, seq, clips)
}
#[test]
fn authored_sequence_primes_overlaps_then_advances_using_selected_properties() {
    let (mut pool, seq, clips) = sequence();
    let mut p = AnimationProperties::default();
    pool.add_time(&seq, 0.15, &mut p).unwrap();
    assert_eq!(pool.time(&clips[0]).unwrap(), FRAME + 0.15);
    assert_eq!(
        pool.time(&clips[1]).unwrap(),
        FRAME - (0.2 - (FRAME + 0.15))
    );
    assert!(pool.sequence(&seq).unwrap().transitioning);
    // The overlapped NEXT has a long duration; its local properties must not
    // overwrite selected expiry, which advances to index1 on this call.
    if let Kind::Clip(c) = &mut pool.node_mut(&clips[1]).unwrap().kind {
        c.length = 20.0;
    }
    pool.add_time(&seq, 0.1, &mut p).unwrap();
    assert!(p.expired_or_looped);
    assert_eq!(pool.sequence(&seq).unwrap().selected, Some(1));
    assert_eq!(pool.sequence(&seq).unwrap().next, Some(2));
    assert_eq!(pool.time(&clips[0]).unwrap(), 0.0);
    assert_eq!(pool.time(&clips[2]).unwrap(), 0.0);
    assert!(!pool.sequence(&seq).unwrap().transitioning);
}
#[test]
fn skip_wraps_even_nonlooping_and_negative_time_keeps_prime_prefix() {
    let (mut pool, seq, clips) = sequence();
    pool.sequence_mut(&seq).unwrap().children[1].enabled = false;
    pool.sequence_move_next(&seq).unwrap();
    assert_eq!(pool.sequence(&seq).unwrap().next, Some(2));
    pool.set_sequence_start(&seq, 2).unwrap();
    assert_eq!(pool.sequence(&seq).unwrap().next, Some(0));
    assert!(matches!(
        pool.add_time(&seq, -0.1, &mut AnimationProperties::default()),
        Err(E::NegativeTime)
    ));
    assert!(pool.sequence(&seq).unwrap().initialized);
    assert_eq!(pool.time(&clips[2]).unwrap(), FRAME);
    let before = pool.sequence(&seq).unwrap().speed;
    assert!(matches!(
        pool.set_speed(&seq, f32::NAN),
        Err(E::InvalidSpeed)
    ));
    assert_eq!(pool.sequence(&seq).unwrap().speed, before);
}
#[test]
fn selector_matches_presence_switches_at_same_time_and_preserves_native_transition_assertion() {
    let mut pool = AnimationTreePool::new(16384, 100);
    let select = pool.push_selector().unwrap();
    let a = clip(&mut pool, 1.0);
    let b = clip(&mut pool, 2.0);
    pool.add_selector_child(&select, a.clone(), FastString36::encode("run").unwrap())
        .unwrap();
    pool.add_selector_child(&select, b.clone(), FastString36::encode("jump").unwrap())
        .unwrap();
    pool.set_time(&a, 0.25).unwrap();
    let mut row = SettableTreeAttribute {
        sequence_id: -1,
        name: FastString30::encode("jump").unwrap(),
        value: 0.0,
        normalized: false,
    };
    pool.set_attributes(&select, &[row.clone()]).unwrap();
    assert_eq!(pool.selected_child(&select).unwrap(), b);
    assert_eq!(pool.time(&b).unwrap(), 0.25);
    row.name = FastString30::encode("run").unwrap();
    row.value = 0.1;
    pool.set_attributes(&select, &[row]).unwrap();
    assert!(pool.selector(&select).unwrap().transitioning);
    assert!(matches!(
        pool.add_time(&select, 0.01, &mut AnimationProperties::default()),
        Err(E::ControllerAssertion(_))
    ));
    assert_eq!(pool.time(&b).unwrap(), 0.25);
}

/// Actual mounted OffBoard databases and caller construction-map words.
pub(crate) fn check_authored_run_sequence_corpus(
    playback: &crate::actor_face_playback::FacePlaybackData,
    map: &[(FastString30, FastString30)],
) {
    let mut pool = AnimationTreePool::new(4 * 1024 * 1024, 10000);
    let seq = playback
        .try_construct_animation_tree(
            &mut pool,
            &FastString36::encode("RunSequence").unwrap(),
            map,
        )
        .unwrap()
        .unwrap();
    let n = pool.sequence(&seq).unwrap();
    assert_eq!(n.children.len(), 12);
    assert_eq!(n.window, 7);
    assert!(!n.looping);
    for (i, child) in n.children.iter().enumerate() {
        assert_eq!(child.authored_stage, u32::from(i == 0 || i == 6));
        assert!(child.enabled);
        assert_eq!(pool.selector(&child.animation).unwrap().children.len(), 14);
    }
    assert_eq!(n.next, Some((n.selected.unwrap() + 1) % 12));
    let mut input = vec![];
    for child in &n.children {
        let selector = pool.selector(&child.animation).unwrap();
        let run = selector
            .children
            .iter()
            .find(|c| c.1 == FastString36::encode("run").unwrap())
            .unwrap();
        let Kind::PhaseBlend(phase) = &pool.node(&run.0).unwrap().kind else {
            panic!("authored run PhaseBlend");
        };
        if !input
            .iter()
            .any(|r: &SettableTreeAttribute| r.name == phase.attribute)
        {
            input.push(SettableTreeAttribute {
                sequence_id: -1,
                name: phase.attribute,
                value: 0.0,
                normalized: true,
            });
        }
    }
    input.insert(
        0,
        SettableTreeAttribute {
            sequence_id: -1,
            name: FastString30::encode("run").unwrap(),
            value: 0.0,
            normalized: false,
        },
    );
    pool.set_attributes(&seq, &input).unwrap();
    let mut scratch = crate::simulation_animation_startup::AnimationAttributePool {
        creation_thread: None,
        memory: vec![std::mem::MaybeUninit::uninit(); 0x19000],
        elements: vec![std::mem::MaybeUninit::uninit(); 20],
        element_count: 0,
    };
    let mut rows = vec![];
    pool.get_attributes(&seq, 0, &mut scratch, &mut rows)
        .unwrap();
    let lengths = rows
        .iter()
        .filter(|r| r.name.eq_ignore_ascii_case("SequenceStateLength"))
        .collect::<Vec<_>>();
    assert_eq!(lengths.len(), 7);
    let start = pool.sequence(&seq).unwrap().selected.unwrap();
    assert_eq!(
        lengths.iter().map(|r| r.sequence_id).collect::<Vec<_>>(),
        (0..7)
            .map(|i| i32::try_from((start + i) % 12).unwrap())
            .collect::<Vec<_>>()
    );
    assert!(rows.iter().all(|r| r.sequence_id >= 0));
    assert_eq!(rows[0].name, "SequenceStateRawTime");
    assert_eq!(rows[0].sequence_id, i32::try_from(start).unwrap());
}
