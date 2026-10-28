//! Original pedestrian playback startup through the shared Andale owners.
//! Render snapshots are published only after actual animation evaluation.
use crate::{
    actor_face_playback::{
        ActorFaceError, AnimationCommandSetup, AnimationPoseRef, FaceDatabase, FacePlaybackData,
        presentation::{PresentationAnimation, PresentationAnimationError},
    },
    actor_graph_construction::ActorGraphIntents,
    simulation_animation_startup::{AnimationStartupSource, SimulationAnimationStartup},
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

pub(crate) type PedestrianPlaybackError<E> = PresentationAnimationError<E, Infallible>;

#[path = "selected_living_world_pedestrian_hands_face.rs"]
pub(crate) mod hands_face;
#[path = "selected_living_world_pedestrian_marquee.rs"]
pub(crate) mod marquee;

/// Retains original SIM/PRES playback identities in their existing registries.
/// Both registries share the existing global database manager.
#[derive(Default)]
pub(crate) struct PedestrianPlaybackStartup {
    /// AnimMan+52, zeroed by Manager constructor82721ef4.
    pub(crate) active_motion_pool: Mutex<u32>,
    attempted: bool,
    pub(crate) full_database: Option<Arc<Mutex<FaceDatabase>>>,
    pub(crate) simulation_database: Option<Arc<Mutex<FaceDatabase>>>,
    pub(crate) simulation: Option<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) trajectory: Option<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) presentation: Option<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) hands_face: Option<hands_face::RegularHandFace>,
    pub(crate) marquee: Option<marquee::MarqueeStartup>,
    pub(crate) marquee_hands_face: Option<hands_face::RegularHandFace>,
    pub(crate) regular_inverse_bind: Mutex<Option<Arc<[crate::animation::AnimationMatrix]>>>,
    pub(crate) default_inverse_bind: Option<Arc<[crate::animation::AnimationMatrix]>>,
    pub(crate) bind_pose: Option<AnimationPoseRef>,
    /// AnimMan+16; callback2 returns this registered fixed-matrix ID.
    pub(crate) simulation_fixed_matrix_id: Option<u32>,
    pub(crate) simulation_scratch: Option<crate::pedestrian_motion_delegate::pose::FullSkeletonScratch>,
    pub(crate) ready: bool,
}

impl PedestrianPlaybackStartup {
    /// `AnimMan::Init82718d08`, `InitRegularPedestrianAnimSys8280cd40`.
    /// Database allocations and registry writes remain retained after failure.
    pub(crate) fn initialize<S: AnimationStartupSource>(
        &mut self,
        simulation: &mut SimulationAnimationStartup,
        presentation: &mut PresentationAnimation,
        source: &mut S,
    ) -> Result<(), PedestrianPlaybackError<S::Error>> {
        use PresentationAnimationError as E;
        if self.attempted {
            return Err(E::AlreadyAttempted);
        }
        self.attempted = true;
        if !Arc::ptr_eq(
            &simulation.managers.database,
            &presentation.managers.database,
        ) {
            return Err(E::Missing("shared pedestrian database manager"));
        }
        let full = simulation
            .load_database(source, "data\\anim/PedestrianSkeletonPres.abin")
            .map_err(E::Startup)?;
        self.full_database = Some(Arc::clone(&full));
        let sim = simulation
            .load_database(source, "data\\anim/PedestrianSkeletonSim.abin")
            .map_err(E::Startup)?;
        self.simulation_database = Some(Arc::clone(&sim));
        let playback = simulation
            .register_playback(
                b"TrajPedestrianACS",
                b"pedestrian",
                0,
                &sim,
                vec![Arc::clone(&full)],
                &command_setup(false),
                Some(source),
            )
            .map_err(E::Startup)?;
        self.simulation = Some(Arc::clone(&playback));
        // The original full playback uses PRES hierarchy while its ACS uses
        // SIM hierarchy. They deliberately do not share this field value.
        playback.lock().map_err(|_| ActorFaceError::Lock)?.database = Arc::clone(&full);
        let pose = playback
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .get_pose(ActorGraphIntents::native_name(b"Pedestrian_rig_Tpose"))?
            .ok_or(ActorFaceError::MissingRecord)?;
        self.bind_pose = Some(pose.clone());
        let bone_count = |database: &Arc<Mutex<FaceDatabase>>| -> Result<usize, ActorFaceError> {
            Ok(database.lock().map_err(|_| ActorFaceError::Lock)?.parsed.as_ref()
                .ok_or(ActorFaceError::MissingRecord)?.hierarchy.parents.len())
        };
        self.simulation_scratch = Some(crate::pedestrian_motion_delegate::pose::FullSkeletonScratch::new(
            bone_count(&full)?, bone_count(&sim)?)?);
        register_bind_pose(&playback, &pose)?;
        let commands = Arc::clone(&playback.lock().map_err(|_| ActorFaceError::Lock)?.commands);
        self.simulation_fixed_matrix_id = Some(
            crate::pedestrian_motion_delegate::pose::register_sim_lod_identity(&commands)?);
        let trajectory = Arc::new(Mutex::new(FacePlaybackData {
            database: Arc::clone(&sim),
            databases: vec![Arc::clone(&sim)],
            commands,
        }));
        {
            let bones = sim
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .parsed
                .as_ref()
                .ok_or(ActorFaceError::MissingRecord)?
                .hierarchy
                .bone_names
                .len();
            if bones > 20 {
                return Err(E::Missing("pedestrian SIM hierarchy <=20 bones"));
            }
            let key = (ActorGraphIntents::native_name(b"pedestriantrajonly"), 0);
            let mut manager = simulation
                .managers
                .playback
                .lock()
                .map_err(|_| ActorFaceError::Lock)?;
            if manager.entries.contains_key(&key) {
                return Err(ActorFaceError::DuplicatePlayback.into());
            }
            manager.entries.insert(key, Arc::clone(&trajectory));
        }
        self.trajectory = Some(trajectory);
        let pres = presentation.register_playback::<S, Infallible>(
            source,
            &full,
            b"FullPedestrianACS",
            b"pedestrian",
            &command_setup(true),
        )?;
        self.presentation = Some(Arc::clone(&pres));
        register_bind_pose(&pres, &pose)?;
        self.default_inverse_bind=Some(vec![[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.];bone_count(&full)?].into());
        pres.lock().map_err(|_|ActorFaceError::Lock)?.commands.lock().map_err(|_|ActorFaceError::Lock)?
            .register_fixed_matrix(self.default_inverse_bind.as_ref().expect("initialized"))
            .map_err(|_|E::Missing("regular default inverse-bind registration"))?;
        self.marquee=Some(marquee::MarqueeStartup::initialize(simulation,presentation,source,&full)?);
        self.hands_face=Some(hands_face::RegularHandFace::initialize(simulation,presentation,source,&full)?);
        self.marquee_hands_face=Some(hands_face::RegularHandFace::initialize_marquee(simulation,presentation,source,
            &self.marquee.as_ref().expect("initialized").database)?);
        self.ready = true;
        Ok(())
    }
}

fn register_bind_pose(
    playback: &Arc<Mutex<FacePlaybackData>>,
    pose: &AnimationPoseRef,
) -> Result<(), ActorFaceError> {
    let commands = Arc::clone(&playback.lock().map_err(|_| ActorFaceError::Lock)?.commands);
    let mut commands = commands.lock().map_err(|_| ActorFaceError::Lock)?;
    let hierarchy = commands.database.lock().map_err(|_| ActorFaceError::Lock)?.parsed.as_ref()
        .ok_or(ActorFaceError::MissingRecord)?.hierarchy.clone();
    let values = pose.constant_vbr_sqt_for(&hierarchy)?;
    if !commands.fixed_frames.is_empty() {
        return Err(ActorFaceError::UnsupportedPose);
    }
    commands.max_frames = 1;
    commands.fixed_frames.push(values);
    Ok(())
}

/// ACSParams64-byte PDB layout and PPC stores82718ed0..f34/8280cdb0..ce30.
fn command_setup(presentation: bool) -> AnimationCommandSetup {
    AnimationCommandSetup {
        capacities: [12, 4, 2, 0, 1],
        averages: [3.6, 1.2, 0.6, 0.0, 1.0],
        max_nblend: 2,
        fixed_sqt: 1,
        fixed_matrix: if presentation { 2 } else { 1 },
        mode: 1,
        fetch_mode: 0,
        category: if presentation { 2 } else { 0 },
        characters: 40,
        truncation_offset: 0.0,
        fetch_time_error: 0.0,
        trajectory: true,
        codec: 1,
    }
}
