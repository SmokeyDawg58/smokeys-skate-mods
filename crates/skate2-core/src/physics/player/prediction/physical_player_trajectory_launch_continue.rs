//! Private Launch82dc2af4..2d04: actual same-owner predictor submissions.
use super::max_time::{MaximumTimeError, try_maximum_time};
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_attributes::{ReckoningAttributeRead, ReckoningAttributeScope},
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        prediction::{PhysicalPredictionOwner, runtime::PredictionRuntimeError},
        world_line_query::SimulationWorldLineQueries,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as Vector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

const RADIUS: u64 = 0xc671_0942_e297_d587;

#[cfg(test)]
#[path = "physical_player_trajectory_launch_continue_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum ContinuationError<E> {
    Source(E),
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    /// `LaunchAIPath`'s assertion: the AI input has no valid trajectory.
    InvalidAiTrajectory,
}
impl<E: std::fmt::Debug> std::fmt::Display for ContinuationError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "private trajectory Launch: {self:?}")
    }
}
#[derive(Debug)]
pub(crate) enum ContinueLiveError<A, N> {
    Attribute(A),
    Layout(AttributeRuntimeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    MaximumTime(MaximumTimeError<N>),
    Prediction(PredictionRuntimeError<N>),
    Missing(&'static str),
    Poisoned,
    Numeric(N),
}
pub(crate) type LaunchContinueError<A, N> = ContinuationError<ContinueLiveError<A, N>>;

/// The processed `AIPhysicsInput` trajectory (+0x660..+0x6a0) and flag word (+0x6b0).
#[derive(Clone, Copy, Debug, PartialEq)]
struct AiTrajectory {
    position: [f32; 4],
    velocity: [f32; 4],
    gravity: [f32; 4],
    offset: [f32; 4],
    flags: u32,
}

/// `Physics_Trajectory` cvars at their defaults: `kAiTrajectoryMinVelRatio` and
/// `kAiTrajectoryMaxVelRatio`.
const AI_TRAJECTORY_MIN_VELOCITY_RATIO: f32 = f32::from_bits(0x3eaa_7efa);
const AI_TRAJECTORY_MAX_VELOCITY_RATIO: f32 = 3.0;
/// `AIPhysicsInput`'s ValidTrajectory bit.
const AI_VALID_TRAJECTORY: u32 = 0x0400_0000;

fn lanes(vector: Vector) -> [f32; 4] {
    [vector.xyz[0], vector.xyz[1], vector.xyz[2], vector.w.unwrap_or(0.0)]
}

/// `TrajectorySelector::AIPathIsUseable` 0x82db6100: the first candidate's start lies within
/// sqrt(1.5) of the path trajectory's, heads no more than square to it, and is either still or
/// within the speed ratio limits of the path's.
fn ai_path_is_useable<N: ReckoningNumeric>(
    start: [f32; 4],
    velocity: [f32; 4],
    time_delta: f32,
    trajectory: &AiTrajectory,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let offset: [f32; 4] =
        std::array::from_fn(|lane| velocity[lane].mul_add(time_delta, start[lane]) - trajectory.position[lane]);
    if !(numeric.dot3(offset, offset)? < 1.5) {
        return Ok(false);
    }
    if 0.0 > numeric.dot3(velocity, trajectory.velocity)? {
        return Ok(false);
    }
    let launch_speed = crate::reckoning_numeric::try_length(velocity, numeric)?;
    if launch_speed <= f32::from_bits(0x38d1_b717) {
        return Ok(true);
    }
    let ratio = crate::reckoning_numeric::try_length(trajectory.velocity, numeric)? / launch_speed;
    Ok(!(ratio < AI_TRAJECTORY_MIN_VELOCITY_RATIO) && !(ratio > AI_TRAJECTORY_MAX_VELOCITY_RATIO))
}

/// `TrajectorySelector::LaunchAIPath` 0x82db94f8: the selector follows the path trajectory: its
/// error from the first candidate's start and its centre-of-mass offset are kept, and one
/// fake-body prediction (`SetupFakeBodyPrediction` 0x82e4bb18 over the AI trajectory's own
/// position, velocity and gravity, both error ratios 1) goes in the first query.
#[expect(clippy::too_many_arguments, reason = "the launch's selector reads and the AI trajectory")]
fn launch_ai_path<N: ReckoningNumeric, S: ContinueSource<N>>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    source: &mut S,
    trajectory: &AiTrajectory,
    velocity: [f32; 4],
    start: [f32; 4],
    time_delta: f32,
    duration: f32,
    numeric: &mut N,
) -> Result<(), ContinuationError<S::Error>> {
    use ContinuationError::{Missing, Poisoned, Source};
    if trajectory.flags & AI_VALID_TRAJECTORY == 0 {
        return Err(ContinuationError::InvalidAiTrajectory);
    }
    {
        let mut trajectories = owner.trajectories().lock().map_err(|_| Poisoned)?;
        let selector = trajectories.onboard.as_mut().ok_or(Missing("onboard selector"))?;
        selector.ai_trajectory_error =
            std::array::from_fn(|lane| velocity[lane].mul_add(time_delta, start[lane]) - trajectory.position[lane]);
        selector.ai_com_position_local = trajectory.offset;
        selector.num_trajectories = 1;
        selector.ai_trajectory_following = true;
    }
    let draw = source.show_trajectories().map_err(Source)?;
    let radius_pointer = source.radius_pointer().map_err(Source)?;
    let radius = source.radius_value(&radius_pointer).map_err(Source)?;
    source
        .submit(
            0,
            BallisticTrajectoryInput {
                position: Vector::known(trajectory.position),
                velocity: Vector::known(trajectory.velocity),
                gravity: Vector::known(trajectory.gravity),
                duration,
                radius,
                maximum_error_start: 1.0,
                maximum_error_end: 1.0,
            },
            draw,
            numeric,
        )
        .map_err(Source)
}

/// Source-time services around the real selector; this trait owns no state.
trait ContinueSource<N: ReckoningNumeric> {
    type Error;
    type Layout;
    type Pointer;
    fn maximum_time(&mut self, velocity: Vector, numeric: &mut N) -> Result<f32, Self::Error>;
    fn ai_active(&mut self) -> Result<bool, Self::Error>;
    fn ai_trajectory(&mut self) -> Result<AiTrajectory, Self::Error>;
    fn numeric(&self, error: N::Error) -> Self::Error;
    fn capture_trajectory(&mut self) -> Result<Self::Layout, Self::Error>;
    fn show_trajectories(&mut self) -> Result<bool, Self::Error>;
    fn radius_pointer(&mut self) -> Result<Self::Pointer, Self::Error>;
    fn error_end(&mut self, layout: &Self::Layout) -> Result<f32, Self::Error>;
    fn radius_value(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn error_start(&mut self, layout: &Self::Layout) -> Result<f32, Self::Error>;
    fn gravity(&self) -> Vector;
    fn submit(
        &mut self,
        slot: usize,
        request: BallisticTrajectoryInput,
        draw: bool,
        numeric: &mut N,
    ) -> Result<(), Self::Error>;
    fn debug_draw(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}

fn continue_launch<N: ReckoningNumeric, S: ContinueSource<N>>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    source: &mut S,
    numeric: &mut N,
) -> Result<(), ContinuationError<S::Error>> {
    use ContinuationError::{Missing, Poisoned, Source};
    let start_velocity = {
        let trajectories = owner.trajectories().lock().map_err(|_| Poisoned)?;
        trajectories
            .onboard
            .as_ref()
            .ok_or(Missing("onboard selector"))?
            .data[0]
            .velocity
    };
    let duration = source
        .maximum_time(start_velocity, numeric)
        .map_err(Source)?;
    // Launch82dc2b00: a non-local skater whose AI path suits the launch follows it instead.
    let ai = if source.ai_active().map_err(Source)? {
        let trajectory = source.ai_trajectory().map_err(Source)?;
        let (start, time_delta) = {
            let trajectories = owner.trajectories().lock().map_err(|_| Poisoned)?;
            let selector = trajectories.onboard.as_ref().ok_or(Missing("onboard selector"))?;
            (lanes(selector.start_trajectory_position), selector.launch.time_delta)
        };
        let useable = ai_path_is_useable(start, lanes(start_velocity), time_delta, &trajectory, numeric)
            .map_err(|error| Source(source.numeric(error)))?;
        useable.then_some((trajectory, start, time_delta))
    } else {
        None
    };
    if let Some((trajectory, start, time_delta)) = ai {
        launch_ai_path(owner, source, &trajectory, lanes(start_velocity), start, time_delta, duration, numeric)?;
    } else {
        let mut slot = 0;
        loop {
            let count = {
                let trajectories = owner.trajectories().lock().map_err(|_| Poisoned)?;
                trajectories
                    .onboard
                    .as_ref()
                    .ok_or(Missing("onboard selector"))?
                    .num_trajectories
            };
            if count <= 0 || slot >= usize::try_from(count).expect("positive signed count") {
                break;
            }
            let layout = source.capture_trajectory().map_err(Source)?;
            let draw = source.show_trajectories().map_err(Source)?;
            let radius_pointer = source.radius_pointer().map_err(Source)?;
            let (velocity, position) = {
                let trajectories = owner.trajectories().lock().map_err(|_| Poisoned)?;
                let selector = trajectories
                    .onboard
                    .as_ref()
                    .ok_or(Missing("onboard selector"))?;
                let velocity = selector
                    .data
                    .get(slot)
                    .ok_or(Missing("onboard candidate"))?
                    .velocity;
                let position = velocity.zip(selector.start_trajectory_position, |v, p| {
                    v.mul_add(selector.launch.time_delta, p)
                });
                (velocity, position)
            }; // REQUIRED: queue submission resets this same owner's future under its lock.
            let maximum_error_end = source.error_end(&layout).map_err(Source)?;
            let radius = source.radius_value(&radius_pointer).map_err(Source)?;
            let maximum_error_start = source.error_start(&layout).map_err(Source)?;
            source
                .submit(
                    slot,
                    BallisticTrajectoryInput {
                        position,
                        velocity,
                        gravity: source.gravity(),
                        duration,
                        radius,
                        maximum_error_start,
                        maximum_error_end,
                    },
                    draw,
                    numeric,
                )
                .map_err(Source)?;
            slot += 1;
        }
    }
    {
        let mut trajectories = owner.trajectories().lock().map_err(|_| Poisoned)?;
        trajectories
            .onboard
            .as_mut()
            .ok_or(Missing("onboard selector"))?
            .just_started_query = true;
    } // Source2c68 store precedes cvar2c74 and overlay/attribute failures.
    if source.debug_draw().map_err(Source)? {
        source.require_overlay().map_err(Source)?;
        // Source2ce4 obtains a pointer/default area but NEVER loads the float.
        let _ = source.radius_pointer().map_err(Source)?;
    }
    Ok(())
}

struct LiveSource<'a, A> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    processed: Arc<Mutex<SkeletonProcessedContact>>,
    attributes: &'a mut A,
    trajectory: &'a WallRideTrajectoryAttributes,
    references: &'a InputCvarReferences,
    overlay: &'a DebugOverlaySlot,
    predictor: &'a PhysicalPredictionOwner,
    world: &'a mut SimulationWorldLineQueries,
    ground: &'a StaticGroundPool,
    dmos: &'a SimulationDmoAssemblyPool,
}
fn word(layout: Option<&AttributeRuntimeData>, offset: usize) -> f32 {
    // Generated ctor828b1068 establishes DefaultDataArea24c for null layout.
    layout.map_or(0.0, |data| {
        f32::from_be_bytes(
            data.bytes()[offset..offset + 4]
                .try_into()
                .expect("captured trajectory layout"),
        )
    })
}
impl<A: ReckoningAttributeRead, N: ReckoningNumeric> ContinueSource<N> for LiveSource<'_, A> {
    type Error = ContinueLiveError<A::Error, N::Error>;
    type Layout = Option<AttributeRuntimeData>;
    type Pointer = A::Pointer;
    fn maximum_time(&mut self, velocity: Vector, numeric: &mut N) -> Result<f32, Self::Error> {
        try_maximum_time(
            self.trajectory,
            self.references,
            self.overlay,
            velocity,
            numeric,
        )
        .map_err(ContinueLiveError::MaximumTime)
    }
    fn ai_active(&mut self) -> Result<bool, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| ContinueLiveError::Poisoned)?
            .known_flags_8c8()
            .map(|word| word & 0x4000_0000 != 0)
            .ok_or(ContinueLiveError::Missing("processed flags8c8"))
    }
    fn ai_trajectory(&mut self) -> Result<AiTrajectory, Self::Error> {
        let processed = self.processed.lock().map_err(|_| ContinueLiveError::Poisoned)?;
        let vectors = &processed.input_controls().ai_vectors;
        let vector = |index: usize| vectors[index].ok_or(ContinueLiveError::Missing("processed AI trajectory"));
        Ok(AiTrajectory {
            position: vector(5)?,
            velocity: vector(6)?,
            gravity: vector(7)?,
            offset: vector(9)?,
            flags: processed.physics_state_fields().ai_flags.0,
        })
    }
    fn numeric(&self, error: N::Error) -> Self::Error {
        ContinueLiveError::Numeric(error)
    }
    fn capture_trajectory(&mut self) -> Result<Self::Layout, Self::Error> {
        self.trajectory.capture().map_err(ContinueLiveError::Layout)
    }
    fn show_trajectories(&mut self) -> Result<bool, Self::Error> {
        self.references
            .try_show_trajectories()
            .map_err(ContinueLiveError::Cvar)
    }
    fn radius_pointer(&mut self) -> Result<Self::Pointer, Self::Error> {
        self.attributes
            .lookup(ReckoningAttributeScope::Global, RADIUS)
            .map_err(ContinueLiveError::Attribute)
    }
    fn error_end(&mut self, layout: &Self::Layout) -> Result<f32, Self::Error> {
        Ok(word(layout.as_ref(), 0x1b4))
    }
    fn radius_value(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        self.attributes
            .float(pointer)
            .map_err(ContinueLiveError::Attribute)
    }
    fn error_start(&mut self, layout: &Self::Layout) -> Result<f32, Self::Error> {
        Ok(word(layout.as_ref(), 0x1b0))
    }
    fn gravity(&self) -> Vector {
        Vector::known(self.predictor.gravity)
    }
    fn submit(
        &mut self,
        slot: usize,
        request: BallisticTrajectoryInput,
        draw: bool,
        numeric: &mut N,
    ) -> Result<(), Self::Error> {
        self.predictor
            .try_add_onboard_query(
                self.owner,
                slot,
                request,
                draw,
                self.world,
                self.ground,
                self.dmos,
                numeric,
            )
            .map_err(ContinueLiveError::Prediction)
    }
    fn debug_draw(&mut self) -> Result<bool, Self::Error> {
        self.references
            .try_trajectory_debug_draw()
            .map_err(ContinueLiveError::Cvar)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(ContinueLiveError::Overlay)
    }
}

/// Call once the same selector's private preparation reaches `BeforeCalcMaxTime`.
/// This completes normal query submission, never Predictor.Update/ready harvest.
#[expect(
    clippy::too_many_arguments,
    reason = "existing world, selector and source-time services are distinct owners"
)]
pub(crate) fn try_continue_private_launch<A: ReckoningAttributeRead, N: ReckoningNumeric>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    attributes: &mut A,
    trajectory: &WallRideTrajectoryAttributes,
    references: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
    predictor: &PhysicalPredictionOwner,
    world: &mut SimulationWorldLineQueries,
    ground: &StaticGroundPool,
    dmos: &SimulationDmoAssemblyPool,
    numeric: &mut N,
) -> Result<(), LaunchContinueError<A::Error, N::Error>> {
    let processed = Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| ContinuationError::Poisoned)?
            .processed(),
    );
    {
        let trajectories = owner
            .trajectories()
            .lock()
            .map_err(|_| ContinuationError::Poisoned)?;
        let selector = trajectories
            .onboard
            .as_ref()
            .ok_or(ContinuationError::Missing("onboard selector"))?;
        if !selector.bindings.matches(owner, &processed) {
            return Err(ContinuationError::ForeignOwner);
        }
    }
    continue_launch(
        owner,
        &mut LiveSource {
            owner,
            processed,
            attributes,
            trajectory,
            references,
            overlay,
            predictor,
            world,
            ground,
            dmos,
        },
        numeric,
    )
}
