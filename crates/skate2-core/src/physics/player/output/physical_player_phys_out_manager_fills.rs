//! Trajectory/Reckoning/StateChanger output callbacks, Update5 4f3c..4f54.
use super::super::{PhysOutAccessError, PhysOutOrigin, PhysOutSystemVector, PhysOutValue};
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
use crate::skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ManagerFlag {
    TargetGrind,
    TargetWallride,
    Flipping,
    FrontFlipping,
    RevertExited,
    RequestTeleport,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ManagerVector {
    Up,
    Heading,
    AnimationTilt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ManagerScalar {
    SignedLean,
    SpinSpeed,
}
pub(crate) trait ManagerFillSource {
    type Error;
    fn flag(&mut self, field: ManagerFlag) -> Result<bool, Self::Error>;
    fn vector(&mut self, field: ManagerVector) -> Result<Vector, Self::Error>;
    fn scalar(&mut self, field: ManagerScalar) -> Result<f32, Self::Error>;
    fn ground_transform(&mut self) -> Result<Matrix, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum ManagerFillError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
}
impl<S: std::fmt::Debug> std::fmt::Display for ManagerFillError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "manager Fill source: {e:?}"),
            Self::Access(e) => write!(f, "manager Fill access: {e:?}"),
            Self::Component(e) => write!(f, "manager Fill component: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_trajectory_selector<S: ManagerFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), ManagerFillError<S::Error>> {
        use ManagerFillError as E;
        for field in [ManagerFlag::TargetGrind, ManagerFlag::TargetWallride] {
            let guard = self.sample_write_guard().map_err(E::Access)?;
            let value = source.flag(field).map_err(E::Source)?;
            self.check_write_guard(&guard).map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::Air)
                .map_err(E::Component)?;
            let air = self.concrete.air.as_mut().unwrap();
            match field {
                ManagerFlag::TargetGrind => air.is_targeting_grind = value,
                ManagerFlag::TargetWallride => air.is_targeting_wall_ride = value,
                _ => unreachable!("selected trajectory field"),
            }
        }
        Ok(())
    }
    pub(crate) fn try_fill_reckoning<S: ManagerFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), ManagerFillError<S::Error>> {
        use ManagerFillError as E;
        for (source_field, destination) in [
            (ManagerVector::Up, PhysOutSystemVector::Up),
            (ManagerVector::Heading, PhysOutSystemVector::Heading),
        ] {
            let guard = self.sample_write_guard().map_err(E::Access)?;
            let value = source.vector(source_field).map_err(E::Source)?;
            self.check_write_guard(&guard).map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::SystemReckoning)
                .map_err(E::Component)?;
            self.publish_system_vector(&guard, destination, value)
                .map_err(E::Access)?;
        }
        let lean = source
            .scalar(ManagerScalar::SignedLean)
            .map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::SystemReckoning)
            .map_err(E::Component)?;
        self.concrete.system.as_mut().unwrap().target_lean_angle = lean;
        let spin = source.scalar(ManagerScalar::SpinSpeed).map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(E::Component)?;
        self.concrete.air.as_mut().unwrap().body_spin_speed = spin;
        //4258 samples guard BEFORE source flipping flag425c; assertion follows.
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let flipping = source.flag(ManagerFlag::Flipping).map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(E::Component)?;
        self.concrete.air.as_mut().unwrap().is_flipping = flipping;
        let front = source.flag(ManagerFlag::FrontFlipping).map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(E::Component)?;
        self.concrete.air.as_mut().unwrap().is_front_flipping = front;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let tilt = source
            .vector(ManagerVector::AnimationTilt)
            .map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Animation)
            .map_err(E::Component)?;
        self.concrete.animation.as_mut().unwrap().anim_tilt = tilt;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let transform = source.ground_transform().map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Ground)
            .map_err(E::Component)?;
        self.ground_transform = Some(PhysOutValue {
            value: transform,
            origin: PhysOutOrigin::Publication,
        });
        Ok(())
    }
    pub(crate) fn try_fill_state_changer<S: ManagerFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), ManagerFillError<S::Error>> {
        use ManagerFillError as E;
        self.check_current_guard().map_err(E::Access)?;
        let revert = source.flag(ManagerFlag::RevertExited).map_err(E::Source)?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(E::Component)?;
        self.concrete.state.as_mut().unwrap().revert_exited_normally = revert;
        if source
            .flag(ManagerFlag::RequestTeleport)
            .map_err(E::Source)?
        {
            self.check_current_guard().map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::State)
                .map_err(E::Component)?;
            self.concrete
                .state
                .as_mut()
                .unwrap()
                .physics_requested_teleport = true;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_manager_fills_tests.rs"]
mod tests;
