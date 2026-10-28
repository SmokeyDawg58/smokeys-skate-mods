//! SAME Skeleton constructor/Normal owners for FillPhysOut58230..58614.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
    dynamic_assembly_session::{
        skater_pool::SimulationSkeletonJumpPairError, skeleton_normal::SkeletonNormalAssembly,
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::{
        com::PhysOutComSource,
        construct::skeleton_fill_prefix::{
            SkeletonFillPrefixOutputError, SkeletonFillPrefixSource,
        },
    },
    reckoning_numeric::ReckoningNumeric,
    skeleton_assembly_construction::constructor::SkeletonConstructorBindings,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SkeletonFillSourceError {
    MissingInput,
    PoisonedInput,
}
#[derive(Debug)]
pub(crate) enum SkeletonFillError<N> {
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    ForeignOwner,
    MissingConstructor,
    MissingBindings,
    PoisonedOutput,
    Output(SkeletonFillPrefixOutputError<SkeletonFillSourceError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for SkeletonFillError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(error) => write!(f, "Skeleton Fill pair: {error}"),
            Self::Pool(error) => write!(f, "Skeleton Fill pool: {error}"),
            Self::Output(error) => error.fmt(f),
            Self::ForeignOwner => f.write_str("Skeleton Fill foreign owner"),
            Self::MissingConstructor => f.write_str("Skeleton Fill missing constructor"),
            Self::MissingBindings => f.write_str("Skeleton Fill missing constructor bindings"),
            Self::PoisonedOutput => f.write_str("Skeleton Fill output lock poisoned"),
        }
    }
}
struct Source<'a> {
    normal: &'a SkeletonNormalAssembly,
    bindings: &'a SkeletonConstructorBindings,
}
impl PhysOutComSource for Source<'_> {
    type Error = SkeletonFillSourceError;
    fn centre_of_mass_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self
            .normal
            .post_physics_records()
            .centre_of_mass_velocity_typed())
    }
    fn centre_of_mass(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.normal.post_physics_records().centre_of_mass_typed())
    }
}
impl SkeletonFillPrefixSource for Source<'_> {
    type Input = Option<Arc<Mutex<AnimOutPhysInOwner>>>;
    fn capture_input(&mut self) -> Result<Self::Input, Self::Error> {
        Ok(self.bindings.animation_input.clone())
    }
    fn animation_deck_z(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let pose = &self.normal.current_animation().record().poses()[0];
        Ok(SkeletonRootVector::known([
            pose[8], pose[9], pose[10], pose[11],
        ]))
    }
    fn board_flipped(&mut self, input: &Self::Input) -> Result<bool, Self::Error> {
        Ok(input
            .as_ref()
            .ok_or(SkeletonFillSourceError::MissingInput)?
            .lock()
            .map_err(|_| SkeletonFillSourceError::PoisonedInput)?
            .flags
            .board_flipped)
    }
    fn root_column(&mut self, column: usize) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.normal.root_transforms().animation_to_world().columns()[column])
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_fill_paired_skeleton_output_prefix<N: ReckoningNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<SkeletonRootVector, SkeletonFillError<N::Error>> {
        use SkeletonFillError as E;
        let owner = self
            .try_with_jump_pair(backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        if !Arc::ptr_eq(&owner, expected_owner) {
            return Err(E::ForeignOwner);
        }
        let index = self.member_index(skeleton).map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &self.members[index].content else {
            unreachable!("validated Skeleton")
        };
        let constructor = &candidate
            .construction()
            .ok_or(E::MissingConstructor)?
            .constructor;
        let bindings = constructor.bindings.as_ref().ok_or(E::MissingBindings)?;
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_fill_skeleton_prefix_capture(
                &mut Source {
                    normal: candidate.owner(),
                    bindings,
                },
                numeric,
            )
            .map_err(E::Output)
    }
}
