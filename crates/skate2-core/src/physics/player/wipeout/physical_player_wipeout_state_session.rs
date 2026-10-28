//! Actual retained `WipeoutGround.Enter82d8eb18`. Outer transition is separate.
use crate::{
    dynamic_assembly_session::{
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
            wipeout::WipeoutSkeletonOperation,
        },
        skeleton_normal::drive_owner::hook_soft::HipHookSoftKind,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_wipeout::WipeoutOwnerError,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        physical_board_forces::board_wipeout::PhysicalBoardWipeoutError,
        physical_skeleton_controller::{
            PhysicalRagdollError, hip_hook_soft::PhysicalHipHookSoftError,
        },
        physical_state::wipeout_helpers::WipeoutHelperError,
    },
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum WipeoutEntryError<N, C> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Poisoned,
    WrongState,
    ForeignBindings,
    Missing(&'static str),
    Helpers(WipeoutHelperError<N>),
    Board(PhysicalBoardWipeoutError),
    Hip(PhysicalHipHookSoftError<N>),
    Wipeout(WipeoutOwnerError),
    Cvar(InputCvarError),
    Ragdoll(PhysicalRagdollError<C>),
    Global(crate::dynamic_assembly_session::skater_pool::wipeout::WipeoutSkeletonError<std::convert::Infallible>),
}
impl<N: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Display for WipeoutEntryError<N, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("wipeout entry pool unavailable"),
            Self::Pair(e) => write!(f, "wipeout entry pair: {e}"),
            Self::Poisoned => f.write_str("wipeout entry owner lock poisoned"),
            Self::WrongState => f.write_str("wipeout entry requires selected WipeoutGround"),
            Self::ForeignBindings => f.write_str("wipeout entry has different pair bindings"),
            Self::Missing(field) => write!(f, "wipeout entry missing {field}"),
            Self::Helpers(e) => write!(f, "wipeout entry operation: {e}"),
            Self::Board(e) => write!(f, "wipeout entry board: {e}"),
            Self::Hip(e) => write!(f, "wipeout entry hip: {e:?}"),
            Self::Wipeout(e) => write!(f, "wipeout entry ledger: {e:?}"),
            Self::Cvar(e) => write!(f, "wipeout entry cvar: {e:?}"),
            Self::Ragdoll(e) => write!(f, "wipeout entry ragdoll: {e}"),
            Self::Global(e)=>write!(f,"wipeout entry global: {e:?}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    /// Source entry supports actual request6 scaling and required Part clamp.
    /// LivingWorld push uses the existing Part/body force accumulator at its
    /// source entry point, before ragdoll collision state is published.
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered WipeoutGround entry preserves every failure prefix"
    )]
    pub(crate) fn try_enter_pair_wipeout<N: ReckoningNumeric, C>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
    ) -> Result<(), WipeoutEntryError<N::Error, C>> {
        use WipeoutEntryError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::WipeoutGround {
            return Err(E::WrongState);
        }
        let wipeout = state
            .wipeout_ground
            .as_mut()
            .ok_or(E::Missing("WipeoutGround owner"))?;
        if !wipeout
            .bindings
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, &owner))
            || !wipeout.bindings.board.same_as(board)
            || !wipeout.bindings.skeleton.same_as(skeleton)
        {
            return Err(E::ForeignBindings);
        }
        self.try_wipeout_skeleton_operation(
            board,
            skeleton,
            WipeoutSkeletonOperation::ClearTruckTilt,
            numeric,
        )
        .map_err(E::Helpers)?;
        wipeout.fields.enter_prefix();
        self.try_set_board_wipeout(board).map_err(E::Board)?;
        self.try_wipeout_board_turn(board).map_err(E::Helpers)?;
        self.try_wipeout_disable_board_hook(board, true)
            .map_err(E::Helpers)?;
        self.try_wipeout_disable_board_hook(board, false)
            .map_err(E::Helpers)?;
        self.try_wipeout_skeleton_operation(
            board,
            skeleton,
            WipeoutSkeletonOperation::DisableIk,
            numeric,
        )
        .map_err(E::Helpers)?;
        self.try_pair_hip_hook_soft(
            board,
            skeleton,
            HipHookSoftKind::Angular,
            0.5,
            &mut |value| numeric.reciprocal_sqrt_estimate(value),
        )
        .map_err(E::Hip)?;
        self.try_pair_hip_hook_soft(
            board,
            skeleton,
            HipHookSoftKind::Linear,
            0.0,
            &mut |value| numeric.reciprocal_sqrt_estimate(value),
        )
        .map_err(E::Hip)?;
        let requested = owner
            .wipeout()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_is_requested(crate::wipeout::WipeoutType::ALL[6])
            .map_err(E::Wipeout)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        if requested {
            // Raw EC60/64 capture targets, EC68 upper value before EC6c lower.
            let (upper, lower) = refs.try_wipeout_up_velocity_scales().map_err(E::Cvar)?;
            let direction = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .reckoning_up()
                .ok_or(E::Missing("Processed UpVector"))?
                .value;
            self.try_wipeout_skeleton_operation(
                board,
                skeleton,
                WipeoutSkeletonOperation::ScaleVelocity {
                    direction,
                    lower,
                    upper,
                },
                numeric,
            )
            .map_err(E::Helpers)?;
        }
        let maximum = refs.try_max_skeleton_part_velocity().map_err(E::Cvar)?;
        self.try_wipeout_skeleton_operation(
            board,
            skeleton,
            WipeoutSkeletonOperation::ClampVelocity(maximum),
            numeric,
        )
        .map_err(E::Helpers)?;
        let flags = processed.lock().map_err(|_| E::Poisoned)?.known_flags_8c4();
        if flags.1 & 8 == 0 {
            return Err(E::Missing("Processed living-world impulse flag"));
        }
        if flags.0 & 8 != 0 {
            //82d8ecb4 loads Processed+5b0 before looking up the fresh global
            //physics_wipeout/default scalar (full64 key from raw PPC).
            let direction=processed.lock().map_err(|_|E::Poisoned)?.animation_input()
                .living_world_impulse.ok_or(E::Missing("Processed living-world impulse"))?;
            let scale=self.skaters.as_mut().ok_or(E::MissingPool)?
                .try_pair_global_wipeout_float(&self.backend,board,skeleton,0x58fc_35c4_9844_acc6)
                .map_err(E::Global)?;
            let displacement=direction.map(|v|v*scale);
            //Skeleton.PushRagdoll82b59868 forwards PART_SPINE2=12 to
            //SkeletonBody.ApplyDisplacementToPart82b545e0. That service reads
            //actual inverse mass/old force, accumulates force and wakes the body.
            self.try_wipeout_skeleton_operation(board,skeleton,WipeoutSkeletonOperation::PushRagdoll(displacement),numeric)
                .map_err(E::Helpers)?;
        }
        self.try_pair_skeleton_ragdoll_collision(board, skeleton, refs, cosine)
            .map_err(E::Ragdoll)?;
        self.try_wipeout_skeleton_operation(
            board,
            skeleton,
            WipeoutSkeletonOperation::LinearDrag(0.0),
            numeric,
        )
        .map_err(E::Helpers)?;
        let processed = processed.lock().map_err(|_| E::Poisoned)?;
        let flags = processed
            .known_flags_8c8()
            .ok_or(E::Missing("Processed impulse-for-wipeout flag"))?;
        wipeout.fields.is_impulse = Some(flags & 0x2000 != 0);
        wipeout.fields.impulse = Some(
            processed
                .prior_output()
                .wipeout_impulse()
                .ok_or(E::Missing("Processed WipeoutImpulse"))?,
        );
        wipeout.fields.impulse_frames = Some(0);
        wipeout.fields.estimated_time_to_collision = f32::MAX;
        wipeout.fields.prediction.ready = false;
        Ok(())
    }
}
