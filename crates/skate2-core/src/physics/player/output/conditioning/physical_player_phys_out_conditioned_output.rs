//! Individually guarded stores made by Physics conditioner Update82e2f970.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut, SkeletonRootVector};
use super::prior_input::PriorInputReadError;
use crate::{
    physics::{FilteredPhysicsState, GrindType},
    play_animation::FastString30,
};

pub(crate) enum ConditionedStore {
    DampedSystemVelocity(SkeletonRootVector),
    EmpiricalVelocity(SkeletonRootVector),
    EmpiricalAcceleration(SkeletonRootVector),
    FramesSinceReset(i32),
    FilteredState(FilteredPhysicsState),
    LastFilteredState(FilteredPhysicsState),
    LastGrindDistance(f32),
    GrindType(GrindType),
    IsGrinding(bool),
    GrindOnFront(bool),
    GrindName(FastString30),
    GrindScoringName(FastString30),
    GrindScore(i32),
    GrindCrouch(f32),
    GrindPathedGuid(u64),
    GrindLocalGuid(u64),
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_store_conditioned(
        &mut self,
        store: ConditionedStore,
    ) -> Result<(), PriorInputReadError> {
        use ConditionedStore as S;
        let component = match &store {
            S::DampedSystemVelocity(_) => PhysOutComponent::SystemReckoning,
            S::EmpiricalVelocity(_) | S::EmpiricalAcceleration(_) => PhysOutComponent::Motion,
            S::FramesSinceReset(_) => PhysOutComponent::State,
            _ => PhysOutComponent::Filtered,
        };
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)?;
        match store {
            S::DampedSystemVelocity(v) => {
                self.concrete
                    .system
                    .as_mut()
                    .expect("checked System")
                    .damped_centre_of_mass_velocity = v;
            }
            S::EmpiricalVelocity(v) => {
                self.concrete
                    .motion
                    .as_mut()
                    .expect("checked Motion")
                    .emperical_deck_velocity = v;
            }
            S::EmpiricalAcceleration(v) => {
                self.concrete
                    .motion
                    .as_mut()
                    .expect("checked Motion")
                    .emperical_deck_acceleration = v;
            }
            S::FramesSinceReset(v) => {
                self.concrete
                    .state
                    .as_mut()
                    .expect("checked State")
                    .frames_since_reset = v;
            }
            other => {
                let output = self.concrete.filtered.as_mut().expect("checked Filtered");
                match other {
                    S::FilteredState(v) => output.filtered_physics_state = v,
                    S::LastFilteredState(v) => output.last_filtered_physics_state = v,
                    S::LastGrindDistance(v) => output.last_grind_dist = v,
                    S::GrindType(v) => output.grind_type = v,
                    S::IsGrinding(v) => output.is_grinding = v,
                    S::GrindOnFront(v) => output.is_grind_on_front = v,
                    S::GrindName(v) => output.skating_grind_name = v,
                    S::GrindScoringName(v) => output.skating_grind_name_scoring = v,
                    S::GrindScore(v) => output.grind_scorable_id = v,
                    S::GrindCrouch(v) => output.grind_crouch = v,
                    S::GrindPathedGuid(v) => output.grind_pathed_guid = v,
                    S::GrindLocalGuid(v) => output.grind_local_guid = v,
                    S::DampedSystemVelocity(_)
                    | S::EmpiricalVelocity(_)
                    | S::EmpiricalAcceleration(_)
                    | S::FramesSinceReset(_) => unreachable!("non-Filtered stores handled above"),
                }
            }
        }
        Ok(())
    }
}
