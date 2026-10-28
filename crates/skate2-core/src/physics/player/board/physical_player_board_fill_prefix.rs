//! Same-owner board `FillPhysOut`82b2f3e8..f7f8, before Pumping.
use super::{
    board_motion::{PhysicalPlayerBoardMotionError, PhysicalPlayerBoardMotionSample},
    physical_update5_prefix::Update5BeforeBoardFill,
};
use crate::{
    debug_overlay::DebugOverlayError,
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        part_transform::DynamicAssemblyPartTransformError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError,
            board_entry::{SkateboardEntrySessionError, try_current_board_effective_transform},
        },
    },
    input_cvar_runtime::InputCvarError,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill::{BoardFillOutputError, BoardFillSource},
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum BoardFillSourceError {
    Pool(SimulationSkaterPoolError),
    Part(DynamicAssemblyPartContactSpyError),
    Transform(DynamicAssemblyPartTransformError),
    EffectiveTransform(SkateboardEntrySessionError),
    Motion(PhysicalPlayerBoardMotionError<Infallible>),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Missing(&'static str),
}
impl std::fmt::Display for BoardFillSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "board pool: {e}"),
            Self::Part(e) => write!(f, "board Part: {e}"),
            Self::Transform(e) => write!(f, "board transform: {e}"),
            Self::EffectiveTransform(e) => write!(f, "effective transform: {e}"),
            Self::Motion(e) => write!(f, "board velocity: {e}"),
            Self::Cvar(e) => write!(f, "board debug cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "board debug overlay: {e}"),
            Self::Missing(field) => write!(f, "board output requires {field}"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum BoardFillPrefixError<N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Output(BoardFillOutputError<BoardFillSourceError, N>),
    Motion(PhysicalPlayerBoardMotionError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for BoardFillPrefixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board output requires actual skater pool"),
            Self::Pair(e) => write!(f, "board output pair: {e:?}"),
            Self::ForeignOwner => {
                f.write_str("board output continuation has a foreign physical owner")
            }
            Self::PoisonedOutput => f.write_str("board output lock poisoned"),
            Self::Output(e) => e.fmt(f),
            Self::Motion(e) => e.fmt(f),
        }
    }
}

/// Pumping82b2f7fc and all later board/output callbacks remain pending.
pub(crate) struct Update5BeforeBoardPumping {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) captured_body: SimulationRigidBodyObservation,
    pub(crate) assembly: super::board_fill_wheels::BoardFillAssemblyDescriptor,
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_fill<N: SkeletonCollisionNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardFill,
        numeric: &mut N,
    ) -> Result<Update5BeforeBoardPumping, BoardFillPrefixError<N::Error>> {
        use BoardFillPrefixError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardFill { owner: prior_owner } = before;
        if !Arc::ptr_eq(&owner, &prior_owner) {
            return Err(E::ForeignOwner);
        }
        let (sample, captured_body, assembly) = {
            let mut output = owner.phys_out().lock().map_err(|_| E::PoisonedOutput)?;
            let mut source = ActualBoardFillSource {
                world: self,
                board,
                owner: &owner,
                output: Arc::clone(output.occurrence()),
                body: None,
                sample: None,
                assembly: None,
            };
            output
                .try_fill_board_transform_directions(&mut source, numeric)
                .map_err(E::Output)?;
            (
                source
                    .sample
                    .expect("successful entry sampled linear velocity"),
                source.body.expect("native entry captured Part6 body"),
                source
                    .assembly
                    .expect("native entry captured board assembly"),
            )
        };
        // Release the output borrow before the existing same-pair/frame bridge.
        self.try_publish_pair_board_motion(board, skeleton, sample, numeric)
            .map_err(E::Motion)?;
        Ok(Update5BeforeBoardPumping {
            owner,
            captured_body,
            assembly,
        })
    }
}

struct ActualBoardFillSource<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    output: Arc<()>,
    body: Option<SimulationRigidBodyObservation>,
    sample: Option<PhysicalPlayerBoardMotionSample>,
    assembly: Option<super::board_fill_wheels::BoardFillAssemblyDescriptor>,
}
impl BoardFillSource for ActualBoardFillSource<'_> {
    type Error = BoardFillSourceError;
    fn capture_body(&mut self) -> Result<(), Self::Error> {
        // Native f430/f438 retains Assembly before f43c captures Part6's body.
        self.world.board(self.board).map_err(Self::Error::Pool)?;
        self.assembly =
            Some(super::board_fill_wheels::BoardFillAssemblyDescriptor::capture(self.board));
        self.body = Some(
            self.world
                .board(self.board)
                .map_err(Self::Error::Pool)?
                .try_part_contact_spy_observation(&self.world.backend, 6)
                .map_err(Self::Error::Part)?,
        );
        Ok(())
    }
    fn part_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.world
            .board(self.board)
            .map_err(Self::Error::Pool)?
            .try_live_part_transform_typed(&self.world.backend, 6)
            .map_err(Self::Error::Transform)
    }
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        try_current_board_effective_transform(
            self.world.board(self.board).map_err(Self::Error::Pool)?,
            &self.world.backend,
            self.owner,
        )
        .map_err(Self::Error::EffectiveTransform)
    }
    fn sample_linear_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let sample = self
            .world
            .try_sample_captured_board_motion(
                self.body
                    .expect("native entry captures the body before this read"),
                Arc::clone(&self.output),
            )
            .map_err(Self::Error::Motion)?;
        let linear = sample.linear_velocity();
        self.sample = Some(sample);
        Ok(linear)
    }
    fn look_direction_flipped(&mut self) -> Result<bool, Self::Error> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Missing("Reckoning lock"))?
                .processed(),
        );
        let flags = processed
            .lock()
            .map_err(|_| Self::Error::Missing("Processed lock"))?
            .physics_state_fields()
            .flags_8cc
            .ok_or(Self::Error::Missing("Processed+8cc"))?;
        Ok(flags & 8 != 0)
    }
    fn draw_velocity_direction(&mut self) -> Result<bool, Self::Error> {
        self.owner
            .inputs()
            .cvars
            .try_skateboard_vel_direction()
            .map_err(Self::Error::Cvar)
    }
    fn draw_look_direction(&mut self) -> Result<bool, Self::Error> {
        self.owner
            .inputs()
            .cvars
            .try_skater_look_direction()
            .map_err(Self::Error::Cvar)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.world
            .debug_overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
