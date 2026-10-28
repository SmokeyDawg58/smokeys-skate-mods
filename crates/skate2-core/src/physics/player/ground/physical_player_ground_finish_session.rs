//! Actual Ground suffix consumes the completed force-to-hook prediction token.
use super::{
    ground_finish::{GroundFinishError, GroundFinishSource, try_finish_ground},
    ground_force_prediction_session::GroundBeforePushExit,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_grab_spline::invalidate::GrabInvalidateError,
    physical_player_physics_state::CurrentPhysicalState,
};
use std::sync::Arc;
#[path = "physical_player_ground_grab_results.rs"]
mod grab_results;
pub(crate) struct GroundUpdateComplete(pub(crate) GroundBeforePushExit);
#[derive(Debug)]
pub(crate) enum GroundFinishRuntimeError {
    Missing(&'static str),
    Poisoned,
    Foreign,
    Pair(SimulationSkeletonJumpPairError),
    Grab(GrabInvalidateError),
    Search(crate::physical_player_grind_manager::view::GrindError),
}
impl std::fmt::Display for GroundFinishRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "Ground finish missing {s}"),
            Self::Poisoned => f.write_str("Ground finish owner lock poisoned"),
            Self::Foreign => f.write_str("Ground finish foreign owner/state"),
            Self::Pair(e) => e.fmt(f),
            Self::Grab(e) => e.fmt(f),
            Self::Search(e) => write!(f, "Ground grab search: {e:?}"),
        }
    }
}
struct Runtime<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    owner: Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
}
impl Runtime<'_> {
    /// CheckForGrabbableObject82d9faa8. On-board RB searches VEHICLES with
    /// the radius overload; off-board DMO pushing has a different box query.
    fn check_ground_grab(
        &mut self,
        ground: &mut crate::physical_player_physics_state::PhysicsGroundFields,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::{
            physical_player_assembly::remaining_states::{StateKind, StatePayload, read_floats},
            physical_player_grab_spline::geometry::GrabBox,
            physical_player_grind_manager::{
                math::GrindNum,
                view::{error, lanes, need},
            },
            reckoning_numeric::HostReckoningNumeric,
            reckoning_sin_cos::HostReckoningSinCosNumeric,
        };
        let processed = Arc::clone(self.owner.reckoning().lock().map_err(error)?.processed());
        let transform = *need(
            processed.lock().map_err(error)?.effective_transform_field(),
            "ground effective transform+80",
        )?;
        // Reuse the existing Skitch state's instance of the same mounted
        // physics_state_skitching/default collection (Global member+188).
        let attributes = {
            let states = self.owner.remaining_states().lock().map_err(error)?;
            let Some(StatePayload::Skitching(skitch)) = states
                .states
                .get(&StateKind::Skitching)
                .filter(|state| state.published)
                .and_then(|state| state.value.as_ref())
            else {
                return Err(error("ground search missing constructed Skitch attributes"));
            };
            need(skitch.attribs.clone(), "physics_state_skitching/default")?
        };
        // Native reads dimensions before offset; absent fields use zero DDA16.
        let half_dimensions =
            read_floats::<4>(&attributes, 0x978b_2a75_33b6_675d).map_err(error)?;
        let offset = read_floats::<4>(&attributes, 0x8e3c_1298_e807_2745).map_err(error)?;
        let mut numeric = HostReckoningNumeric;
        let mut sin_cos = HostReckoningSinCosNumeric;
        let bounds = GrabBox::from_player(
            transform.columns().map(lanes),
            offset,
            half_dimensions,
            true,
            &mut GrindNum {
                n: &mut numeric,
                s: &mut sin_cos,
            },
        )?;
        let refs = &self.owner.inputs().cvars;
        let mut retained = self.owner.grab_spline().lock().map_err(error)?;
        let grab = need(retained.as_mut(), "PlayerGrabSpline")?;
        if grab.world != self.owner.manager() || !grab.returned {
            return Err(error(
                "ground search foreign/unconstructed PlayerGrabSpline",
            ));
        }
        if grab.query_is_ready() {
            let disable_hands = grab_results::consider(
                ground,
                grab,
                &processed,
                transform.columns().map(lanes),
                &bounds,
                &attributes,
                refs,
                &self.world.debug_overlay,
                &mut GrindNum {
                    n: &mut numeric,
                    s: &mut sin_cos,
                },
            )?;
            if disable_hands {
                self.world
                    .try_with_offboard_pair(self.board, self.skeleton, &self.owner, |normal| {
                        normal.disable_hand_collisions(2)
                    })
                    .map_err(error)?;
            }
        }
        let radius = bounds.half_dimensions[2] + refs.try_skitch_search_radius().map_err(error)?;
        grab.start_radius_query(2, bounds.transform[3], radius)?;
        if refs.try_draw_grabbable_object_test().map_err(error)? {
            self.world.debug_overlay.require().map_err(error)?;
        }
        Ok(())
    }
}
impl GroundFinishSource for Runtime<'_> {
    type Error = GroundFinishRuntimeError;
    fn check_for_grabbable_object(
        &mut self,
        ground: &mut crate::physical_player_physics_state::PhysicsGroundFields,
    ) -> Result<(), Self::Error> {
        self.check_ground_grab(ground).map_err(Self::Error::Search)
    }
    fn request_push_exit(&mut self) -> Result<(), Self::Error> {
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Missing("skater pool"))?
            .try_with_jump_pair(
                &self.world.backend,
                self.board,
                self.skeleton,
                |normal, _| normal.try_request_push_exit(),
            )
            .map_err(Self::Error::Pair)?
            .map_err(|_| Self::Error::Missing("actual Skeleton IK"))
    }
    fn mark_add_skateboard_error(&mut self) -> Result<(), Self::Error> {
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Missing("skater pool"))?
            .try_with_jump_pair(
                &self.world.backend,
                self.board,
                self.skeleton,
                |normal, _| normal.mark_add_skateboard_error(),
            )
            .map_err(Self::Error::Pair)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.owner
            .reckoning()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .processed()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(Self::Error::Missing("DT93c"))
    }
    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.owner
            .reckoning()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .processed()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physics_state_fields()
            .flags_8cc
            .ok_or(Self::Error::Missing("flags8cc"))
    }
    fn invalidate_grab_query(&mut self) -> Result<(), Self::Error> {
        let mut owner = self
            .owner
            .grab_spline()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?;
        let grab = owner
            .as_mut()
            .ok_or(Self::Error::Missing("PlayerGrabSpline"))?;
        if grab.world != self.owner.manager() {
            return Err(Self::Error::Foreign);
        }
        if !grab.returned {
            return Err(Self::Error::Missing(
                "completed PlayerGrabSpline constructor",
            ));
        }
        grab.try_invalidate_query(self.world.world_line_owners().0)
            .map_err(Self::Error::Grab)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_finish_pair_ground_update(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: GroundBeforePushExit,
    ) -> Result<GroundUpdateComplete, GroundFinishError<GroundFinishRuntimeError>> {
        use GroundFinishRuntimeError as E;
        let owner = Arc::clone(&before.0.owner);
        let same = self
            .skaters
            .as_mut()
            .ok_or(GroundFinishError::Source(E::Missing("skater pool")))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(|e| GroundFinishError::Source(E::Pair(e)))?;
        if !same {
            return Err(GroundFinishError::Source(E::Foreign));
        }
        let mut state = owner
            .physics_state()
            .lock()
            .map_err(|_| GroundFinishError::Source(E::Poisoned))?;
        if state.current != CurrentPhysicalState::Ground {
            return Err(GroundFinishError::Source(E::Foreign));
        }
        let ground = state
            .ground
            .as_mut()
            .ok_or(GroundFinishError::Source(E::Missing("Ground")))?;
        if !ground.bindings.board.same_as(board) || !ground.bindings.skeleton.same_as(skeleton) {
            return Err(GroundFinishError::Source(E::Foreign));
        }
        try_finish_ground(
            &mut ground.fields,
            &mut Runtime {
                world: self,
                board,
                skeleton,
                owner: Arc::clone(&owner),
            },
        )?;
        Ok(GroundUpdateComplete(before))
    }
}
