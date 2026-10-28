//! First `SkaterAnim::Initialize`82cb9920 and hand Init calls on the retained Actor.
//! Matrices allocated by `SetPlaybackData`82d6e450 remain unknown until evaluated.
use super::{
    ActorAnimatableBase, ActorAnimationConstruction, ActorAnimationPhase, ActorChannelAnimatable,
    ActorFaceError, AnimationPointerState, AnimationPoolStorage, AnimationSequence,
    AnimationTransitionBlend, AnimationTransitionStore, FastString30,
};
use crate::{
    actor_face_playback::{AnimationPoseRef, FaceDatabase, FacePlaybackData},
    actor_graph_construction::ActorGraphIntents,
    animation_job_owners::{
        AnimationEntryPoint, AnimationJobCode, AnimationJobManager, AnimationJobScheduler,
        DormantAnimationJob,
    },
    simulation_animation_startup::AnimationAttributePool,
};
use std::{
    mem::MaybeUninit,
    sync::{Arc, Mutex},
};

#[cfg(test)]
#[path = "actor_animation_initialization_tests.rs"]
mod tests;

#[path = "../pose/actor_animation_bind_pose.rs"]
pub(crate) mod bind_pose;
#[path = "actor_animation_body_dispatch.rs"]
pub(crate) mod body_dispatch;

pub(crate) trait ActorAnimationInitializeSource {
    type Error;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error>;
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error>;
}
pub(crate) struct ActorAnimationInitializeInputs {
    pub(crate) playback: Option<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) scheduler: Option<Arc<AnimationJobScheduler>>,
    pub(crate) shared_attributes: Option<Arc<Mutex<AnimationAttributePool>>>,
    pub(crate) stance: u32,
    pub(crate) gestures: [u32; 4],
    pub(crate) pro_skater: Option<FastString30>,
}
#[derive(Debug, PartialEq)]
pub(crate) enum ActorAnimationInitializeError<E> {
    ConstructorRequired,
    AlreadyAttempted,
    BodyRequired,
    HandAlreadyAttempted,
    MissingPlayback,
    MissingHierarchy,
    MissingSharedAttributes,
    MissingBindPose,
    Allocation,
    Source(E),
    Animation(ActorFaceError),
}
impl<E> From<ActorFaceError> for ActorAnimationInitializeError<E> {
    fn from(error: ActorFaceError) -> Self {
        Self::Animation(error)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimationInitializeStage {
    PoseQueries,
    Attributes,
    EntryPointAllocation,
    TreeStores,
    Playback,
    BindPose,
    Job,
    Channels,
    BodyReturned,
}

pub(crate) struct OwnedAnimationPool {
    pub(crate) runtime: crate::actor_animation_tree::AnimationTreePool,
    pub(crate) memory: Vec<MaybeUninit<u8>>,
    pub(crate) elements: Vec<MaybeUninit<usize>>,
}

impl OwnedAnimationPool {
    fn allocate<E>(
        bytes: usize,
        elements: usize,
    ) -> Result<Self, ActorAnimationInitializeError<E>> {
        Ok(Self {
            runtime: crate::actor_animation_tree::AnimationTreePool::new(bytes, elements),
            elements: unknown_storage(elements)?,
            memory: unknown_storage(bytes)?,
        })
    }
}

/// The `ChannelBlend` constructor only initializes its inherited evaluated flag.
pub(crate) struct InitializedAnimationChannel {
    pub(crate) runtime: Option<super::channel_runtime::ChannelBlendRuntime>,
    pub(crate) channel_tree: Option<usize>,
    pub(crate) transition_tree: Option<usize>,
    pub(crate) node_evaluated: bool,
    pub(crate) base_animation: AnimationPointerState,
    pub(crate) channel_animation: AnimationPointerState,
    pub(crate) blend_coefficient: Option<f32>,
    pub(crate) influence: Option<f32>,
    pub(crate) blend_in_time: Option<f32>,
    pub(crate) blend_in_length: Option<f32>,
    pub(crate) blend_out_time: Option<f32>,
    pub(crate) blend_out_length: Option<f32>,
    pub(crate) resurrect_time: Option<f32>,
    pub(crate) resurrect_length: Option<f32>,
    pub(crate) channel_flags: Option<u32>,
    pub(crate) transition: AnimationPointerState,
    /// Channel+0x44 target: inline Sequence (+0x48) or TransitionBlend (+0x64).
    pub(crate) transition_kind: Option<super::channel_runtime::transition::ChannelTransitionKind>,
    pub(crate) sequence: AnimationSequence,
    pub(crate) transition_blend: AnimationTransitionBlend,
    pub(crate) transition_to: Option<crate::actor_animation_tree::AnimationNodeHandle>,
    pub(crate) transitioning: bool,
    pub(crate) parent: Option<usize>,
    pub(crate) base: Option<usize>,
    pub(crate) priority: u32,
}
impl InitializedAnimationChannel {
    fn new() -> Self {
        let transitions = AnimationTransitionStore::new();
        Self {
            runtime: None,
            channel_tree: None,
            transition_tree: None,
            node_evaluated: false,
            base_animation: AnimationPointerState::Unwritten,
            channel_animation: AnimationPointerState::Unwritten,
            blend_coefficient: None,
            influence: None,
            blend_in_time: None,
            blend_in_length: None,
            blend_out_time: None,
            blend_out_length: None,
            resurrect_time: None,
            resurrect_length: None,
            channel_flags: None,
            transition: AnimationPointerState::Null,
            transition_kind: None,
            sequence: transitions.sequences[0],
            transition_blend: transitions.blends[0].clone(),
            transition_to: None,
            transitioning: false,
            parent: None,
            base: None,
            priority: 0,
        }
    }
}
pub(crate) struct InitializedAnimationChannels {
    pub(crate) root: Option<usize>,
    pub(crate) channels: Vec<InitializedAnimationChannel>,
    pub(crate) dictionary: super::channel_runtime::dictionary::ChannelDictionary,
    pub(crate) free_channels: Vec<usize>,
    pub(crate) trees: Vec<OwnedAnimationPool>,
    pub(crate) free_trees: Vec<usize>,
}
pub(crate) struct AnimationEntryPointAllocation {
    pub(crate) manager: Option<Arc<AnimationJobManager>>,
    pub(crate) value: Option<AnimationEntryPoint>,
}
pub(crate) struct InitializedAnimationHand {
    pub(crate) playback: Arc<Mutex<FacePlaybackData>>,
    pub(crate) hierarchy: Arc<Mutex<FaceDatabase>>,
    pub(crate) local_pose: bind_pose::sqt_output::OwnedAnimationSqtBuffer,
}
pub(crate) struct ActorAnimationInitialization {
    pub(crate) stage: AnimationInitializeStage,
    pub(crate) bind_pose: Option<AnimationPoseRef>,
    pub(crate) board_backwards: Option<AnimationPoseRef>,
    pub(crate) board_backwards_ik: Option<AnimationPoseRef>,
    pub(crate) attributes: Option<Arc<Mutex<AnimationAttributePool>>>,
    pub(crate) uses_own_attributes: Option<bool>,
    pub(crate) entry_point: Option<Box<AnimationEntryPointAllocation>>,
    pub(crate) tree_pools: Vec<OwnedAnimationPool>,
    pub(crate) playback: Option<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) hierarchy: Option<Arc<Mutex<FaceDatabase>>>,
    pub(crate) global_matrices: bind_pose::OwnedAnimationPoseBuffer,
    pub(crate) local_matrices: bind_pose::OwnedAnimationPoseBuffer,
    pub(crate) scheduler: Option<Arc<AnimationJobScheduler>>,
    pub(crate) job: Option<Arc<Mutex<DormantAnimationJob>>>,
    pub(crate) channels: Option<InitializedAnimationChannels>,
    pub(crate) hands: [Option<InitializedAnimationHand>; 2],
    hand_attempted: [bool; 2],
}
impl ActorAnimationInitialization {
    fn new() -> Self {
        Self {
            stage: AnimationInitializeStage::PoseQueries,
            bind_pose: None,
            board_backwards: None,
            board_backwards_ik: None,
            attributes: None,
            uses_own_attributes: None,
            entry_point: None,
            tree_pools: Vec::new(),
            playback: None,
            hierarchy: None,
            global_matrices: bind_pose::OwnedAnimationPoseBuffer::new(),
            local_matrices: bind_pose::OwnedAnimationPoseBuffer::new(),
            scheduler: None,
            job: None,
            channels: None,
            hands: [None, None],
            hand_attempted: [false; 2],
        }
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// `cPedestrianAnim82e90390`: original shared attributes, three1024-byte
    /// tree stores, two transitions,32 clips, three256-byte channel stores.
    pub(crate) fn try_initialize_pedestrian(
        &mut self,
        playback: Arc<Mutex<FacePlaybackData>>,
        bind_pose: AnimationPoseRef,
        attributes: Arc<Mutex<AnimationAttributePool>>,
    ) -> Result<(), ActorAnimationInitializeError<std::convert::Infallible>> {
        if self.flavor != super::AnimationConstructionFlavor::Pedestrian
            || self.phase != ActorAnimationPhase::ConstructorReturned
        {
            return Err(ActorAnimationInitializeError::ConstructorRequired);
        }
        if self.initialization.is_some() {
            return Err(ActorAnimationInitializeError::AlreadyAttempted);
        }
        self.initialization = Some(ActorAnimationInitialization::new());
        let state = self
            .initialization
            .as_mut()
            .expect("retained pedestrian initialization");
        state.attributes = Some(attributes);
        state.uses_own_attributes = Some(false);
        state.initialize_animatable(
            self.base.as_mut().expect("pedestrian base"),
            &playback,
            Some(bind_pose),
            None,
            true,
        )?;
        state.initialize_channels(self.channel.as_mut().expect("pedestrian channels"), true)?;
        state.stage = AnimationInitializeStage::BodyReturned;
        Ok(())
    }

    pub(crate) fn initialization(&self) -> Option<&ActorAnimationInitialization> {
        self.initialization.as_ref()
    }

    pub(crate) fn try_initialize_body<S: ActorAnimationInitializeSource>(
        &mut self,
        inputs: ActorAnimationInitializeInputs,
        source: &mut S,
    ) -> Result<(), ActorAnimationInitializeError<S::Error>> {
        if self.phase != ActorAnimationPhase::ConstructorReturned {
            return Err(ActorAnimationInitializeError::ConstructorRequired);
        }
        if self.initialization.is_some() {
            return Err(ActorAnimationInitializeError::AlreadyAttempted);
        }
        self.initialization = Some(ActorAnimationInitialization::new());
        let state = self
            .initialization
            .as_mut()
            .expect("retained Initialize occurrence");
        let playback = inputs
            .playback
            .ok_or(ActorAnimationInitializeError::MissingPlayback)?;
        let skater = self.skater.as_mut().expect("constructor skater");
        let rig = {
            let data = playback.lock().map_err(|_| ActorFaceError::Lock)?;
            let rig = data.get_pose(ActorGraphIntents::native_name(b"rig_Tpose"))?;
            state.board_backwards =
                data.get_pose(ActorGraphIntents::native_name(b"board_backwards"))?;
            skater.board_backwards = pointer_for(state.board_backwards.is_some());
            state.board_backwards_ik =
                data.get_pose(ActorGraphIntents::native_name(b"board_backwards_ik"))?;
            skater.board_backwards_ik = pointer_for(state.board_backwards_ik.is_some());
            rig
        };
        state.stage = AnimationInitializeStage::Attributes;
        let own = source
            .read_use_jobs_for_skater_anim_update()
            .map_err(ActorAnimationInitializeError::Source)?;
        state.uses_own_attributes = Some(own);
        if own {
            let attributes = Arc::new(Mutex::new(AnimationAttributePool {
                creation_thread: None,
                memory: Vec::new(),
                elements: Vec::new(),
                element_count: 0,
            }));
            state.attributes = Some(Arc::clone(&attributes));
            skater.attributes_pool.max_size = 0x26400;
            skater.attributes_pool.max_elements = 20;
            let mut pool = attributes.lock().map_err(|_| ActorFaceError::Lock)?;
            pool.elements = unknown_storage(20)?;
            pool.memory = unknown_storage(0x26400)?;
            skater.attributes_pool.elements = AnimationPointerState::Constructed;
            skater.attributes_pool.memory = AnimationPointerState::Constructed;
            skater.attributes_pool.memory_cursor = AnimationPointerState::Constructed;
        } else {
            state.attributes = Some(
                inputs
                    .shared_attributes
                    .ok_or(ActorAnimationInitializeError::MissingSharedAttributes)?,
            );
        }
        state.stage = AnimationInitializeStage::EntryPointAllocation;
        state.entry_point = Some(Box::new(AnimationEntryPointAllocation {
            manager: None,
            value: None,
        }));
        let entry = state
            .entry_point
            .as_mut()
            .expect("retained entry allocation");
        entry.manager = Some(
            source
                .require_job_manager()
                .map_err(ActorAnimationInitializeError::Source)?,
        );
        entry.value = Some(AnimationEntryPoint::new(
            Some(AnimationJobCode::BlendSpace),
            Some(b"AnimBlendSpace"),
        ));
        self.before_face
            .as_mut()
            .expect("constructor suffix")
            .blend_space_entry_point = AnimationPointerState::Constructed;
        let base = self.base.as_mut().expect("constructor base");
        state.initialize_animatable(base, &playback, rig, inputs.scheduler, false)?;
        state.initialize_channels(self.channel.as_mut().expect("constructor channel"), false)?;
        let player = self.player.as_mut().expect("constructor player");
        player.last_stance = inputs.stance;
        player.natural_stance = inputs.stance;
        player.gestures = inputs.gestures;
        if let Some(pro) = inputs.pro_skater {
            player.pro_skater = pro;
        }
        player.flags_known_mask |= 0xc000_0000;
        if inputs.stance == 1 {
            player.relative_stance = 0;
            player.flags_bits &= 0x3fff_ffff;
        } else {
            player.flags_bits |= 0xc000_0000;
            player.relative_stance = u32::from(inputs.stance != 0);
        }
        state.stage = AnimationInitializeStage::BodyReturned;
        Ok(())
    }

    /// References: `InitLeftHand`82c96a48, `InitRightHand`82c96ae0. Each only allocates
    /// local pose storage and resets its clip/flag; no pose is evaluated here.
    pub(crate) fn try_initialize_hand(
        &mut self,
        right: bool,
        playback: Option<Arc<Mutex<FacePlaybackData>>>,
    ) -> Result<(), ActorAnimationInitializeError<std::convert::Infallible>> {
        let state = self
            .initialization
            .as_mut()
            .ok_or(ActorAnimationInitializeError::BodyRequired)?;
        if state.stage != AnimationInitializeStage::BodyReturned {
            return Err(ActorAnimationInitializeError::BodyRequired);
        }
        let index = usize::from(right);
        if state.hand_attempted[index] {
            return Err(ActorAnimationInitializeError::HandAlreadyAttempted);
        }
        state.hand_attempted[index] = true;
        let playback = playback.ok_or(ActorAnimationInitializeError::MissingPlayback)?;
        let hierarchy = Arc::clone(&playback.lock().map_err(|_| ActorFaceError::Lock)?.database);
        let bones = bone_count(&hierarchy)?;
        state.hands[index] = Some(InitializedAnimationHand {
            playback,
            hierarchy,
            local_pose: bind_pose::sqt_output::OwnedAnimationSqtBuffer::new(),
        });
        let skater = self.skater.as_mut().expect("constructor skater");
        skater.hand_and_face_hierarchies[index] = AnimationPointerState::Constructed;
        state.hands[index]
            .as_mut()
            .expect("retained hand")
            .local_pose = bind_pose::sqt_output::OwnedAnimationSqtBuffer::allocate_unknown(bones);
        skater.hand_and_face_poses[index] = AnimationPointerState::Constructed;
        let mask = if right { 0x0100_0000 } else { 0x0200_0000 };
        let player = self.player.as_mut().expect("constructor flags");
        player.flags_bits &= !mask;
        player.flags_known_mask |= mask;
        let suffix = self.before_sprint.as_mut().expect("constructor clips");
        if right {
            suffix.right_hand_clip = AnimationPointerState::Null;
        } else {
            suffix.left_hand_clip = AnimationPointerState::Null;
        }
        Ok(())
    }
}

impl ActorAnimationInitialization {
    fn initialize_animatable<E>(
        &mut self,
        base: &mut ActorAnimatableBase,
        playback: &Arc<Mutex<FacePlaybackData>>,
        rig: Option<AnimationPoseRef>,
        scheduler: Option<Arc<AnimationJobScheduler>>,
        pedestrian: bool,
    ) -> Result<(), ActorAnimationInitializeError<E>> {
        self.stage = AnimationInitializeStage::TreeStores;
        base.attributes_pool = AnimationPointerState::Constructed;
        let (tree_count, tree_bytes, tree_elements) = if pedestrian {
            (3, 1024, 32)
        } else {
            (4, 0x1cc00, 4096)
        };
        for index in 0..tree_count {
            self.tree_pools
                .push(OwnedAnimationPool::allocate(tree_bytes, tree_elements)?);
            base.tree_store.trees[index] = AnimationPoolStorage {
                max_size: tree_bytes as u32,
                max_elements: tree_elements as u32,
                count: 0,
                location: 0,
                elements: AnimationPointerState::Constructed,
                memory: AnimationPointerState::Constructed,
                memory_cursor: AnimationPointerState::Constructed,
            };
        }
        base.tree_store.free_order = [0, 1, 2, 3];
        base.tree_store.max_trees = tree_count;
        base.tree_store.free_count = tree_count;
        base.transition_store = AnimationTransitionStore::new();
        if pedestrian {
            base.transition_store.max_trees = 3;
            base.transition_store.max_transition_blends = 2;
            base.transition_store.free_blend_count = 2;
            base.transition_store.free_sequence_count = 2;
        }
        base.settable_attributes.len = 0;
        base.tree_attributes.len = 0;
        base.construction_map.len = 0;
        base.trajectory = Some([[1.0; 4], [0.0, 0.0, 0.0, 1.0], [0.0; 4]]);
        base.is_playing = false;
        base.expired_or_looped_by_time = -1.0;
        base.expired_or_looped = false;
        base.time_to_expire_or_loop = -1.0;
        base.blend_space_data = Some(Arc::new(Mutex::new(
            crate::actor_animation_tree::blend_space::jobs::BlendSpaceJobBuffer::initialized(),
        )));
        self.stage = AnimationInitializeStage::Playback;
        self.playback = Some(Arc::clone(playback));
        base.playback = AnimationPointerState::Constructed;
        let hierarchy = Arc::clone(&playback.lock().map_err(|_| ActorFaceError::Lock)?.database);
        let bones = bone_count(&hierarchy)?;
        self.hierarchy = Some(Arc::clone(&hierarchy));
        base.bone_count =
            i32::try_from(bones).map_err(|_| ActorAnimationInitializeError::MissingHierarchy)?;
        base.global_hierarchy = AnimationPointerState::Constructed;
        self.global_matrices =
            bind_pose::OwnedAnimationPoseBuffer::allocate_unknown(Arc::clone(&hierarchy), bones)
                .map_err(|_| ActorAnimationInitializeError::Allocation)?;
        base.global_pose = AnimationPointerState::Constructed;
        base.local_hierarchy = AnimationPointerState::Constructed;
        self.local_matrices =
            bind_pose::OwnedAnimationPoseBuffer::allocate_unknown(hierarchy, bones)
                .map_err(|_| ActorAnimationInitializeError::Allocation)?;
        base.local_pose = AnimationPointerState::Constructed;
        self.stage = AnimationInitializeStage::BindPose;
        self.bind_pose = Some(rig.ok_or(ActorAnimationInitializeError::MissingBindPose)?);
        base.bind_pose_header = AnimationPointerState::Constructed;
        base.bind_pose = AnimationPointerState::Constructed;
        base.max_clips_used = Some(if pedestrian { 32 } else { 2048 });
        self.scheduler = scheduler;
        base.scheduler = pointer_for(self.scheduler.is_some());
        self.stage = AnimationInitializeStage::Job;
        if let Some(scheduler) = &self.scheduler {
            self.job = Some(scheduler.create_job());
            base.blend_space_job = AnimationPointerState::Constructed;
            base.blend_space_job_code = AnimationPointerState::Constructed;
        }
        Ok(())
    }

    fn initialize_channels<E>(
        &mut self,
        channel: &mut ActorChannelAnimatable,
        pedestrian: bool,
    ) -> Result<(), ActorAnimationInitializeError<E>> {
        self.stage = AnimationInitializeStage::Channels;
        let (count, bytes) = if pedestrian { (3, 256) } else { (6, 1024) };
        channel.max_channels = Some(count as i32);
        channel.max_channel_tree_size = Some(bytes as i32);
        self.channels = Some(InitializedAnimationChannels {
            root: None,
            channels: Vec::new(),
            dictionary: super::channel_runtime::dictionary::ChannelDictionary::new(),
            free_channels: Vec::new(),
            trees: Vec::new(),
            free_trees: Vec::new(),
        });
        let retained = self.channels.as_mut().expect("retained channels");
        retained
            .channels
            .try_reserve_exact(count)
            .map_err(|_| ActorAnimationInitializeError::Allocation)?;
        retained
            .channels
            .extend((0..count).map(|_| InitializedAnimationChannel::new()));
        channel.channels = AnimationPointerState::Constructed;
        channel.dictionary = AnimationPointerState::Constructed;
        retained
            .free_channels
            .try_reserve_exact(count)
            .map_err(|_| ActorAnimationInitializeError::Allocation)?;
        channel.free_channels = AnimationPointerState::Constructed;
        retained
            .trees
            .try_reserve_exact(count * 2)
            .map_err(|_| ActorAnimationInitializeError::Allocation)?;
        retained
            .trees
            .extend((0..count * 2).map(|_| OwnedAnimationPool {
                runtime: crate::actor_animation_tree::AnimationTreePool::new(0, 0),
                memory: Vec::new(),
                elements: Vec::new(),
            }));
        channel.trees = AnimationPointerState::Constructed;
        retained
            .free_trees
            .try_reserve_exact(count * 2)
            .map_err(|_| ActorAnimationInitializeError::Allocation)?;
        channel.free_trees = AnimationPointerState::Constructed;
        retained.free_channels.extend(0..count);
        for index in 0..count * 2 {
            retained.free_trees.push(index);
            retained.trees[index] = OwnedAnimationPool::allocate(bytes, 20)?;
        }
        Ok(())
    }
}

fn pointer_for(present: bool) -> AnimationPointerState {
    if present {
        AnimationPointerState::Constructed
    } else {
        AnimationPointerState::Null
    }
}
fn bone_count<E>(
    database: &Arc<Mutex<FaceDatabase>>,
) -> Result<usize, ActorAnimationInitializeError<E>> {
    let database = database.lock().map_err(|_| ActorFaceError::Lock)?;
    Ok(database
        .parsed
        .as_ref()
        .ok_or(ActorAnimationInitializeError::MissingHierarchy)?
        .hierarchy
        .bone_names
        .len())
}
fn unknown_storage<T, E>(
    count: usize,
) -> Result<Vec<MaybeUninit<T>>, ActorAnimationInitializeError<E>> {
    let mut storage = Vec::new();
    storage
        .try_reserve_exact(count)
        .map_err(|_| ActorAnimationInitializeError::Allocation)?;
    storage.resize_with(count, MaybeUninit::uninit);
    Ok(storage)
}
