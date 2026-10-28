//! Two separate Update5 publication spans; intervening output work is not implied.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GRIND, PHYSICS_CATEGORY_GROUND,
    PHYSICS_CATEGORY_GROUNDTRICK, PHYSICS_CATEGORY_INVALID, PHYSICS_CATEGORY_OFFBOARD,
    PHYSICS_CATEGORY_OTHER, PHYSICS_CATEGORY_WIPEOUT, PHYSICS_STATE_INVALID, PHYSICS_STATE_OFFBOARD,
    PHYSICS_STATE_OFFBOARD_LOCOMOTION, PHYSICS_STATE_UPPER_LIMIT,
};
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut};
use super::prior_input::PriorInputReadError;
pub(crate) trait Update5StateSource {
    type Error;
    fn current_type(&mut self) -> Result<i32, Self::Error>;
    fn update_count(&mut self) -> Result<i32, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum Update5StateOutputError<E> {
    Source(E),
    Output(PriorInputReadError),
    InvalidType(i32),
}
impl<E: std::fmt::Debug> std::fmt::Display for Update5StateOutputError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "Update5 state source: {e:?}"),
            Self::Output(e) => write!(f, "Update5 state output: {e:?}"),
            Self::InvalidType(value) => write!(f, "Update5 invalid state type: {value}"),
        }
    }
}
/// `PhysicsState.GetPhysicsCategory82d83b80` asserts signed -1 < input < 703.
fn category<E>(state: i32) -> Result<i32, Update5StateOutputError<E>> {
    if !(PHYSICS_STATE_INVALID..PHYSICS_STATE_UPPER_LIMIT).contains(&state) {
        return Err(Update5StateOutputError::InvalidType(state));
    }
    Ok(if state >= PHYSICS_CATEGORY_OTHER {
        PHYSICS_CATEGORY_OTHER
    } else if state >= PHYSICS_CATEGORY_GROUNDTRICK {
        PHYSICS_CATEGORY_GROUNDTRICK
    } else if state >= PHYSICS_CATEGORY_OFFBOARD {
        PHYSICS_CATEGORY_OFFBOARD
    } else if state >= PHYSICS_CATEGORY_GRIND {
        PHYSICS_CATEGORY_GRIND
    } else if state >= PHYSICS_CATEGORY_WIPEOUT {
        PHYSICS_CATEGORY_WIPEOUT
    } else if state >= PHYSICS_CATEGORY_AIR {
        PHYSICS_CATEGORY_AIR
    } else if state >= PHYSICS_CATEGORY_GROUND {
        PHYSICS_CATEGORY_GROUND
    } else {
        PHYSICS_CATEGORY_INVALID
    })
}
impl PhysicalPlayerPhysOut {
    fn update5_state_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    ///53f0..5434: virtual type read precedes the output lock assertion.
    pub(crate) fn try_publish_update5_current_type<S: Update5StateSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), Update5StateOutputError<S::Error>> {
        let state = source
            .current_type()
            .map_err(Update5StateOutputError::Source)?;
        self.update5_state_guard(PhysOutComponent::State)
            .map_err(Update5StateOutputError::Output)?;
        self.concrete.state.as_mut().unwrap().physics_state = state;
        Ok(())
    }
    ///55e4..5798: fresh type/category, lazy wheels, then six ordered stores.
    pub(crate) fn try_publish_update5_category_and_dt<S: Update5StateSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), Update5StateOutputError<S::Error>> {
        use Update5StateOutputError as E;
        let state = source.current_type().map_err(E::Source)?;
        let category = category(state)?;
        let airborne = if (PHYSICS_CATEGORY_AIR..PHYSICS_CATEGORY_WIPEOUT).contains(&state) {
            self.update5_state_guard(PhysOutComponent::Collision)
                .map_err(E::Output)?;
            self.concrete.collision.as_ref().unwrap().wheels_in_contact == 0
        } else {
            false
        };
        self.update5_state_guard(PhysOutComponent::Air)
            .map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().is_skateboard_airborne = airborne;
        self.update5_state_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete.state.as_mut().unwrap().physics_state_category = category;
        self.update5_state_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete.state.as_mut().unwrap().update_count =
            source.update_count().map_err(E::Source)?;
        self.update5_state_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete.state.as_mut().unwrap().is_offboard = category == PHYSICS_CATEGORY_OFFBOARD;
        self.update5_state_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete.state.as_mut().unwrap().is_offboard_ground = matches!(state, PHYSICS_STATE_OFFBOARD | PHYSICS_STATE_OFFBOARD_LOCOMOTION);
        self.update5_state_guard(PhysOutComponent::Physics)
            .map_err(E::Output)?;
        self.concrete.physics.as_mut().unwrap().timestep =
            source.time_delta().map_err(E::Source)?;
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_update5_state_tests.rs"]
mod tests;
