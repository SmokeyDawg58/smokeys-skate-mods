//! Pedestrian animation scheduling, shared by ordinary pedestrians and guards.
//!
//! This is the `BuildAnimTree` scheduling owner, not a replacement motion graph.
//! The caller must execute the original graph and shared Andale channel/tree
//! runtime before publishing a pose. No default walk or bind-pose display is
//! synthesized when those owners are absent.

use crate::{
    actor_animation_construction::{
        ActorAnimationConstruction, frame_phase::AnimationPhaseError,
        initialization::ActorAnimationInitializeError,
    },
    actor_animation_tree::time::AnimationProperties,
    actor_face_playback::{AnimationPoseRef, FacePlaybackData},
    motion_graph_entity_authority::ActorGraphAuthority,
    simulation_animation_startup::AnimationAttributePool,
};
use std::sync::{Arc, Mutex};
#[path = "pedestrian_sim_lod_pose.rs"]
pub(crate) mod sim_lod_pose;
#[path = "pedestrian_animation_pose.rs"]
mod pose_production;


/// Intent portion of tPedMotionGraphInput. Scheduling time remains in the
/// existing PedestrianAnimationSchedule; no second time accumulator is added.
pub(crate) struct PedestrianMotionIntents {
    names: [crate::play_animation::FastString30; 4],
    count: usize,
    pub(crate) valid: bool,
    /// tPedMotionGraphInput+96, initialized by82e867a0.
    pub(crate) cycle_swapping_percentage: f32,
    pub(crate) frame_offset: f32,
    /// Native RegisteredClipBlock/RegisteredTransitionInfo, null in82e86868/6c.
    pub(crate) registered_clips: PedestrianRegisteredClips,
}
impl PedestrianMotionIntents {
    /// tPedMotionGraphInput82e867a0, then cPedAnimData82e8d488 adds Idle.
    fn new() -> Self {
        let mut names = [crate::play_animation::FastString30 { words: [0; 5] }; 4];
        names[0] = crate::play_animation::FastString30::encode("Idle").expect("native name");
        Self { names, count: 1, valid: false, cycle_swapping_percentage: 0.0, frame_offset: 0.,
            registered_clips: PedestrianRegisteredClips::default() }
    }
    pub(crate) fn names(&self) -> &[crate::play_animation::FastString30] {
        &self.names[..self.count]
    }
    /// CompleteLocomotionTransition/Turn82e6e030/48 write only IntentCount.
    pub(crate) fn retain_first(&mut self) { self.count = 1; }
    /// SetMotionGraphFromIntentPacket82e87740 copies only the active prefix.
    /// It does NOT set IntentsValid; IntentGen::Update does that separately.
    pub(crate) fn set_packet(&mut self, stage: &crate::pedestrian_intent_packet::IntentStage) {
        self.count = stage.intents().len();
        self.names[..self.count].copy_from_slice(stage.intents());
    }
}

/// Native cPedestrianAnim on the SAME shared Andale tree/channel owner. Generic
/// world is the actual retained physical world; no skater interface is created.
pub(crate) struct PedestrianAnimation<K, W> {
    pub(crate) core: ActorAnimationConstruction<K, W>,
    pub(crate) schedule: PedestrianAnimationSchedule,
    pub(crate) mirrored: bool,
    pub(crate) sim_lod_bone_global: Option<sim_lod_pose::SimLodBoneGlobal>,
    pub(crate) properties: AnimationProperties,
    pub(crate) motion_intents: PedestrianMotionIntents,
    pub(crate) ai_input: crate::pedestrian_intent_packet::queue::AiGraphInput,
    pub(crate) motion: Option<crate::pedestrian_motion_delegate::PedestrianMotionDelegate<K>>,
    pub(crate) motion_objects: Option<crate::pedestrian_motion_delegate::backend::MotionObjects>,
    pub(crate) graph_intents: Option<crate::actor_graph_construction::ActorGraphIntents>,
    pub(crate) channels: crate::pedestrian_motion_delegate::channel::PedestrianChannels,
    pub(crate) frame: crate::pedestrian_motion_delegate::frame::FrameMetadata,
    pub(crate) sim_lod_pose: Option<crate::pedestrian_motion_delegate::pose::SimLodPose>,
    pub(crate) presentation: Option<crate::selected_living_world_recipes::pedestrian_animation::presentation::PedestrianPresentationPose>,
    /// cPedAnimData82e8d3c4..40c, both initially bound to the authored RefSpec.
    pub(crate) entity_animation_data: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    pub(crate) override_animation_data: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    pub(crate) use_override_animation_data: bool,
    /// cPedAnimData.MotionGraphInput.pAnimAttributes, native +0x70.
    pub(crate) registered_attributes: Option<crate::attribute_runtime::PedestrianAnimAttributes>,
    pub(crate) transition_override: crate::actor_graph_construction::ActorGraphTransition,
    pub(crate) model_category: u32,
    pub(crate) blend_pose_buffer: Option<crate::actor_face_playback::blend_pose_stream::BlendPoseBuffer>,
}

impl<K, W> PedestrianAnimation<K, W> {
    /// tAnimData::~tAnimData82e93528 first destroys ChannelAnimatable and its
    /// base job owner, then MotionGraph and attribute instances. Remaining
    /// members use ordinary owned Rust destruction; no End callbacks are added.
    pub(crate) fn prepare_destruction(&mut self)->Result<(),String>{
        self.core.try_prepare_pedestrian_destruction().map_err(|e|e.to_string())
    }
    pub(crate) fn selected_animation_data(&self) -> Result<&crate::attribute_runtime::AttributeRuntimeInstance, String> {
        (if self.use_override_animation_data { &self.override_animation_data }
            else { &self.entity_animation_data }).as_ref().ok_or("pedestrian animation data not initialized".into())
    }
    /// Selected generated MotionGraph_Pedestrian load82e97fd0. Load82cef180
    /// passes call_behaviours=true; declaration objects remain owned once loaded.
    /// The animation data's motion graph names the delegate (marquee entities
    /// load motiongraph_marquee or Motiongraph_Photographer).
    pub(crate) fn load_pedestrian_motion(&mut self, authority: &ActorGraphAuthority<K>,
        delegate: crate::pedestrian_motion_delegate::MotionDelegate) -> Result<(), String> {
        if self.motion.is_some() || self.motion_objects.is_some() || self.graph_intents.is_some() {
            return Err("pedestrian motion graph already loaded".into());
        }
        let intents = crate::actor_graph_construction::ActorGraphIntents::try_new::<std::convert::Infallible>()
            .map_err(|e| format!("pedestrian graph intent construction: {e:?}"))?;
        self.motion_objects = Some(crate::pedestrian_motion_delegate::backend::MotionObjects::bind(delegate)?);
        self.graph_intents = Some(intents);
        self.motion = Some(crate::pedestrian_motion_delegate::PedestrianMotionDelegate::new(authority, true, delegate));
        Ok(())
    }
    /// UpdateAnimationData82e9a078. Snapshot once in this phase; graph reads
    /// retain it even when behaviour callbacks subsequently change the input.
    pub(crate) fn update_animation_data(&mut self) -> Result<(), String> {
        let snapshot = self.graph_intents.as_mut()
            .ok_or("pedestrian motion graph intents not loaded")?;
        crate::pedestrian_motion_delegate::live::capture_motion_intents(snapshot, &self.motion_intents)
    }
    /// CompleteLocomotionTransition82e6e030, including motion-input mutation.
    pub(crate) fn complete_locomotion_transition(&mut self) {
        self.ai_input.locomotion_transition_complete = true;
        self.motion_intents.retain_first();
    }
    /// CompleteLocomotionTurn82e6e048: turn packet consumption is deferred.
    pub(crate) fn complete_locomotion_turn(&mut self) {
        self.ai_input.locomotion_turn_complete = true;
        self.ai_input.decrement_turn_packet = true;
        self.motion_intents.retain_first();
    }
    pub(crate) fn new(
        authority: &ActorGraphAuthority<K>,
        world: Arc<W>,
        playback: Arc<Mutex<FacePlaybackData>>,
        bind_pose: AnimationPoseRef,
        attributes: Arc<Mutex<AnimationAttributePool>>,
        motion_pool: u32,
    ) -> Result<Self, ActorAnimationInitializeError<std::convert::Infallible>> {
        let mut core = ActorAnimationConstruction::new_pedestrian(authority, world);
        core.try_initialize_pedestrian(playback, bind_pose, attributes)?;
        Ok(Self {
            core,
            schedule: PedestrianAnimationSchedule::new(motion_pool),
            mirrored: false,
            // tAnimData82e934a0 leaves these matrices untouched. Its MemStore
            // zeroes every block at Init82a6de38 and FreeBlock82a6df68, so
            // perception can read zeros before the first animation sync.
            sim_lod_bone_global: Some(sim_lod_pose::SimLodBoneGlobal {
                matrices: [[[0.; 4]; 4]; 9],
            }),
            properties: AnimationProperties::default(),
            motion_intents: PedestrianMotionIntents::new(),
            ai_input: crate::pedestrian_intent_packet::queue::AiGraphInput::new(),
            motion: None,
            motion_objects: None,
            graph_intents: None,
            channels: crate::pedestrian_motion_delegate::channel::PedestrianChannels::new(),
            frame: crate::pedestrian_motion_delegate::frame::FrameMetadata::new(),
            sim_lod_pose: None,
            presentation: None,
            entity_animation_data: None,
            override_animation_data: None,
            use_override_animation_data: false,
            registered_attributes: None,
            transition_override: crate::actor_graph_construction::ActorGraphTransition::new(),
            model_category: 0,
            blend_pose_buffer: None,
        })
    }

    pub(crate) fn has_current_animation(&self) -> bool {
        self.core
            .base()
            .is_some_and(|base| base.body_root.is_some())
            || self
                .core
                .initialization()
                .and_then(|i| i.channels.as_ref())
                .is_some_and(|channels| channels.root.is_some())
    }

    /// Complete BuildAnimTree after its optional graph update. Separate native
    /// pedestrian mAnimProp retains prior expiration even if new playback resets
    /// the base properties during the frame.
    pub(crate) fn advance(
        &mut self,
        dt: f32,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), AnimationPhaseError> {
        self.core.try_animatable_set_attributes(cvars)?;
        let old_expired = self
            .core
            .base()
            .ok_or(AnimationPhaseError::Missing("pedestrian Animatable"))?
            .expired_or_looped;
        self.core
            .try_add_animation_time(dt)
            .map_err(AnimationPhaseError::Time)?;
        let base = self
            .core
            .base()
            .ok_or(AnimationPhaseError::Missing("pedestrian Animatable"))?;
        self.properties = AnimationProperties {
            is_playing: base.is_playing,
            expired_or_looped: base.expired_or_looped | old_expired,
            expired_by: base.expired_or_looped_by_time,
            time_to_expire: base.time_to_expire_or_loop,
        };
        self.core.try_process_tree_attributes()
    }
}

/// Native `tAnimData::MotionGraphPoolIndex` and motion-input time carry.
#[derive(Clone, Debug)]
pub(crate) struct PedestrianAnimationSchedule {
    pub(crate) motion_graph_pool: u32,
    pub(crate) motion_graph_dt_offset: f32,
}

impl PedestrianAnimationSchedule {
    pub(crate) fn new(motion_graph_pool: u32) -> Self {
        Self {
            motion_graph_pool,
            motion_graph_dt_offset: 0.0,
        }
    }

    /// `Pedestrian::BuildAnimTree` `0x82e6e878`.
    ///
    /// `Some(dt)` requests a graph update. Regardless of this result, native
    /// calls SetAttributes, AddTime with the original dt, then ProcessTreeAttrs.
    /// The initial pool values 2/3 force one update before becoming 0/1.
    pub(crate) fn graph_update_dt(
        &mut self,
        dt: f32,
        active_pool: u32,
        pools_enabled: bool,
        intents_valid: bool,
        has_current_animation: bool,
    ) -> Result<Option<f32>, InvalidPedestrianMotionPool> {
        let mut update = true;
        let mut graph_dt = dt;
        if pools_enabled {
            if self.motion_graph_pool < 2 {
                if self.motion_graph_pool == active_pool {
                    if intents_valid {
                        graph_dt = dt * 2.0 + self.motion_graph_dt_offset;
                        self.motion_graph_dt_offset = 0.0;
                    } else {
                        self.motion_graph_pool = self.motion_graph_pool.wrapping_sub(1) & 1;
                        self.motion_graph_dt_offset = dt;
                        update = false;
                    }
                } else {
                    update = false;
                }
            } else {
                self.motion_graph_pool -= 2;
                if self.motion_graph_pool >= 2 {
                    return Err(InvalidPedestrianMotionPool(self.motion_graph_pool));
                }
            }
        }
        if !update && !has_current_animation {
            update = true;
            graph_dt = dt;
        }
        Ok(update.then_some(graph_dt))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InvalidPedestrianMotionPool(pub(crate) u32);

impl std::fmt::Display for InvalidPedestrianMotionPool {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "pedestrian motion pool {} violates native <2 assertion after initialization",
            self.0
        )
    }
}

impl std::error::Error for InvalidPedestrianMotionPool {}

/// The four original registered-name routes in
/// `cPedAnimData::GetRegisteredInformation82e8d4e8`. These are snapshots of
/// the selected clip-block/transition-info owners, not a general override map.
#[derive(Clone, Debug, Default)]
pub(crate) struct PedestrianRegisteredClips {
    pub(crate) clip_block: Option<[String; 3]>,
    pub(crate) transition: Option<PedestrianTransitionInfo>,
}
/// PDB Anim::tAnimTransitionInfo. One option preserves the native pointer gate
/// shared by clip remapping and the special transition blend-time tag.
#[derive(Clone, Debug)]
pub(crate) struct PedestrianTransitionInfo {
    pub(crate) clip: String,
    pub(crate) branch_window: f32,
    pub(crate) blend_time: f32,
    pub(crate) target_offset: f32,
}

#[derive(Debug)]
pub(crate) enum PedestrianRemapError {
    Attribute(crate::attribute_runtime::AttributeRuntimeError),
    Text(std::str::Utf8Error),
}

impl std::fmt::Display for PedestrianRemapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(error) => error.fmt(f),
            Self::Text(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PedestrianRemapError {}

/// `Pedestrian::RemapAnim82e8d8e0`; selected data is the existing entity
/// animation RefSpec instance or its original explicit override.
pub(crate) fn remap_animation(
    original: &str,
    registered: &PedestrianRegisteredClips,
    selected_data: &crate::attribute_runtime::AttributeRuntimeInstance,
    random: impl FnMut() -> u32,
) -> Result<String, PedestrianRemapError> {
    use skate2_formats::attrib::string_hash64;
    let key = string_hash64(original);
    if let Some(block) = &registered.clip_block {
        for (name, clip) in [
            "RegisteredClipBlock_Into",
            "RegisteredClipBlock_Cycle",
            "RegisteredClipBlock_Out",
        ]
        .into_iter()
        .zip(block)
        {
            if key == string_hash64(name) {
                return Ok(clip.clone());
            }
        }
    }
    if key == string_hash64("RegisteredTransitionInfo_Clip") {
        if let Some(info) = &registered.transition {
            return Ok(info.clip.clone());
        }
    }
    match selected_data
        .try_pedestrian_animation_clip(key, random)
        .map_err(PedestrianRemapError::Attribute)?
    {
        Some(data) => Ok(std::str::from_utf8(data.bytes())
            .map_err(PedestrianRemapError::Text)?
            .to_owned()),
        None => Ok(original.to_owned()),
    }
}
