//! Deferred interactable fallback:827b06e8,827ae980,827ac108.
use super::super::InteractableFuture;
use super::GrabQueryManager;
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_grind_manager::{
        math::{J, madd, scale},
        view::{GrindError, error, need},
    },
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineBatch, SimulationWorldLineQueries,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

pub(super) struct InteractableBatch {
    batch: SimulationWorldLineBatch,
    // Begin clears the batch, and only these AddTest calls append to it.
    // Count successful appends, not requested tests (capacity may refuse).
    count: u32,
}

impl GrabQueryManager {
    pub(crate) fn start_interactable_query(
        &self,
        output: &Arc<Mutex<InteractableFuture>>,
        transform: [[f32; 4]; 4],
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
    ) -> Result<(), GrindError> {
        let mut state = self.state.lock().map_err(error)?;
        output.lock().map_err(error)?.ready = false;
        // The original fixed-vector accepts only the first request that tick.
        if !state.interactable_queries.is_empty() {
            return Ok(());
        }
        state.interactable_queries.push(Arc::clone(output));
        let count = refs.try_interactable_line_count().map_err(error)? as u32;
        if count == 0 || state.line_batch.is_some() {
            return Ok(());
        }
        // Unsigned count -> float (fcfid/frsp), then scalar fdivs, not vrefp.
        let step = scale(
            J,
            refs.try_interactable_max_height().map_err(error)? / count as f32,
        );
        let overlay = world.debug_overlay.clone();
        let (lines, dmos) = world.world_line_owners();
        let lines = need(lines, "interactable World line queries")?;
        let dmos = need(dmos, "interactable DMO pool")?;
        state.line_batch = Some(InteractableBatch {
            batch: lines
                .try_acquire_line_batch(
                    dmos,
                    "GrabSplineQueryManager::AddDeferredInteractableObjectQuery",
                    2,
                )
                .map_err(error)?,
            count: 0,
        });
        let retained = state.line_batch.as_mut().unwrap();
        lines.try_begin(&retained.batch).map_err(error)?;
        for i in 0..count {
            let start = madd(step, i as f32, transform[3]);
            let end = madd(
                transform[2],
                refs.try_interactable_distance().map_err(error)?,
                start,
            );
            let accepted = lines
                .try_add_line(
                    &retained.batch,
                    CollisionLineInput {
                        start: SkeletonRootVector::known(start),
                        end: SkeletonRootVector::known(end),
                        fatness: refs.try_interactable_fatness().map_err(error)?,
                    },
                )
                .map_err(error)?;
            retained.count += u32::from(accepted);
            if refs.try_draw_interactable_lines().map_err(error)? {
                // Native debug drawing is folded out; only singleton assertion remains.
                overlay.require().map_err(error)?;
            }
        }
        lines
            .try_end(&retained.batch, ground, dmos, &mut HostCollisionLineNumeric)
            .map_err(error)
    }

    /// Runs after box queries and spline data requests in Manager.Update.
    pub(crate) fn update_deferred_interactable(
        &self,
        mut lines: Option<&mut SimulationWorldLineQueries>,
        mut movable_owner: impl FnMut(&SimulationDmoAssemblyHandle) -> Result<Option<u32>, GrindError>,
    ) -> Result<(), GrindError> {
        let mut state = self.state.lock().map_err(error)?;
        for future in &state.interactable_queries {
            let mut future = future.lock().map_err(error)?;
            future.result_count = 0;
            future.cursor = 0;
            if let Some(retained) = state.line_batch.as_ref() {
                let lines = need(lines.as_deref_mut(), "interactable World line queries")?;
                lines.try_sync(&retained.batch).map_err(error)?;
                let mut best_distance = f32::MAX;
                let mut best_owner = 0;
                for i in 0..retained.count {
                    if let Some(hit) = lines.try_get_result(&retained.batch, i).map_err(error)? {
                        if hit.line_parameter != 0.0 && hit.line_parameter < best_distance {
                            if let Some(owner) = hit.owner.as_ref() {
                                if let Some(id) = movable_owner(owner)? {
                                    best_distance = hit.line_parameter;
                                    best_owner = id;
                                }
                            }
                        }
                    }
                }
                if best_owner != 0 {
                    future.result_count = 1;
                    future.object_id = best_owner;
                    future.ready = true;
                }
                // No-hit deliberately does not publish ready=true.
                lines.try_release(&retained.batch).map_err(error)?;
            }
        }
        state.line_batch = None;
        state.interactable_queries.clear();
        Ok(())
    }
}
