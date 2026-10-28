//! Same-owner private Launch82dc2800 prefix through the stores at82dc2af0.
//! Returns before `CalcMaxTime82dc2af4`; no query is submitted or marked ready.
use super::OnboardTrajectoryOwner;
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    dynamic_assembly_session::skeleton_normal::postphysics_preparation::transform_point,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physics::{
        PointGraph,
        trajectory_fan::{FanAttributes, FanError, FanLaunch, FanStore, initialize_fan},
    },
    reckoning_attributes::{ReckoningAttributeRead, ReckoningAttributeScope},
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
use std::sync::{Arc, Mutex};

const RADIUS: u64 = 0xc671_0942_e297_d587;
const START_OFFSET: u64 = 0xf85a_91f6_a233_ddae;

#[cfg(test)]
#[path = "physical_player_trajectory_launch_prepare_tests.rs"]
mod tests;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LaunchPreparation {
    BeforeCalcMaxTime,
}
#[derive(Debug)]
pub(crate) enum PrepareError<A, N, R> {
    Source(A),
    Numeric(N),
    Rounding(R),
}
#[derive(Debug)]
pub(crate) enum PrepareLiveError<A> {
    Missing(&'static str),
    ForeignOwner,
    Poisoned,
    Attribute(A),
    Layout(AttributeRuntimeError),
}
impl<A: std::fmt::Display> std::fmt::Display for PrepareLiveError<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "trajectory launch missing {name}"),
            Self::ForeignOwner => f.write_str("trajectory launch owner mismatch"),
            Self::Poisoned => f.write_str("trajectory launch processed lock poisoned"),
            Self::Attribute(error) => error.fmt(f),
            Self::Layout(error) => error.fmt(f),
        }
    }
}
impl<A: std::fmt::Display, N: std::fmt::Display, R: std::fmt::Display> std::fmt::Display
    for PrepareError<A, N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => error.fmt(f),
            Self::Numeric(error) => write!(f, "trajectory launch numeric: {error}"),
            Self::Rounding(error) => write!(f, "trajectory launch rounding: {error}"),
        }
    }
}
trait PrepareSource: FanAttributes {
    fn ground_normal(&mut self) -> Result<Vector, Self::Error>;
    fn velocity_y(&mut self) -> Result<f32, Self::Error>;
    fn ground_position(&mut self) -> Result<Vector, Self::Error>;
    fn reckoning_graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error>;
    fn reckoning_scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
}

#[expect(
    clippy::type_complexity,
    reason = "separate attribute, vector and rounding error authorities"
)]
fn prepare<S: PrepareSource, N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    owner: &mut OnboardTrajectoryOwner,
    source: &mut S,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<LaunchPreparation, PrepareError<S::Error, N::Error, R::Error>> {
    use PrepareError::{Numeric, Source};
    owner.targeting_wallride = false; // 281c
    let launch = owner.launch;
    initialize_fan(
        FanLaunch {
            start: launch.start_velocity,
            before_jump: launch.velocity_before_jump,
            cone_x: launch.cone_angle_x,
            cone_z: launch.cone_angle_z,
            jumped: launch.player_jumped,
            count: launch.num_trajectories,
        },
        source,
        numeric,
        rounding,
        |store| match store {
            FanStore::Count(value) => owner.num_trajectories = value,
            FanStore::HighestWallride(value) => owner.highest_wallride_position = value,
            FanStore::Velocity(index, value) => owner.data[index].velocity = value,
        },
    )
    .map_err(|error| match error {
        FanError::Attribute(error) => Source(error),
        FanError::Numeric(error) => Numeric(error),
        FanError::Rounding(error) => PrepareError::Rounding(error),
    })?;
    owner.start_board_position = launch.board_position; // 2844
    let normal = source.ground_normal().map_err(Source)?;
    let slope_input = f32::from_bits(normal.xyz[1].to_bits() & 0x7fff_ffff);
    let slope_factor = source.reckoning_graph(0x260, slope_input).map_err(Source)?;
    let velocity_y = source.velocity_y().map_err(Source)?;
    let velocity_factor = source
        .reckoning_graph(0x220, velocity_y * 0.1)
        .map_err(Source)?;
    let factor = if velocity_factor - slope_factor >= 0.0 {
        velocity_factor
    } else {
        slope_factor
    };
    let ground = source.ground_position().map_err(Source)?;
    let delta = launch.com_position.zip(ground, |a, b| a - b);
    let up = launch.reckoning_orientation.columns()[1];
    let distance = numeric.dot_vector(delta, up).map_err(Numeric)?;
    let radius = source.reckoning_scalar(RADIUS).map_err(Source)?;
    let available = distance - radius;
    let offset = source.reckoning_scalar(START_OFFSET).map_err(Source)?;
    let limited = if available - offset >= 0.0 {
        offset
    } else {
        available
    };
    let authored_part = offset * factor; // rounded fmuls29f0
    let displacement = limited.mul_add(1.0 - factor, authored_part); // fmadds2a10
    let correction = up.map(|lane| lane * displacement);
    owner.start_trajectory_position = launch.com_position.zip(correction, |a, b| a - b);
    if launch.use_start_override {
        owner.start_trajectory_position = launch.trajectory_start_override;
        owner.start_board_position = launch.board_position_override;
    }
    // Raw2a64 uses original launch.boardPos, even when startBoardPos is overridden.
    let board_offset = launch
        .board_position
        .zip(owner.start_trajectory_position, |a, b| a - b);
    let initial_offset = launch
        .com_position
        .zip(owner.start_trajectory_position, |a, b| a - b);
    owner.board_position_local =
        transform_point(&launch.reckoning_orientation_inverse, board_offset.xyz);
    owner.com_position_local =
        transform_point(&launch.reckoning_orientation_inverse, initial_offset.xyz);
    owner.initial_offset = initial_offset;
    Ok(LaunchPreparation::BeforeCalcMaxTime)
}

struct LiveSource<'a, A> {
    processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    attributes: &'a mut A,
    trajectory: &'a WallRideTrajectoryAttributes,
}
fn layout_word(layout: Option<&AttributeRuntimeData>, offset: usize) -> f32 {
    // Generated trajectory owner828b1068 establishes DefaultDataArea24c on null.
    layout.map_or(0.0, |data| {
        f32::from_be_bytes(
            data.bytes()[offset..offset + 4]
                .try_into()
                .expect("captured trajectory layout"),
        )
    })
}
impl<A: ReckoningAttributeRead> FanAttributes for LiveSource<'_, A> {
    type Error = PrepareLiveError<A::Error>;
    fn cone_z_graph(&mut self, input: f32) -> Result<f32, Self::Error> {
        let layout = self.trajectory.capture().map_err(Self::Error::Layout)?;
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|i| layout_word(layout.as_ref(), 0x130 + i * 4)),
            y: std::array::from_fn(|i| layout_word(layout.as_ref(), 0x150 + i * 4)),
        }
        .evaluate(input))
    }
    fn speed_bounds(&mut self) -> Result<(f32, f32), Self::Error> {
        let layout = self.trajectory.capture().map_err(Self::Error::Layout)?;
        Ok((
            layout_word(layout.as_ref(), 0x1b8),
            layout_word(layout.as_ref(), 0x1bc),
        ))
    }
}
impl<A: ReckoningAttributeRead> PrepareSource for LiveSource<'_, A> {
    fn ground_normal(&mut self) -> Result<Vector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .ground_normal()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("groundNormal1d0"))
    }
    fn velocity_y(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .velocity()
            .map(|v| v.value.xyz[1])
            .ok_or(Self::Error::Missing("velocity190.Y"))
    }
    fn ground_position(&mut self) -> Result<Vector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .ground_position_all_states()
            .ok_or(Self::Error::Missing("groundPosAllStates1f0"))
    }
    fn reckoning_graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error> {
        let layout = self
            .attributes
            .capture_layout(ReckoningAttributeScope::Global)
            .map_err(Self::Error::Attribute)?;
        self.attributes
            .evaluate_graph8(&layout, offset, input)
            .map_err(Self::Error::Attribute)
    }
    fn reckoning_scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        let pointer = self
            .attributes
            .lookup(ReckoningAttributeScope::Global, key)
            .map_err(Self::Error::Attribute)?;
        self.attributes
            .float(&pointer)
            .map_err(Self::Error::Attribute)
    }
}
impl OnboardTrajectoryOwner {
    #[expect(
        clippy::type_complexity,
        reason = "separate attribute, vector and rounding error authorities"
    )]
    pub(crate) fn try_prepare_private_launch<
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        attributes: &mut A,
        trajectory: &WallRideTrajectoryAttributes,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<LaunchPreparation, PrepareError<PrepareLiveError<A::Error>, N::Error, R::Error>>
    {
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| PrepareError::Source(PrepareLiveError::Poisoned))?
                .processed(),
        );
        if !self.bindings.matches(owner, &processed) {
            return Err(PrepareError::Source(PrepareLiveError::ForeignOwner));
        }
        prepare(
            self,
            &mut LiveSource {
                processed: &processed,
                attributes,
                trajectory,
            },
            numeric,
            rounding,
        )
    }
}
