//! Complete actual retained selector consumption and optional same-owner relaunch.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_trajectory_owners::{
        launch_continue::{LaunchContinueError, try_continue_private_launch},
        launch_prepare::{PrepareError, PrepareLiveError},
        selector_finish::FinishSource,
        selector_update::{SelectorError, SelectorProgress},
    },
    physics::PointGraph,
    reckoning_attributes::ReckoningAttributeError,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::{
        ground_pre_solve::GroundPreparationGrindPolicy,
        physical_selector_runtime::SelectorRuntimeError,
    },
    skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum FinishLiveError {
    Missing(&'static str),
    Poisoned,
    Layout(AttributeRuntimeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for FinishLiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "selector finish missing {name}"),
            Self::Poisoned => f.write_str("selector finish poisoned"),
            Self::Layout(e) => write!(f, "selector finish layout: {e}"),
            Self::Cvar(e) => write!(f, "selector finish cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "selector finish overlay: {e:?}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum SelectorCompletionError<N, R> {
    Prefix(SelectorRuntimeError<N>),
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Finish(SelectorError<FinishLiveError, N>),
    Prepare(PrepareError<PrepareLiveError<ReckoningAttributeError>, N, R>),
    Launch(LaunchContinueError<ReckoningAttributeError, N>),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display for SelectorCompletionError<N, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prefix(e) => write!(f, "selector prefix: {e}"),
            Self::Missing(name) => write!(f, "selector completion missing {name}"),
            Self::Poisoned => f.write_str("selector completion poisoned"),
            Self::ForeignOwner => f.write_str("selector completion foreign owner"),
            Self::Pair(e) => write!(f, "selector completion pair: {e:?}"),
            Self::Board(e) => write!(f, "selector completion board: {e:?}"),
            Self::Finish(e) => write!(f, "selector finish: {e}"),
            Self::Prepare(e) => write!(f, "selector second-pass preparation: {e:?}"),
            Self::Launch(e) => write!(f, "selector second-pass launch: {e}"),
        }
    }
}
struct Source<'a> {
    processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    trajectory: &'a WallRideTrajectoryAttributes,
    cvars: &'a InputCvarReferences,
    overlay: &'a DebugOverlaySlot,
}
fn word(layout: Option<&AttributeRuntimeData>, offset: usize) -> f32 {
    layout.map_or(0.0, |d| {
        f32::from_be_bytes(
            d.bytes()[offset..offset + 4]
                .try_into()
                .expect("captured24c"),
        )
    })
}
impl FinishSource for Source<'_> {
    type Error = FinishLiveError;
    type Layout = Option<AttributeRuntimeData>;
    fn layout(&mut self) -> Result<Self::Layout, Self::Error> {
        self.trajectory.capture().map_err(Self::Error::Layout)
    }
    fn scalar(&mut self, layout: &Self::Layout, offset: usize) -> Result<f32, Self::Error> {
        Ok(word(layout.as_ref(), offset))
    }
    fn graph(&mut self, layout: &Self::Layout, offset: usize, x: f32) -> Result<f32, Self::Error> {
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| word(layout.as_ref(), offset + i * 4)),
            y: std::array::from_fn(|i| word(layout.as_ref(), offset + 32 + i * 4)),
        }
        .evaluate(x))
    }
    fn up(&mut self) -> Result<V, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .reckoning_up()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("up220"))
    }
    fn ground_normal(&mut self) -> Result<V, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .ground_normal()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("ground1d0"))
    }
    fn show_wall(&mut self) -> Result<bool, Self::Error> {
        self.cvars.try_show_wall_info().map_err(Self::Error::Cvar)
    }
    fn show_trajectories(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_show_trajectories()
            .map_err(Self::Error::Cvar)
    }
    fn show_select(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_show_trajectory_select()
            .map_err(Self::Error::Cvar)
    }
    fn overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
impl PhysicalAssemblyWorld {
    /// Whole ordinary selector.Update82dc6528 with explicit host grind policy.
    /// Predictor.Update/harvest remains in its separate source scheduling phase.
    pub(crate) fn try_complete_pair_selector<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut N,
        rounding: &mut R,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<bool, SelectorCompletionError<N::Error, R::Error>> {
        use SelectorCompletionError as E;
        match self
            .try_complete_pair_selector_prefix(board, skeleton, ground, numeric, policy)
            .map_err(E::Prefix)?
        {
            SelectorProgress::Retained(ready) => return Ok(ready),
            SelectorProgress::NearLanding => return Ok(false),
            SelectorProgress::BeforeCom => {}
        }
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut attributes = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .reckoning()
            .ok_or(E::Missing("Reckoning"))?
            .cloned_attribute_bindings();
        pool.try_with_ground_control_board(&self.backend, board, |runtime, _| {
            let trajectory = &runtime.wallride_trajectory;
            {
                let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
                let selector = trajectories
                    .onboard
                    .as_mut()
                    .ok_or(E::Missing("onboard selector"))?;
                if !selector.bindings.matches(&owner, &processed) {
                    return Err(E::ForeignOwner);
                }
                if !selector
                    .try_finish_completed_query(
                        &mut Source {
                            processed: &processed,
                            trajectory,
                            cvars: &owner.inputs().cvars,
                            overlay: &self.debug_overlay,
                        },
                        numeric,
                    )
                    .map_err(E::Finish)?
                {
                    return Ok(selector.ready);
                }
                selector
                    .try_prepare_private_launch(
                        &owner,
                        &mut attributes,
                        trajectory,
                        numeric,
                        rounding,
                    )
                    .map_err(E::Prepare)?;
            }
            // Submission resets these same futures under selector lock.
            // Winning buffer and ready=true remain published during second pass.
            try_continue_private_launch(
                &owner,
                &mut attributes,
                trajectory,
                &owner.inputs().cvars,
                &self.debug_overlay,
                self.predictor
                    .value
                    .as_ref()
                    .ok_or(E::Missing("World predictor"))?,
                self.world_line_queries
                    .as_mut()
                    .ok_or(E::Missing("World queries"))?,
                ground,
                self.dmos.as_ref().ok_or(E::Missing("DMO collection"))?,
                numeric,
            )
            .map_err(E::Launch)?;
            let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_mut()
                .ok_or(E::Missing("onboard selector"))?;
            selector.pass = 2; //4088 only after private Launch completes
            Ok(selector.ready)
        })
        .map_err(E::Board)?
    }
}
