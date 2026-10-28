//! Pose production on the retained tAnimData owner and shared Andale core.
use super::PedestrianAnimation;
use crate::{animation::AnimationMatrix, input_cvar_runtime::InputCvarReferences,
    pedestrian_motion_delegate::pose::SimLodPose};

impl<K, W> PedestrianAnimation<K, W> {
    /// After the real AnimMan Init registered callback2's matrix allocation.
    pub(crate) fn initialize_sim_lod_pose(&mut self, registered_matrix_id: u32) -> Result<(), String> {
        if self.sim_lod_pose.is_some() { return Err("pedestrian SimLOD pose already initialized".into()); }
        self.sim_lod_pose = Some(SimLodPose::new(&self.core, registered_matrix_id)?);
        Ok(())
    }

    /// SetUpSimLODAnimation82e6ea28's LOSim2/non-Marquee branch. The shared
    /// AnimMan command set is already begun; its caller submits and syncs once.
    pub(crate) fn queue_sim_lod_pose(&mut self, update_old_time: bool,
        blend_cull_threshold: f32, cvars: Option<InputCvarReferences>) -> Result<(), String> {
        self.sim_lod_pose.as_ref().ok_or("pedestrian SimLOD pose absent")?
            // SetUpSimLODAnimation82e6ea80: FETCHBLEND is enum value1,
            // not a count of the two sampled frames.
            .queue(&mut self.core, update_old_time, 1, blend_cull_threshold, cvars)
    }

    /// UpdateTargetTransform82e7fc94: after the final SetTargetTransform and
    /// before hand-prop placement. A failed/incomplete evaluation does not
    /// replace the last completed global-bone allocation with a default pose.
    pub(crate) fn update_sim_lod_global(&mut self, final_target: AnimationMatrix) -> Result<(), String> {
        let global = self.sim_lod_pose.as_ref().ok_or("pedestrian SimLOD pose absent")?
            .global_bones(final_target)?;
        self.sim_lod_bone_global = Some(global);
        Ok(())
    }

    /// Native SetUpSimLODAnimation chooses full extraction for LO!=2 or a
    /// Marquee model. Both paths feed the same retained simulation pose.
    pub(crate) fn queue_simulation_pose(&mut self, losim: u32, marquee: bool,
        update_old_time: bool, blend_cull_threshold: f32, cvars: Option<InputCvarReferences>,
        scratch: &crate::pedestrian_motion_delegate::pose::FullSkeletonScratch) -> Result<(), String> {
        if losim == 2 && !marquee {
            self.queue_sim_lod_pose(update_old_time, blend_cull_threshold, cvars)
        } else {
            self.sim_lod_pose.as_ref().ok_or("pedestrian SimLOD pose absent")?
                .queue_full(&mut self.core, update_old_time, blend_cull_threshold, cvars, scratch)
        }
    }
}
