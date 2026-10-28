//! Selected original hand/face RAW `NewACS` allocations on the existing owner.
use super::{
    AnimationCommandSetup, AnimationJobCode, AnimationStartupSource, BindPoseError,
    BufferedBodyCommands, BufferedJobPool,
};
use crate::{
    actor_face_playback::ActorFaceError, simulation_animation_startup::AnimationStartupError,
};
use std::mem::MaybeUninit;

impl BufferedBodyCommands {
    /// `NewACS824d63e4..6430`, `FetchSys82d6f370`, `Data82a301f0`,
    /// `FetchEngine82a31de8`, `SQTEngine82a31c40`, `SetDataEngine82a31fe0`.
    pub(crate) fn try_construct_raw_stream<S: AnimationStartupSource>(
        &mut self,
        setup: &AnimationCommandSetup,
        source: &mut S,
    ) -> Result<(), AnimationStartupError<S::Error>> {
        use AnimationStartupError as E;
        if self.constructed || !self.entries.is_empty() {
            return Err(E::AlreadyAttempted);
        }
        let hands = setup.fetch_mode == 0
            && setup.category == 0
            && !setup.trajectory
            && setup.averages.map(f32::to_bits) == [25.0_f32, 1.0, 1.0, 1.0, 1.0].map(f32::to_bits)
            && matches!(setup.capacities, [200, 1, 1, 1, 1] | [100, 8, 1, 1, 1]);
        let full = setup.fetch_mode == 1
            && setup.category == 2
            && setup.trajectory
            && setup.capacities == [13, 6, 3, 1, 2]
            && setup.averages.map(f32::to_bits) == [6.0_f32, 3.0, 1.0, 1.0, 2.0].map(f32::to_bits);
        let face = setup.fetch_mode == 0
            && setup.category == 2
            && !setup.trajectory
            && setup.capacities == [2, 1, 1, 0, 0]
            && setup.averages.map(f32::to_bits) == [1.0_f32, 0.5, 0.5, 0.0, 0.0].map(f32::to_bits);
        let presentation = (full || face)
            && setup.max_nblend == 0
            && setup.fixed_sqt == 0
            && setup.fixed_matrix == 0;
        // Pedestrian InitHandAnimSys8280d5d0 / InitFaceAnimSys8280d8c0.
        let pedestrian=setup.fetch_mode==0&&setup.category==2&&!setup.trajectory
            &&setup.capacities==[12,6,0,0,0]
            &&setup.averages.map(f32::to_bits)==[4.0_f32,2.,0.,0.,0.].map(f32::to_bits)
            &&setup.max_nblend==0&&setup.fixed_sqt==0&&setup.fixed_matrix==0
            &&matches!((setup.characters,setup.truncation_offset.to_bits()),(20,0x3f00_0000)|(12,0));
        let marquee=setup.characters==3&&setup.category==2&&setup.fetch_mode==0&&setup.trajectory
            &&setup.capacities==[6,0,5,1,2]
            &&setup.averages.map(f32::to_bits)==[3.0_f32,0.,2.5,0.5,1.].map(f32::to_bits)
            &&setup.max_nblend==0&&setup.fixed_sqt==0&&setup.fixed_matrix==1;
        if setup.codec != 0 || setup.mode != 1
            || !(marquee||pedestrian||(setup.characters==9&&(hands||presentation)))
        {
            return Err(E::BindPose(BindPoseError::UnsupportedSetup));
        }
        self.nblend.configure(setup);
        self.job_category = setup.category;
        // Data82a302e0: characters * ceil(MaxMatrixCommands / 2).
        self.max_dynamic_matrices =
            usize::try_from(setup.characters * ((setup.capacities[4] + 1) >> 1))
                .map_err(|_| E::Allocation)?;
        self.max_set_replace_sqts =
            usize::try_from(if marquee{2*setup.capacities[2]*setup.characters}else{setup.capacities[2]}).map_err(|_| E::Allocation)?;
        self.load_entry(source, AnimationJobCode::FetchRaw, b"AnimFetch")?;
        let parts = self
            .hierarchy_owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .parsed
            .as_ref()
            .ok_or(ActorFaceError::MissingRecord)?
            .hierarchy
            .parts
            .len();
        self.null_job = Some(self.scheduler.create_job());
        self.load_entry(source, AnimationJobCode::ReuseBuffer, b"ACSreuseBuffer")?;
        // Native FetchSys stores numChars*MaxFetchNodes, then returns one
        // extraction record per destination part; RAW returns512 per job.
        let fetches =
            usize::try_from(setup.characters * setup.capacities[1]).map_err(|_| E::Allocation)?;
        self.extract_capacity = fetches.checked_mul(parts).ok_or(E::Allocation)?;
        self.max_extracts_per_job = 512;
        self.max_extracts_per_fetch = parts;
        // Native3237c truncates (9*max(avgFetch+avgSet,2)+8)/9;
        // Data3028c then multiplies by9. Exact admitted descriptors give36/18.
        self.max_dynamic_sqts = if marquee{9}else if pedestrian {2*setup.characters as usize} else if full { 36 } else { 18 };
        let jobs = self.extract_capacity.div_ceil(512) + 9;
        self.fetch_pool = Some(BufferedJobPool::new(
            &self.scheduler,
            jobs,
            1,
            self.extract_capacity,
        ));
        let bytes = (self.extract_capacity * 64 + jobs * 128).div_ceil(128) * 128 + 256;
        self.extract_memory
            .try_reserve_exact(bytes)
            .map_err(|_| E::Allocation)?;
        self.extract_memory.resize(bytes, MaybeUninit::uninit());
        self.load_entry(source, AnimationJobCode::SqtBlend, b"ACSsqtBlend")?;
        let operations =
            usize::try_from((setup.capacities[0] - setup.capacities[1]) * setup.characters)
                .map_err(|_| E::Allocation)?;
        let per_job = if operations > 164 {
            operations / 10 + 1
        } else {
            operations
        };
        self.blend_pool = Some(BufferedJobPool::new(&self.scheduler, 18, jobs + 1, per_job));
        self.load_entry(source, AnimationJobCode::SetData, b"ACSsetData")?;
        // Native32658 passes2*MaxSetReplace*9. All admitted counts are <=104.
        let set_operations = usize::try_from(2 * setup.capacities[2] * setup.characters)
            .map_err(|_| E::Allocation)?;
        self.set_pool = Some(BufferedJobPool::new(&self.scheduler, 12, 1, set_operations));
        // PoseEngine82a2a170 only initializes null lazy pools and this capacity.
        self.max_pose_operations =
            usize::try_from((setup.capacities[4] - setup.capacities[3]) * setup.characters)
                .map_err(|_| E::Allocation)?;
        self.constructed = true;
        Ok(())
    }
}
