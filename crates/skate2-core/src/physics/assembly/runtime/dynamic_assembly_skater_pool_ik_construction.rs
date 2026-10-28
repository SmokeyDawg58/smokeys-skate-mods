//! Constructor82b5743c on the unadmitted Skeleton and its already admitted
//! board. Both retain the SAME physical owner and selected manager.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError,
};
use crate::{
    dynamic_assembly_session::skater_pool::physical::SimulationPhysicalBodyKind,
    reckoning_numeric::{ReckoningHostNumericError, ReckoningNumeric},
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_attributes::SkateboardAttributeError,
    skeleton_ik_geometry::SkeletonIkGeometryError,
};
use std::{convert::Infallible, fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum SimulationSkeletonIkConstructionError<N = ReckoningHostNumericError, R = Infallible>
{
    Pool(SimulationSkaterPoolError),
    DifferentPhysicalOwner,
    Children(crate::skeleton_assembly_construction::constructor::SkeletonConstructorRuntimeError),
    Construction(SkeletonIkGeometryError<SkateboardAttributeError, N, R>),
}
impl<N: fmt::Debug, R: fmt::Debug> fmt::Display for SimulationSkeletonIkConstructionError<N, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "IK constructor owner: {e}"),
            Self::DifferentPhysicalOwner => {
                f.write_str("IK constructor board belongs to a different player")
            }
            Self::Children(e) => write!(f, "Skeleton constructor children: {e}"),
            Self::Construction(e) => write!(f, "IK constructor: {e:?}"),
        }
    }
}
impl<N: fmt::Debug, R: fmt::Debug> std::error::Error
    for SimulationSkeletonIkConstructionError<N, R>
{
}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_construct_paired_skeleton_members<
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &mut crate::skeleton_assembly_construction::SkeletonAssemblyConstruction,
        board: &SimulationSkaterAssemblyHandle,
        current_entity_flags: &mut impl FnMut() -> u32,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), SimulationSkeletonIkConstructionError<N::Error, R::Error>> {
        use SimulationSkeletonIkConstructionError as Error;
        let (kind, board_owner) = self
            .try_physical_member_owner(backend, board)
            .map_err(Error::Pool)?;
        if kind != SimulationPhysicalBodyKind::Board {
            return Err(Error::Pool(SimulationSkaterPoolError::WrongMemberKind));
        }
        let owner = Arc::clone(skeleton.owner());
        if owner.manager() != self.manager {
            return Err(Error::Pool(SimulationSkaterPoolError::ForeignManager));
        }
        if !Arc::ptr_eq(&owner, board_owner) {
            return Err(Error::DifferentPhysicalOwner);
        }
        let vault = Arc::clone(&owner.inputs().vault);
        let index = self.member_index(board).map_err(Error::Pool)?;
        let MemberContent::Board { candidate, .. } = &mut self.members[index].content else {
            unreachable!("actual board checked")
        };
        let deck = candidate
            .construction
            .as_mut()
            .expect("actual construction checked")
            .attributes_mut();
        skeleton
            .constructor
            .try_begin(&vault, &owner, current_entity_flags)
            .map_err(Error::Children)?;
        skeleton
            .constructor
            .current_animation
            .as_mut()
            .expect("early current animation")
            .try_construct_ik(&vault, deck, numeric, rounding)
            .map_err(Error::Construction)?;
        skeleton
            .constructor
            .try_finish_children(&vault)
            .map_err(Error::Children)
    }
}
