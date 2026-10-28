//! `PlaybackData::GetAnimTree82d7e468` and `DataBase::GetAnimTree82d7def0`.
use super::{FaceDatabase, FacePlaybackData};
use crate::{
    actor_animation_tree::{
        AnimationClipRef, AnimationNodeHandle, AnimationTreeError, AnimationTreePool,
    },
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, Mutex};

#[path = "../runtime/actor_animation_clip_fetch.rs"]
pub(crate) mod clip_fetch;

impl FacePlaybackData {
    /// Search actual retained database order; recurse within the winning database.
    pub(crate) fn try_construct_animation_tree(
        &self,
        pool: &mut AnimationTreePool,
        name: &FastString36,
        construction_map: &[(FastString30, FastString30)],
    ) -> Result<Option<AnimationNodeHandle>, AnimationTreeError> {
        for database in &self.databases {
            if let Some(node) = construct_database_tree(database, pool, name, construction_map)? {
                return Ok(Some(node));
            }
        }
        Ok(None)
    }
}

enum Descriptor {
    Clip(usize),
    PhaseBlend {
        attribute: FastString30,
        children: Vec<FastString36>,
    },
    BlendSpace {
        index: usize,
        data: Arc<skate2_formats::andale::AndaleBlendSpace>,
        children: Vec<FastString36>,
    },
    Sequence {
        looping: bool,
        window: u8,
        children: Vec<(FastString36, u32)>,
    },
    Selector {
        children: Vec<(FastString36, FastString36)>,
    },
    ConstructionChoice(FastString36),
    Unsupported(u32),
}
#[expect(
    clippy::too_many_lines,
    reason = "preserve database lookup and recursive pool mutation order"
)]
fn construct_database_tree(
    database: &Arc<Mutex<FaceDatabase>>,
    pool: &mut AnimationTreePool,
    name: &FastString36,
    construction_map: &[(FastString30, FastString30)],
) -> Result<Option<AnimationNodeHandle>, AnimationTreeError> {
    use AnimationTreeError as E;
    let descriptor = {
        let retained = database.lock().map_err(|_| E::Lock)?;
        let parsed = retained.parsed.as_ref().ok_or(E::MissingRecord)?;
        // Native mAnimations is shared by Clip and controller records. Last
        // same-name insertion wins across those families, before lookup.
        let Some((index, record)) = parsed.records.iter().enumerate().rev().find(|(_, record)| {
            record.header.name == *name && matches!(record.header.data_type, 2 | 6..=10)
        }) else {
            return Ok(None);
        };
        match record.header.data_type {
            2 => Descriptor::Clip(index),
            6 => {
                let data = parsed
                    .controllers
                    .iter()
                    .find_map(|controller| match controller {
                        skate2_formats::andale::AndaleAnimationController::BlendSpace(data)
                            if data.record_offset == record.offset =>
                        {
                            Some(Arc::new(data.clone()))
                        }
                        _ => None,
                    })
                    .ok_or(E::MissingRecord)?;
                // Preserve exact packed child words; decoded parser names are display data.
                let bytes = retained.bytes.as_deref().ok_or(E::MissingRecord)?;
                let payload = bytes
                    .get(record.data_range.clone())
                    .ok_or(E::MissingRecord)?;
                let count =
                    usize::try_from(read_word(payload, 0)?).map_err(|_| E::MissingRecord)?;
                let names =
                    usize::try_from(read_word(payload, 4)?).map_err(|_| E::MissingRecord)?;
                let mut children = Vec::new();
                children
                    .try_reserve_exact(count)
                    .map_err(|_| E::Allocation)?;
                for index in 0..count {
                    let offset = index
                        .checked_mul(24)
                        .and_then(|v| names.checked_add(v))
                        .ok_or(E::MissingRecord)?;
                    children.push(FastString36 {
                        words: read_words::<6>(payload, offset)?,
                    });
                }
                Descriptor::BlendSpace {
                    index,
                    data,
                    children,
                }
            }
            7 => {
                let bytes = retained.bytes.as_deref().ok_or(E::MissingRecord)?;
                let payload = bytes
                    .get(record.data_range.clone())
                    .ok_or(E::MissingRecord)?;
                let count =
                    usize::try_from(read_word(payload, 8)?).map_err(|_| E::MissingRecord)?;
                let attribute = FastString30 {
                    words: read_words::<5>(payload, 16)?,
                };
                let mut children = Vec::new();
                children
                    .try_reserve_exact(count)
                    .map_err(|_| E::Allocation)?;
                for index in 0..count {
                    let offset = index
                        .checked_mul(24)
                        .and_then(|value| value.checked_add(36))
                        .ok_or(E::MissingRecord)?;
                    children.push(FastString36 {
                        words: read_words::<6>(payload, offset)?,
                    });
                }
                Descriptor::PhaseBlend {
                    attribute,
                    children,
                }
            }
            8 => {
                let bytes = retained.bytes.as_deref().ok_or(E::MissingRecord)?;
                let payload = bytes
                    .get(record.data_range.clone())
                    .ok_or(E::MissingRecord)?;
                Descriptor::ConstructionChoice(construction_choice(payload, construction_map)?)
            }
            9 | 10 => {
                let bytes = retained.bytes.as_deref().ok_or(E::MissingRecord)?;
                let payload = bytes
                    .get(record.data_range.clone())
                    .ok_or(E::MissingRecord)?;
                if record.header.data_type == 9 {
                    let header = payload.get(..4).ok_or(E::MissingRecord)?;
                    let count = usize::from(u16::from_be_bytes([header[0], header[1]]));
                    let mut children = Vec::new();
                    children
                        .try_reserve_exact(count)
                        .map_err(|_| E::Allocation)?;
                    for i in 0..count {
                        let offset = 4 + i * 28;
                        children.push((
                            FastString36 {
                                words: read_words::<6>(payload, offset)?,
                            },
                            read_word(payload, offset + 24)?,
                        ));
                    }
                    Descriptor::Sequence {
                        looping: header[2] != 0,
                        window: header[3],
                        children,
                    }
                } else {
                    let count =
                        usize::try_from(read_word(payload, 0)?).map_err(|_| E::MissingRecord)?;
                    let mut children = Vec::new();
                    children
                        .try_reserve_exact(count)
                        .map_err(|_| E::Allocation)?;
                    for i in 0..count {
                        let offset = 4 + i * 44;
                        let words = read_words::<5>(payload, offset + 24)?;
                        children.push((
                            FastString36 {
                                words: read_words::<6>(payload, offset)?,
                            },
                            FastString36 {
                                words: [words[0], words[1], words[2], words[3], words[4], 0],
                            },
                        ));
                    }
                    Descriptor::Selector { children }
                }
            }
            kind => Descriptor::Unsupported(kind),
        }
    };
    match descriptor {
        Descriptor::Clip(record_index) => {
            let node = pool.push_clip()?;
            pool.initialize_clip(
                &node,
                AnimationClipRef {
                    database: Arc::clone(database),
                    record_index,
                },
                1.0,
                0.0,
            )?;
            Ok(Some(node))
        }
        Descriptor::PhaseBlend {
            attribute,
            children,
        } => {
            let node = pool.push_phase_blend(attribute)?;
            for name in children {
                let Some(child) = construct_database_tree(database, pool, &name, construction_map)?
                else {
                    // Source pops only the latest node, including after earlier
                    // children were appended; do not invent full rollback.
                    pool.pop()?;
                    return Ok(None);
                };
                pool.add_phase_child(&node, &child)?;
            }
            Ok(Some(node))
        }
        Descriptor::BlendSpace {
            index,
            data,
            children,
        } => {
            let node = pool.push_blend_space()?;
            pool.initialize_blend_space(
                &node,
                AnimationClipRef {
                    database: Arc::clone(database),
                    record_index: index,
                },
                &data,
            )?;
            for (index, name) in children.iter().enumerate() {
                let child = construct_database_tree(database, pool, name, construction_map)?;
                pool.set_blend_space_child(&node, index, child)?;
            }
            Ok(Some(node))
        }
        Descriptor::ConstructionChoice(name) => {
            construct_database_tree(database, pool, &name, construction_map)
        }
        Descriptor::Sequence {
            looping,
            window,
            children,
        } => {
            let node = pool.push_sequence(looping, window)?;
            for (name, stage) in children {
                let child = construct_database_tree(database, pool, &name, construction_map)?
                    .ok_or(E::MissingChild)?;
                pool.add_sequence_child(&node, child, stage)?;
            }
            let key = FastString30::encode("StartSequenceID").expect("native literal");
            if let Some((_, value)) = construction_map.iter().find(|(name, _)| *name == key) {
                let value = value.decode().map_err(|_| E::MissingRecord)?;
                let index = source_atoi(value.as_bytes());
                pool.set_sequence_start(
                    &node,
                    usize::try_from(index).map_err(|_| E::MissingChild)?,
                )?;
            }
            Ok(Some(node))
        }
        Descriptor::Selector { children } => {
            let node = pool.push_selector()?;
            for (name, attribute) in children {
                let child = construct_database_tree(database, pool, &name, construction_map)?
                    .ok_or(E::MissingChild)?;
                pool.add_selector_child(&node, child, attribute)?;
            }
            let key = FastString30::encode("StartSelected").expect("native literal");
            if let Some((_, value)) = construction_map.iter().find(|(name, _)| *name == key) {
                let words = value.words;
                pool.set_selector_selected(
                    &node,
                    FastString36 {
                        words: [words[0], words[1], words[2], words[3], words[4], 0],
                    },
                )?;
            }
            Ok(Some(node))
        }
        Descriptor::Unsupported(kind) => Err(E::UnsupportedRecord(kind)),
    }
}

/// `DataBase::GetAnimTree`82d7e100..e21c, type8. Missing key/value selects
/// child0; the first equal packed value selects its SAME index, with no +1.
fn construction_choice(
    payload: &[u8],
    construction_map: &[(FastString30, FastString30)],
) -> Result<FastString36, AnimationTreeError> {
    use AnimationTreeError as E;
    let key = FastString30 {
        words: read_words::<5>(payload, 0)?,
    };
    let names = usize::try_from(read_word(payload, 20)?).map_err(|_| E::MissingRecord)?;
    let values = usize::try_from(read_word(payload, 24)?).map_err(|_| E::MissingRecord)?;
    let count = usize::try_from(read_word(payload, 28)?).map_err(|_| E::MissingRecord)?;
    let mut selected = 0;
    if let Some((_, wanted)) = construction_map.iter().find(|(name, _)| *name == key) {
        for index in 0..count {
            let offset = index
                .checked_mul(20)
                .and_then(|n| values.checked_add(n))
                .ok_or(E::MissingRecord)?;
            let value = FastString30 {
                words: read_words::<5>(payload, offset)?,
            };
            if value == *wanted {
                selected = index;
                break;
            }
        }
    }
    let offset = selected
        .checked_mul(24)
        .and_then(|n| names.checked_add(n))
        .ok_or(E::MissingRecord)?;
    Ok(FastString36 {
        words: read_words::<6>(payload, offset)?,
    })
}

fn read_word(bytes: &[u8], offset: usize) -> Result<u32, AnimationTreeError> {
    let end = offset
        .checked_add(4)
        .ok_or(AnimationTreeError::MissingRecord)?;
    let word = bytes
        .get(offset..end)
        .ok_or(AnimationTreeError::MissingRecord)?;
    Ok(u32::from_be_bytes(
        word.try_into()
            .map_err(|_| AnimationTreeError::MissingRecord)?,
    ))
}
fn read_words<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u32; N], AnimationTreeError> {
    let mut words = [0; N];
    for (index, word) in words.iter_mut().enumerate() {
        *word = read_word(
            bytes,
            offset
                .checked_add(index * 4)
                .ok_or(AnimationTreeError::MissingRecord)?,
        )?;
    }
    Ok(words)
}

#[cfg(test)]
#[test]
fn construction_choice_keeps_default_zero_first_match_and_same_child_index() {
    let key = FastString30::encode("ProSkater").unwrap();
    let values = ["Default", "TonyHawk", "TonyHawk"].map(|s| FastString30::encode(s).unwrap());
    let names =
        ["default_clip", "hawk_clip", "duplicate_clip"].map(|s| FastString36::encode(s).unwrap());
    let mut bytes = vec![0; 32 + 3 * 24 + 3 * 20];
    let mut put = |offset: usize, words: &[u32]| {
        for (i, word) in words.iter().enumerate() {
            bytes[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
    };
    put(0, &key.words);
    put(20, &[32, 104, 3]);
    for (index, name) in names.iter().enumerate() {
        put(32 + index * 24, &name.words);
    }
    for (index, value) in values.iter().enumerate() {
        put(104 + index * 20, &value.words);
    }
    assert_eq!(construction_choice(&bytes, &[]).unwrap(), names[0]);
    assert_eq!(
        construction_choice(&bytes, &[(key, values[1])]).unwrap(),
        names[1]
    );
    assert_eq!(
        construction_choice(&bytes, &[(key, FastString30::encode("Absent").unwrap())]).unwrap(),
        names[0]
    );
    // A missing key must not walk the serialized option values at all.
    bytes[24..28].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(construction_choice(&bytes, &[]).unwrap(), names[0]);
    assert!(matches!(
        construction_choice(&bytes, &[(key, values[1])]),
        Err(AnimationTreeError::MissingRecord)
    ));
}

#[cfg(test)]
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_gesture_phase_trees_retain_both_clip_owners() {
    use crate::actor_animation_tree::AnimationNodeKind;
    let startup = crate::simulation_animation_startup::corpus::load_original_startup();
    let database = &startup.databases[1];
    let names: Vec<_> = {
        let retained = database.lock().unwrap();
        retained
            .parsed
            .as_ref()
            .unwrap()
            .records
            .iter()
            .filter(|record| {
                record.header.data_type == 7
                    && record.header.name.decode().unwrap().starts_with("b_gstr_")
            })
            .map(|record| record.header.name)
            .collect()
    };
    assert_eq!(names.len(), 105);
    for name in names {
        // These are the actual Initialize-time channel tree limits.
        let mut pool = AnimationTreePool::new(1024, 20);
        // Clip/PhaseBlend forward an actual nonempty construction map; neither
        // source case reads it. The type8 case is its conditional consumer.
        let construction_map = [(
            FastString30::encode("ProSkater").unwrap(),
            FastString30::encode("Default").unwrap(),
        )];
        let root = construct_database_tree(database, &mut pool, &name, &construction_map)
            .unwrap()
            .unwrap();
        let AnimationNodeKind::PhaseBlend(phase) = &pool.node(&root).unwrap().kind else {
            panic!("original gesture phase");
        };
        assert_eq!(phase.children.len(), 2);
        let children: Vec<_> = phase
            .children
            .iter()
            .map(|(_, handle)| handle.clone())
            .collect();
        for child in &children {
            let AnimationNodeKind::Clip(clip) = &pool.node(child).unwrap().kind else {
                panic!("original gesture clip");
            };
            assert!(Arc::ptr_eq(
                &clip.header.as_ref().unwrap().database,
                database
            ));
            assert_eq!(clip.speed.to_bits(), 1.0_f32.to_bits());
            assert_eq!(clip.time.to_bits(), 0.0_f32.to_bits());
            assert!(!pool.node(child).unwrap().evaluated);
        }
        // NewChannel calls SetSpeed even when Init already used speed1.
        pool.set_speed(&root, 1.0).unwrap();
        assert!(pool.length(&root).unwrap() > 0.0);
        for child in &children {
            let AnimationNodeKind::Clip(clip) = &pool.node(child).unwrap().kind else {
                unreachable!()
            };
            assert_eq!(clip.time.to_bits(), 0.0_f32.to_bits());
        }
    }
}

// ConvertAToI82bc3ce0 skips arbitrary nondigit bytes until '-' or a digit.
fn source_atoi(bytes: &[u8]) -> i32 {
    let mut i = 0;
    while bytes
        .get(i)
        .is_some_and(|v| *v != 0 && *v != b'-' && !v.is_ascii_digit())
    {
        i += 1;
    }
    let negative = bytes.get(i) == Some(&b'-');
    if negative {
        i += 1;
    }
    let mut value = 0_i32;
    while let Some(&digit) = bytes.get(i) {
        if !digit.is_ascii_digit() {
            break;
        }
        value = value.wrapping_mul(10).wrapping_add(i32::from(digit - b'0'));
        i += 1;
    }
    if negative {
        value.wrapping_neg()
    } else {
        value
    }
}
