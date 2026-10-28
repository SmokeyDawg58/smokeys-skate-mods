//! Animatable DispatchBSJob/WaitForBSJob/PostBSJob on the retained local backend.
use super::ActorAnimationConstruction;
use crate::{
    actor_animation_tree::blend_space::jobs::{
        BlendSpaceJobBuffer, BlendSpaceJobError, BlendSpacePool, BlendSpaceWork,
    },
    animation_job_owners::{
        AnimationSchedulerBackend, DormantAnimationJob, execution::AnimationQueuedJob,
    },
    input_cvar_runtime::InputCvarReferences,
};
use std::sync::{Arc, Mutex};
impl<K, W> ActorAnimationConstruction<K, W> {
    pub(crate) fn blend_space_buffer(
        &self,
    ) -> Result<Arc<Mutex<BlendSpaceJobBuffer>>, BlendSpaceJobError> {
        self.base
            .as_ref()
            .and_then(|base| base.blend_space_data.as_ref())
            .cloned()
            .ok_or(BlendSpaceJobError::Missing("Init BlendSpace data"))
    }
    fn blend_space_job_owner(
        &self,
    ) -> Result<
        (
            Arc<AnimationSchedulerBackend>,
            Arc<Mutex<DormantAnimationJob>>,
        ),
        BlendSpaceJobError,
    > {
        use BlendSpaceJobError as E;
        let initialized = self
            .initialization
            .as_ref()
            .ok_or(E::Missing("Initialize"))?;
        let entry = initialized
            .entry_point
            .as_ref()
            .ok_or(E::Missing("BlendSpace entry point"))?;
        let backend = entry
            .manager
            .as_ref()
            .and_then(|manager| manager.scheduler.as_ref())
            .ok_or(E::Missing("initialized job manager"))?;
        let job = initialized
            .job
            .as_ref()
            .ok_or(E::Missing("BlendSpace job"))?;
        Ok((Arc::clone(backend), Arc::clone(job)))
    }
    pub(crate) fn dispatch_blend_space(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        let buffer = self.blend_space_buffer()?;
        if buffer.lock().map_err(|_| E::Lock)?.records.is_empty() {
            return Ok(());
        }
        let (backend, job) = self.blend_space_job_owner()?;
        let entry = self
            .initialization
            .as_ref()
            .and_then(|state| state.entry_point.as_ref())
            .and_then(|entry| entry.value.as_ref())
            .ok_or(E::Missing("BlendSpace code"))?
            .clone();
        job.lock().map_err(|_| E::Lock)?.entry_point = entry;
        // Native Param0/Param1 are pointers; retain actual typed buffer/count
        // owner as work, never truncate host addresses into placeholder words.
        let work = Arc::new(BlendSpaceWork {
            buffer,
            cvars: cvars.clone(),
        });
        backend
            .submit_blend_space(&[AnimationQueuedJob {
                job,
                dependencies: vec![],
                work: Some(work),
            }])
            .map_err(E::Scheduler)?;
        self.base
            .as_mut()
            .expect("retained base")
            .blend_space_job_started = true;
        Ok(())
    }
    pub(crate) fn wait_blend_space(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        if !self
            .base
            .as_ref()
            .ok_or(E::Missing("Animatable"))?
            .blend_space_job_started
        {
            return Ok(());
        }
        let (backend, job) = self.blend_space_job_owner()?;
        if let Err(error) = backend.wait_in_category(&job, 0) {
            let failure = self
                .blend_space_buffer()?
                .lock()
                .map_err(|_| E::Lock)?
                .failure
                .clone();
            return Err(failure.map_or_else(|| E::Scheduler(error), E::Worker));
        }
        self.post_blend_space(cvars)?;
        backend
            .release_selected(&[AnimationQueuedJob {
                job,
                dependencies: vec![],
                work: None,
            }])
            .map_err(E::Scheduler)
    }
    fn post_blend_space(&mut self, cvars: &InputCvarReferences) -> Result<(), BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        let buffer = self.blend_space_buffer()?;
        let mut index = 0;
        loop {
            let record = buffer
                .lock()
                .map_err(|_| E::Lock)?
                .records
                .get(index)
                .cloned();
            let Some(record) = record else {
                break;
            };
            let initialized = self
                .initialization
                .as_mut()
                .ok_or(E::Missing("Initialize"))?;
            let pool = match record.pool {
                BlendSpacePool::Body(index) => initialized.tree_pools.get_mut(index),
                BlendSpacePool::Channel(index) => initialized
                    .channels
                    .as_mut()
                    .and_then(|channels| channels.trees.get_mut(index)),
            }
            .ok_or(E::Missing("retained BlendSpace pool"))?;
            let node = pool.runtime.blend_space(&record.node).map_err(E::Tree)?;
            if !Arc::ptr_eq(&node.parameters, &record.parameters) {
                return Err(E::Missing("retained BlendSpace occurrence"));
            }
            pool.runtime.complete_blend_space3(&record.node, cvars)?;
            index += 1;
        }
        buffer.lock().map_err(|_| E::Lock)?.records.clear();
        self.base
            .as_mut()
            .expect("retained base")
            .blend_space_job_started = false;
        Ok(())
    }
}
