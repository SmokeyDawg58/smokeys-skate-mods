//! `PhysState_OffBoard::CalculateBadGroundCorrection` 0x82d870d8.
//! The returned vector is the native stack result; the response direction and
//! its validity are retained on the OffBoard state.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::remaining_states::{StateKind, StatePayload},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, try_normalize_safe,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

pub(crate) trait BadGroundInputs {
    type Error;
    fn correction(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn hook_to_hips(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn trajectory_translation(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn min_traj_w(&mut self) -> Result<f32, Self::Error>;
    fn max_skeleton_disp(&mut self) -> Result<f32, Self::Error>;
    fn debug_coll_response_a(&mut self) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum BadGroundCorrectionError<E> {
    Input(E),
    Missing(&'static str),
    Numeric(ReckoningHostNumericError),
}

pub(crate) fn calculate_bad_ground_correction<S: BadGroundInputs>(
    direction: &mut Option<SkeletonRootVector>,
    valid: &mut Option<bool>,
    source: &mut S,
) -> Result<SkeletonRootVector, BadGroundCorrectionError<S::Error>> {
    use BadGroundCorrectionError as E;
    let correction = source.correction().map_err(E::Input)?;
    let hook = source.hook_to_hips().map_err(E::Input)?;
    let traj = source.trajectory_translation().map_err(E::Input)?;
    let mut numeric = HostReckoningNumeric;
    let traj_sq = numeric.dot_vector(traj, traj).map_err(E::Numeric)?;
    let min_traj = source.min_traj_w().map_err(E::Input)?;
    if traj_sq > min_traj * min_traj {
        *valid = Some(false);
    }
    let hook_sq = numeric.dot_vector(hook, hook).map_err(E::Numeric)?;
    let max_disp = source.max_skeleton_disp().map_err(E::Input)?;
    if hook_sq > max_disp * max_disp {
        let normalized =
            try_normalize_safe(hook, SkeletonRootVector::known([0.0; 4]), &mut numeric)
                .map_err(E::Numeric)?;
        *direction = Some(normalized);
        *valid = Some(true);
    }
    let is_valid = valid.ok_or(E::Missing("collision response validity"))?;
    if !is_valid {
        return Ok(correction);
    }
    let direction = direction.ok_or(E::Missing("collision response direction"))?;
    let dot = numeric
        .dot_vector(correction, direction)
        .map_err(E::Numeric)?;
    if !(dot < 0.0) {
        return Ok(correction);
    }
    if source.debug_coll_response_a().map_err(E::Input)? {
        Ok(correction.zip(direction.map(|lane| lane * dot), |a, b| a - b))
    } else {
        Ok(SkeletonRootVector::known([0.0; 4]))
    }
}

#[derive(Debug)]
pub(crate) enum LiveBadGroundCorrectionError {
    Poisoned,
    Missing(&'static str),
    ForeignOwner,
    Pool(SimulationSkaterPoolError),
    Cvar(InputCvarError),
    Correction(BadGroundCorrectionError<BadGroundSourceError>),
}

#[derive(Debug)]
pub(crate) enum BadGroundSourceError {
    Poisoned,
    Missing(&'static str),
    Pool(SimulationSkaterPoolError),
    Cvar(InputCvarError),
}

struct LiveSource<'a> {
    world: &'a PhysicalAssemblyWorld,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    cvars: &'a InputCvarReferences,
}

impl BadGroundInputs for LiveSource<'_> {
    type Error = BadGroundSourceError;
    fn correction(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self
            .owner
            .remaining()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .feet_ik
            .as_ref()
            .ok_or(Self::Error::Missing("OffBoardFeetIK"))?
            .bad_ground_correction)
    }
    fn hook_to_hips(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self
            .world
            .skeleton(self.skeleton)
            .map_err(Self::Error::Pool)?
            .post_physics_records()
            .hook_to_hips())
    }
    fn trajectory_translation(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let construction = self
            .world
            .skeleton_construction(self.skeleton)
            .map_err(Self::Error::Pool)?
            .ok_or(Self::Error::Missing("Skeleton construction"))?;
        let outer = construction
            .constructor
            .outer
            .as_ref()
            .ok_or(Self::Error::Missing("Skeleton outer"))?;
        let t = &outer.trajectory_transform;
        Ok(SkeletonRootVector::known([t[12], t[13], t[14], t[15]]))
    }
    fn min_traj_w(&mut self) -> Result<f32, Self::Error> {
        self.cvars.try_ob_min_traj_w().map_err(Self::Error::Cvar)
    }
    fn max_skeleton_disp(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_ob_max_skeleton_disp()
            .map_err(Self::Error::Cvar)
    }
    fn debug_coll_response_a(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_ob_debug_coll_response_a()
            .map_err(Self::Error::Cvar)
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_calculate_pair_offboard_bad_ground_correction(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<SkeletonRootVector, LiveBadGroundCorrectionError> {
        use LiveBadGroundCorrectionError as E;
        let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
        let Some(StatePayload::OffBoard(offboard)) = states
            .states
            .get_mut(&StateKind::OffBoard)
            .filter(|state| state.published)
            .and_then(|state| state.value.as_mut())
        else {
            return Err(E::Missing("retained OffBoard state"));
        };
        let binding = &offboard.base.physical;
        if !binding.board.same_as(board)
            || !binding.skeleton.same_as(skeleton)
            || !binding
                .owner
                .upgrade()
                .is_some_and(|value| Arc::ptr_eq(&value, owner))
        {
            return Err(E::ForeignOwner);
        }
        let mut source = LiveSource {
            world: self,
            owner,
            skeleton,
            cvars,
        };
        calculate_bad_ground_correction(
            &mut offboard.last_collision_response_direction,
            &mut offboard.last_collision_response_direction_valid,
            &mut source,
        )
        .map_err(E::Correction)
    }
}

#[cfg(test)]
#[path = "physical_player_offboard_bad_ground_correction_tests.rs"]
mod tests;
