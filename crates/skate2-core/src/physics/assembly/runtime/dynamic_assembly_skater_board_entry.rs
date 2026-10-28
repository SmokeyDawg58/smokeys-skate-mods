//! Actual board-owned hook retention and `DisableBoardAnimation` consumer.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
};
use crate::{
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    simulation_drive_pool::SimulationDriveDynamics,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_force_runtime::entry::DisableBoardAnimationError,
};
use std::sync::{Arc, RwLock};
#[path = "dynamic_assembly_skater_air_drive.rs"]
pub(crate) mod air_drive;
#[path = "dynamic_assembly_skater_board_animation.rs"]
pub(crate) mod animation;
#[path = "dynamic_assembly_skater_wipeout_board.rs"]
pub(crate) mod wipeout;

#[derive(Debug)]
pub(crate) enum SkateboardEntrySessionError {
    Pool(SimulationSkaterPoolError),
    MissingConstruction,
    ForeignOwner,
    HookAlreadyRetained,
    MissingHook,
    SpeedWobble(crate::skateboard_force_runtime::speed_wobble::SkateboardSpeedWobbleError),
    Disable(DisableBoardAnimationError<std::convert::Infallible>),
    ReckoningPoisoned,
    ProcessedPoisoned,
    UnknownBoardFlip,
    UnknownGroundMotion,
    Transform(crate::dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError),
    Part(crate::dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError),
    Body(crate::simulation_rigid_body_pool::SimulationRigidBodyPoolError),
    Attribute(crate::skateboard_attributes::SkateboardAttributeError),
}
impl std::fmt::Display for SkateboardEntrySessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "board entry pool: {error}"),
            Self::MissingConstruction => f.write_str("board entry requires its actual constructor"),
            Self::ForeignOwner => f.write_str("board hook handoff has a different physical owner"),
            Self::HookAlreadyRetained => f.write_str("board hook dynamics already retained"),
            Self::MissingHook => f.write_str("board hook dynamics have not been constructed"),
            Self::SpeedWobble(error) => write!(f, "board speed wobble: {error:?}"),
            Self::Disable(error) => write!(f, "DisableBoardAnimation: {error:?}"),
            Self::ReckoningPoisoned => f.write_str("board entry Reckoning lock poisoned"),
            Self::ProcessedPoisoned => f.write_str("board entry processed lock poisoned"),
            Self::UnknownBoardFlip => f.write_str("current processed board-flip bit is unknown"),
            Self::UnknownGroundMotion => {
                f.write_str("ground entry processed normal or deck angular velocity is unknown")
            }
            Self::Transform(error) => write!(f, "current deck transform: {error}"),
            Self::Part(error) => write!(f, "current deck Part: {error}"),
            Self::Body(error) => write!(f, "current deck body: {error}"),
            Self::Attribute(error) => write!(f, "board entry attribute: {error}"),
        }
    }
}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_clear_board_abort_push(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        self.members[index]
            .content
            .board_mut()
            .map_err(E::Pool)?
            .force_runtime
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .entry
            .clear_abort_push();
        Ok(())
    }

    /// Ground Enter82d99d48..da4 reads the retained manual default at the call
    /// site, then scales SAME board torque/integral. No caller tuning scalar.
    pub(crate) fn try_scale_board_manual_for_ground_entry(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        use crate::skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead};
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let scalar = candidate
            .construction
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .attributes_mut()
            .read_float(SkateboardAttributeClass::Manual, 0x0765_82f2_a4a7_cd27)
            .map_err(E::Attribute)?;
        candidate
            .force_runtime
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .entry
            .scale_manual_force(scalar);
        Ok(())
    }
    pub(crate) fn try_reset_board_manual(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        self.members[index]
            .content
            .board_mut()
            .map_err(E::Pool)?
            .force_runtime
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .entry
            .reset_manual();
        Ok(())
    }
    /// Source 82b2a1d0 on the SAME registered outer board and SAME processed
    /// owner. This reads current bodies without requiring a prior solve frame.
    pub(crate) fn try_reset_board_speed_model<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<
        (),
        crate::skateboard_force_runtime::entry::ResetSpeedModelError<
            SkateboardEntrySessionError,
            N::Error,
        >,
    > {
        use crate::skateboard_force_runtime::entry::ResetSpeedModelError as R;
        use SkateboardEntrySessionError as E;
        self.check_backend(backend)
            .map_err(|e| R::Source(E::Pool(e)))?;
        let index = self
            .member_index(board)
            .map_err(|e| R::Source(E::Pool(e)))?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(|e| R::Source(E::Pool(e)))?;
        let owner = candidate
            .construction
            .as_ref()
            .and_then(|c| c.physical_owner())
            .ok_or(R::Source(E::MissingConstruction))?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(R::Source(E::MissingConstruction))?;
        let mut source = CurrentBoardSpeedSource {
            session: &candidate.session,
            backend,
            owner,
        };
        runtime.entry.try_reset_speed_model(&mut source, numeric)
    }
    /// 0x82b26620 on the existing outer+10c; no body, drive or collision writes.
    pub(crate) fn try_reset_board_speed_wobble(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        candidate
            .force_runtime
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .speed_wobble
            .try_reset()
            .map_err(E::SpeedWobble)
    }
    #[cfg(test)]
    pub(crate) fn with_board_speed_wobble_for_test<T>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        inspect: impl FnOnce(
            &mut crate::skateboard_force_runtime::speed_wobble::SkateboardSpeedWobbleOwner,
        ) -> T,
    ) -> T {
        let index = self.member_index(board).unwrap();
        let candidate = self.members[index].content.board_mut().unwrap();
        inspect(&mut candidate.force_runtime.as_mut().unwrap().speed_wobble)
    }

    /// Factory handoff at the actual Hook constructor, BEFORE prepare/admission.
    pub(crate) fn try_retain_board_hook_dynamics(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<(), SkateboardEntrySessionError> {
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let actual = candidate
            .construction
            .as_ref()
            .and_then(|c| c.physical_owner())
            .ok_or(E::MissingConstruction)?;
        if !Arc::ptr_eq(actual, owner) {
            return Err(E::ForeignOwner);
        }
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(E::MissingConstruction)?;
        if runtime.hook_dynamics.is_some() {
            return Err(E::HookAlreadyRetained);
        }
        runtime.hook_dynamics = Some(Arc::clone(dynamics));
        Ok(())
    }

    pub(crate) fn try_finish_board_entry_constructor(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        candidate
            .force_runtime
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .entry
            .finish_body_constructor();
        Ok(())
    }

    /// Source 0x82b2a100 on the same continuing board. No body numeric, topology
    /// or collision-volume writes: existing bounds/active publications survive.
    /// Drive preparation samples the shared dynamics at build time; already
    /// prepared batches retain their original snapshots, as other drive writes do.
    pub(crate) fn try_disable_board_animation(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(E::MissingConstruction)?;
        let hook = runtime.hook_dynamics.as_ref().ok_or(E::MissingHook)?;
        // The exclusive candidate borrow fixes its Body association across both
        // native captures. The payload lock is reacquired at each store group.
        runtime
            .entry
            .try_disable_board_animation(|| Ok(Arc::clone(hook)))
            .map_err(E::Disable)
    }

    #[cfg(test)]
    pub(crate) fn board_entry_snapshot(
        &self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> (bool, f32, Arc<RwLock<SimulationDriveDynamics>>) {
        let index = self.member_index(board).unwrap();
        let super::MemberContent::Board { candidate, .. } = &self.members[index].content else {
            panic!("board");
        };
        let runtime = candidate.force_runtime.as_ref().unwrap();
        (
            runtime.entry.animated(),
            runtime.entry.target_speed(),
            Arc::clone(runtime.hook_dynamics.as_ref().unwrap()),
        )
    }
}

struct CurrentBoardSpeedSource<'a> {
    session: &'a crate::skateboard_assembly::SkateboardAssemblySession,
    backend: &'a SkateboardSimulationBackend,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
}
impl crate::skateboard_force_runtime::entry::SkateboardSpeedModelSource
    for CurrentBoardSpeedSource<'_>
{
    type Error = SkateboardEntrySessionError;
    fn effective_transform(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, Self::Error> {
        try_current_board_effective_transform(self.session, self.backend, self.owner)
    }
    fn deck_velocity(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        try_current_board_deck_velocity(self.session, self.backend)
    }
}

/// Shared live Part6 position read for `GetTransform` and the first half of
/// `GetEffectiveTransform`, before that getter's separate flip read.
pub(crate) fn try_current_board_unflipped_transform(
    session: &crate::skateboard_assembly::SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, SkateboardEntrySessionError> {
    session
        .try_live_part_transform_typed(backend, 6)
        .map_err(SkateboardEntrySessionError::Transform)
}

/// Actual `GetEffectiveTransform`82b23778, including the late same-owner flip read.
pub(crate) fn try_current_board_effective_transform(
    session: &crate::skateboard_assembly::SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
    owner: &PhysicalPlayerAssemblyOwner,
) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, SkateboardEntrySessionError> {
    use SkateboardEntrySessionError as E;
    // GetPosition(6)2379c precedes the SAME processed late flag read237a8.
    let transform = try_current_board_unflipped_transform(session, backend)?;
    let processed = Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::ReckoningPoisoned)?
            .processed(),
    );
    let flipped = processed
        .lock()
        .map_err(|_| E::ProcessedPoisoned)?
        .board_flipped()
        .ok_or(E::UnknownBoardFlip)?;
    Ok(transform.with_board_flip(flipped))
}

/// Actual `GetDeckVelocity`82b25ff8; native intrusive fourth lane remains unknown.
pub(crate) fn try_current_board_deck_velocity(
    session: &crate::skateboard_assembly::SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, SkateboardEntrySessionError> {
    use SkateboardEntrySessionError as E;
    let body = session
        .try_part_contact_spy_observation(backend, 6)
        .map_err(E::Part)?;
    let xyz = backend
        .collision_body_pool()
        .try_contact_spy_linear_velocity(body)
        .map_err(E::Body)?;
    Ok(crate::skeleton_root_transforms::SkeletonRootVector { xyz, w: None })
}
