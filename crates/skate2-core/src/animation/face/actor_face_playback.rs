//! Selected loaded-face constructor ownership and fixed-frame evaluation.
//! References: `SetupFacePose` 0x82cbf700, database constructor 0x82d74128,
//! `SetDBContent` 0x82d7f7e8, `NewACS` 0x824d62c0, `PlaybackData` 0x82d810b8.
//! These concrete managers currently execute the constructor's fixed RAW fetch
//! command sets. General body animation commands/jobs remain unsupported.
#![expect(dead_code, reason = "face construction owners await Actor mounting")]

use crate::{
    actor_graph_construction::ActorGraphIntents,
    animation_job_owners::AnimationJobScheduler,
    character_face_manager::{CharacterFaceError, CharacterFaceManager},
    play_animation::FastString36,
};
use skate2_formats::andale::{AndaleDatabase, AndaleError, AndaleHierarchy, AndaleRawError, AndaleSqt};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock},
};

#[cfg(test)]
#[path = "actor_face_playback_tests.rs"]
mod tests;

#[path = "../trees/actor_animation_tree_database.rs"]
pub(crate) mod tree_database;

#[path = "../pose/actor_animation_blend_pose_stream.rs"]
pub(crate) mod blend_pose_stream;
#[path = "../pose/actor_animation_raw_pose_fetch.rs"]
mod raw_pose_fetch;
use crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer;

#[derive(Clone)]
pub(crate) struct FaceAnimationManagers {
    pub(crate) command: Arc<Mutex<FaceCommandManager>>,
    pub(crate) playback: Arc<Mutex<FacePlaybackManager>>,
    pub(crate) database: Arc<Mutex<FaceDatabaseManager>>,
}
impl FaceAnimationManagers {
    /// Presentation has separate command/playback registries and the SAME global database manager.
    pub(crate) fn new_with_database(database: Arc<Mutex<FaceDatabaseManager>>) -> Self {
        Self {
            command: Arc::new(Mutex::new(FaceCommandManager {
                entries: BTreeMap::new(),
                scheduler: Arc::new(AnimationJobScheduler::new()),
            })),
            playback: Arc::new(Mutex::new(FacePlaybackManager {
                entries: BTreeMap::new(),
            })),
            database,
        }
    }
    pub(crate) fn new() -> Self {
        Self {
            command: Arc::new(Mutex::new(FaceCommandManager {
                entries: BTreeMap::new(),
                scheduler: Arc::new(AnimationJobScheduler::new()),
            })),
            playback: Arc::new(Mutex::new(FacePlaybackManager {
                entries: BTreeMap::new(),
            })),
            database: Arc::new(Mutex::new(FaceDatabaseManager {
                slots: std::array::from_fn(|_| None),
                part_names: Vec::new(),
            })),
        }
    }
}
pub(crate) struct FaceCommandManager {
    /// Actual vtable-only scheduler constructed by the manager at824d56b8.
    pub(crate) scheduler: Arc<AnimationJobScheduler>,
    pub(crate) entries: BTreeMap<(FastString36, i32), Arc<Mutex<FaceCommands>>>,
}
pub(crate) struct FacePlaybackManager {
    pub(crate) entries: BTreeMap<(FastString36, i32), Arc<Mutex<FacePlaybackData>>>,
}
pub(crate) struct FaceDatabaseManager {
    slots: [Option<Arc<Mutex<FaceDatabase>>>; 32],
    part_names: Vec<FastString36>,
}
pub(crate) struct FaceDatabase {
    pub(crate) slot: u8,
    /// The copied allocation owns source data independently of the load holder.
    /// Runtime header/part-name writes live in the parsed native structures.
    bytes: Option<Box<[u8]>>,
    /// Read-only once registered: readers may clone it and drop the lock.
    pub(crate) parsed: Option<Arc<AndaleDatabase>>,
    /// Immutable command-stream hierarchy, shared after its first use. Cleared
    /// whenever database content is registered or removed.
    hierarchy_snapshot: OnceLock<Arc<AndaleHierarchy>>,
    /// Bone-name view for presentation snapshots, with the same registration
    /// lifetime as the command-stream hierarchy above.
    character_hierarchy_snapshot: OnceLock<Arc<crate::character::CharacterHierarchy>>,
    part_mapping: Vec<u8>,
    records_by_kind: BTreeMap<u32, BTreeMap<FastString36, usize>>,
    /// Native mClips.mpBegin[guid >> 5], in the same surviving name order.
    clip_records_by_ordinal: Vec<usize>,
    /// Host equivalent of a retained ClipHeader pointer into parsed payloads.
    /// Includes every record, independently of duplicate-name map winners.
    clip_payload_indices: Vec<Option<usize>>,
}
pub(crate) struct FacePlaybackData {
    /// Hierarchy owner (`OnBoard` for body playback).
    pub(crate) database: Arc<Mutex<FaceDatabase>>,
    /// GetPose/GetAnimTree search this original insertion order.
    pub(crate) databases: Vec<Arc<Mutex<FaceDatabase>>>,
    pub(crate) commands: Arc<Mutex<FaceCommands>>,
}

#[derive(Clone)]
pub(crate) struct AnimationPoseRef {
    pub(crate) database: Arc<Mutex<FaceDatabase>>,
    pub(crate) record_index: usize,
}
#[derive(Clone)]
pub(crate) struct AnimationPhysicsRef {
    pub(crate) database: Arc<Mutex<FaceDatabase>>,
    pub(crate) record_index: usize,
}
impl AnimationPhysicsRef {
    /// Checked immutable snapshot for physical consumers; the reference retains
    /// the actual winning database and record identity alongside this copy.
    pub(crate) fn snapshot(
        &self,
    ) -> Result<Arc<skate2_formats::andale::AndalePhysicsParameters>, ActorFaceError> {
        let database = self.database.lock().map_err(|_| ActorFaceError::Lock)?;
        let parsed = database
            .parsed
            .as_ref()
            .ok_or(ActorFaceError::MissingRecord)?;
        let record = parsed
            .records
            .get(self.record_index)
            .ok_or(ActorFaceError::MissingRecord)?;
        parsed
            .physics_parameters
            .iter()
            .find(|parameters| parameters.record_offset == record.offset)
            .cloned()
            .map(Arc::new)
            .ok_or(ActorFaceError::MissingRecord)
    }
}
impl FacePlaybackData {
    /// `GetPhysicsParam82d7cb30`: first database with a surviving kind5 name wins.
    /// A missing key returns the source null result without inventing a fallback.
    pub(crate) fn get_physics_parameters(
        &self,
        name: FastString36,
    ) -> Result<Option<AnimationPhysicsRef>, ActorFaceError> {
        for database in &self.databases {
            let retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
            if let Some(record_index) = retained
                .records_by_kind
                .get(&5)
                .and_then(|map| map.get(&name))
                .copied()
            {
                return Ok(Some(AnimationPhysicsRef {
                    database: Arc::clone(database),
                    record_index,
                }));
            }
        }
        Ok(None)
    }
    /// Reference: `PlaybackData::GetPose` 0x82d7cab0. First database hit wins.
    pub(crate) fn get_pose(
        &self,
        name: FastString36,
    ) -> Result<Option<AnimationPoseRef>, ActorFaceError> {
        for database in &self.databases {
            let retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
            if let Some(record_index) = retained
                .records_by_kind
                .get(&3)
                .and_then(|map| map.get(&name))
                .copied()
            {
                return Ok(Some(AnimationPoseRef {
                    database: Arc::clone(database),
                    record_index,
                }));
            }
        }
        Ok(None)
    }
}
impl FaceDatabase {
    /// Clip::GetAttribute82d77ccc and BatchFetch82d737d0 already have a header
    /// pointer. Resolve our record handle in constant time, without scanning
    /// the entire clip database for each attribute query or animation fetch.
    pub(crate) fn clip_payload_index(&self, record_index: usize) -> Option<usize> {
        self.clip_payload_indices.get(record_index).copied().flatten()
    }

    pub(crate) fn clip_by_record(&self, record_index: usize)
        -> Option<&skate2_formats::andale::AndaleClip> {
        self.parsed.as_ref()?.clips.get(self.clip_payload_index(record_index)?)
    }

    /// ACS streams retain their hierarchy (SetUpBlendPoseStream 0x825f8398),
    /// rather than copying its vectors and bone names for each command stream.
    /// Registration finishes part-name mapping before any runtime stream reads
    /// this snapshot. An already queued stream keeps its old content on removal.
    pub(crate) fn hierarchy_snapshot(&self) -> Option<Arc<AndaleHierarchy>> {
        let parsed = self.parsed.as_ref()?;
        Some(Arc::clone(self.hierarchy_snapshot.get_or_init(|| {
            Arc::new(parsed.hierarchy.clone())
        })))
    }

    /// InitSkinning/RemapBones retain their database hierarchy. A visual
    /// snapshot shares its names instead of allocating each string per tick.
    pub(crate) fn character_hierarchy_snapshot(&self) -> Option<Arc<crate::character::CharacterHierarchy>> {
        let parsed = self.parsed.as_ref()?;
        Some(Arc::clone(self.character_hierarchy_snapshot.get_or_init(|| {
            Arc::new(crate::character::CharacterHierarchy {
                bone_names: parsed.hierarchy.bone_names.clone(),
            })
        })))
    }

    /// `DataBase::GetClipByName`: the clip's record.
    pub(crate) fn clip_record(&self, name: FastString36) -> Option<usize> {
        self.parsed.as_ref()?;
        self.records_by_kind.get(&2).and_then(|map| map.get(&name)).copied()
    }

    pub(crate) fn clip_index(&self, name: FastString36) -> Result<usize, ActorFaceError> {
        self.parsed.as_ref().ok_or(ActorFaceError::MissingClip)?;
        let record = self
            .records_by_kind
            .get(&2)
            .and_then(|map| map.get(&name))
            .ok_or(ActorFaceError::MissingClip)?;
        self.clip_payload_index(*record)
            .ok_or(ActorFaceError::MissingClip)
    }
}
impl AnimationPoseRef {
    pub(crate) fn guid(&self) -> Result<u16, ActorFaceError> {
        let retained = self.database.lock().map_err(|_| ActorFaceError::Lock)?;
        let guid = retained
            .parsed
            .as_ref()
            .and_then(|parsed| parsed.records.get(self.record_index))
            .ok_or(ActorFaceError::MissingRecord)?
            .header
            .guid;
        let bytes = guid.to_le_bytes();
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// `PoseBatchFetch82d6f508` walks the destination hierarchy and matches the
    /// globally mapped part IDs, even when the pose belongs to another DB.
    pub(crate) fn constant_vbr_sqt_for(
        &self,
        hierarchy: &skate2_formats::andale::AndaleHierarchy,
    ) -> Result<Vec<AndaleSqt>, ActorFaceError> {
        let parsed = Arc::clone(
            self.database
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .parsed
                .as_ref()
                .ok_or(ActorFaceError::MissingRecord)?,
        );
        let record = parsed
            .records
            .get(self.record_index)
            .ok_or(ActorFaceError::MissingRecord)?;
        if record.header.codec != 0x0052_4256 {
            return Err(ActorFaceError::UnsupportedPose);
        }
        let pose = parsed
            .poses
            .iter()
            .find(|pose| pose.record_offset == record.offset)
            .ok_or(ActorFaceError::MissingRecord)?;
        let mut output = Vec::with_capacity(hierarchy.bone_names.len());
        for target in &hierarchy.parts {
            if usize::try_from(target.bone_offset).ok() != Some(output.len()) {
                return Err(ActorFaceError::UnsupportedPose);
            }
            let part = pose
                .parts
                .iter()
                .find(|part| u16::from(part.part_name_index) == target.name_index)
                .ok_or(ActorFaceError::UnsupportedPose)?;
            let values = part
                .constant_vbr_sqt()
                .ok_or(ActorFaceError::UnsupportedPose)?;
            if u32::try_from(values.len()).ok() != Some(target.bone_count) {
                return Err(ActorFaceError::UnsupportedPose);
            }
            output.extend(values);
        }
        if output.len() != hierarchy.bone_names.len() {
            return Err(ActorFaceError::UnsupportedPose);
        }
        Ok(output)
    }
    /// Constant VBR pose channels only; never substitutes a RAW presentation rig.
    pub(crate) fn constant_vbr_sqt(&self) -> Result<Vec<AndaleSqt>, ActorFaceError> {
        let parsed = Arc::clone(
            self.database
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .parsed
                .as_ref()
                .ok_or(ActorFaceError::MissingRecord)?,
        );
        let record = parsed
            .records
            .get(self.record_index)
            .ok_or(ActorFaceError::MissingRecord)?;
        let pose = parsed
            .poses
            .iter()
            .find(|pose| pose.record_offset == record.offset)
            .ok_or(ActorFaceError::MissingRecord)?;
        if pose.parts.len() != parsed.hierarchy.parts.len() {
            return Err(ActorFaceError::UnsupportedPose);
        }
        let mut sqt = Vec::with_capacity(parsed.hierarchy.bone_names.len());
        for (part, hierarchy) in pose.parts.iter().zip(&parsed.hierarchy.parts) {
            if usize::try_from(hierarchy.bone_offset).ok() != Some(sqt.len())
                || u16::from(part.part_name_index) != hierarchy.name_index
            {
                return Err(ActorFaceError::UnsupportedPose);
            }
            let values = part
                .constant_vbr_sqt()
                .ok_or(ActorFaceError::UnsupportedPose)?;
            if u32::try_from(values.len()).ok() != Some(hierarchy.bone_count) {
                return Err(ActorFaceError::UnsupportedPose);
            }
            sqt.extend(values);
        }
        if sqt.len() != parsed.hierarchy.bone_names.len() {
            return Err(ActorFaceError::UnsupportedPose);
        }
        Ok(sqt)
    }
}
impl FaceDatabaseManager {
    /// Native GUID low five bits select the actual registered database;
    /// remaining eleven bits select its sorted surviving pose-name map.
    pub(crate) fn pose_by_guid(&self, guid: u16) -> Result<AnimationPoseRef, ActorFaceError> {
        let database = self.slots[usize::from(guid & 31)]
            .as_ref()
            .ok_or(ActorFaceError::MissingRecord)?;
        let retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
        let record_index = retained
            .records_by_kind
            .get(&3)
            .and_then(|poses| poses.values().nth(usize::from(guid >> 5)))
            .copied()
            .ok_or(ActorFaceError::MissingRecord)?;
        Ok(AnimationPoseRef {
            database: Arc::clone(database),
            record_index,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FaceCommandPhase {
    Ready,
    Begun,
    Fetched,
    Extracted,
    Ended,
    Synced,
}
pub(crate) struct FaceCommands {
    pub(crate) database: Arc<Mutex<FaceDatabase>>,
    pub(crate) fixed_frames: Vec<Vec<AndaleSqt>>,
    pub(crate) fixed_matrices: Vec<Arc<[crate::animation::AnimationMatrix]>>,
    pub(crate) max_frames: usize,
    pub(crate) phase: FaceCommandPhase,
    pub(crate) scratch: Option<Vec<AndaleSqt>>,
    pub(crate) selected_frames: Vec<u32>,
    pub(crate) setup: Option<AnimationCommandSetup>,
    pub(crate) buffered: Option<Arc<Mutex<crate::actor_animation_construction::initialization::bind_pose::BufferedBodyCommands>>>,
}

/// Retained `NewACS` inputs, not a claim that general commands/jobs execute.
/// Reference: `Simulation::InitAnimation` 0x828a5640.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnimationCommandSetup {
    /// SQT/fetch/replace/matrix and total matrix capacities, in `ACSParams` order.
    pub(crate) capacities: [u32; 5],
    pub(crate) averages: [f32; 5],
    pub(crate) max_nblend: u32,
    pub(crate) fixed_sqt: u32,
    pub(crate) fixed_matrix: u32,
    pub(crate) mode: u32,
    pub(crate) fetch_mode: u32,
    pub(crate) category: u32,
    pub(crate) characters: u32,
    pub(crate) truncation_offset: f32,
    /// `FetchSys` ctor writes zero; completed `NewACS824d6540` writes its query result.
    pub(crate) fetch_time_error: f32,
    pub(crate) trajectory: bool,
    pub(crate) codec: u32,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ActorFaceError {
    Face(CharacterFaceError),
    Format(AndaleError),
    Raw(AndaleRawError),
    Lock,
    DatabaseCapacity,
    PartNamesCapacity,
    PartMapping,
    RecordCapacity,
    AlreadyAttempted,
    UnsupportedDatabaseReuse,
    MissingClip,
    DuplicatePlayback,
    UnsupportedTiming,
    MissingRecord,
    UnsupportedPose,
}
#[derive(Debug)]
pub(crate) enum FaceSetupError<E> {
    Face(ActorFaceError),
    Commands(E),
}
impl<E> From<ActorFaceError> for FaceSetupError<E> {
    fn from(error: ActorFaceError) -> Self {
        Self::Face(error)
    }
}
impl From<AndaleError> for ActorFaceError {
    fn from(value: AndaleError) -> Self {
        Self::Format(value)
    }
}
impl From<AndaleRawError> for ActorFaceError {
    fn from(value: AndaleRawError) -> Self {
        Self::Raw(value)
    }
}

pub(crate) struct ActorFacePlayback {
    pub(crate) face_name: FastString36,
    pub(crate) playback_name: FastString36,
    pub(crate) managers: FaceAnimationManagers,
    pub(crate) database: Option<Arc<Mutex<FaceDatabase>>>,
    pub(crate) commands: Option<Arc<Mutex<FaceCommands>>>,
    pub(crate) playback: Option<Arc<Mutex<FacePlaybackData>>>,
    /// Init allocates but does not initialize these local pose elements.
    pub(crate) local_pose: OwnedAnimationSqtBuffer,
    pub(crate) returned: bool,
    attempted: bool,
}
impl ActorFacePlayback {
    pub(crate) fn new(face: &[u8], player: u32, managers: FaceAnimationManagers) -> Self {
        let face_c = face
            .iter()
            .position(|byte| *byte == 0)
            .map_or(face, |end| &face[..end]);
        let mut playback = face_c.to_vec();
        let signed_player = i32::from_ne_bytes(player.to_ne_bytes());
        playback.extend_from_slice(format!("_{signed_player:03}").as_bytes());
        Self {
            face_name: ActorGraphIntents::native_name(face_c),
            playback_name: ActorGraphIntents::native_name(&playback),
            managers,
            database: None,
            commands: None,
            playback: None,
            local_pose: OwnedAnimationSqtBuffer::new(),
            returned: false,
            attempted: false,
        }
    }

    /// `SkaterAnim::PurgeFacePose` 0x82ca9b38, run by `~SkaterAnim` 0x82cb7430: `DeleteACS`
    /// 0x824d5ff0 drops the face's command system, the playback's databases are destroyed and
    /// `~PlaybackData` 0x82d80f78 removes it from the `PlaybackDataManager`.
    pub(crate) fn try_purge(&mut self) -> Result<(), ActorFaceError> {
        self.managers
            .command
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .remove(&(self.playback_name, 0));
        self.commands = None;
        let Some(playback) = self.playback.take() else {
            return Ok(());
        };
        let databases = std::mem::take(&mut playback.lock().map_err(|_| ActorFaceError::Lock)?.databases);
        {
            let mut manager = self.managers.database.lock().map_err(|_| ActorFaceError::Lock)?;
            for database in &databases {
                manager.remove(database)?;
            }
        }
        self.database = None;
        self.managers
            .playback
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .remove(&(self.playback_name, 0));
        Ok(())
    }

    /// Retains every successful constructor publication if a later operation
    /// fails. Caller stores this owner before entering; failed setup is not
    /// silently replayed into manager registries.
    pub(crate) fn try_setup(
        &mut self,
        name: &[u8],
        faces: &CharacterFaceManager,
    ) -> Result<(), ActorFaceError> {
        self.try_setup_with_commands(
            name,
            faces,
            |_, _, _| Ok::<(), std::convert::Infallible>(()),
        )
        .map_err(|error| match error {
            FaceSetupError::Face(error) => error,
            FaceSetupError::Commands(error) => match error {},
        })
    }

    /// Actual `NewACS` construction callback occurs before manager publication.
    pub(crate) fn try_setup_with_commands<E>(
        &mut self,
        name: &[u8],
        faces: &CharacterFaceManager,
        mut construct: impl FnMut(
            &FaceAnimationManagers,
            &Arc<Mutex<FaceCommands>>,
            &AnimationCommandSetup,
        ) -> Result<(), E>,
    ) -> Result<(), FaceSetupError<E>> {
        if self.attempted {
            return Err(ActorFaceError::AlreadyAttempted.into());
        }
        self.attempted = true;
        let copied = faces.copy_face_data(name).map_err(ActorFaceError::Face)?;
        let database = self
            .managers
            .database
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .allocate()?;
        self.database = Some(Arc::clone(&database));
        {
            let mut retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
            self.managers
                .database
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .set_content(&mut retained, copied)?;
        }
        let (clip_index, frames, bones) = {
            let retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
            let parsed = retained.parsed.as_ref().expect("parsed before clip query");
            let clip_index = retained.clip_index(self.face_name)?;
            let frames = checked_frame_count(parsed.clips[clip_index].frame_count)?;
            (clip_index, frames, parsed.hierarchy.bone_names.len())
        };
        let commands = Arc::new(Mutex::new(FaceCommands {
            database: Arc::clone(&database),
            fixed_frames: Vec::new(),
            fixed_matrices: Vec::new(),
            max_frames: usize::from(frames),
            phase: FaceCommandPhase::Ready,
            scratch: None,
            selected_frames: Vec::new(),
            setup: Some(AnimationCommandSetup {
                capacities: [100, 8, 1, 1, 1],
                averages: [25.0, 1.0, 1.0, 1.0, 1.0],
                max_nblend: 4,
                fixed_sqt: u32::from(frames),
                fixed_matrix: 0,
                mode: 1,
                fetch_mode: 0,
                category: 0,
                characters: 9,
                truncation_offset: 0.5,
                fetch_time_error: 0.0,
                trajectory: false,
                codec: 0,
            }),
            buffered: None,
        }));
        // Retain any successful NewACS allocation if its later singleton read
        // fails. Registry publication remains after the completed constructor.
        self.commands = Some(Arc::clone(&commands));
        let setup = commands
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .setup
            .clone()
            .ok_or(ActorFaceError::UnsupportedTiming)?;
        construct(&self.managers, &commands, &setup).map_err(FaceSetupError::Commands)?;
        commands
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .setup
            .as_mut()
            .ok_or(ActorFaceError::UnsupportedTiming)?
            .fetch_time_error = f32::from_bits(0x3ada_1019);
        // NewACS uses operator[]: an equal name/index replaces its registry entry.
        self.managers
            .command
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .insert((self.playback_name, 0), Arc::clone(&commands));
        self.commands = Some(Arc::clone(&commands));
        let playback = Arc::new(Mutex::new(FacePlaybackData {
            database: Arc::clone(&database),
            databases: vec![Arc::clone(&database)],
            commands: Arc::clone(&commands),
        }));
        {
            let mut manager = self
                .managers
                .playback
                .lock()
                .map_err(|_| ActorFaceError::Lock)?;
            if manager.entries.contains_key(&(self.playback_name, 0)) {
                return Err(ActorFaceError::DuplicatePlayback.into());
            }
            manager
                .entries
                .insert((self.playback_name, 0), Arc::clone(&playback));
        }
        self.playback = Some(playback);
        self.local_pose = OwnedAnimationSqtBuffer::allocate_unknown(bones);
        evaluate_fixed_clip(&commands, clip_index)?;
        self.returned = true;
        Ok(())
    }
}

/// Executes only the recovered fixed RAW clip setup command sequence.
/// References: face 82cbfac0 and hands 828a5ec8 / 828a6300.
pub(crate) fn evaluate_fixed_clip(
    commands: &Arc<Mutex<FaceCommands>>,
    clip_index: usize,
) -> Result<(), ActorFaceError> {
    let database = Arc::clone(&commands.lock().map_err(|_| ActorFaceError::Lock)?.database);
    let frames = {
        let retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
        checked_frame_count(
            retained
                .parsed
                .as_ref()
                .ok_or(ActorFaceError::MissingClip)?
                .clips
                .get(clip_index)
                .ok_or(ActorFaceError::MissingClip)?
                .frame_count,
        )?
    };
    for frame in 0..frames {
        let mut command = commands.lock().map_err(|_| ActorFaceError::Lock)?;
        command.phase = FaceCommandPhase::Begun;
        let retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
        let parsed = retained.parsed.as_ref().expect("retained database");
        let clip = &parsed.clips[clip_index];
        let selected = select_face_frame(
            clip.frame_count,
            clip.frames_per_second,
            clip.base_speed,
            frame,
        )?;
        command.selected_frames.push(u32::from(selected));
        command.phase = FaceCommandPhase::Fetched;
        let extracted = clip.sample_raw_frame_sqt(&parsed.hierarchy, usize::from(selected))?;
        command.scratch = Some(extracted);
        command.phase = FaceCommandPhase::Extracted;
        // EndCommands -> EndCommandSet -> Sync complete this selected native
        // host command set before RegisterFixedSQT copies its 48*N bytes.
        command.phase = FaceCommandPhase::Ended;
        command.phase = FaceCommandPhase::Synced;
        let fixed = command
            .scratch
            .as_ref()
            .expect("successful extraction")
            .clone();
        command.fixed_frames.push(fixed);
        command.phase = FaceCommandPhase::Ready;
    }
    // Setup's temporary PoseBuffer is released after its fixed copies are
    // registered (82cbfb54..5c); the retained local pose is separate.
    commands.lock().map_err(|_| ActorFaceError::Lock)?.scratch = None;
    Ok(())
}

impl FaceDatabaseManager {
    /// `DataBase` constructor publishes the first free slot before content IO.
    /// Reference: 0x82d74128.
    pub(crate) fn allocate(&mut self) -> Result<Arc<Mutex<FaceDatabase>>, ActorFaceError> {
        let slot = self
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(ActorFaceError::DatabaseCapacity)?;
        let database = Arc::new(Mutex::new(FaceDatabase {
            slot: u8::try_from(slot).map_err(|_| ActorFaceError::DatabaseCapacity)?,
            bytes: None,
            parsed: None,
            hierarchy_snapshot: OnceLock::new(),
            character_hierarchy_snapshot: OnceLock::new(),
            part_mapping: Vec::new(),
            records_by_kind: BTreeMap::new(),
            clip_records_by_ordinal: Vec::new(),
            clip_payload_indices: Vec::new(),
        }));
        self.slots[slot] = Some(Arc::clone(&database));
        Ok(database)
    }

    /// `DataBase::Clear` 0x82d72344: `RemoveDataBase` 0x82d6bcd0 empties the database's
    /// slot, then its content and record tables go.
    pub(crate) fn remove(&mut self, database: &Arc<Mutex<FaceDatabase>>) -> Result<(), ActorFaceError> {
        let mut retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
        let slot = &mut self.slots[usize::from(retained.slot)];
        if !slot.as_ref().is_some_and(|actual| Arc::ptr_eq(actual, database)) {
            return Err(ActorFaceError::MissingRecord);
        }
        *slot = None;
        retained.bytes = None;
        retained.parsed = None;
        retained.hierarchy_snapshot.take();
        retained.character_hierarchy_snapshot.take();
        retained.records_by_kind.clear();
        retained.clip_records_by_ordinal.clear();
        retained.clip_payload_indices.clear();
        Ok(())
    }

    pub(crate) fn set_content(
        &mut self,
        database: &mut FaceDatabase,
        bytes: Box<[u8]>,
    ) -> Result<(), ActorFaceError> {
        if database.bytes.is_some() {
            return Err(ActorFaceError::UnsupportedDatabaseReuse);
        }
        database.bytes = Some(bytes);
        let parsed = AndaleDatabase::parse(database.bytes.as_deref().expect("stored content"))?;
        if parsed.file_header.guid != 0 {
            return Err(ActorFaceError::UnsupportedDatabaseReuse);
        }
        database.parsed = Some(Arc::new(parsed));
        self.initialize_content(database)
    }

    /// Reference: `SetDBContent` 0x82d7f7e8; `UpdateDatabase` 0x82d76628;
    /// `UpdatePartNames` 0x82d6e728. Slot/name assignments retain source order.
    fn initialize_content(&mut self, retained: &mut FaceDatabase) -> Result<(), ActorFaceError> {
        assign_record_guids(retained)?;
        let parsed = retained
            .parsed
            .as_mut()
            .map(Arc::make_mut)
            .expect("content parse precedes registration");
        for part in &parsed.hierarchy.parts {
            let index = if let Some(index) = self
                .part_names
                .iter()
                .position(|name| *name == part.packed_name)
            {
                index
            } else {
                if self.part_names.len() >= 31 {
                    return Err(ActorFaceError::PartNamesCapacity);
                }
                self.part_names.push(part.packed_name);
                self.part_names.len() - 1
            };
            let mapped = u8::try_from(index).map_err(|_| ActorFaceError::PartMapping)?;
            retained.part_mapping.push(mapped);
        }
        for (part, mapped) in parsed
            .hierarchy
            .parts
            .iter_mut()
            .zip(&retained.part_mapping)
        {
            part.name_index = u16::from(*mapped);
        }
        for pose in &mut parsed.poses {
            if !retained.records_by_kind.get(&3).is_some_and(|map| {
                map.values()
                    .any(|index| parsed.records[*index].offset == pose.record_offset)
            }) {
                continue;
            }
            for part in &mut pose.parts {
                let local = usize::try_from((part.header >> 1) & 31)
                    .map_err(|_| ActorFaceError::PartMapping)?;
                let mapped = *retained
                    .part_mapping
                    .get(local)
                    .ok_or(ActorFaceError::PartMapping)?;
                part.header = part.header & !0x3e | ((u32::from(mapped) * 2) & 0x3e);
                part.encoded_offset |= u32::from(mapped) << 24;
                part.part_name_index = (part.encoded_offset >> 24).to_le_bytes()[0];
            }
        }
        for clip in &mut parsed.clips {
            if !retained.records_by_kind.get(&2).is_some_and(|map| {
                map.values()
                    .any(|index| parsed.records[*index].offset == clip.record_offset)
            }) {
                continue;
            }
            for (encoded_offset, part) in clip.encoded_part_offsets.iter_mut().zip(&mut clip.parts)
            {
                let local = usize::try_from((part.header >> 1) & 31)
                    .map_err(|_| ActorFaceError::PartMapping)?;
                let mapped = *retained
                    .part_mapping
                    .get(local)
                    .ok_or(ActorFaceError::PartMapping)?;
                part.header = part.header & !0x3e | ((u32::from(mapped) * 2) & 0x3e);
                part.encoded_offset |= u32::from(mapped) << 24;
                *encoded_offset = part.encoded_offset;
                part.part_name_index = (part.encoded_offset >> 24).to_le_bytes()[0];
            }
        }
        Ok(())
    }
}

fn assign_record_guids(retained: &mut FaceDatabase) -> Result<(), ActorFaceError> {
    retained.hierarchy_snapshot.take();
    retained.character_hierarchy_snapshot.take();
    retained.clip_payload_indices.clear();
    let parsed = retained.parsed.as_mut().map(Arc::make_mut).expect("parsed content");
    parsed.file_header.guid = 1;
    // Native vector_map::operator[] overwrites previous same-name records.
    // Only surviving map entries receive GUID and part-name mutations.
    for (index, record) in parsed.records.iter().enumerate() {
        retained
            .records_by_kind
            .entry(record.header.data_type)
            .or_default()
            .insert(record.header.name, index);
    }
    // SetDBContent 0x82d7f93c assigns GUIDs in mClips vector order.
    // Build that index once; GetClipHeader 0x82d6e8e8 indexes it directly.
    retained.clip_records_by_ordinal = retained.records_by_kind.get(&2)
        .map(|map| map.values().copied().collect()).unwrap_or_default();
    for kind in [2, 3] {
        let order: Vec<usize> = retained
            .records_by_kind
            .get(&kind)
            .map(|map| map.values().copied().collect())
            .unwrap_or_default();
        for (ordinal, index) in order.into_iter().enumerate() {
            let ordinal = u32::try_from(ordinal).map_err(|_| ActorFaceError::RecordCapacity)?;
            if ordinal > 2048 {
                return Err(ActorFaceError::RecordCapacity);
            }
            parsed.records[index].header.guid |=
                u32::from(retained.slot) | ((ordinal * 32) & 0xffe0);
        }
    }
    for clip in &mut parsed.clips {
        clip.guid = parsed
            .records
            .iter()
            .find(|record| record.offset == clip.record_offset)
            .ok_or(ActorFaceError::PartMapping)?
            .header
            .guid;
    }
    let mut first_clip_by_offset = BTreeMap::new();
    for (index, clip) in parsed.clips.iter().enumerate() {
        // Match the previous iter().find() even for repeated payload offsets.
        first_clip_by_offset.entry(clip.record_offset).or_insert(index);
    }
    retained.clip_payload_indices = parsed.records.iter()
        .map(|record| first_clip_by_offset.get(&record.offset).copied())
        .collect();
    Ok(())
}

pub(crate) fn checked_frame_count(frames: f32) -> Result<u16, ActorFaceError> {
    if !frames.is_finite()
        || frames < 2.0
        || frames > f32::from(u16::MAX)
        || frames.fract().to_bits() != 0
    {
        return Err(ActorFaceError::UnsupportedTiming);
    }
    // Checked exact integer range above matches source fctidz; no saturation.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "validated integer u16 domain"
    )]
    Ok(frames as u16)
}

/// `SetFrame` 825ecdb8 -> `Eval` 82d6ef78 -> buffered `Fetch` 82a2f740 ->
/// `EnQueueCmd` 82a30458 -> unblended `BatchFetch` 82d737d0. Setup uses speed1,
/// equal old/current time, loops0, `FETCHFRAME`, and truncation offset0.5.
pub(crate) fn select_face_frame(
    frames: f32,
    fps: f32,
    base_speed: f32,
    frame: u16,
) -> Result<u16, ActorFaceError> {
    let count = checked_frame_count(frames)?;
    if frame >= count
        || !fps.is_finite()
        || fps <= 0.0
        || !base_speed.is_finite()
        || base_speed <= 0.0
    {
        return Err(ActorFaceError::UnsupportedTiming);
    }
    let last = frames - 1.0;
    let length = last / (base_speed * fps);
    let time = length * (f32::from(frame) / last);
    let scaled = time * base_speed;
    let maximum = last / fps;
    let current = if scaled < 0.0 {
        0.0
    } else if scaled > maximum {
        maximum
    } else {
        scaled
    };
    let encoded_previous = (current * f32::from_bits(0x44b6_0b33)).floor();
    if !encoded_previous.is_finite() || !(0.0..262_144.0).contains(&encoded_previous) {
        return Err(ActorFaceError::UnsupportedTiming);
    }
    // Equal old/current times always produce zero delta after Fetch's epsilon
    // comparison. Preserve that comparison rather than assuming it silently.
    let decoded = encoded_previous * f32::from_bits(0x3a34_002d);
    let delta = current - decoded;
    if delta.abs() > f32::from_bits(0x3a34_002d) {
        return Err(ActorFaceError::UnsupportedTiming);
    }
    let position = fps * decoded;
    let position = if position >= last { last } else { position };
    let selected = (position + 0.5).trunc();
    if !selected.is_finite() || selected < 0.0 || selected > last {
        return Err(ActorFaceError::UnsupportedTiming);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded by checked frame-count u16 domain"
    )]
    Ok(selected as u16)
}

#[path = "../../rendering/skater/skater_presentation_animation.rs"]
pub(crate) mod presentation;
