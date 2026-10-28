//! Actual Update5 manager callbacks from `GrindManager` (unless explicitly
//! excluded) through `StateChanger`. Successful return stops before `HandPlant`
//! at82e04f60, not a complete frame.
use super::update5_pumping::Update5BeforeGrindManagerFill;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::manager_fills::{
        ManagerFillError, ManagerFillSource, ManagerFlag, ManagerScalar, ManagerVector,
    },
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GrindManagerFillPolicy {
    /// `GrindManager::FillPhysOut` runs from the retained manager.
    RequireNative,
    /// The first test's exclusion. No `GrindManager` output is synthesized.
    /// The live facade runs the native fill; only the retained corpora select this.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "only the retained test corpora select the exclusion"
        )
    )]
    ExcludeForFirstSkatingTest,
}
#[derive(Debug)]
pub(crate) enum ManagerRuntimeError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    StaleReckoning,
}
impl std::fmt::Display for ManagerRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "manager output requires {field}"),
            Self::Poisoned => f.write_str("manager output owner poisoned"),
            Self::ForeignOwner => f.write_str("manager output belongs to another physical owner"),
            Self::StaleReckoning => {
                f.write_str("manager output requires the current completed Reckoning phase")
            }
        }
    }
}
#[derive(Debug)]
pub(crate) enum Update5ManagersError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    Grind(super::grind_states::GrindStateError),
    Fill(ManagerFillError<ManagerRuntimeError>),
}
impl std::fmt::Display for Update5ManagersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fill(e) => e.fmt(f),
            Self::MissingPool => f.write_str("Update5 managers require the skater pool"),
            Self::Pair(e) => write!(f, "Update5 manager pair: {e:?}"),
            Self::ForeignOwner => {
                f.write_str("Update5 manager continuation belongs to another pair")
            }
            Self::Grind(e) => write!(f, "GrindManager::FillPhysOut: {e}"),
        }
    }
}
pub(crate) struct Update5BeforeHandPlant {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) grind_policy: GrindManagerFillPolicy,
}
/// Ground/Air require their completed update. Wipeout, Teleporting and the
/// Pushing and 600-series plant states call the retained Reckoning FillPhysOut without
/// running Reckoning.Update.
pub(crate) enum Update5ReckoningRead {
    Completed(PhysicalPlayerReckoningPhase),
    RetainedWipeout(super::physical_grounded::wipeout::WipeoutUpdateComplete),
    RetainedTeleporting(Arc<PhysicalPlayerAssemblyOwner>),
    RetainedPushing(Arc<PhysicalPlayerAssemblyOwner>),
    RetainedPlant(
        Arc<PhysicalPlayerAssemblyOwner>,
        crate::physical_player_physics_state::CurrentPhysicalState,
    ),
}
struct Source<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    phase: &'a Update5ReckoningRead,
}
impl Source<'_> {
    fn reckoning<T>(
        &self,
        read: impl FnOnce(&Reckoning<ReckoningVaultAttributes>) -> T,
    ) -> Result<T, ManagerRuntimeError> {
        let retained = match self.phase {
            Update5ReckoningRead::Completed(_) => None,
            Update5ReckoningRead::RetainedWipeout(completed) => Some((
                completed.owner(),
                crate::physical_player_physics_state::CurrentPhysicalState::WipeoutGround,
            )),
            Update5ReckoningRead::RetainedTeleporting(owner) => Some((
                owner,
                crate::physical_player_physics_state::CurrentPhysicalState::Teleporting,
            )),
            Update5ReckoningRead::RetainedPlant(owner, state) => Some((owner, *state)),
            Update5ReckoningRead::RetainedPushing(owner) => Some((
                owner,
                crate::physical_player_physics_state::CurrentPhysicalState::OffBoardPushing,
            )),
        };
        if let Some((actual, expected)) = retained {
            if !Arc::ptr_eq(actual, self.owner)
                || self
                    .owner
                    .physics_state()
                    .lock()
                    .map_err(|_| ManagerRuntimeError::Poisoned)?
                    .current
                    != expected
            {
                return Err(ManagerRuntimeError::ForeignOwner);
            }
        }
        let owner = self
            .owner
            .reckoning()
            .lock()
            .map_err(|_| ManagerRuntimeError::Poisoned)?;
        let state = match self.phase {
            Update5ReckoningRead::Completed(phase) => owner
                .try_completed(phase)
                .map_err(|_| ManagerRuntimeError::StaleReckoning)?,
            Update5ReckoningRead::RetainedWipeout(_)
            | Update5ReckoningRead::RetainedTeleporting(_)
            | Update5ReckoningRead::RetainedPushing(_)
            | Update5ReckoningRead::RetainedPlant(..) => owner
                .reckoning()
                .ok_or(ManagerRuntimeError::Missing("constructed Reckoning"))?,
        };
        Ok(read(state))
    }
}
impl ManagerFillSource for Source<'_> {
    type Error = ManagerRuntimeError;
    fn flag(&mut self, field: ManagerFlag) -> Result<bool, Self::Error> {
        use ManagerRuntimeError as E;
        match field {
            ManagerFlag::TargetGrind | ManagerFlag::TargetWallride => {
                let processed = Arc::clone(
                    self.owner
                        .reckoning()
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .processed(),
                );
                let trajectories = self.owner.trajectories().lock().map_err(|_| E::Poisoned)?;
                let onboard = trajectories
                    .onboard
                    .as_ref()
                    .ok_or(E::Missing("actual onboard TrajectorySelector"))?;
                if !onboard.bindings.matches(self.owner, &processed) {
                    return Err(E::ForeignOwner);
                }
                Ok(match field {
                    ManagerFlag::TargetGrind => onboard.targeting_grind,
                    _ => onboard.targeting_wallride,
                })
            }
            ManagerFlag::Flipping => self.reckoning(Reckoning::fill_is_flipping),
            ManagerFlag::FrontFlipping => self.reckoning(Reckoning::fill_is_front_flipping),
            ManagerFlag::RevertExited | ManagerFlag::RequestTeleport => {
                let state = self.owner.state_changer().lock().map_err(|_| E::Poisoned)?;
                if !state
                    .owner
                    .as_ref()
                    .and_then(std::sync::Weak::upgrade)
                    .is_some_and(|actual| Arc::ptr_eq(&actual, self.owner))
                {
                    return Err(E::ForeignOwner);
                }
                let fields = state
                    .fields
                    .as_ref()
                    .ok_or(E::Missing("actual StateChanger fields"))?;
                Ok(match field {
                    ManagerFlag::RevertExited => fields.revert_exited_normally,
                    _ => fields.request_teleport,
                })
            }
        }
    }
    fn vector(&mut self, field: ManagerVector) -> Result<SkeletonRootVector, Self::Error> {
        self.reckoning(|state| match field {
            ManagerVector::Up => state.up(),
            ManagerVector::Heading => state.fill_heading(),
            ManagerVector::AnimationTilt => state.animation_tilt(),
        })
    }
    fn scalar(&mut self, field: ManagerScalar) -> Result<f32, Self::Error> {
        match field {
            ManagerScalar::SpinSpeed => self.reckoning(|state| state.body_spin().0),
            ManagerScalar::SignedLean => {
                // Raw41e0..4200 reads current Processed8c4 bit11 before dynamic lean.
                let processed = Arc::clone(
                    self.owner
                        .reckoning()
                        .lock()
                        .map_err(|_| ManagerRuntimeError::Poisoned)?
                        .processed(),
                );
                let (bits, known) = processed
                    .lock()
                    .map_err(|_| ManagerRuntimeError::Poisoned)?
                    .known_flags_8c4();
                if known & 0x0010_0000 == 0 {
                    return Err(ManagerRuntimeError::Missing(
                        "Processed8c4 tail-lifting bit",
                    ));
                }
                self.reckoning(|state| {
                    if bits & 0x0010_0000 != 0 {
                        state.dynamic_lean()
                    } else {
                        -state.dynamic_lean()
                    }
                })
            }
        }
    }
    fn ground_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.reckoning(|state| *state.ground_transform())
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_managers_with_reckoning(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeGrindManagerFill,
        phase: &Update5ReckoningRead,
        policy: GrindManagerFillPolicy,
    ) -> Result<Update5BeforeHandPlant, Update5ManagersError> {
        use Update5ManagersError as E;
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
        // GrindManager::FillPhysOut 0x82dd3970 at 4f30: GrindJumper::FillPhysOut
        // 0x82dc7140 publishes mJustJumpedFromGrind, then investigation bit 27.
        let grind = if policy == GrindManagerFillPolicy::RequireNative {
            let manager =
                before.owner.grind().lock().map_err(|_| {
                    E::Fill(ManagerFillError::Source(ManagerRuntimeError::Poisoned))
                })?;
            let manager = manager.as_ref().ok_or(E::Fill(ManagerFillError::Source(
                ManagerRuntimeError::Missing("GrindManager"),
            )))?;
            let jumper = manager
                .jumper
                .as_ref()
                .ok_or(E::Fill(ManagerFillError::Source(
                    ManagerRuntimeError::Missing("GrindJumper"),
                )))?;
            Some((jumper.just_jumped, manager.investigation.hang_exists))
        } else {
            None
        };
        {
            let mut output =
                before.owner.phys_out().lock().map_err(|_| {
                    E::Fill(ManagerFillError::Source(ManagerRuntimeError::Poisoned))
                })?;
            if let Some((did_jump, hang_exists)) = grind {
                output
                    .try_fill_grind_manager(did_jump, hang_exists)
                    .map_err(E::Grind)?; //4f30
            }
            let mut source = Source {
                owner: &before.owner,
                phase,
            };
            output
                .try_fill_trajectory_selector(&mut source)
                .map_err(E::Fill)?; //4f3c
            output.try_fill_reckoning(&mut source).map_err(E::Fill)?; //4f48
            output
                .try_fill_state_changer(&mut source)
                .map_err(E::Fill)?; //4f54
        }
        Ok(Update5BeforeHandPlant {
            owner: before.owner,
            grind_policy: policy,
        })
    }
}

#[cfg(test)]
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_managers(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeGrindManagerFill,
        phase: &PhysicalPlayerReckoningPhase,
        policy: GrindManagerFillPolicy,
    ) -> Result<Update5BeforeHandPlant, Update5ManagersError> {
        self.try_continue_update5_managers_with_reckoning(
            board,
            skeleton,
            before,
            &Update5ReckoningRead::Completed(phase.clone()),
            policy,
        )
    }
}
