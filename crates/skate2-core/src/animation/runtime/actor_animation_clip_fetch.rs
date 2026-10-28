//! Actual retained clip and destination hierarchy -> native fetch descriptors.
use crate::{
    actor_animation_tree::{
        AnimationClipRef, AnimationNodeHandle, AnimationNodeKind, AnimationTreePool,
    },
    actor_face_playback::{FaceCommands, FaceDatabase, FaceDatabaseManager},
};
use std::sync::{Arc, Mutex};

#[path = "actor_animation_fetch_timing.rs"]
pub(crate) mod timing;
use timing::{BufferedFetchTiming, FetchFramePair, FetchTimingError, frame_pair};

impl FaceDatabaseManager {
    /// `GetClipHeader82d6e878`: low5 database bits and high11 sorted clip ordinal.
    fn resolve_clip_fetch(&self, guid: u16) -> Result<AnimationClipRef, ClipFetchError> {
        let database = self.slots[usize::from(guid & 31)]
            .as_ref()
            .ok_or(ClipFetchError::MissingOwner("GUID database"))?;
        let retained = database
            .lock()
            .map_err(|_| ClipFetchError::MissingOwner("GUID database lock"))?;
        let record_index = retained
            .clip_records_by_ordinal
            .get(usize::from(guid >> 5))
            .copied()
            .ok_or(ClipFetchError::MissingOwner("GUID clip ordinal"))?;
        Ok(AnimationClipRef {
            database: Arc::clone(database),
            record_index,
        })
    }
}

impl AnimationTreePool {
    #[cfg(test)]
    pub(crate) fn check_clip_fetch_plans(
        &self,
        root: &AnimationNodeHandle,
        commands: &Arc<Mutex<FaceCommands>>,
    ) {
        let mut pending = vec![root.clone()];
        let mut visited = Vec::new();
        let mut clips = 0;
        while let Some(node) = pending.pop() {
            if visited.contains(&node) {
                continue;
            }
            visited.push(node.clone());
            match &self.node(&node).unwrap().kind {
                AnimationNodeKind::Sequence(state) => {
                    pending.extend(state.children.iter().map(|c| c.animation.clone()))
                }
                AnimationNodeKind::Selector(state) => {
                    pending.extend(state.children.iter().map(|c| c.0.clone()))
                }
                AnimationNodeKind::BlendSpace(state) => {
                    pending.extend(state.children.iter().map(|child| {
                        child
                            .as_ref()
                            .expect("initialized child")
                            .as_ref()
                            .expect("non-null fixture child")
                            .clone()
                    }));
                }
                AnimationNodeKind::Clip(_) => {
                    let plan = self.prepare_clip_fetch(&node, commands, 2).unwrap();
                    assert!(Arc::ptr_eq(&plan.commands, commands));
                    assert!(plan.total_bones > 0 && !plan.parts.is_empty());
                    assert!(plan.timing.blend_frames);
                    for part in &plan.parts {
                        assert!(part.bone_offset + part.bone_count <= plan.total_bones);
                        assert!(part.current.frames[0] <= part.current.frames[1]);
                        if let Some(weights) = &part.channel_weights {
                            assert_eq!(weights.len(), part.bone_count);
                        }
                        assert_eq!(part.previous.is_some(), part.trajectory_delta.is_some());
                    }
                    clips += 1;
                }
                AnimationNodeKind::PhaseBlend(phase) => {
                    pending.extend(phase.children.iter().map(|(_, child)| child.clone()));
                }
                AnimationNodeKind::AddSqt { a, b, .. } => {
                    pending.push(a.clone());
                    pending.push(b.clone());
                }
                AnimationNodeKind::Mirror { child, .. } => pending.push(child.clone().unwrap()),
                AnimationNodeKind::Pose { .. } | AnimationNodeKind::FixedSqt { .. } => {}
            }
        }
        assert_eq!(clips, 2);
    }

    /// Prepare the actual `Clip::Eval82d6ef78` Fetch call; node state commits only
    /// after a future command sink has accepted this command.
    pub(crate) fn prepare_clip_fetch(
        &self,
        node: &AnimationNodeHandle,
        commands: &Arc<Mutex<FaceCommands>>,
        mode: u32,
    ) -> Result<ClipFetchPlan, ClipFetchError> {
        use ClipFetchError as E;
        let node = self.node(node).map_err(|_| E::ForeignOwner)?;
        let AnimationNodeKind::Clip(clip) = &node.kind else {
            return Err(E::MissingOwner("Clip node"));
        };
        let header = clip.header.as_ref().ok_or(E::MissingOwner("Clip header"))?;
        let speed = clip.base_speed * clip.speed;
        let maximum = (clip.frames - 1.0) / clip.fps;
        let clamp = |time: f32| {
            let time = time * speed;
            let time = if -time >= 0.0 { 0.0 } else { time };
            if maximum - time >= 0.0 { time } else { maximum }
        };
        let mut previous = clamp(clip.old_time);
        let current = clamp(clip.time);
        if clip.loops == 0 && previous > current {
            previous = current;
        }
        ClipFetchPlan::new(
            header.clone(),
            commands,
            previous,
            current,
            u32::from_ne_bytes(clip.loops.to_ne_bytes()),
            mode,
        )
    }
}

#[derive(Clone)]
pub(crate) struct ClipPartExtract {
    pub(crate) part_index: usize,
    pub(crate) bone_offset: usize,
    pub(crate) bone_count: usize,
    pub(crate) current: FetchFramePair,
    pub(crate) previous: Option<FetchFramePair>,
    pub(crate) channel_weights: Option<Vec<f32>>,
    pub(crate) trajectory_delta: Option<([f32; 4], [f32; 3])>,
}
#[derive(Clone)]
pub(crate) struct ClipFetchPlan {
    pub(crate) clip: AnimationClipRef,
    pub(crate) hierarchy: Arc<Mutex<FaceDatabase>>,
    pub(crate) commands: Arc<Mutex<FaceCommands>>,
    pub(crate) guid: u16,
    pub(crate) timing: BufferedFetchTiming,
    pub(crate) channel_animation: bool,
    pub(crate) total_bones: usize,
    /// One extract per destination part the clip authors. Empty is `BatchFetch`'s
    /// null-source entry (0x82d73e78): `Vector_ExtractPackedN` (0x82d7116c) writes the
    /// identity over all `total_bones`.
    pub(crate) parts: Vec<ClipPartExtract>,
}
#[derive(Debug)]
pub(crate) enum ClipFetchError {
    MissingOwner(&'static str),
    ForeignOwner,
    Timing(FetchTimingError),
    InvalidPart,
    UnsupportedCodec,
    TimeOutsideClip,
}
impl std::fmt::Display for ClipFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOwner(owner) => write!(f, "clip fetch requires {owner}"),
            Self::Timing(error) => write!(f, "clip fetch: {error}"),
            other => write!(f, "clip fetch: {other:?}"),
        }
    }
}

impl ClipFetchPlan {
    /// The extract entries `BatchFetch` (0x82d737d0) returns: one per matched part, or its
    /// single null-source entry when no destination part matched.
    pub(crate) fn extract_count(&self) -> usize {
        self.parts.len().max(1)
    }
    /// LoadCmdBufferInfo replays the encoded fetch against the destination ACS.
    /// Keep the packed time fields; re-encoding decoded seconds loses ticks.
    pub(crate) fn replay_on(&self, commands: &Arc<Mutex<FaceCommands>>)
        -> Result<Self, ClipFetchError> {
        let hierarchy = Arc::clone(&commands.lock()
            .map_err(|_| ClipFetchError::MissingOwner("replay command lock"))?.database);
        let target = hierarchy.lock()
            .map_err(|_| ClipFetchError::MissingOwner("replay hierarchy lock"))?
            .parsed.as_ref().ok_or(ClipFetchError::MissingOwner("replay hierarchy"))?
            .clone();
        let (previous, current) = self.timing.decode();
        Self::descriptors(self.clip.clone(), commands, hierarchy, &target.hierarchy,
            self.timing, previous, current)
    }
    /// Buffered `Fetch`/`EnQueueCmd`/`BatchFetch`; retains original source owners.
    pub(crate) fn new(
        clip: AnimationClipRef,
        commands: &Arc<Mutex<FaceCommands>>,
        previous: f32,
        current: f32,
        loops: u32,
        mode: u32,
    ) -> Result<Self, ClipFetchError> {
        use ClipFetchError as E;
        let command = commands
            .lock()
            .map_err(|_| E::MissingOwner("command lock"))?;
        let setup = command
            .setup
            .as_ref()
            .ok_or(E::MissingOwner("NewACS setup"))?;
        let buffered = command
            .buffered
            .as_ref()
            .ok_or(E::MissingOwner("buffered ACS"))?
            .lock()
            .map_err(|_| E::MissingOwner("buffered lock"))?;
        if !Arc::ptr_eq(&buffered.hierarchy_owner, &command.database) {
            return Err(E::ForeignOwner);
        }
        let hierarchy = Arc::clone(&buffered.hierarchy_owner);
        let guid = {
            let retained = clip
                .database
                .lock()
                .map_err(|_| E::MissingOwner("clip lock"))?;
            let record = retained
                .parsed
                .as_ref()
                .and_then(|p| p.records.get(clip.record_index))
                .ok_or(E::MissingOwner("clip header"))?;
            u16::try_from(record.header.guid & 0xffff).expect("GUID low16")
        };
        let resolved = buffered
            .database_manager
            .lock()
            .map_err(|_| E::MissingOwner("database manager lock"))?
            .resolve_clip_fetch(guid)?;
        if !Arc::ptr_eq(&resolved.database, &clip.database)
            || resolved.record_index != clip.record_index
        {
            return Err(E::ForeignOwner);
        }
        let timing = BufferedFetchTiming::encode(previous, current, loops, mode, setup.fetch_mode)
            .map_err(E::Timing)?;
        let (previous, current) = timing.decode();
        // Body playback may name the same database for clips and hierarchy.
        // BatchFetch82d737d0 reads its retained mSkel; it does not copy the
        // hierarchy for each clip. Retain our immutable parsed owner instead
        // of cloning all bone/name/part vectors on every fetch. Release the
        // target lock before acquiring the (possibly identical) clip owner.
        let target = {
            let retained = hierarchy
                .lock()
                .map_err(|_| E::MissingOwner("hierarchy lock"))?;
            retained
                .parsed
                .as_ref()
                .ok_or(E::MissingOwner("hierarchy"))?
                .clone()
        };
        drop(buffered);
        drop(command);
        Self::descriptors(
            clip, commands, hierarchy, &target.hierarchy, timing, previous, current,
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "ordered clip fetch descriptor acquisition and source failure prefixes"
    )]
    fn descriptors(
        clip: AnimationClipRef,
        commands: &Arc<Mutex<FaceCommands>>,
        hierarchy: Arc<Mutex<FaceDatabase>>,
        target: &skate2_formats::andale::AndaleHierarchy,
        timing: BufferedFetchTiming,
        previous: f32,
        current: f32,
    ) -> Result<Self, ClipFetchError> {
        use ClipFetchError as E;
        let command = commands
            .lock()
            .map_err(|_| E::MissingOwner("command lock"))?;
        let setup = command
            .setup
            .as_ref()
            .ok_or(E::MissingOwner("NewACS setup"))?;
        let database = clip
            .database
            .lock()
            .map_err(|_| E::MissingOwner("clip lock"))?;
        let parsed = database
            .parsed
            .as_ref()
            .ok_or(E::MissingOwner("clip database"))?;
        let record = parsed
            .records
            .get(clip.record_index)
            .ok_or(E::MissingOwner("clip record"))?;
        let authored = database
            .clip_by_record(clip.record_index)
            .ok_or(E::MissingOwner("clip payload"))?;
        if !matches!((authored.codec,setup.codec),(0x0052_4256,1)|(0x0000_4452,0)) {
            return Err(E::UnsupportedCodec);
        }
        let maximum =
            (authored.frame_count - 1.0) / authored.frames_per_second + setup.fetch_time_error;
        if previous < 0.0 || previous > maximum || current < 0.0 || current > maximum {
            return Err(E::TimeOutsideClip);
        }
        let current_pair = frame_pair(
            current,
            authored.frames_per_second,
            authored.frame_count,
            timing.blend_frames,
            setup.truncation_offset,
        )
        .map_err(E::Timing)?;
        let channel_animation = authored.flags & 0x2000_0000 != 0;
        let mut parts = Vec::new();
        let mut cursor = 0;
        for destination in &target.parts {
            if cursor >= authored.parts.len() {
                break;
            }
            let Some(relative) = authored.parts[cursor..]
                .iter()
                .position(|part| u16::from(part.part_name_index) == destination.name_index)
            else {
                continue;
            };
            let index = cursor + relative;
            if relative == 0 {
                cursor += 1;
            }
            let source = &authored.parts[index];
            let bone_offset =
                usize::try_from(destination.bone_offset).map_err(|_| E::InvalidPart)?;
            let bone_count = usize::try_from(destination.bone_count).map_err(|_| E::InvalidPart)?;
            if source.bone_count() != bone_count {
                return Err(E::InvalidPart);
            }
            let trajectory = setup.trajectory && bone_offset == 0;
            let previous_pair = if trajectory {
                Some(
                    frame_pair(
                        previous,
                        authored.frames_per_second,
                        authored.frame_count,
                        timing.blend_frames,
                        setup.truncation_offset,
                    )
                    .map_err(E::Timing)?,
                )
            } else {
                None
            };
            let trajectory_delta = trajectory.then(|| {
                if timing.looped {
                    (
                        std::array::from_fn(|i| f32::from_bits(authored.fixed_words[4 + i])),
                        std::array::from_fn(|i| f32::from_bits(authored.fixed_words[i])),
                    )
                } else {
                    ([0.0, 0.0, 0.0, 1.0], [0.0; 3])
                }
            });
            let weights = if channel_animation {
                let bytes = database
                    .bytes
                    .as_ref()
                    .ok_or(E::MissingOwner("clip backing bytes"))?;
                let start = record.data_range.start
                    + usize::try_from(source.encoded_offset & 0x00ff_ffff)
                        .map_err(|_| E::InvalidPart)?
                    + 16;
                let mut weights = Vec::with_capacity(bone_count);
                for i in 0..bone_count {
                    let value = bytes
                        .get(start + i * 4..start + i * 4 + 4)
                        .ok_or(E::InvalidPart)?;
                    weights.push(f32::from_bits(u32::from_be_bytes(
                        value.try_into().map_err(|_| E::InvalidPart)?,
                    )));
                }
                Some(weights)
            } else {
                None
            };
            parts.push(ClipPartExtract {
                part_index: index,
                bone_offset,
                bone_count,
                current: current_pair,
                previous: previous_pair,
                channel_weights: weights,
                trajectory_delta,
            });
        }
        let guid = u16::try_from(authored.guid & 0xffff).expect("GUID low16");
        drop(database);
        Ok(Self {
            clip,
            hierarchy,
            commands: Arc::clone(commands),
            guid,
            timing,
            channel_animation,
            total_bones: target.bone_names.len(),
            parts,
        })
    }
}
