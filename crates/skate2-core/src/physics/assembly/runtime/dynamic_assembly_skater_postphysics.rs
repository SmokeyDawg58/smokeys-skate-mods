//! Actual activation/compression stages preceding board contact processing.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardAssemblySession, SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::{
        part_activation::DynamicAssemblyActivationError,
        part_transform::DynamicAssemblyPartTransformError,
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_body_construction::wheel_compressions::{
        SkateboardWheelCompressionError, SkateboardWheelCompressionServices,
    },
    skeleton_root_transforms::SkeletonRootMatrix,
};

#[derive(Debug)]
pub(crate) enum BoardCompressionServiceError {
    Part(DynamicAssemblyPartTransformError),
    MissingDistance,
}
impl std::fmt::Display for BoardCompressionServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Part(e) => write!(f, "compression Part: {e}"),
            Self::MissingDistance => f.write_str("constructed wheel distance unavailable"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum SimulationBoardPostPrefixError<N> {
    Pool(SimulationSkaterPoolError),
    InvalidTimestep,
    Activation(DynamicAssemblyActivationError<SimulationRigidBodyPoolError>),
    Compression(SkateboardWheelCompressionError<BoardCompressionServiceError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for SimulationBoardPostPrefixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "board prefix: {e}"),
            Self::InvalidTimestep => f.write_str("board PostPhysics requires dt > 0"),
            Self::Activation(e) => write!(f, "{e}"),
            Self::Compression(e) => write!(f, "{e}"),
        }
    }
}
struct CompressionServices<'a> {
    session: &'a SkateboardAssemblySession,
    backend: &'a SkateboardSimulationBackend,
    distance: &'a Option<f32>,
}
impl SkateboardWheelCompressionServices for CompressionServices<'_> {
    type Error = BoardCompressionServiceError;
    fn body_transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error> {
        self.session
            .try_attached_part_body_transform(self.backend, part)
            .map_err(Self::Error::Part)
    }
    fn wheel_distance(&mut self) -> Result<f32, Self::Error> {
        self.distance.ok_or(Self::Error::MissingDistance)
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn with_board_post_fields<T>(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(&mut crate::skateboard_body_construction::SkateboardBodyPostFields) -> T,
    ) -> Result<T, SimulationSkaterPoolError> {
        let index = self.member_index(handle)?;
        let MemberContent::Board { candidate, .. } = &mut self.members[index].content else {
            return Err(SimulationSkaterPoolError::WrongMemberKind);
        };
        let fields = candidate
            .construction
            .as_mut()
            .and_then(|c| c.post_fields_mut())
            .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?;
        Ok(operation(fields))
    }

    /// Actual Assembly group, sampled at the per-contact read site82b36ca8.
    /// It is distinct from each collision volume's group and Part userdata.
    pub(crate) fn physical_assembly_group(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<u32, SimulationSkaterPoolError> {
        let index = self.member_index(handle)?;
        match &self.members[index].content {
            MemberContent::Board { candidate, .. } => candidate
                .construction()
                .and_then(crate::skateboard_body_construction::SkateboardBodyConstruction::group)
                .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction),
            MemberContent::Skeleton { candidate, .. } => {
                candidate
                    .construction()
                    .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?;
                Ok(candidate.owner().state().assembly_group)
            }
        }
    }

    /// Entry 0x82b36a58 through compression return0x82b36ab8, before the actual
    /// collision-list getter. Remaining `PostPhysics` stages must follow once.
    pub(crate) fn try_board_post_physics_prefix<N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        timestep: f32,
        numeric: &mut N,
    ) -> Result<(), SimulationBoardPostPrefixError<N::Error>> {
        use SimulationBoardPostPrefixError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let MemberContent::Board { candidate, .. } = &mut self.members[index].content else {
            return Err(E::Pool(SimulationSkaterPoolError::WrongMemberKind));
        };
        if timestep.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(E::InvalidTimestep);
        }
        candidate
            .session
            .try_activate(backend)
            .map_err(E::Activation)?;
        let construction = candidate.construction.as_mut().ok_or(E::Pool(
            SimulationSkaterPoolError::MissingPhysicalConstruction,
        ))?;
        let (distance, output) = construction.compression_parts();
        output
            .try_update(
                &mut CompressionServices {
                    session: &candidate.session,
                    backend,
                    distance,
                },
                numeric,
            )
            .map_err(E::Compression)
    }
    pub(crate) fn board_wheel_compressions(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<Option<[f32; 2]>, SimulationSkaterPoolError> {
        let index = self.member_index(handle)?;
        let MemberContent::Board { candidate, .. } = &self.members[index].content else {
            return Err(SimulationSkaterPoolError::WrongMemberKind);
        };
        Ok(candidate
            .construction
            .as_ref()
            .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?
            .wheel_compressions()
            .current())
    }
}
