//! Actual retained controller output82e04f78, stopping before Wipeout4f84.
use super::{
    update5_managers::GrindManagerFillPolicy, update5_plants::Update5BeforeControllerFill,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::controller_fill::{
        ControllerFillError, ControllerFillSource,
    },
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum ControllerFillRuntimeError {
    Poisoned,
    Missing(&'static str),
}
impl std::fmt::Display for ControllerFillRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("controller Fill owner poisoned"),
            Self::Missing(name) => write!(f, "controller Fill requires {name}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum Update5ControllerError<N> {
    MissingPool,
    ForeignOwner,
    Pair(SimulationSkeletonJumpPairError),
    Fill(ControllerFillError<ControllerFillRuntimeError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for Update5ControllerError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("controller Fill requires skater pool"),
            Self::ForeignOwner => {
                f.write_str("controller Fill continuation belongs to another pair")
            }
            Self::Pair(e) => write!(f, "controller Fill pair: {e:?}"),
            Self::Fill(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeWipeout {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) grind_policy: GrindManagerFillPolicy,
}
struct Source<'a> {
    world: &'a PhysicalAssemblyWorld,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
}
impl ControllerFillSource for Source<'_> {
    type Error = ControllerFillRuntimeError;
    fn state(&mut self) -> Result<u32, Self::Error> {
        self.owner
            .skateboard_controller()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .state()
            .ok_or(Self::Error::Missing("constructed controller state1bc"))
    }
    fn spine_world(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        let skeleton = self
            .world
            .skeleton(self.skeleton)
            .map_err(|_| Self::Error::Missing("same Skeleton"))?;
        // GetAnimPartTransformWorld82b3b878: exact volume row3040 +11*40.
        // GetPartTransform's optional physics-history branch does NOT belong here.
        let volume = skeleton
            .animation_targets()
            .volume_row(11)
            .ok_or(Self::Error::Missing("Spine3 animation volume row"))?;
        Ok(skeleton
            .root_transforms()
            .animation_to_world()
            .compose(&SkeletonRootMatrix::known(*volume)))
    }
    fn effective_animation(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        );
        let mut processed = processed.lock().map_err(|_| Self::Error::Poisoned)?;
        // Existing retained Update1 writer supplies +c0; this read changes no field.
        processed
            .ground_position_fields_mut()
            .effective_animation
            .ok_or(Self::Error::Missing(
                "Processed effective animation transform+c0",
            ))
    }
    fn retrieval_start_position(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let controller = self
            .owner
            .skateboard_controller()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?;
        let start = &controller
            .retrieval()
            .ok_or(Self::Error::Missing("actual retrieval start transform"))?
            .start;
        Ok(start.columns()[3])
    }
    fn deck_position(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        );
        let processed = processed.lock().map_err(|_| Self::Error::Poisoned)?;
        Ok(processed
            .deck_transform()
            .ok_or(Self::Error::Missing("Processed deck transform+40"))?
            .columns()[3])
    }
    fn flags_8cc(&mut self, _mask: u32) -> Result<u32, Self::Error> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        );
        let processed = processed.lock().map_err(|_| Self::Error::Poisoned)?;
        processed
            .physics_state_fields()
            .flags_8cc
            .ok_or(Self::Error::Missing("Processed flags8cc"))
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_controller<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeControllerFill,
        numeric: &mut N,
    ) -> Result<Update5BeforeWipeout, Update5ControllerError<N::Error>> {
        use Update5ControllerError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                Arc::ptr_eq(owner, &before.owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        {
            let mut output = before.owner.phys_out().lock().map_err(|_| {
                E::Fill(ControllerFillError::Source(
                    ControllerFillRuntimeError::Poisoned,
                ))
            })?;
            let mut source = Source {
                world: self,
                skeleton,
                owner: &before.owner,
            };
            output
                .try_fill_controller(&mut source, numeric)
                .map_err(E::Fill)?;
        }
        Ok(Update5BeforeWipeout {
            owner: before.owner,
            grind_policy: before.grind_policy,
        })
    }
}
