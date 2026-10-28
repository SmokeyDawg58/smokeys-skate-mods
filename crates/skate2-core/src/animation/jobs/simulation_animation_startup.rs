//! Retained animation databases/playbacks produced before Actor construction.
//! Reference: `Sk8::Simulation::InitAnimation` 0x828a5640.
//! General body ACS/job execution is not implemented by this startup owner.
#![expect(
    dead_code,
    reason = "animation startup awaits Simulation ownership integration"
)]

use crate::{
    actor_animation_construction::initialization::bind_pose::{
        BindPoseError, BufferedBodyCommands,
    },
    actor_face_playback::{
        ActorFaceError, AnimationCommandSetup, FaceAnimationManagers, FaceCommandPhase,
        FaceCommands, FaceDatabase, FacePlaybackData, checked_frame_count, evaluate_fixed_clip,
    },
    actor_graph_construction::ActorGraphIntents,
    animation_job_owners::AnimationJobManager,
};
use std::{
    mem::MaybeUninit,
    sync::{Arc, Mutex},
};

#[cfg(test)]
#[path = "simulation_animation_startup_corpus.rs"]
pub(crate) mod corpus;
#[cfg(test)]
#[path = "simulation_animation_startup_tests.rs"]
mod tests;

pub(crate) trait AnimationStartupSource {
    type Error;
    fn current_thread_id(&mut self) -> Result<u64, Self::Error>;
    /// Read the current linked `kUseVBR` target separately at each source call.
    fn read_use_vbr(&mut self) -> Result<bool, Self::Error>;
    fn read_animation(&mut self, source_path: &str) -> Result<Box<[u8]>, Self::Error>;
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum AnimationStartupError<E> {
    Source(E),
    Animation(ActorFaceError),
    Allocation,
    AlreadyAttempted,
    BindPose(BindPoseError),
}
impl<E> From<ActorFaceError> for AnimationStartupError<E> {
    fn from(error: ActorFaceError) -> Self {
        Self::Animation(error)
    }
}

/// Allocated but initially uninitialized `PoolStack` storage, never an identity pose.
/// `Andale::Init` 82d74228 uses 0x19000 bytes and 20 element entries.
pub(crate) struct AnimationAttributePool {
    pub(crate) creation_thread: Option<u64>,
    pub(crate) memory: Vec<MaybeUninit<u8>>,
    pub(crate) elements: Vec<MaybeUninit<usize>>,
    pub(crate) element_count: usize,
}

pub(crate) struct SimulationAnimationStartup {
    pub(crate) managers: FaceAnimationManagers,
    pub(crate) attributes: Option<Arc<Mutex<AnimationAttributePool>>>,
    /// Original allocation order, including an allocated database whose IO failed.
    pub(crate) databases: Vec<Arc<Mutex<FaceDatabase>>>,
    pub(crate) body: Vec<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) hands: Vec<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) selected_vbr_files: Option<bool>,
    pub(crate) selected_vbr_codec: Option<bool>,
    pub(crate) databases_and_fixed_hands_ready: bool,
    pub(crate) pending_commands: Vec<Arc<Mutex<FaceCommands>>>,
    attempted: bool,
}
impl SimulationAnimationStartup {
    pub(crate) fn retained_job_manager(
        &self,
    ) -> Result<Option<Arc<AnimationJobManager>>, ActorFaceError> {
        let Some(commands) = self.pending_commands.first() else {
            return Ok(None);
        };
        let commands = commands.lock().map_err(|_| ActorFaceError::Lock)?;
        let Some(buffered) = &commands.buffered else {
            return Ok(None);
        };
        let buffered = buffered.lock().map_err(|_| ActorFaceError::Lock)?;
        Ok(buffered
            .entries
            .first()
            .and_then(|entry| entry.manager.clone()))
    }
    pub(crate) fn new(managers: FaceAnimationManagers) -> Self {
        Self {
            managers,
            attributes: None,
            databases: Vec::new(),
            body: Vec::new(),
            hands: Vec::new(),
            selected_vbr_files: None,
            selected_vbr_codec: None,
            databases_and_fixed_hands_ready: false,
            pending_commands: Vec::new(),
            attempted: false,
        }
    }

    /// Preserves database/registry effects if any later IO/decode/registration fails.
    pub(crate) fn try_initialize<S: AnimationStartupSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), AnimationStartupError<S::Error>> {
        if self.attempted {
            return Err(AnimationStartupError::AlreadyAttempted);
        }
        self.attempted = true;
        let attributes = Arc::new(Mutex::new(AnimationAttributePool {
            creation_thread: None,
            memory: Vec::new(),
            elements: Vec::new(),
            element_count: 0,
        }));
        self.attributes = Some(Arc::clone(&attributes));
        let thread = source
            .current_thread_id()
            .map_err(AnimationStartupError::Source)?;
        {
            let mut pool = attributes.lock().map_err(|_| ActorFaceError::Lock)?;
            pool.creation_thread = Some(thread);
            pool.elements
                .try_reserve_exact(20)
                .map_err(|_| AnimationStartupError::Allocation)?;
            pool.elements.resize(20, MaybeUninit::uninit());
            pool.memory
                .try_reserve_exact(0x19000)
                .map_err(|_| AnimationStartupError::Allocation)?;
            pool.memory.resize(0x19000, MaybeUninit::uninit());
        }
        let vbr_files = source
            .read_use_vbr()
            .map_err(AnimationStartupError::Source)?;
        self.selected_vbr_files = Some(vbr_files);
        let paths = if vbr_files {
            ["data\\anim/OffBoard.abin", "data\\anim/OnBoard.abin"]
        } else {
            ["data\\anim/OffBoardRAW.abin", "data\\anim/OnBoardRAW.abin"]
        };
        let offboard = self.load_database(source, paths[0])?;
        let onboard = self.load_database(source, paths[1])?;
        let vbr_codec = source
            .read_use_vbr()
            .map_err(AnimationStartupError::Source)?;
        self.selected_vbr_codec = Some(vbr_codec);
        for player in 0..9 {
            let playback = self.register_playback(
                b"Skater",
                b"Skater",
                player,
                &onboard,
                vec![Arc::clone(&offboard), Arc::clone(&onboard)],
                &body_setup(vbr_codec),
                Some(source),
            )?;
            self.body.push(playback);
        }
        for (path, acs_name, playback_name) in [
            (
                "data\\anim/SkaterLeftHand.abin",
                b"SkaterLeftHand".as_slice(),
                b"SkaterLeftHand".as_slice(),
            ),
            (
                "data\\anim/SkaterRightHand.abin",
                b"SkaterRightHand".as_slice(),
                b"SkaterRightHand".as_slice(),
            ),
        ] {
            let database = self.load_database(source, path)?;
            let (clip_index, count) = {
                let retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
                let clip_index = retained.clip_index(ActorGraphIntents::native_name(b"Poses"))?;
                let count = checked_frame_count(
                    retained.parsed.as_ref().expect("loaded database").clips[clip_index]
                        .frame_count,
                )?;
                (clip_index, count)
            };
            let playback = self.register_playback(
                acs_name,
                playback_name,
                0,
                &database,
                vec![Arc::clone(&database)],
                &hand_setup(count),
                Some(source),
            )?;
            self.hands.push(Arc::clone(&playback));
            let commands = Arc::clone(&playback.lock().map_err(|_| ActorFaceError::Lock)?.commands);
            evaluate_fixed_clip(&commands, clip_index)?;
        }
        self.databases_and_fixed_hands_ready = true;
        Ok(())
    }

    pub(crate) fn load_database<S: AnimationStartupSource>(
        &mut self,
        source: &mut S,
        path: &str,
    ) -> Result<Arc<Mutex<FaceDatabase>>, AnimationStartupError<S::Error>> {
        let database = self
            .managers
            .database
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .allocate()?;
        self.databases.push(Arc::clone(&database));
        let bytes = source
            .read_animation(path)
            .map_err(AnimationStartupError::Source)?;
        {
            let mut retained = database.lock().map_err(|_| ActorFaceError::Lock)?;
            self.managers
                .database
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .set_content(&mut retained, bytes)?;
        }
        Ok(database)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Keep distinct native command and playback registration inputs explicit"
    )]
    pub(crate) fn register_playback<S: AnimationStartupSource>(
        &mut self,
        command_name: &[u8],
        playback_name: &[u8],
        player: i32,
        hierarchy: &Arc<Mutex<FaceDatabase>>,
        databases: Vec<Arc<Mutex<FaceDatabase>>>,
        setup: &AnimationCommandSetup,
        source: Option<&mut S>,
    ) -> Result<Arc<Mutex<FacePlaybackData>>, AnimationStartupError<S::Error>> {
        let commands = Arc::new(Mutex::new(FaceCommands {
            database: Arc::clone(hierarchy),
fixed_frames: Vec::new(),
            fixed_matrices: Vec::new(),
            max_frames: usize::try_from(setup.fixed_sqt)
                .map_err(|_| ActorFaceError::UnsupportedTiming)?,
            phase: FaceCommandPhase::Ready,
            scratch: None,
            selected_frames: Vec::new(),
            setup: Some(setup.clone()),
            buffered: None,
        }));
        self.pending_commands.push(Arc::clone(&commands));
        if let Some(source) = source {
            let scheduler = Arc::clone(
                &self
                    .managers
                    .command
                    .lock()
                    .map_err(|_| ActorFaceError::Lock)?
                    .scheduler,
            );
            let buffered = Arc::new(Mutex::new(BufferedBodyCommands::new(
                Arc::clone(hierarchy),
                Arc::clone(&self.managers.database),
                scheduler,
            )));
            commands.lock().map_err(|_| ActorFaceError::Lock)?.buffered =
                Some(Arc::clone(&buffered));
            buffered
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .try_construct(setup, source)?;
        }
        // NewACS824d6540: after ACS construction, before registry publication.
        commands
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .setup
            .as_mut()
            .ok_or(ActorFaceError::UnsupportedTiming)?
            .fetch_time_error = f32::from_bits(0x3ada_1019);
        self.managers
            .command
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .insert(
                (ActorGraphIntents::native_name(command_name), player),
                Arc::clone(&commands),
            );
        let playback = Arc::new(Mutex::new(FacePlaybackData {
            database: Arc::clone(hierarchy),
            databases,
            commands,
        }));
        let mut manager = self
            .managers
            .playback
            .lock()
            .map_err(|_| ActorFaceError::Lock)?;
        let key = (ActorGraphIntents::native_name(playback_name), player);
        if manager.entries.contains_key(&key) {
            return Err(ActorFaceError::DuplicatePlayback.into());
        }
        manager.entries.insert(key, Arc::clone(&playback));
        Ok(playback)
    }
}

fn body_setup(vbr: bool) -> AnimationCommandSetup {
    AnimationCommandSetup {
        capacities: [200, 100, 0, 0, 2],
        averages: [30.0, 20.0, 0.0, 0.0, 2.0],
        max_nblend: 4,
        fixed_sqt: 0,
        fixed_matrix: 0,
        mode: 1,
        fetch_mode: 1,
        category: 0,
        characters: 1,
        truncation_offset: 0.5,
        fetch_time_error: 0.0,
        trajectory: true,
        codec: u32::from(vbr),
    }
}
fn hand_setup(frames: u16) -> AnimationCommandSetup {
    AnimationCommandSetup {
        capacities: [200, 1, 1, 1, 1],
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
    }
}

#[path = "simulation_animation_mounted_source.rs"]
pub(crate) mod mounted_source;
