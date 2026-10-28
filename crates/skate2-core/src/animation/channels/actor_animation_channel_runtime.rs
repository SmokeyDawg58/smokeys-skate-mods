//! Owned gesture `NewChannel82d7eb98` and `SkaterAnim::AddBindPose82c96b78`.
use super::{ActorAnimationConstruction, AnimationPointerState};
use crate::{
    actor_animation_tree::{AnimationNodeHandle, AnimationTreeError, AnimationTreePool},
    actor_face_playback::{AnimationPoseRef, FaceDatabase},
    play_animation::FastString36,
};
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[path = "actor_animation_channel_blends_tests.rs"]
pub(crate) mod blend_tests;
#[path = "actor_animation_channel_dictionary.rs"]
pub(crate) mod dictionary;
#[path = "actor_animation_channel_time.rs"]
pub(crate) mod time;
#[path = "actor_animation_channel_transition.rs"]
pub(crate) mod transition;

#[derive(Clone)]
pub(crate) enum ChannelBaseReference {
    /// The actual Animatable base pointer is NULL, not an evaluated bind pose.
    Empty,
    Channel(usize),
    Body(super::blend_anim::BodyAnimationReference),
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "seven independently stored source ChannelBlend flags"
)]
pub(crate) struct ChannelBlendRuntime {
    pub(crate) base: ChannelBaseReference,
    pub(crate) animation: AnimationNodeHandle,
    pub(crate) cycle: bool,
    pub(crate) blend_in_first_frame: bool,
    pub(crate) blend_out_last_frame: bool,
    pub(crate) blend_out_early: bool,
    pub(crate) blending_out: bool,
    pub(crate) expired: bool,
    pub(crate) use_attributes: bool,
}
#[derive(Debug)]
pub(crate) enum GestureChannelError {
    MissingOwner(&'static str),
    UnknownFlags(u32),
    InvalidChannel,
    ChannelCapacity,
    Lock,
    Tree(AnimationTreeError),
}
impl std::fmt::Display for GestureChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOwner(name) => write!(f, "channel requires {name}"),
            Self::UnknownFlags(mask) => write!(f, "channel source flags are unwritten: {mask:08x}"),
            Self::Tree(error) => write!(f, "channel tree: {error}"),
            other => write!(f, "channel: {other:?}"),
        }
    }
}
/// Native `tChannelParams` (0x1c bytes), passed by value after the names.
#[derive(Clone, Copy, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independently passed native tChannelParams flags"
)]
pub(crate) struct ChannelParams {
    pub(crate) priority: i32,
    pub(crate) cycle: bool,
    pub(crate) mirror: bool,
    pub(crate) playback_speed: f32,
    pub(crate) blend_in_length: f32,
    pub(crate) blend_in_to_first_frame: bool,
    pub(crate) blend_out_length: f32,
    pub(crate) blend_out_from_last_frame: bool,
}
/// `Andale::eChannelStatus`. StartTransition also reports ALREADY_EXISTS for
/// an absent channel when it may not create one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NewGestureChannelStatus {
    Created = 0,
    BlendingOut = 1,
    AlreadyPresent = 2,
    NoFreeChannel = 3,
    AnimationMissing = 4,
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Exact gesture params: priority0, noncycle/nonmirror, speed1,
    /// blend lengths0.3, both endpoint flagsfalse, useChannelAttributesfalse.
    pub(crate) fn try_new_gesture_channel(
        &mut self,
        name: FastString36,
        animation_name: FastString36,
    ) -> Result<NewGestureChannelStatus, GestureChannelError> {
        self.try_new_channel_with_blends(name, animation_name, 0.3, 0.3, false)
    }

    /// Shared `NewChannel` route with priority0, noncycle/nonmirror, speed1,
    /// first-frame=false and use-attributes=false. Native Init82d6d500 samples
    /// the supplied lengths and last-frame flag before dictionary publication.
    #[expect(
        clippy::too_many_lines,
        reason = "source channel allocation, initialization and priority publication order"
    )]
    pub(crate) fn try_new_channel_with_blends(
        &mut self,
        name: FastString36,
        animation_name: FastString36,
        blend_in: f32,
        blend_out: f32,
        last_frame: bool,
    ) -> Result<NewGestureChannelStatus, GestureChannelError> {
        self.try_new_channel_with_cycle(
            name,
            animation_name,
            blend_in,
            blend_out,
            last_frame,
            false,
        )
    }

    /// Same source NewChannel pipeline; cycle is a ChannelBlend.Init input.
    pub(crate) fn try_new_channel_with_cycle(
        &mut self,
        name: FastString36,
        animation_name: FastString36,
        blend_in: f32,
        blend_out: f32,
        last_frame: bool,
        cycle: bool,
    ) -> Result<NewGestureChannelStatus, GestureChannelError> {
        self.try_new_channel(
            name,
            animation_name,
            ChannelParams {
                priority: 0,
                cycle,
                mirror: false,
                playback_speed: 1.0,
                blend_in_length: blend_in,
                blend_in_to_first_frame: false,
                blend_out_length: blend_out,
                blend_out_from_last_frame: last_frame,
            },
            false,
        )
    }

    /// `ChannelAnimatable::NewChannel82d7eb98` with its by-value params and
    /// useChannelAttributes (r10).
    #[expect(
        clippy::too_many_lines,
        reason = "source channel allocation, initialization and priority publication order"
    )]
    pub(crate) fn try_new_channel(
        &mut self,
        name: FastString36,
        animation_name: FastString36,
        params: ChannelParams,
        use_attributes: bool,
    ) -> Result<NewGestureChannelStatus, GestureChannelError> {
        use GestureChannelError as E;
        let initialized = self
            .initialization
            .as_mut()
            .ok_or(E::MissingOwner("Initialize"))?;
        let channels = initialized
            .channels
            .as_mut()
            .ok_or(E::MissingOwner("channel stores"))?;
        if channels.dictionary.contains_key(&name) {
            return Ok(NewGestureChannelStatus::AlreadyPresent);
        }
        let Some(&index) = channels.free_channels.last() else {
            return Ok(NewGestureChannelStatus::NoFreeChannel);
        };
        let tree_index = *channels
            .free_trees
            .last()
            .ok_or(E::MissingOwner("free tree"))?;
        channels
            .channels
            .get_mut(index)
            .ok_or(E::InvalidChannel)?
            .channel_tree = Some(tree_index);
        let pool = &mut channels
            .trees
            .get_mut(tree_index)
            .ok_or(E::InvalidChannel)?
            .runtime;
        let playback = initialized
            .playback
            .as_ref()
            .ok_or(E::MissingOwner("PlaybackData"))?;
        let map = &self
            .base
            .as_ref()
            .ok_or(E::MissingOwner("Animatable construction map"))?
            .construction_map;
        let map = map.slots[..map.len]
            .iter()
            .map(|entry| {
                entry
                    .as_ref()
                    .copied()
                    .ok_or(E::MissingOwner("construction map entry"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let node = playback
            .lock()
            .map_err(|_| E::Lock)?
            .try_construct_animation_tree(pool, &animation_name, &map)
            .map_err(E::Tree)?;
        let Some(node) = node else {
            return Ok(NewGestureChannelStatus::AnimationMissing);
        };
        pool.set_speed(&node, params.playback_speed)
            .map_err(E::Tree)?;
        let node = if self.flavor == super::AnimationConstructionFlavor::Pedestrian {
            add_pedestrian_bind_pose(pool, node)?
        } else {
            let player = self
                .player
                .as_mut()
                .ok_or(E::MissingOwner("SkaterAnim flags"))?;
            add_bind_pose(
                pool,
                node,
                initialized.bind_pose.as_ref(),
                initialized.board_backwards.as_ref(),
                initialized.board_backwards_ik.as_ref(),
                initialized.hierarchy.as_ref(),
                &mut player.flags_bits,
                &mut player.flags_known_mask,
            )?
        };
        // 82d7ecb4..ecf4: Mirror pushed in the same channel tree over the
        // AddBindPose result, GetHierarchy, UseTrajectoryWithoutAdjtoChildren.
        let node = if params.mirror {
            pool.push_mirror(initialized.hierarchy.clone(), node, 2)
                .map_err(E::Tree)?
        } else {
            node
        };
        let base = if let Some(root) = channels.root {
            ChannelBaseReference::Channel(root)
        } else {
            let base = self.base.as_ref().ok_or(E::MissingOwner("Animatable"))?;
            if let Some(root) = &base.body_root {
                ChannelBaseReference::Body(root.clone())
            } else {
                if base.base_animation != AnimationPointerState::Null {
                    return Err(E::MissingOwner("runtime body base animation"));
                }
                ChannelBaseReference::Empty
            }
        };
        let channel = channels.channels.get_mut(index).ok_or(E::InvalidChannel)?;
        // ChannelBlend::Init82d6d500 writes all semantic fields; it preserves
        // irrelevant lower25 padding bits rather than assigning them meaning.
        let blending_out = params.blend_out_length > 0.0;
        channel.blend_in_length = Some(params.blend_in_length);
        channel.blend_out_length = Some(params.blend_out_length);
        channel.base_animation = match base {
            ChannelBaseReference::Empty => AnimationPointerState::Null,
            ChannelBaseReference::Channel(_) | ChannelBaseReference::Body(_) => {
                AnimationPointerState::Constructed
            }
        };
        channel.channel_animation = AnimationPointerState::Constructed;
        channel.blend_coefficient = Some(0.0);
        channel.blend_in_time = Some(0.0);
        channel.blend_out_time = Some(0.0);
        channel.resurrect_time = Some(0.0);
        channel.influence = Some(1.0);
        channel.resurrect_length = Some(0.0);
        channel.runtime = Some(ChannelBlendRuntime {
            base,
            animation: node,
            cycle: params.cycle,
            blend_in_first_frame: params.blend_in_to_first_frame,
            blend_out_last_frame: params.blend_out_from_last_frame,
            blend_out_early: false,
            blending_out,
            expired: false,
            use_attributes,
        });
        // Only upper7 contain semantic flags. Consumers use the runtime fields;
        // this mirror exists for source-address diagnostics and EndChannel.
        channel.channel_flags = Some(
            (if params.blend_in_to_first_frame {
                0x8000_0000
            } else {
                0
            }) | (if params.blend_out_from_last_frame {
                0x4000_0000
            } else {
                0
            }) | (if params.cycle { 0x1000_0000 } else { 0 })
                | (if blending_out { 0x0800_0000 } else { 0 })
                | (if use_attributes { 0x0200_0000 } else { 0 }),
        );
        channels.free_channels.pop();
        channels.free_trees.pop();
        channels
            .dictionary
            .try_insert(name, index)
            .map_err(|_| E::ChannelCapacity)?;
        let maximum = self
            .channel
            .as_ref()
            .and_then(|channel| channel.max_channels)
            .and_then(|maximum| usize::try_from(maximum).ok())
            .ok_or(E::MissingOwner("channel maximum"))?;
        if channels.dictionary.len() > maximum {
            return Err(E::ChannelCapacity);
        }
        // 82d7eda4: the params priority word; the list walk compares signed.
        channel.priority = u32::from_ne_bytes(params.priority.to_ne_bytes());
        let priority = |channel: &super::initialization::InitializedAnimationChannel| {
            i32::from_ne_bytes(channel.priority.to_ne_bytes())
        };
        if let Some(mut cursor) = channels.root {
            while priority(&channels.channels[cursor]) > params.priority {
                let Some(next) = channels.channels[cursor].base else {
                    break;
                };
                cursor = next;
            }
            if priority(&channels.channels[cursor]) > params.priority {
                let base = channels.channels[cursor].base;
                channels.channels[index].parent = Some(cursor);
                channels.channels[index].base = base;
                channels.channels[cursor].base = Some(index);
                if let Some(base) = base {
                    channels.channels[base].parent = Some(index);
                }
                return Ok(NewGestureChannelStatus::Created);
            }
            let parent = channels.channels[cursor].parent;
            channels.channels[index].base = Some(cursor);
            channels.channels[index].parent = parent;
            channels.channels[cursor].parent = Some(index);
            if let Some(parent) = parent {
                channels.channels[parent].base = Some(index);
                return Ok(NewGestureChannelStatus::Created);
            }
        }
        channels.root = Some(index);
        self.channel
            .as_mut()
            .ok_or(E::MissingOwner("ChannelAnimatable"))?
            .root_channel = AnimationPointerState::Constructed;
        Ok(NewGestureChannelStatus::Created)
    }
}

/// Tree adjustment always retains bind-pose source data; it does not evaluate
/// or substitute a pose. Board and mirror flags are read in original order.
#[expect(
    clippy::too_many_arguments,
    reason = "actual SkaterAnim owned pose references and source flag word"
)]
pub(super) fn add_bind_pose(
    pool: &mut AnimationTreePool,
    animation: AnimationNodeHandle,
    bind_pose: Option<&AnimationPoseRef>,
    board: Option<&AnimationPoseRef>,
    board_ik: Option<&AnimationPoseRef>,
    hierarchy: Option<&Arc<Mutex<FaceDatabase>>>,
    flags: &mut u32,
    known: &mut u32,
) -> Result<AnimationNodeHandle, GestureChannelError> {
    use GestureChannelError as E;
    let pose = pool.push_pose(bind_pose.cloned()).map_err(E::Tree)?;
    let mut node = pool
        .push_add_sqt(animation.clone(), pose, animation)
        .map_err(E::Tree)?;
    if *known & 0x8000_0000 == 0 {
        return Err(E::UnknownFlags(0x8000_0000));
    }
    if *flags & 0x8000_0000 != 0 {
        let pose = pool.push_pose(board.cloned()).map_err(E::Tree)?;
        let added = pool
            .push_add_sqt(pose, node.clone(), node)
            .map_err(E::Tree)?;
        let ik = pool.push_pose(board_ik.cloned()).map_err(E::Tree)?;
        node = pool
            .push_add_sqt(added.clone(), ik, added)
            .map_err(E::Tree)?;
    }
    if *known & 0x4000_0000 == 0 {
        return Err(E::UnknownFlags(0x4000_0000));
    }
    if *flags & 0x4000_0000 != 0 {
        node = pool
            .push_mirror(hierarchy.cloned(), node, 2)
            .map_err(E::Tree)?;
    }
    if *known & 0x0040_0000 == 0 {
        return Err(E::UnknownFlags(0x0040_0000));
    }
    if *flags & 0x0040_0000 != 0 {
        if *known & 0x0020_0000 == 0 {
            return Err(E::UnknownFlags(0x0020_0000));
        }
        let trajectory_use = if *flags & 0x0020_0000 != 0 { 1 } else { 2 };
        node = pool
            .push_mirror(hierarchy.cloned(), node, trajectory_use)
            .map_err(E::Tree)?;
        *flags &= !0x0040_0000;
        *known |= 0x0040_0000;
    }
    *flags &= !0x0020_0000;
    *known |= 0x0020_0000;
    Ok(node)
}

/// `cPedestrianAnim::AddBindPose82e7f460`: registered fixed bind SQTs are
/// operand B, while animation remains operand A and the motion source.
pub(super) fn add_pedestrian_bind_pose(
    pool: &mut AnimationTreePool,
    animation: AnimationNodeHandle,
) -> Result<AnimationNodeHandle, GestureChannelError> {
    let pose = pool.push_fixed_sqt(0).map_err(GestureChannelError::Tree)?;
    pool.push_add_sqt(animation.clone(), pose, animation)
        .map_err(GestureChannelError::Tree)
}

#[cfg(test)]
#[expect(
    clippy::too_many_lines,
    reason = "one original Actor fixture checks constructor graph ownership and successive channel mutations"
)]
pub(crate) fn check_initialized_actor(actor: &crate::actor_construction::ActorConstruction) {
    use crate::actor_animation_tree::AnimationNodeKind;
    let owner = actor.retained_animation().unwrap().unwrap();
    let mut animation = owner.lock().unwrap();
    let initialized = animation.initialization.as_ref().unwrap();
    let channels = initialized.channels.as_ref().unwrap();
    assert!(channels.dictionary.is_empty());
    assert_eq!(
        (channels.free_channels.len(), channels.free_trees.len()),
        (6, 12)
    );
    let flags = animation.player.as_ref().unwrap().flags_bits;
    let both = FastString36::encode("GestureBoth").unwrap();
    let clip = FastString36::encode("B_GSTR_SKY_BOTH").unwrap();
    assert_eq!(
        animation.try_new_gesture_channel(both, clip).unwrap(),
        NewGestureChannelStatus::Created
    );
    let initialized = animation.initialization.as_ref().unwrap();
    let channels = initialized.channels.as_ref().unwrap();
    assert_eq!(
        (channels.free_channels.len(), channels.free_trees.len()),
        (5, 11)
    );
    let index = *channels.dictionary.get(&both).unwrap();
    assert_eq!(index, 5);
    assert_eq!(channels.root, Some(index));
    let channel = &channels.channels[index];
    assert_eq!(channel.channel_tree, Some(11));
    let runtime = channel.runtime.as_ref().unwrap();
    assert!(matches!(runtime.base, ChannelBaseReference::Empty));
    assert!(!runtime.cycle && !runtime.blend_in_first_frame && !runtime.blend_out_last_frame);
    assert!(runtime.blending_out);
    assert!(!runtime.blend_out_early && !runtime.expired && !runtime.use_attributes);
    assert_eq!(channel.channel_flags, Some(0x0800_0000));
    let pool = &channels.trees[11].runtime;
    let mut pending = vec![runtime.animation.clone()];
    let mut counts = [0; 5];
    while let Some(handle) = pending.pop() {
        let node = pool.node(&handle).unwrap();
        assert!(!node.evaluated);
        match &node.kind {
            AnimationNodeKind::Sequence(_) | AnimationNodeKind::Selector(_) => {
                panic!("gesture fixture unexpectedly selected locomotion controller")
            }
            AnimationNodeKind::BlendSpace(_) => {
                panic!("fixed gesture fixture unexpectedly selected BlendSpace")
            }
            AnimationNodeKind::Clip(clip) => {
                counts[0] += 1;
                assert!(clip.header.is_some());
            }
            AnimationNodeKind::PhaseBlend(phase) => {
                counts[1] += 1;
                pending.extend(phase.children.iter().map(|(_, child)| child.clone()));
            }
            AnimationNodeKind::Pose { header } => {
                counts[2] += 1;
                let header = header.as_ref().unwrap();
                assert!(
                    initialized
                        .playback
                        .as_ref()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .databases
                        .iter()
                        .any(|database| Arc::ptr_eq(database, &header.database))
                );
            }
            AnimationNodeKind::FixedSqt { .. } => {
                panic!("skater gesture fixture unexpectedly selected pedestrian fixed SQTs")
            }
            AnimationNodeKind::AddSqt { a, b, motion } => {
                counts[3] += 1;
                assert!(motion == a || motion == b);
                pending.push(a.clone());
                pending.push(b.clone());
            }
            AnimationNodeKind::Mirror {
                hierarchy,
                child,
                trajectory_use,
            } => {
                counts[4] += 1;
                assert!(Arc::ptr_eq(
                    hierarchy.as_ref().unwrap(),
                    initialized.hierarchy.as_ref().unwrap()
                ));
                assert!(matches!(trajectory_use, Some(1 | 2)));
                pending.push(child.clone().unwrap());
            }
        }
    }
    let pose_count = if flags & 0x8000_0000 == 0 { 1 } else { 3 };
    let mirror_count = i32::from(flags & 0x4000_0000 != 0) + i32::from(flags & 0x0040_0000 != 0);
    assert_eq!(counts, [2, 1, pose_count, pose_count, mirror_count]);
    assert_eq!(
        animation.try_new_gesture_channel(both, clip).unwrap(),
        NewGestureChannelStatus::AlreadyPresent
    );
    animation.try_end_gesture_channel(&both).unwrap();
    let channels = animation
        .initialization
        .as_ref()
        .unwrap()
        .channels
        .as_ref()
        .unwrap();
    assert_eq!(
        (channels.free_channels.len(), channels.free_trees.len()),
        (5, 11)
    );
    assert_eq!(channels.channels[index].channel_flags, Some(0x2800_0000));
    let runtime = channels.channels[index].runtime.as_ref().unwrap();
    assert!(runtime.blending_out && runtime.blend_out_early);
    // A second real channel verifies the equal-priority native root insertion.
    let left = FastString36::encode("GestureLeft").unwrap();
    assert_eq!(
        animation
            .try_new_gesture_channel(left, FastString36::encode("B_GSTR_SKY_LEFT").unwrap())
            .unwrap(),
        NewGestureChannelStatus::Created
    );
    let channels = animation
        .initialization
        .as_ref()
        .unwrap()
        .channels
        .as_ref()
        .unwrap();
    assert_eq!(channels.root, Some(4));
    assert_eq!(channels.channels[4].base, Some(5));
    assert_eq!(channels.channels[5].parent, Some(4));
    assert!(matches!(
        channels.channels[4].runtime.as_ref().unwrap().base,
        ChannelBaseReference::Channel(5)
    ));
    animation.check_play_body_animation();
}
