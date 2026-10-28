//! `DoPhysicsSurfaceCheck`82dfef60 and `ConsiderSurface`82dfeeb0 output accesses.
use crate::physics_state_ids::{
    PHYSICS_STATE_FOOTPLANT, PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_LOCOMOTION,
    PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_STATE_SKITCHING,
    PHYSICS_STATE_WIPEOUT_GROUND,
};
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::prior_input::PriorInputReadError;

pub(crate) trait Update5SurfaceCheckSource {
    type Error;
    type Processed;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error>;
    fn physics_state(&mut self, owner: &Self::Processed) -> Result<i32, Self::Error>;
    fn hips_valid(&mut self, owner: &Self::Processed) -> Result<bool, Self::Error>;
    fn hips_surface_id(&mut self, owner: &Self::Processed) -> Result<u32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum Update5SurfaceCheckError<E> {
    Source(E),
    Output(PriorInputReadError),
}
impl<E: std::fmt::Debug> std::fmt::Display for Update5SurfaceCheckError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(f, "Update5 surface source: {error:?}"),
            Self::Output(error) => write!(f, "Update5 surface output: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn surface_check_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    fn surface_request_teleport(&mut self) -> Result<(), PriorInputReadError> {
        self.surface_check_guard(PhysOutComponent::State)?;
        if !self
            .concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
        {
            self.surface_check_guard(PhysOutComponent::State)?;
            self.concrete
                .state
                .as_mut()
                .unwrap()
                .physics_requested_teleport = true;
        }
        Ok(())
    }
    /// Source enum6 Unrideable and9 Wipeout. No table lookup or enum clamping.
    fn consider_physics_surface(&mut self, surface: u32) -> Result<(), PriorInputReadError> {
        match surface {
            6 => self.surface_request_teleport()?,
            9 => {
                self.surface_check_guard(PhysOutComponent::State)?;
                if !self
                    .concrete
                    .state
                    .as_ref()
                    .unwrap()
                    .physics_requests_wipeout
                {
                    self.surface_check_guard(PhysOutComponent::State)?;
                    self.concrete
                        .state
                        .as_mut()
                        .unwrap()
                        .physics_requests_wipeout = true;
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub(crate) fn try_do_update5_surface_check<S: Update5SurfaceCheckSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), Update5SurfaceCheckError<S::Error>> {
        use Update5SurfaceCheckError as E;
        // ef70 captures once. The602 branch reuses this pointer atf0f4/f100.
        let processed = source.capture_processed().map_err(E::Source)?;
        let state = source.physics_state(&processed).map_err(E::Source)?;
        match state {
            PHYSICS_STATE_PHYSICS_GROUND..=PHYSICS_STATE_SKITCHING => {
                self.surface_check_guard(PhysOutComponent::Collision)
                    .map_err(E::Output)?;
                let surface = self
                    .collision_type
                    .ok_or(E::Output(PriorInputReadError::Access(
                        PhysOutAccessError::NotConstructed,
                    )))?
                    .value;
                self.consider_physics_surface(surface).map_err(E::Output)?;
            }
            PHYSICS_STATE_WIPEOUT_GROUND => {
                self.surface_check_guard(PhysOutComponent::Collision)
                    .map_err(E::Output)?;
                if self
                    .concrete
                    .collision
                    .as_ref()
                    .unwrap()
                    .skeleton
                    .unrideable_contact
                {
                    self.surface_request_teleport().map_err(E::Output)?;
                }
            }
            PHYSICS_STATE_OFFBOARD | PHYSICS_STATE_OFFBOARD_PUSHING | PHYSICS_STATE_OFFBOARD_LOCOMOTION => {
                self.surface_check_guard(PhysOutComponent::OffBoard)
                    .map_err(E::Output)?;
                if self.concrete.offboard.as_ref().unwrap().left_foot_down {
                    self.surface_check_guard(PhysOutComponent::OffBoard)
                        .map_err(E::Output)?;
                    let surface = (self
                        .concrete
                        .offboard
                        .as_ref()
                        .unwrap()
                        .left_foot_surface_id
                        >> 7)
                        & 31;
                    self.consider_physics_surface(surface).map_err(E::Output)?;
                }
                self.surface_check_guard(PhysOutComponent::OffBoard)
                    .map_err(E::Output)?;
                if self.concrete.offboard.as_ref().unwrap().right_foot_down {
                    self.surface_check_guard(PhysOutComponent::OffBoard)
                        .map_err(E::Output)?;
                    let surface = (self
                        .concrete
                        .offboard
                        .as_ref()
                        .unwrap()
                        .right_foot_surface_id
                        >> 7)
                        & 31;
                    self.consider_physics_surface(surface).map_err(E::Output)?;
                }
            }
            PHYSICS_STATE_FOOTPLANT => {
                self.surface_check_guard(PhysOutComponent::Air)
                    .map_err(E::Output)?;
                if self.concrete.air.as_ref().unwrap().is_foot_planting {
                    self.surface_check_guard(PhysOutComponent::Air)
                        .map_err(E::Output)?;
                    let surface =
                        (self.concrete.air.as_ref().unwrap().foot_plant_surface_id >> 7) & 31;
                    self.consider_physics_surface(surface).map_err(E::Output)?;
                }
            }
            602 if source.hips_valid(&processed).map_err(E::Source)? => {
                let surface = (source.hips_surface_id(&processed).map_err(E::Source)? >> 7) & 31;
                self.consider_physics_surface(surface).map_err(E::Output)?;
            }
            _ => {}
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_update5_surface_check_tests.rs"]
mod tests;
