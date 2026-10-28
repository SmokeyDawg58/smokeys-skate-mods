//! Original forty-pedestrian ACS descriptor on the existing command/job owner.
use super::{
    AnimationCommandSetup, AnimationJobCode, AnimationStartupSource, BindPoseError,
    BufferedBodyCommands, BufferedJobPool,
};
use crate::{
    actor_face_playback::ActorFaceError, simulation_animation_startup::AnimationStartupError,
};
use std::mem::MaybeUninit;

impl BufferedBodyCommands {
    /// TrajPedestrianACS82718d08 / FullPedestrianACS8280cd40;
    /// shared ACS82a32168, FetchEngine82a31de8, SQT82a31c40, Set82a31fe0.
    pub(crate) fn try_construct_pedestrian_stream<S: AnimationStartupSource>(
        &mut self,
        setup: &AnimationCommandSetup,
        source: &mut S,
    ) -> Result<(), AnimationStartupError<S::Error>> {
        use AnimationStartupError as E;
        if self.constructed || !self.entries.is_empty() {
            return Err(E::AlreadyAttempted);
        }
        let category = matches!((setup.category, setup.fixed_matrix), (0, 1) | (2, 2));
        if setup.codec != 1
            || setup.mode != 1
            || setup.fetch_mode != 0
            || setup.characters != 40
            || setup.capacities != [12, 4, 2, 0, 1]
            || setup.averages.map(f32::to_bits) != [3.6_f32, 1.2, 0.6, 0.0, 1.0].map(f32::to_bits)
            || setup.max_nblend != 2
            || setup.fixed_sqt != 1
            || !category
            || !setup.trajectory
            || setup.truncation_offset.to_bits() != 0
        {
            return Err(E::BindPose(BindPoseError::UnsupportedSetup));
        }
        self.nblend.configure(setup);
        self.job_category = setup.category;
        self.max_dynamic_matrices = 40;
        self.max_set_replace_sqts = 2;
        self.load_entry(source, AnimationJobCode::FetchVbr, b"AnimFetchVBR")?;
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
        self.extract_capacity = 160_usize.checked_mul(parts).ok_or(E::Allocation)?;
        self.max_extracts_per_job = 256;
        self.max_extracts_per_fetch = parts;
        // Data's per-entity ceil((40*max(1.2+.6,2)+39)/40)=2, times40.
        self.max_dynamic_sqts = 80;
        let jobs = self.extract_capacity.div_ceil(256) + 9;
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
        // (12-4)*40=320; >164 becomes320/10+1.
        self.blend_pool = Some(BufferedJobPool::new(&self.scheduler, 18, jobs + 1, 33));
        self.load_entry(source, AnimationJobCode::SetData, b"ACSsetData")?;
        // 2*2*40=160; >104 becomes(160>>2)+1.
        self.set_pool = Some(BufferedJobPool::new(&self.scheduler, 12, 1, 41));
        self.max_pose_operations = 40;
        self.constructed = true;
        Ok(())
    }
}
