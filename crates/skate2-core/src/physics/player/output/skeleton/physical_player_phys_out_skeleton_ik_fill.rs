//! Unregistered construct child: IK Fill writes into the existing output allocation.
use super::{
    super::{PhysOutAccessError, PhysOutComponent, PhysOutWriteGuard, PhysicalPlayerPhysOut},
    ConcretePhysOutError,
};
use crate::{
    skeleton_ik_state::fill_phys_out::{IkFillFlag, IkFillVector, SkeletonIkFillOutput},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;
#[derive(Debug)]
pub(crate) enum SkeletonIkOutputError {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
}
impl std::fmt::Display for SkeletonIkOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "output access: {e:?}"),
            Self::Component(e) => write!(f, "output component: {e:?}"),
        }
    }
}
impl SkeletonIkFillOutput for PhysicalPlayerPhysOut {
    type Error = SkeletonIkOutputError;
    type Guard = PhysOutWriteGuard;
    fn sample_guard(&mut self) -> Result<Self::Guard, Self::Error> {
        self.sample_write_guard().map_err(Self::Error::Access)
    }
    fn vector(
        &mut self,
        guard: Self::Guard,
        field: IkFillVector,
        value: SkeletonRootVector,
    ) -> Result<(), Self::Error> {
        self.check_write_guard(&guard)
            .map_err(Self::Error::Access)?;
        self.require_complete_component(PhysOutComponent::Skeleton)
            .map_err(Self::Error::Component)?;
        let output = self.concrete.skeleton.as_mut().expect("guarded Skeleton");
        match field {
            IkFillVector::LeftDeckVelocity => output.skeleton_foot_l_vel_deck_space = value,
            IkFillVector::RightDeckVelocity => output.skeleton_foot_r_vel_deck_space = value,
            IkFillVector::LeftWorldVelocity => output.skeleton_foot_left_vel = value,
            IkFillVector::RightWorldVelocity => output.skeleton_foot_right_vel = value,
        }
        Ok(())
    }
    fn flag(
        &mut self,
        guard: Self::Guard,
        field: IkFillFlag,
        value: impl FnOnce() -> bool,
    ) -> Result<(), Self::Error> {
        self.check_write_guard(&guard)
            .map_err(Self::Error::Access)?;
        self.require_complete_component(PhysOutComponent::Skeleton)
            .map_err(Self::Error::Component)?;
        let value = value();
        let output = self.concrete.skeleton.as_mut().expect("guarded Skeleton");
        match field {
            IkFillFlag::GrabbingLeft => output.is_grabbing_left = value,
            IkFillFlag::GrabbingRight => output.is_grabbing_right = value,
            IkFillFlag::LeftOnDeck => output.is_left_foot_on_deck = value,
            IkFillFlag::RightOnDeck => output.is_right_foot_on_deck = value,
            IkFillFlag::DontPush => output.skateboard_dont_push = value,
            IkFillFlag::DontPushOrBrake => output.tell_anim_dont_push_or_brake = value,
        }
        Ok(())
    }
}
