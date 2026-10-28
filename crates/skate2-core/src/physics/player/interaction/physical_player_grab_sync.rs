//! PlayerGrabSpline::Sync82dd0110 and its retained reach-validation batch.
use super::{GrabQueryBuffer, PlayerGrabSpline};
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric, input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld, physical_player_grind_manager::math::GrindNum,
    physical_player_trajectory_owners::selector_grind::closest_point_on_line,
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
    skeleton_root_transforms::SkeletonRootVector, static_ground_pool::StaticGroundPool,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GrabSyncError {
    QueryPoisoned,
    Missing(&'static str),
    Service(String),
}
impl std::fmt::Display for GrabSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "grab spline sync: {self:?}")
    }
}
fn service(e: impl std::fmt::Display) -> GrabSyncError {
    GrabSyncError::Service(e.to_string())
}

/// Borrow the same World and ground used by all other player line queries.
/// Only an active query needs these services; idle Sync reads no tuning.
impl PlayerGrabSpline {
    pub(crate) fn try_sync_before_state_update(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        cvars: &InputCvarReferences,
        num: &mut GrindNum<'_>,
        collision: &mut HostCollisionLineNumeric,
    ) -> Result<(), GrabSyncError> {
        self.flags &= !0x40;
        if let Some(batch) = self.line_batch.as_ref() {
            let (lines, _) = world.world_line_owners();
            let lines = lines.ok_or(GrabSyncError::Missing("World line queries"))?;
            lines.try_sync(batch).map_err(service)?;
            if self.flags & 0x80 != 0 {
                process_line_results(
                    &mut self.pending_validation,
                    || {
                        cvars
                            .try_grab_line_test_count()
                            .map(|v| v as u32)
                            .map_err(service)
                    },
                    |index| {
                        lines
                            .try_get_result(batch, index)
                            .map(|hit| hit.map(|v| v.owner))
                            .map_err(service)
                    },
                )?;
            }
            lines.try_release(batch).map_err(service)?;
            self.line_batch = None;
            if self.flags & 0x80 != 0 {
                let mut future = self
                    .query
                    .lock()
                    .map_err(|_| GrabSyncError::QueryPoisoned)?;
                if future.ready {
                    fix_old_positions(&mut self.pending_validation, &mut future.result);
                }
            }
            copy_query(&mut self.validated, &mut self.pending_validation);
            self.flags = ((self.flags >> 1) & 0x40) | (self.flags & 0x3f);
            self.pending_position = [0.0; 4];
        }
        let spawn = {
            let mut future = self
                .query
                .lock()
                .map_err(|_| GrabSyncError::QueryPoisoned)?;
            if future.ready {
                copy_query(&mut self.pending_validation, &mut future.result);
                self.pending_position = self.position;
                future.ready = false;
                self.flags |= 0x80;
                self.position = [0.0; 4];
                true
            } else {
                false
            }
        };
        if spawn {
            self.spawn_line_batch(world, ground, cvars, num, collision)?;
        }
        Ok(())
    }

    /// SpawnLineBatch82dcbcd0; AcquireLineBatch(name)827b1a58 uses BF_Default=7.
    fn spawn_line_batch(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        cvars: &InputCvarReferences,
        num: &mut GrindNum<'_>,
        collision: &mut HostCollisionLineNumeric,
    ) -> Result<(), GrabSyncError> {
        let (lines, dmos) = world.world_line_owners();
        let lines = lines.ok_or(GrabSyncError::Missing("World line queries"))?;
        let dmos = dmos.ok_or(GrabSyncError::Missing("World DMO pool"))?;
        if let Some(batch) = self.line_batch.as_ref() {
            lines.try_release(batch).map_err(service)?;
            self.line_batch = None;
        }
        self.line_batch = Some(
            lines
                .try_acquire_line_batch(dmos, "PlayerGrabSpline::SplineReachableTest", 7)
                .map_err(service)?,
        );
        let batch = self.line_batch.as_ref().expect("acquired above");
        lines.try_begin(batch).map_err(service)?;
        self.pending_validation.cursor = 0;
        while let Some(index) = next_index(&mut self.pending_validation) {
            let spline = &self.pending_validation.splines[index];
            let mut end = closest_point_on_line(self.pending_position, spline.p1, spline.p2, num)
                .map_err(service)?;
            let radius = cvars.try_grab_line_test_radius().map_err(service)?;
            let offset = cvars
                .try_grab_line_test_vertical_offset()
                .map_err(service)?;
            let count = cvars.try_grab_line_test_count().map_err(service)?;
            // Raw82dcbe0c: f12 = radius * 2; v126 retains that diameter.
            let spacing = radius * 2.0;
            let mut y = (self.pending_position[1] - (count / 2) as f32 * spacing) + offset;
            if count % 2 == 0 {
                y -= spacing * 0.5;
            }
            let mut start = self.pending_position;
            let mut test = 0u32;
            while test < cvars.try_grab_line_test_count().map_err(service)? as u32 {
                start[1] = y;
                end[1] = y;
                // Native ignores AddTest's capacity refusal; the batch owns it.
                let _ = lines
                    .try_add_line(
                        batch,
                        CollisionLineInput {
                            start: SkeletonRootVector::known(start),
                            end: SkeletonRootVector::known(end),
                            fatness: cvars.try_grab_line_test_radius().map_err(service)?,
                        },
                    )
                    .map_err(service)?;
                test = test.wrapping_add(1);
                y += spacing;
            }
        }
        lines
            .try_end(batch, ground, dmos, collision)
            .map_err(service)
    }
}

fn next_index<const N: usize>(buffer: &mut GrabQueryBuffer<N>) -> Option<usize> {
    if buffer.cursor >= buffer.result_count {
        return None;
    }
    let index = buffer.cursor as usize;
    buffer.cursor += 1;
    Some(index)
}

/// CopyGrabSplineQuery82dcf1f0/82dcf2a0 also advances the SOURCE cursor.
/// At capacity it consumes the rejected item and fetches the next before exit.
fn copy_query<const D: usize, const S: usize>(
    destination: &mut GrabQueryBuffer<D>,
    source: &mut GrabQueryBuffer<S>,
) {
    destination.result_count = 0;
    destination.cursor = 0;
    source.cursor = 0;
    let mut next = next_index(source);
    while let Some(index) = next {
        let added = destination.result_count < D as u32;
        if added {
            destination.splines[destination.result_count as usize] = source.splines[index].clone();
            destination.result_count += 1;
        }
        next = next_index(source);
        if !added {
            break;
        }
    }
}

/// ProcessLineBatch82dcbf80. The extra index increment at82dcc0fc and
/// swap-removal are both present in the raw instructions; retain that order.
fn process_line_results(
    buffer: &mut GrabQueryBuffer<5>,
    mut count: impl FnMut() -> Result<u32, GrabSyncError>,
    mut result: impl FnMut(u32) -> Result<Option<Option<SimulationDmoAssemblyHandle>>, GrabSyncError>,
) -> Result<(), GrabSyncError> {
    buffer.cursor = 0;
    let mut test_index = 0u32;
    while let Some(index) = next_index(buffer) {
        let mut blocked = false;
        let mut test = 0;
        while test < count()? {
            if let Some(owner) = result(test_index)? {
                blocked = owner.as_ref().is_none_or(|hit| {
                    buffer.splines[index]
                        .assembly
                        .as_ref()
                        .is_some_and(|grab| !grab.same_as(hit))
                });
            }
            test += 1;
            test_index = test_index.wrapping_add(1);
            if blocked {
                break;
            }
        }
        if blocked {
            buffer.result_count -= 1;
            buffer.cursor -= 1;
            buffer.splines[buffer.cursor as usize] =
                buffer.splines[buffer.result_count as usize].clone();
        }
        test_index = test_index.wrapping_add(1);
    }
    Ok(())
}

/// FixUpOldPositions82dcc110 refreshes matching descriptors from the NEW future
/// after validating the old batch. It does not reject an absent new match.
fn fix_old_positions(old: &mut GrabQueryBuffer<5>, new: &mut GrabQueryBuffer<32>) {
    old.cursor = 0;
    let mut next = next_index(old);
    let _ = next_index(new);
    while let Some(index) = next {
        new.cursor = 0;
        while let Some(fresh) = next_index(new) {
            let a = &old.splines[index];
            let b = &new.splines[fresh];
            if a.descriptor_type == b.descriptor_type && a.descriptor_object == b.descriptor_object
            {
                old.splines[index] = b.clone();
                break;
            }
        }
        next = next_index(old);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_world_empty_query_validates_next_sync_and_invalidation_releases_batch() {
        use crate::input_cvar_runtime::InputCvarManager;
        use crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig;
        let (mut world, owner) =
            crate::physical_player_assembly::tests::issued_owner_for_grind_tests();
        let cvars = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&cvars).unwrap();
        world
            .try_initialize_world_line_queries(
                SimulationWorldLineQueryConfig::try_sample_selected(&cvars, 3).unwrap(),
                false,
            )
            .unwrap();
        let query_manager = std::sync::Arc::new(super::super::manager::GrabQueryManager::new(
            owner.manager(),
        ));
        let mut grab = PlayerGrabSpline::new_prefix(&owner, Some(query_manager));
        let ground = StaticGroundPool::try_new(1).unwrap();
        let mut n = crate::reckoning_numeric::HostReckoningNumeric;
        let mut s = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
        let mut num = GrindNum {
            n: &mut n,
            s: &mut s,
        };
        let mut collision = HostCollisionLineNumeric;
        grab.flags = 0x15;
        grab.query.lock().unwrap().ready = true;
        grab.position = [1.0, 2.0, 3.0, 4.0];
        grab.try_sync_before_state_update(&mut world, &ground, &cvars, &mut num, &mut collision)
            .unwrap();
        assert_eq!(grab.flags, 0x95);
        assert!(!grab.query_is_ready());
        assert_eq!(grab.pending_position, [1.0, 2.0, 3.0, 4.0]);
        assert!(grab.line_batch.is_some());
        grab.try_sync_before_state_update(&mut world, &ground, &cvars, &mut num, &mut collision)
            .unwrap();
        assert_eq!(grab.flags, 0x55);
        assert!(grab.query_is_ready());
        assert_eq!(grab.validated.result_count, 0);
        assert!(grab.line_batch.is_none());
        assert_eq!(grab.pending_position, [0.0; 4]);
        grab.query.lock().unwrap().ready = true;
        grab.try_sync_before_state_update(&mut world, &ground, &cvars, &mut num, &mut collision)
            .unwrap();
        assert!(grab.line_batch.is_some());
        grab.try_invalidate_query(world.world_line_owners().0)
            .unwrap();
        assert_eq!(grab.flags, 0x15);
        assert!(grab.line_batch.is_none());
    }
    #[test]
    fn copy_preserves_native_source_cursor_after_capacity_refusal() {
        let mut source = GrabQueryBuffer::<32>::in_sim_allocation();
        source.result_count = 10;
        for (i, spline) in source.splines.iter_mut().enumerate() {
            spline.descriptor_object = i as i32;
        }
        let mut dest = GrabQueryBuffer::<5>::in_sim_allocation();
        copy_query(&mut dest, &mut source);
        assert_eq!((dest.result_count, dest.cursor, source.cursor), (5, 0, 7));
        assert_eq!(dest.splines[4].descriptor_object, 4);
    }
    #[test]
    fn obstruction_swaps_last_candidate_and_keeps_native_result_indices() {
        let mut buffer = GrabQueryBuffer::<5>::in_sim_allocation();
        buffer.result_count = 3;
        for i in 0..3 {
            buffer.splines[i].descriptor_object = i as i32;
        }
        let mut indices = Vec::new();
        process_line_results(
            &mut buffer,
            || Ok(3),
            |index| {
                indices.push(index);
                Ok(if index == 0 { Some(None) } else { None })
            },
        )
        .unwrap();
        assert_eq!(indices, [0, 2, 3, 4, 6, 7, 8]);
        assert_eq!(buffer.result_count, 2);
        assert_eq!(buffer.splines[0].descriptor_object, 2);
        assert_eq!(buffer.splines[1].descriptor_object, 1);
    }
    #[test]
    fn reach_test_distinguishes_ground_target_and_another_actual_assembly() {
        use crate::simulation_dmo_assembly_part_session::dynamic_pool::{
            SimulationDmoAssemblyCandidate, SimulationDmoAssemblyPool, tests::fixture,
        };
        let (mut backend, mut manager, owner) = fixture(5, 1);
        let mut pool = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
        let mut insert = || {
            pool.try_insert(
                &mut manager,
                &mut backend,
                SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap(),
                0,
                &mut crate::serialised_assembly_bbox::HostSerialisedAssemblyBBoxNumeric,
            )
            .unwrap()
        };
        let target = insert();
        let other = insert();
        assert_eq!(target, target.clone());
        assert_ne!(target, other);
        for (grab, hit, expected) in [
            (Some(target.clone()), Some(target.clone()), 1),
            (Some(target.clone()), Some(other.clone()), 0),
            (Some(target), None, 0),
            (None, Some(other), 1),
        ] {
            let mut buffer = GrabQueryBuffer::<5>::in_sim_allocation();
            buffer.result_count = 1;
            buffer.splines[0].assembly = grab;
            process_line_results(&mut buffer, || Ok(1), |_| Ok(Some(hit.clone()))).unwrap();
            assert_eq!(buffer.result_count, expected);
        }
    }
    #[test]
    fn refresh_matches_both_descriptor_words_and_keeps_missing_candidates() {
        let mut old = GrabQueryBuffer::<5>::in_sim_allocation();
        let mut new = GrabQueryBuffer::<32>::in_sim_allocation();
        old.result_count = 2;
        old.splines[0].descriptor_type = 2;
        old.splines[0].descriptor_object = 8;
        old.splines[1].descriptor_object = 9;
        old.splines[1].p1 = [9.0; 4];
        new.result_count = 2;
        new.splines[0].descriptor_type = 1;
        new.splines[0].descriptor_object = 8;
        new.splines[1].descriptor_type = 2;
        new.splines[1].descriptor_object = 8;
        new.splines[1].p1 = [42.0; 4];
        fix_old_positions(&mut old, &mut new);
        assert_eq!(old.splines[0].p1, [42.0; 4]);
        assert_eq!(old.splines[1].p1, [9.0; 4]);
        assert_eq!((old.cursor, new.cursor), (2, 2));
    }
}
