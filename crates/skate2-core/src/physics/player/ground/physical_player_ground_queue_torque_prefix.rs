//! Unregistered actual Ground82d9a7e8..a980, before `ManageHangUps`.
use super::ground_manual_nudge::GroundBeforeForceQueues;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
        forces::SimulationSkateboardForceError, target_angular::TargetAngularOwnerError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    skateboard_slide_collision_attributes::{
        SkateboardSlideCollisionAttributeError, SkateboardSlideCollisionAttributeRead,
        SlideCollisionGlobal,
    },
};
use std::sync::Arc;
pub(crate) struct GroundBeforeManageHangUps(pub(crate) GroundBeforeForceQueues);
#[derive(Debug)]
pub(crate) enum GroundQueueTorqueError<N> {
    Missing(&'static str),
    Poisoned,
    Foreign,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Attribute(SkateboardSlideCollisionAttributeError),
    Target(TargetAngularOwnerError<N>),
    Apply(TargetAngularOwnerError<N>),
    Drag(SimulationSkateboardForceError),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundQueueTorqueError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "ground queue/torque missing {s}"),
            Self::Poisoned => f.write_str("ground queue owner lock poisoned"),
            Self::Foreign => f.write_str("ground queue foreign pair or state"),
            Self::Pair(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Attribute(e) => e.fmt(f),
            Self::Target(e) | Self::Apply(e) => e.fmt(f),
            Self::Drag(e) => e.fmt(f),
            Self::Numeric(e) => write!(f, "ground net force numeric: {e:?}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_lines,
        reason = "ordered queue, torque, drag and retained flag commits preserve late failure behavior"
    )]
    pub(crate) fn try_update_pair_ground_queue_torque_prefix<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: GroundBeforeForceQueues,
        numeric: &mut N,
    ) -> Result<GroundBeforeManageHangUps, GroundQueueTorqueError<N::Error>> {
        use GroundQueueTorqueError as E;
        let owner = Arc::clone(&before.before.before.owner);
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        if !pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(E::Pair)?
        {
            return Err(E::Foreign);
        }
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::Ground {
            return Err(E::Foreign);
        }
        let ground = state.ground.as_mut().ok_or(E::Missing("Ground"))?;
        if !ground.bindings.board.same_as(board) || !ground.bindings.skeleton.same_as(skeleton) {
            return Err(E::Foreign);
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let collision = pool
            .try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
                // NOTLT at a7f0 includes unordered. The threshold is captured a6b0.
                if ground.fields.time_wall_riding < before.before.foot_threshold {
                    runtime.queue.add(4, before.before.front);
                    runtime.queue.add(5, before.before.rear);
                }
                runtime.queue.add(2, before.before.brake);
                if std::env::var_os("SKATE2_TRACE_PUSH_FORCE").is_some() {
                    eprintln!(
                        "queued ground push force: time={} xyz={:?}",
                        ground.fields.time_in_state, before.before.push.world_space_force.xyz
                    );
                }
                runtime.queue.add(3, before.before.push);
                if ground.fields.collision_timer > 0.0 {
                    let attributes = &mut runtime.slide_collision_attributes;
                    let global = attributes
                        .capture_global(SlideCollisionGlobal::Collision)
                        .map_err(E::Attribute)?;
                    let pointer = attributes
                        .lookup(&global, 0x1573_c41a_dd19_aa95)
                        .map_err(E::Attribute)?;
                    let old_timer = ground.fields.collision_timer; //a858 before pointer load
                    let fade = attributes.float(&pointer).map_err(E::Attribute)?;
                    let dt = processed
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .physics_state_fields()
                        .time_delta
                        .ok_or(E::Missing("Processed DT93c"))?;
                    ground.fields.collision_timer = old_timer - dt;
                    ground.fields.collision_torque =
                        ground.fields.collision_torque.map(|v| v * fade);
                    Ok::<_, E<N::Error>>(Some(ground.fields.collision_torque))
                } else {
                    Ok(None)
                }
            })
            .map_err(E::Board)??;
        if let Some(torque) = collision {
            pool.try_target_board_angular_displacement(&mut self.backend, board, torque, numeric)
                .map_err(E::Target)?;
        }
        pool.try_target_board_angular_displacement(
            &mut self.backend,
            board,
            before.before.straighten,
            numeric,
        )
        .map_err(E::Target)?;
        pool.try_target_board_angular_displacement(
            &mut self.backend,
            board,
            before.before.heading,
            numeric,
        )
        .map_err(E::Target)?;
        // The distinct direct angular-displacement method preserves both calls.
        pool.try_apply_board_angular_displacement(
            &mut self.backend,
            board,
            before.before.antiflip,
            numeric,
        )
        .map_err(E::Apply)?;
        pool.try_apply_board_angular_displacement(
            &mut self.backend,
            board,
            before.manual.angular_displacement,
            numeric,
        )
        .map_err(E::Apply)?;
        let (ai_bits, ai_known) = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .ai_flags;
        if ai_known & 0x8000_0000 == 0 {
            return Err(E::Missing("Processed6b0 high bit"));
        }
        pool.try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
            if ai_bits & 0x8000_0000 == 0 {
                runtime.queue.add(6, before.before.speed);
            }
            runtime.queue.add(7, before.manual.corrective_force);
            runtime.queue.add(1, before.before.slide);
            runtime.queue.add(8, before.before.pump);
            runtime.queue.add(16, before.before.before.wallride_force);
        })
        .map_err(E::Board)?;
        pool.try_set_board_linear_drag(&mut self.backend, board, before.linear_drag)
            .map_err(E::Drag)?;
        let net = before
            .combined_push_brake
            .zip(before.before.pump.world_space_force, |a, b| a + b);
        let squared = numeric.dot_vector(net, net).map_err(E::Numeric)?;
        if squared > f32::from_bits(0x3780_0000) {
            pool.try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
                runtime.entry.mark_collision_response();
            })
            .map_err(E::Board)?;
        }
        Ok(GroundBeforeManageHangUps(before))
    }
}
