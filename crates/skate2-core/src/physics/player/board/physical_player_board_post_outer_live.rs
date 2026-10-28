//! `Skateboard::UpdatePostPhysics`82b37d20: ordinary processing and both
//! native board-recovery branches, followed by the shared force-queue clear.
use super::{
    board_post_end::PhysicalPlayerBoardPostEndError,
    board_postphysics::PhysicalPlayerBoardPostError, board_up_post::PhysicalPlayerBoardUpPostError,
    physical_board_forces::PhysicalPlayerBoardForceError,
    physical_island::SimulationPhysicalBodyError,
    physical_teleport_now_prefix::TeleportNowPrefixError,
    physical_wheel_lines::PhysicalPlayerWheelLineError,
};
use crate::{
    dynamic_assembly_session::{
        part_transform::DynamicAssemblyPartTransformError,
        skater_pool::{SimulationSkaterAssemblyHandle, SimulationSkaterPoolError},
        skater_pool::board_construction::SimulationConstructedBoardError,
    },
    input_cvar_runtime::{InputCvarError, board_post_live::BoardPostCvars},
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_truck_drives::SkateboardTruckDriveNumeric,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

#[path = "../../board/runtime/skateboard_post_outer_predicates.rs"]
mod predicates;

/// All capabilities remain supplied by the caller; this introduces no alternate
/// host numeric policy inside the simulation world.
pub(crate) struct BoardPostNumerics<'a, L, N, R, T> {
    pub(crate) line: &'a mut L,
    pub(crate) vector: &'a mut N,
    pub(crate) rounding: &'a mut R,
    pub(crate) truck: &'a mut T,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoardPostResetReason {
    BadData,
    StrangeAcceleration,
}

#[derive(Debug)]
pub(crate) enum BoardPostOuterError<L, N, R, T> {
    Owner(SimulationPhysicalBodyError),
    ForeignPair,
    Pool(SimulationSkaterPoolError),
    Transform(DynamicAssemblyPartTransformError),
    Body(SimulationRigidBodyPoolError),
    Wheel(PhysicalPlayerWheelLineError<L>),
    Cvar(InputCvarError),
    Force(PhysicalPlayerBoardForceError),
    Missing(&'static str),
    Poisoned(&'static str),
    ResetInitialization(SimulationConstructedBoardError),
    ResetTeleport(Box<TeleportNowPrefixError>),
    Post(PhysicalPlayerBoardPostError<N, InputCvarError, R>),
    Finish(PhysicalPlayerBoardPostEndError<T>),
    Up(PhysicalPlayerBoardUpPostError<N>),
}
impl<L: std::fmt::Debug, N: std::fmt::Debug, R: std::fmt::Debug, T: std::fmt::Debug>
    std::fmt::Display for BoardPostOuterError<L, N, R, T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => error.fmt(f),
            Self::ForeignPair => f.write_str("board post-physics foreign pair"),
            Self::Pool(error) => error.fmt(f),
            Self::Transform(error) => error.fmt(f),
            Self::Body(error) => error.fmt(f),
            Self::Wheel(error) => write!(f, "board wheel lines: {error:?}"),
            Self::Cvar(error) => error.fmt(f),
            Self::Force(error) => error.fmt(f),
            Self::Missing(stage) => write!(f, "board post-physics missing {stage}"),
            Self::Poisoned(owner) => write!(f, "board post-physics poisoned {owner}"),
            Self::ResetInitialization(error) => write!(f, "board reset initialization: {error:?}"),
            Self::ResetTeleport(error) => write!(f, "board recovery teleport: {error:?}"),
            Self::Post(error) => write!(f, "board body post-physics: {error:?}"),
            Self::Finish(error) => write!(f, "board post-physics finish: {error:?}"),
            Self::Up(error) => write!(f, "board post-physics up: {error:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    /// Exceptional arms82b37d60..80 and82b37da8..e18. Keep both calls to
    /// InitializeTransforms: the first is here, the second is inside the
    /// shared TeleportReset. Each has its original attached-Part writes.
    fn try_recover_board_postphysics<L, N, R, T>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        reason: BoardPostResetReason,
    ) -> Result<i32, BoardPostOuterError<L, N, R, T>> {
        use BoardPostOuterError as E;
        if reason == BoardPostResetReason::BadData {
            self.try_with_board_force_runtime(board, |runtime| {
                runtime.entry.request_respawn_for_bad_data();
            }).map_err(E::Force)?; //37d68 before initialization
        }
        self.try_teleport_initialize_board_transforms(board)
            .map_err(E::ResetInitialization)?;
        let processed = Arc::clone(
            owner.reckoning().lock().map_err(|_| E::Poisoned("Reckoning"))?.processed(),
        );
        let reset_transform = {
            let current = processed.lock().map_err(|_| E::Poisoned("Processed"))?;
            match reason {
                BoardPostResetReason::BadData => *current.deck_transform()
                    .ok_or(E::Missing("Processed deck transform+40"))?,
                BoardPostResetReason::StrangeAcceleration => *current.effective_transform_field()
                    .ok_or(E::Missing("Processed effective transform+80"))?,
            }
        };
        self.try_teleport_reset_board_live(board, owner, reset_transform)
            .map_err(|error| E::ResetTeleport(Box::new(error)))?;
        let result = if reason == BoardPostResetReason::StrangeAcceleration {
            // SetVelocity82b244d8..528, called at37df4: resolve and write
            // each attached Part in order. These post-solve stores retain
            // the solved contacts consumed next by Skeleton.UpdatePostPhysics.
            let mut part = 0;
            while part < self.board(board).map_err(E::Pool)?.template().part_payloads().len() {
                let body = self.board(board).map_err(E::Pool)?
                    .try_part_contact_spy_observation(&self.backend, part)
                    .map_err(|error| E::Pool(SimulationSkaterPoolError::PhysicalPart(error)))?;
                self.backend.try_store_post_solve_linear_velocity(body, [0.0; 3])
                    .map_err(E::Body)?;
                part += 1;
            }
            //37e00..18: late deck-body lookup; omega XYZ=+0, packed W kept.
            let deck = self.board(board).map_err(E::Pool)?
                .try_part_contact_spy_observation(&self.backend, 6)
                .map_err(|error| E::Pool(SimulationSkaterPoolError::PhysicalPart(error)))?;
            self.backend.try_store_post_solve_angular_velocity(deck, [0.0; 3])
                .map_err(E::Body)?;
            1
        } else {
            2
        };
        self.try_with_board_force_runtime(board, |runtime| runtime.queue.clear_post_physics())
            .map_err(E::Force)?; //37e78..80, repeated after TeleportReset's clear
        if std::env::var_os("SKATE2_TRACE_PHYSICS_RECOVERY").is_some() {
            eprintln!("physics: board post recovered {reason:?}, result={result}, at {:?}",
                reset_transform.columns()[3].xyz);
        }
        Ok(result)
    }

    /// Native0 follows ordinary Body/up processing,1 follows acceleration
    /// recovery,2 follows bad-data recovery. All successful arms clear forces.
    #[expect(
        clippy::too_many_lines,
        clippy::type_complexity,
        reason = "one source-ordered outer caller over four existing numeric capabilities"
    )]
    pub(crate) fn try_update_board_postphysics_live<L, N, R, T>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut BoardPostNumerics<'_, L, N, R, T>,
    ) -> Result<
        i32,
        BoardPostOuterError<<L as ReckoningNumeric>::Error, N::Error, R::Error, T::Error>,
    >
    where
        L: AssemblyGroundNumeric + ReckoningNumeric,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
        T: SkateboardTruckDriveNumeric,
    {
        use BoardPostOuterError as E;
        // Host identity validation samples no branch inputs and mutates nothing.
        let owner = Arc::clone(
            self.physical_body_owner_for_contacts(board)
                .map_err(E::Owner)?,
        );
        if !Arc::ptr_eq(
            self.physical_body_owner_for_contacts(skeleton)
                .map_err(E::Owner)?,
            &owner,
        ) {
            return Err(E::ForeignPair);
        }
        self.try_end_board_wheel_lines(board, ground, numeric.line)
            .map_err(E::Wheel)?; //37d48
        let transform = self
            .board(board)
            .map_err(E::Pool)?
            .try_live_part_transform_typed(&self.backend, 6)
            .map_err(E::Transform)?;
        if predicates::is_bad_data(&transform) {
            //IsBadData82b2b398
            return self.try_recover_board_postphysics(board, &owner, BoardPostResetReason::BadData);
        }
        // GetDeckVelocity82b25ff8 reads current attached Part6 body+20. W is an
        // intrusive link and does not participate in either source comparison.
        let observation = self
            .board(board)
            .map_err(E::Pool)?
            .try_part_contact_spy_observation(&self.backend, 6)
            .map_err(|error| E::Pool(SimulationSkaterPoolError::PhysicalPart(error)))?;
        let current_velocity = self
            .backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(observation)
            .map_err(E::Body)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned("Reckoning"))?
                .processed(),
        );
        let min_bad = owner
            .inputs()
            .cvars
            .try_min_bad_velocity_y()
            .map_err(E::Cvar)?; //260bc
        let old_velocity = processed
            .lock()
            .map_err(|_| E::Poisoned("Processed"))?
            .prior_output()
            .velocity()
            .ok_or(E::Missing("Processed velocity+190"))?
            .value; //260d0 before first test
        let accelerated =
            predicates::try_accelerated(current_velocity[1], min_bad, old_velocity, || {
                owner.inputs().cvars.try_max_good_velocity_y()
            })
            .map_err(E::Cvar)?;
        if accelerated {
            let animated = self
                .try_with_board_force_runtime(board, |runtime| runtime.entry.animated())
                .map_err(E::Force)?; //37d9c
            if !animated {
                return self.try_recover_board_postphysics(board, &owner, BoardPostResetReason::StrangeAcceleration);
            }
        }
        // Source37e3c up precedes37e40 timestep. Drop each short lock before
        // entering the existing Body services; no retained snapshot or outer guard.
        let up = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned("Reckoning"))?
            .reckoning()
            .ok_or(E::Missing("actual Reckoning"))?
            .up();
        let timestep = processed
            .lock()
            .map_err(|_| E::Poisoned("Processed"))?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("Processed dt+93c"))?;
        let mut cvars = BoardPostCvars::new(&owner.inputs().cvars);
        self.try_update_board_post_ground_normals(
            board,
            timestep,
            up,
            &mut cvars,
            numeric.vector,
            numeric.rounding,
        )
        .map_err(E::Post)?;
        self.try_finish_board_postphysics(board, numeric.truck)
            .map_err(E::Finish)?;
        let current = processed.lock().map_err(|_| E::Poisoned("Processed"))?;
        self.try_finish_board_ordinary_up_post(board, skeleton, &current, numeric.vector)
            .map_err(E::Up)?;
        Ok(0)
    }
}
