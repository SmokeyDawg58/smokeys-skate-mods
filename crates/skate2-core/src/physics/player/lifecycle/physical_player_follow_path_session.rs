//! `PhysState_FollowPath` 105 (vtable 0x82371678): an AI skater whose local world is not loaded
//! follows its path without ground geometry. `Enter` 0x82d868b0 disables the board animation;
//! `Update` 0x82d868b8 drives the deck body toward the path's position, runs the grounded
//! reckoning against world up, the ground skateboard skeleton and the physics error; `Exit`,
//! `FillPhysOut` and `GetSyncLog` are the shared empty callback.
use super::ground_pre_solve::HostGroundPreparationNumeric;
use crate::{
    ai::actor::physics_flags,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle as Handle, board_construction::ground_control::Source as PhysicsErrorSource,
    },
    input_cvar_runtime::{InputCvarReferences, reckoning::ReckoningCvarAdapter},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner, remaining_states::StateKind},
    physical_player_grind_manager::view::{GrindError as E, error, need},
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    reckoning_numeric::{ReckoningNumeric, try_clamp, try_length},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_processed_contact::SkeletonProcessedContact,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

/// `Physics_AI` cvars at their defaults: `kMaxAIPathFollowingVelocity` ("Max speed nogeom") and
/// `kMaxAIPathFollowingAcceleration`.
const MAX_AI_PATH_FOLLOWING_VELOCITY: f32 = 40.0;
const MAX_AI_PATH_FOLLOWING_ACCELERATION: f32 = 5.0;

fn check_bindings(owner: &Arc<PhysicalPlayerAssemblyOwner>, board: &Handle, skeleton: &Handle) -> Result<(), E> {
    let states = owner.remaining_states().lock().map_err(error)?;
    let bindings = need(states.published_bindings(StateKind::FollowPath), "retained FollowPath105")?;
    if !bindings.board.same_as(board)
        || !bindings.skeleton.same_as(skeleton)
        || !bindings.owner.upgrade().is_some_and(|bound| Arc::ptr_eq(&bound, owner))
    {
        return Err(error("foreign FollowPath pair"));
    }
    Ok(())
}

/// `PhysicsUtility::ClampVectorAWithinRangeOfVectorB` 0x82e496d0: `a` within `range` of `b`,
/// else the point `range` from `b` toward `a`.
fn try_clamp_within_range<N: ReckoningNumeric>(a: [f32; 4], b: [f32; 4], range: f32, numeric: &mut N) -> Result<[f32; 4], E>
where
    N::Error: std::fmt::Debug,
{
    if range < 0.0 {
        return Err(error("physicsutility.cpp range assertion"));
    }
    let delta: [f32; 4] = std::array::from_fn(|lane| a[lane] - b[lane]);
    let length = try_length(delta, numeric).map_err(error)?;
    if range >= length {
        return Ok(a);
    }
    if !(length > 0.0) {
        return Err(error("physicsutility.cpp length assertion"));
    }
    let scale = range / length;
    Ok(std::array::from_fn(|lane| delta[lane].mul_add(scale, b[lane])))
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_enter_pair_follow_path(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), E> {
        check_bindings(owner, board, skeleton)?;
        self.try_disable_board_animation(board).map_err(error)
    }

    /// `Skateboard::FollowPath_NoGeometry` 0x82b2aee0: with a deck position to follow, the deck
    /// body's velocity becomes the step to it over the frame, at most
    /// `kMaxAIPathFollowingVelocity`, and within `kMaxAIPathFollowingAcceleration` of the
    /// body's velocity.
    fn try_follow_path_no_geometry<N: ReckoningNumeric>(
        &mut self,
        board: &Handle,
        processed: &Mutex<SkeletonProcessedContact>,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        let (flags, dt, target) = {
            let processed = processed.lock().map_err(error)?;
            (
                processed.physics_state_fields().ai_flags.0,
                need(processed.physics_state_fields().time_delta, "Processed DT")?,
                need(processed.input_controls().ai_vectors[3], "Processed AI deck position")?,
            )
        };
        if flags & physics_flags::DO_DECK_POSITION == 0 {
            return Ok(());
        }
        let position = self.try_deck_part_transform(board).map_err(error)?.columns()[3];
        let estimate = numeric.reciprocal_estimate(dt).map_err(error)?;
        let inverse = refine_drive_reciprocal(dt, estimate);
        let velocity = self.try_deck_velocity(board).map_err(error)?;
        let wanted = [
            inverse * (target[0] - position.xyz[0]),
            inverse * (target[1] - position.xyz[1]),
            inverse * (target[2] - position.xyz[2]),
            0.0,
        ];
        let limited = try_clamp(wanted, MAX_AI_PATH_FOLLOWING_VELOCITY, numeric).map_err(error)?;
        let stepped = try_clamp_within_range(
            limited,
            [velocity[0], velocity[1], velocity[2], 0.0],
            MAX_AI_PATH_FOLLOWING_ACCELERATION,
            numeric,
        )?;
        self.try_set_deck_body_velocity(board, [stepped[0], stepped[1], stepped[2]]).map_err(error)
    }

    /// `PhysState_FollowPath::Update` 0x82d868b8.
    pub(crate) fn try_update_pair_follow_path(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, E> {
        check_bindings(owner, board, skeleton)?;
        let processed = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
        self.try_follow_path_no_geometry(board, &processed, &mut numeric.reckoning)?;
        let phase = owner
            .reckoning()
            .lock()
            .map_err(error)?
            .try_update_grounded_follow_path(&processed, &mut ReckoningCvarAdapter(refs), &mut numeric.reckoning)
            .map_err(error)?;
        let progress = self
            .try_update_pair_ground_skateboard(
                board,
                skeleton,
                ground,
                refs,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.spawn_math,
            )
            .map_err(error)?;
        if progress.next
            != crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralContinuation::Complete
        {
            return Err(error("incomplete FollowPath Skeleton"));
        }
        let backend = &self.backend;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_ground_force_board(backend, board, |session, runtime, _| {
                runtime
                    .steering
                    .try_update_physics_error(
                        &mut PhysicsErrorSource { processed: &processed, board: session, backend },
                        &mut numeric.reckoning,
                    )
                    .map_err(error)
            })
            .map_err(error)??;
        Ok(phase)
    }
}
