//! Actual PhysState_HandPlant600: Enter82d89330, Update82d97408. Exit82d893b0
//! and UpdatePostPhysics82d893b8 are empty. Manager and Skeleton are shared.
use super::{
    ground_pre_solve::HostGroundPreparationNumeric, prediction::runtime::future::PredictionTarget,
};
use crate::{
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    dynamic_assembly_session::{
        skater_pool::SimulationSkaterAssemblyHandle as Handle,
        skeleton_normal::general_owner::SkeletonGeneralContinuation,
    },
    input_cvar_runtime::{
        InputCvarReferences,
        handplant::{HandPlantBool as B, HandPlantFloat as F},
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{HandPlant, StateKind, StatePayload},
    },
    physical_player_footplant::RetainedTrajectory,
    physical_player_grind_manager::{
        math::{GrindNum, scale, sub},
        view::{error, lanes, need, root},
    },
    physical_player_handplant::manager::{E, GRAVITY},
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

fn state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    f: impl FnOnce(&mut HandPlant) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(error)?;
    let state = need(
        states
            .states
            .get_mut(&StateKind::HandPlant)
            .filter(|v| v.published)
            .and_then(|v| v.value.as_mut()),
        "HandPlant state",
    )?;
    let StatePayload::HandPlant(s) = state else {
        return Err(error("HandPlant payload"));
    };
    let b = &s.base.physical;
    if !b.board.same_as(board)
        || !b.skeleton.same_as(skeleton)
        || !b.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
    {
        return Err(error("foreign HandPlant state"));
    }
    f(s)
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_handplant_pair_ik(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        reset: bool,
        refs: &InputCvarReferences,
        n: &mut GrindNum<'_>,
    ) -> Result<(), E> {
        let p = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
        let natural = need(
            p.lock().map_err(error)?.physics_state_fields().flags_8cc,
            "HandPlant stance flag",
        )? & 8
            != 0;
        let mut hand = owner.handplant().lock().map_err(error)?;
        let manager = need(hand.as_mut(), "HandPlant manager")?;
        let active = manager.prepare_ik(reset)?;
        let right = manager.right_hand(natural);
        use crate::ragdoll::SkeletonPartIndex as P;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, actual| {
                if !Arc::ptr_eq(actual, owner) {
                    return Err(error("foreign HandPlant skeleton"));
                }
                // GetAnimPartTransformWorldRaw(hand) precedes shoulder.
                let raw_hand = lanes(need(
                    normal.anim_part_world_raw_position(if right {
                        P::RightHand as usize
                    } else {
                        P::LeftHand as usize
                    }),
                    "raw hand",
                )?);
                let shoulder = lanes(need(
                    normal.anim_part_world_raw_position(if right {
                        P::RightShoulder as usize
                    } else {
                        P::LeftShoulder as usize
                    }),
                    "raw shoulder",
                )?);
                if let Some((target, maximum, blend)) =
                    manager.update_ik(raw_hand, shoulder, active, refs, n)?
                {
                    normal
                        .try_set_world_hand_ik_target(!right, root(target), maximum, blend, n.n)
                        .map_err(error)?;
                }
                Ok(())
            })
            .map_err(error)??;
        // UpdateIK82db7d30 reads the authored debug flag after the target write.
        let debug = manager
            .attributes
            .try_get_data(0xa492_8ce7_9e99_ab4f, 0)
            .map_err(error)?
            .is_some_and(|data| data.bytes().first().is_some_and(|v| *v != 0));
        if debug || refs.try_handplant_bool(B::ManagerDebug).map_err(error)? {
            self.debug_overlay.require().map_err(error)?;
        }
        Ok(())
    }
    pub(crate) fn try_enter_pair_handplant<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        n: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        state(owner, board, skeleton, |_| Ok(()))?;
        self.try_enable_board_angular_drive_only(board, 1., refs, n)
            .map_err(error)?;
        let p = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
        let (com, velocity) = {
            let p = p.lock().map_err(error)?;
            (
                need(p.prior_output().centre_of_mass(), "HandPlant COM")?.value,
                need(
                    p.prior_output().centre_of_mass_velocity(),
                    "HandPlant COM velocity",
                )?
                .value,
            )
        };
        let (hand_position, predictor) = {
            let mut hand = owner.handplant().lock().map_err(error)?;
            let m = need(hand.as_mut(), "HandPlant manager")?;
            m.trajectory_physics_start = RetainedTrajectory {
                position: com,
                velocity,
                gravity: root(GRAVITY),
                time: [-1.; 4],
            };
            m.parametric_time = [0.; 4];
            m.real_time = [0.; 4];
            m.trajectory_ready = false;
            let surface = need(
                m.trajectory_selector.surface_query.as_ref(),
                "HandPlant surface",
            )?;
            (
                m.animation.position,
                Arc::clone(need(surface.predictor.as_ref(), "surface predictor")?),
            )
        };
        let radius = refs.try_handplant_float(F::SurfaceRadius).map_err(error)?;
        let down = [0., -1., 0., 0.];
        let input = BallisticTrajectoryInput {
            position: root(sub(hand_position, scale(scale(down, radius), 2.))),
            velocity: root(down),
            gravity: root(predictor.gravity),
            radius,
            duration: 1.,
            maximum_error_start: 1.,
            maximum_error_end: 1.,
        };
        predictor
            .try_add_query(
                owner,
                PredictionTarget::HandPlantSurface,
                input,
                false,
                need(self.world_line_queries.as_mut(), "World queries")?,
                ground,
                need(self.dmos.as_ref(), "DMOs")?,
                n,
            )
            .map_err(error)?;
        let requests = {
            let mut hand = owner.handplant().lock().map_err(error)?;
            let m = need(hand.as_mut(), "HandPlant manager")?;
            let mut host = HostReckoningNumeric;
            let mut trig = HostReckoningSinCosNumeric;
            m.trajectory_selector.prepare(
                m.trajectory_into,
                &m.grind_spline,
                m.animation.position,
                refs,
                &mut GrindNum {
                    n: &mut host,
                    s: &mut trig,
                },
            )?
        };
        let predictor = Arc::clone(need(self.predictor.value.as_ref(), "World predictor")?);
        for (i, trajectory) in requests.into_iter().enumerate() {
            let input = BallisticTrajectoryInput {
                position: trajectory.position,
                velocity: trajectory.velocity,
                gravity: root(predictor.gravity),
                radius: 0.2,
                duration: 3.,
                maximum_error_start: 1.,
                maximum_error_end: 1.,
            };
            predictor
                .try_add_query(
                    owner,
                    PredictionTarget::HandPlant(i),
                    input,
                    false,
                    need(self.world_line_queries.as_mut(), "World queries")?,
                    ground,
                    need(self.dmos.as_ref(), "DMOs")?,
                    n,
                )
                .map_err(error)?;
        }
        state(owner, board, skeleton, |s| {
            s.hand_plant_state = Some(0);
            s.time_to_apex = Some(100.);
            Ok(())
        })
    }
    pub(crate) fn try_update_pair_handplant(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<crate::physical_player_reckoning_owner::PhysicalPlayerReckoningPhase, E> {
        state(owner, board, skeleton, |_| Ok(()))?;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.disable_hand_collisions(2)
            })
            .map_err(error)?;
        let (position, up, heading) = {
            let mut hand = owner.handplant().lock().map_err(error)?;
            let m = need(hand.as_mut(), "HandPlant manager")?;
            let time = m.advance_time()?;
            state(owner, board, skeleton, |s| {
                s.time_to_apex = Some(time);
                Ok(())
            })?;
            m.calc_values(
                refs,
                &mut GrindNum {
                    n: &mut numeric.reckoning,
                    s: &mut numeric.sin_cos,
                },
            )?
        };
        let phase = owner
            .reckoning()
            .lock()
            .map_err(error)?
            .try_update_handplant_live(
                root(up),
                refs.try_handplant_float(F::UpVectorBlend).map_err(error)?,
                root(heading),
                refs.try_handplant_float(F::UpVectorMaxDelta)
                    .map_err(error)?,
                &mut crate::input_cvar_runtime::reckoning::ReckoningCvarAdapter(refs),
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
            )
            .map_err(error)?;
        self.try_handplant_pair_ik(
            owner,
            board,
            skeleton,
            false,
            refs,
            &mut GrindNum {
                n: &mut numeric.reckoning,
                s: &mut numeric.sin_cos,
            },
        )?;
        let progress = self
            .try_update_pair_hand_plant_skeleton(
                owner,
                board,
                skeleton,
                root(position),
                ground,
                refs,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
            )
            .map_err(error)?;
        if progress.next != SkeletonGeneralContinuation::Complete {
            return Err(error("complete HandPlant Skeleton"));
        }
        if refs.try_handplant_bool(B::StateDebug).map_err(error)?
            || refs.try_handplant_bool(B::ManagerDebug).map_err(error)?
        {
            self.debug_overlay.require().map_err(error)?;
        }
        Ok(phase)
    }
}
