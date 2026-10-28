//! Actual Update5 animation stores82e05cd0..60cc.
use super::super::{PhysOutComponent, PhysOutSystemVector, PhysicalPlayerPhysOut};
use super::prior_input::PriorInputReadError;
use crate::skeleton_collision_numeric::SkeletonCollisionNumeric;
#[derive(Clone, Copy)]
pub(crate) enum AnimationScalar {
    Turn,
    Spin,
    TimeSinceInput,
}
#[derive(Clone, Copy)]
pub(crate) enum AnimationFlag {
    Carving,
    Mirrored,
    Switch,
    Fakie,
    Regular,
    Jumping,
    StandingJump,
    NoInput,
    Pushing,
    Transitioning,
    OffBoard,
    RunningOut,
}
pub(crate) trait Update5AnimationSource {
    type Error;
    fn capture_carving_input(&mut self) -> Result<(), Self::Error>;
    fn scalar(&mut self, field: AnimationScalar) -> Result<f32, Self::Error>;
    fn flag(&mut self, field: AnimationFlag) -> Result<bool, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum Update5AnimationError<S, N> {
    Source(S),
    Numeric(N),
    Output(PriorInputReadError),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for Update5AnimationError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "Update5 animation source: {e:?}"),
            Self::Numeric(e) => write!(f, "Update5 animation COM dot: {e:?}"),
            Self::Output(e) => write!(f, "Update5 animation output: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn animation_output_guard(
        &self,
        component: PhysOutComponent,
    ) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_publish_update5_animation<
        S: Update5AnimationSource,
        N: SkeletonCollisionNumeric,
    >(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), Update5AnimationError<S::Error, N::Error>> {
        use Update5AnimationError as E;
        source.capture_carving_input().map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        let carving = source.flag(AnimationFlag::Carving).map_err(E::Source)?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::Animation)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.animation.as_mut().unwrap().is_carving = carving;
        self.animation_output_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete.animation.as_mut().unwrap().input_turn =
            source.scalar(AnimationScalar::Turn).map_err(E::Source)?;
        self.animation_output_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete.animation.as_mut().unwrap().input_kickturn =
            source.scalar(AnimationScalar::Spin).map_err(E::Source)?;
        self.publish_update5_com(numeric).map_err(|e| match e {
            Update5AnimationError::Output(e) => E::Output(e),
            Update5AnimationError::Numeric(e) => E::Numeric(e),
            Update5AnimationError::Source(e) => match e {},
        })?;
        for field in [
            AnimationFlag::Mirrored,
            AnimationFlag::Switch,
            AnimationFlag::Fakie,
            AnimationFlag::Regular,
        ] {
            self.animation_output_guard(PhysOutComponent::Animation)
                .map_err(E::Output)?;
            let value = source.flag(field).map_err(E::Source)?;
            let output = self.concrete.animation.as_mut().unwrap();
            match field {
                AnimationFlag::Mirrored => output.is_mirrored = value,
                AnimationFlag::Switch => output.is_riding_switch = value,
                AnimationFlag::Fakie => output.is_riding_fakie = value,
                _ => output.is_regular_stance = value,
            }
        }
        for field in [AnimationFlag::Jumping, AnimationFlag::StandingJump] {
            let value = source.flag(field).map_err(E::Source)?;
            self.animation_output_guard(PhysOutComponent::OffBoard)
                .map_err(E::Output)?;
            let output = self.concrete.offboard.as_mut().unwrap();
            match field {
                AnimationFlag::Jumping => output.is_jumping = value,
                _ => output.is_starting_standing_jump = value,
            }
        }
        let no_input = source.flag(AnimationFlag::NoInput).map_err(E::Source)?;
        self.animation_output_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete.animation.as_mut().unwrap().is_no_input = no_input;
        self.animation_output_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete
            .animation
            .as_mut()
            .unwrap()
            .time_since_last_input = source
            .scalar(AnimationScalar::TimeSinceInput)
            .map_err(E::Source)?;
        for field in [
            AnimationFlag::Pushing,
            AnimationFlag::Transitioning,
            AnimationFlag::OffBoard,
            AnimationFlag::RunningOut,
        ] {
            let value = source.flag(field).map_err(E::Source)?;
            self.animation_output_guard(PhysOutComponent::Animation)
                .map_err(E::Output)?;
            let output = self.concrete.animation.as_mut().unwrap();
            match field {
                AnimationFlag::Pushing => output.is_generally_pushing = value,
                AnimationFlag::Transitioning => output.transitioning_on_off_board = value,
                AnimationFlag::OffBoard => output.skater_off_board = value,
                _ => output.running_out = value,
            }
        }
        Ok(())
    }
    fn publish_update5_com<N: SkeletonCollisionNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), Update5AnimationError<std::convert::Infallible, N::Error>> {
        use Update5AnimationError as E;
        self.animation_output_guard(PhysOutComponent::SkateboardReckoning)
            .map_err(E::Output)?; //5d74
        self.animation_output_guard(PhysOutComponent::SystemReckoning)
            .map_err(E::Output)?; //5d9c
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?; //5dc8
        let position = self
            .concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .transform
            .columns()[3]
            .xyz;
        let com = self.system_vectors[PhysOutSystemVector::CentreOfMass as usize]
            .ok_or_else(|| {
                E::Output(PriorInputReadError::Access(
                    super::super::PhysOutAccessError::NotConstructed,
                ))
            })?
            .value
            .xyz;
        let delta = std::array::from_fn(|i| com[i] - position[i]);
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::SystemReckoning)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        let up = self.system_vectors[PhysOutSystemVector::Up as usize]
            .ok_or_else(|| {
                E::Output(PriorInputReadError::Access(
                    super::super::PhysOutAccessError::NotConstructed,
                ))
            })?
            .value
            .xyz;
        let dot = numeric.dot3(delta, up).map_err(E::Numeric)?; //5e08; W unused
        self.animation_output_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?; //5e0c
        self.concrete
            .animation
            .as_mut()
            .unwrap()
            .centre_of_mass_to_deck = dot;
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_update5_animation_tests.rs"]
mod tests;
