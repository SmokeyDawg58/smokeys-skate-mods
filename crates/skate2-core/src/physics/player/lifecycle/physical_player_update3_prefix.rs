//! Update3 82dfe708..e7bc: prediction, push-veto reset, environmental grab sync.
//! Success stops before the actual state.Update call at82dfe7d0.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_entry::{SkateboardEntrySessionError, try_current_board_deck_velocity},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_grab_spline::sync::GrabSyncError,
    physical_player_physics_state::CurrentPhysicalState,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum Update3PrefixError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pair(SimulationSkeletonJumpPairError),
    Board(SkateboardEntrySessionError),
    Pool(crate::dynamic_assembly_session::skater_pool::SimulationSkaterPoolError),
    Grab(GrabSyncError),
}
impl std::fmt::Display for Update3PrefixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "Update3 prefix missing {v}"),
            Self::Poisoned => f.write_str("Update3 owner lock poisoned"),
            Self::ForeignOwner => f.write_str("Update3 state or grab belongs to another owner"),
            Self::Pair(e) => write!(f, "Update3 pair: {e:?}"),
            Self::Board(e) => e.fmt(f),
            Self::Pool(e) => e.fmt(f),
            Self::Grab(e) => e.fmt(f),
        }
    }
}
/// Base82d83b5c uses one fused multiply-add per lane. Unknown W remains unknown,
/// even when DT is zero; no scratch lane is published into the retained prediction.
fn predict(
    mut columns: [SkeletonRootVector; 4],
    dt: f32,
    velocity: SkeletonRootVector,
) -> SkeletonRootMatrix {
    let old = columns[3];
    columns[3] = SkeletonRootVector {
        xyz: std::array::from_fn(|lane| velocity.xyz[lane].mul_add(dt, old.xyz[lane])),
        w: velocity.w.zip(old.w).map(|(v, p)| v.mul_add(dt, p)),
    };
    SkeletonRootMatrix::from_columns(columns)
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_prediction_before_state(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &crate::static_ground_pool::StaticGroundPool,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        num: &mut crate::physical_player_grind_manager::math::GrindNum<'_>,
        collision: &mut crate::collision_line_numeric::HostCollisionLineNumeric,
    ) -> Result<(), Update3PrefixError> {
        use Update3PrefixError as E;
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let count = state.state_count.ok_or(E::Missing("state count"))?;
            state.state_count = Some(count.wrapping_add(1)); //e730 before prediction
            let binding = match state.current {
                CurrentPhysicalState::Ground => state.ground.as_ref().map(|s| s.bindings.clone()),
                CurrentPhysicalState::Air => state.air.as_ref().map(|s| s.bindings.clone()),
                CurrentPhysicalState::Sleeping => state.sleeping.clone(),
                CurrentPhysicalState::WipeoutGround => {
                    state.wipeout_ground.as_ref().map(|s| s.bindings.clone())
                }
                CurrentPhysicalState::Teleporting => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::Teleporting)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::Teleporting(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::RevertGround => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::RevertGround)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::RevertGround(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::SlideGround => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::SlideGround)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::SlideGround(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::GroundAnimation => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::GroundAnimation)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::GroundAnimation(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::OffBoard => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::OffBoard)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::OffBoard(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::OffBoardAir => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::OffBoardAir)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::OffBoardAir(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::OffBoardJump => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::OffBoardJump)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::OffBoardJump(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::OffBoardPushing => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::OffBoardPushing)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::OffBoardPushing(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::OffBoardLocomotion => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::OffBoardLocomotion)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::OffBoardLocomotion(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::KnownAir => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::KnownAir)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::KnownAir(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::LandingOnDeck => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::LandingOnDeck)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::LandingOnDeck(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::Boneless => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::Boneless)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::Boneless(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::HandPlant => {
                    use crate::physical_player_assembly::remaining_states::StateKind;
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    states.published_bindings(StateKind::HandPlant).cloned()
                }
                CurrentPhysicalState::Skitching => {
                    use crate::physical_player_assembly::remaining_states::StateKind;
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    states.published_bindings(StateKind::Skitching).cloned()
                }
                CurrentPhysicalState::FollowPath => {
                    use crate::physical_player_assembly::remaining_states::StateKind;
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    states.published_bindings(StateKind::FollowPath).cloned()
                }
                CurrentPhysicalState::FootPlant => {
                    use crate::physical_player_assembly::remaining_states::{
                        StateKind, StatePayload,
                    };
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    match states
                        .states
                        .get(&StateKind::FootPlant)
                        .filter(|s| s.published)
                        .and_then(|s| s.value.as_ref())
                    {
                        Some(StatePayload::FootPlant(s)) => Some(s.base.physical.clone()),
                        _ => None,
                    }
                }
                CurrentPhysicalState::Grind(grind) => {
                    use crate::physical_player_assembly::remaining_states::StateKind;
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    states.published_bindings(StateKind::grind(grind)).cloned()
                }
                CurrentPhysicalState::Nonspecific => {
                    use crate::physical_player_assembly::remaining_states::StateKind;
                    let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                    states.published_bindings(StateKind::Nonspecific).cloned()
                }
                CurrentPhysicalState::Unselected => None,
            }
            .ok_or(E::Missing("selected physical state"))?;
            if !binding.board.same_as(board)
                || !binding.skeleton.same_as(skeleton)
                || !binding
                    .owner
                    .upgrade()
                    .is_some_and(|o| Arc::ptr_eq(&o, &owner))
            {
                return Err(E::ForeignOwner);
            }
        }
        // Hidden-return ABI output r3 / actual state r4. Source captures board
        // transform, then processed DT, then fresh Part6 velocity in that order.
        let session = pool.session(board).map_err(E::Pool)?;
        let transform = session
            .try_live_part_transform_typed(&self.backend, 6)
            .map_err(|e| E::Board(SkateboardEntrySessionError::Transform(e)))?;
        let dt = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .processed()
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("processed DT"))?;
        let velocity = try_current_board_deck_velocity(session, &self.backend).map_err(E::Board)?;
        let prediction = predict(*transform.columns(), dt, velocity);
        pool.try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
            normal
                .root_transforms_mut()
                .publish_board_prediction(prediction.columns()[3]);
        })
        .map_err(E::Pair)?;
        pool.try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
            normal.try_reset_push_abort()
        })
        .map_err(E::Pair)?
        .map_err(|_| E::Missing("constructed Skeleton IK"))?;
        pool.try_clear_board_abort_push(&self.backend, board)
            .map_err(E::Board)?;
        let mut grab = owner.grab_spline().lock().map_err(|_| E::Poisoned)?;
        if let Some(grab) = grab.as_mut() {
            if grab.world != owner.manager() {
                return Err(E::ForeignOwner);
            }
            if !grab.returned {
                return Err(E::Missing("completed grab constructor"));
            }
            grab.try_sync_before_state_update(self, ground, cvars, num, collision)
                .map_err(E::Grab)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{SkeletonRootMatrix, SkeletonRootVector, predict};
    #[test]
    fn prediction_preserves_basis_and_full_known_or_unknown_w() {
        let matrix =
            SkeletonRootMatrix::translation(SkeletonRootVector::known([3.0, 5.0, 7.0, 11.0]));
        let velocity = SkeletonRootVector::known([2.0, 4.0, 6.0, 8.0]);
        let predicted = predict(*matrix.columns(), 0.5, velocity);
        assert_eq!(
            predicted.columns()[3],
            SkeletonRootVector::known([4.0, 7.0, 10.0, 15.0])
        );
        assert_eq!(predicted.columns()[..3], matrix.columns()[..3]);
        let unknown = SkeletonRootVector {
            xyz: velocity.xyz,
            w: None,
        };
        assert!(
            predict(*matrix.columns(), 0.0, unknown).columns()[3]
                .w
                .is_none()
        );
    }
}
