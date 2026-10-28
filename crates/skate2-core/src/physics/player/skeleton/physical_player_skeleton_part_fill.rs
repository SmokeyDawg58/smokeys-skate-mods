//! Same constructor data and Normal owner for Skeleton Fill58618..589b4.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SimulationSkeletonJumpPairError, SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::{
        SkeletonNormalAssembly, part_transform_query::SkeletonPartTransformQueryError,
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::construct::skeleton_part_fill::{
        SkeletonPartFillError, SkeletonPartFillSource,
    },
    skeleton_data::SkeletonData,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum SkeletonPartOutputSourceError {
    Query(SkeletonPartTransformQueryError),
    MissingData,
    PartOutsideData(usize),
}
#[derive(Debug)]
pub(crate) enum PairedSkeletonPartFillError {
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    ForeignOwner,
    MissingConstructor,
    PoisonedOutput,
    Output(SkeletonPartFillError<SkeletonPartOutputSourceError>),
}
impl std::fmt::Display for SkeletonPartOutputSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(error) => error.fmt(f),
            Self::MissingData => f.write_str("Skeleton Fill requires constructed data"),
            Self::PartOutsideData(part) => write!(f, "Skeleton Fill part {part} outside data"),
        }
    }
}
impl std::fmt::Display for PairedSkeletonPartFillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(error) => error.fmt(f),
            Self::Pool(error) => error.fmt(f),
            Self::ForeignOwner => f.write_str("Skeleton part Fill foreign owner"),
            Self::MissingConstructor => f.write_str("Skeleton part Fill missing constructor"),
            Self::PoisonedOutput => f.write_str("Skeleton part Fill output lock poisoned"),
            Self::Output(error) => error.fmt(f),
        }
    }
}
struct Source<'a> {
    normal: &'a SkeletonNormalAssembly,
    data: Option<&'a Arc<SkeletonData>>,
}
impl SkeletonPartFillSource for Source<'_> {
    type Error = SkeletonPartOutputSourceError;
    fn volume_to_bone_translation(&mut self, part: usize) -> Result<[f32; 4], Self::Error> {
        let matrix = self
            .data
            .ok_or(Self::Error::MissingData)?
            .volume_to_bone()
            .get(part)
            .ok_or(Self::Error::PartOutsideData(part))?;
        Ok([matrix[12], matrix[13], matrix[14], matrix[15]])
    }
    fn part_transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error> {
        self.normal
            .try_query_part_transform(part)
            .map_err(Self::Error::Query)
    }
    fn hips_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.normal.physics_history().record().vector_typed(
            crate::skeleton_physics_history::lane_knowledge::RecordVectorField::Velocity(23),
        ))
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_fill_paired_skeleton_parts(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), PairedSkeletonPartFillError> {
        use PairedSkeletonPartFillError as E;
        let owner = self
            .try_with_jump_pair(backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        if !Arc::ptr_eq(&owner, expected_owner) {
            return Err(E::ForeignOwner);
        }
        let index = self.member_index(skeleton).map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &self.members[index].content else {
            unreachable!("validated Skeleton");
        };
        let construction = candidate.construction().ok_or(E::MissingConstructor)?;
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_fill_skeleton_parts(&mut Source {
                normal: candidate.owner(),
                data: construction.data(),
            })
            .map_err(E::Output)
    }
}
