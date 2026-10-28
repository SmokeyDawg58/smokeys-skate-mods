//! Actual independently guarded component reads in Update1's prior-input copy.
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut, grinds, simple};
use crate::physical_player_phys_out::PhysOutAccessError;

#[derive(Debug, PartialEq)]
pub(crate) enum PriorInputReadError {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
}
#[derive(Clone, Copy)]
pub(crate) enum PriorContactFlag {
    Skeleton,
    Deck,
    Truck,
    Anything,
}

/// Native Collision +cc0..cc3 indices used by Update1's short-circuit pairs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PriorWheelContact {
    Wheel0,
    Wheel1,
    Wheel2,
    Wheel3,
}

impl PhysicalPlayerPhysOut {
    #[cfg(test)]
    pub(crate) fn edit_prior_wheel_timer_test_outputs(
        &mut self,
        edit: impl FnOnce(&mut [bool; 4], &mut simple::State, &mut simple::GroundRemainder),
    ) {
        edit(
            &mut self
                .concrete
                .collision
                .as_mut()
                .expect("constructed collision")
                .wheel_in_contact,
            self.concrete.state.as_mut().expect("constructed state"),
            self.concrete.ground.as_mut().expect("constructed ground"),
        );
    }
    /// Each reached Update1 wheel read repeats the Collision component guard.
    pub(crate) fn try_guarded_wheel_in_contact(
        &self,
        wheel: PriorWheelContact,
    ) -> Result<bool, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Collision)?;
        let index = match wheel {
            PriorWheelContact::Wheel0 => 0,
            PriorWheelContact::Wheel1 => 1,
            PriorWheelContact::Wheel2 => 2,
            PriorWheelContact::Wheel3 => 3,
        };
        Ok(self
            .concrete
            .collision
            .as_deref()
            .expect("checked component")
            .wheel_in_contact[index])
    }
    pub(crate) fn try_guarded_skeleton_output(
        &self,
    ) -> Result<&simple::Skeleton, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Skeleton)?;
        Ok(self
            .concrete
            .skeleton
            .as_deref()
            .expect("checked component"))
    }
    pub(crate) fn try_guarded_filtered_output(
        &self,
    ) -> Result<&grinds::Filtered, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Filtered)?;
        Ok(self
            .concrete
            .filtered
            .as_deref()
            .expect("checked component"))
    }
    pub(crate) fn try_guarded_offboard_output(
        &self,
    ) -> Result<&simple::OffBoard, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::OffBoard)?;
        Ok(self
            .concrete
            .offboard
            .as_deref()
            .expect("checked component"))
    }
    #[cfg(test)]
    pub(crate) fn edit_prior_body_test_outputs(
        &mut self,
        edit: impl FnOnce(&mut simple::Skeleton, &mut grinds::Filtered, &mut simple::OffBoard),
    ) {
        edit(
            self.concrete
                .skeleton
                .as_deref_mut()
                .expect("test Skeleton"),
            self.concrete
                .filtered
                .as_deref_mut()
                .expect("test Filtered"),
            self.concrete
                .offboard
                .as_deref_mut()
                .expect("test OffBoard"),
        );
    }
    #[cfg(test)]
    pub(crate) fn edit_prior_jump_test_outputs(
        &mut self,
        edit: impl FnOnce(&mut simple::Air, &mut simple::GroundRemainder),
    ) {
        edit(
            self.concrete.air.as_deref_mut().expect("test Air"),
            self.concrete.ground.as_deref_mut().expect("test Ground"),
        );
    }
    pub(crate) fn try_guarded_ground_output(
        &self,
    ) -> Result<&simple::GroundRemainder, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Ground)?;
        Ok(self.concrete.ground.as_deref().expect("checked component"))
    }
    pub(crate) fn try_guarded_air_output(&self) -> Result<&simple::Air, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Air)?;
        Ok(self.concrete.air.as_deref().expect("checked component"))
    }
    pub(crate) fn try_guarded_physics_output(
        &self,
    ) -> Result<&simple::Physics, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Physics)?;
        Ok(self.concrete.physics.as_deref().expect("checked component"))
    }
    pub(crate) fn try_guarded_collision_contact(
        &self,
        flag: PriorContactFlag,
    ) -> Result<bool, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Collision)?;
        let collision = self
            .concrete
            .collision
            .as_deref()
            .expect("checked component");
        Ok(match flag {
            PriorContactFlag::Skeleton => collision.skeleton_in_contact,
            PriorContactFlag::Deck => collision.deck_in_contact,
            PriorContactFlag::Truck => collision.a_truck_in_contact,
            PriorContactFlag::Anything => collision.anything_in_contact,
        })
    }
    /// Independent Collision getter at Update1 call2060.
    pub(crate) fn try_guarded_wheels_in_contact(&self) -> Result<i32, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Collision)?;
        Ok(self
            .concrete
            .collision
            .as_deref()
            .expect("checked component")
            .wheels_in_contact)
    }
    /// One captured Collision getter2090 serves both following flag reads.
    pub(crate) fn try_guarded_skeleton_collision_flags(
        &self,
    ) -> Result<(bool, bool), PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Collision)?;
        let skeleton = &self
            .concrete
            .collision
            .as_deref()
            .expect("checked component")
            .skeleton;
        Ok((skeleton.is_partial_ragdoll, skeleton.is_impaled_on_fence))
    }
    /// `Update0_ProcessCollection`82e01f34..1f60: the guarded Collision+0x94
    /// skeleton collision audio `mVehicleImpact` read.
    pub(crate) fn try_guarded_vehicle_impact(&self) -> Result<f32, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Collision)?;
        Ok(self
            .concrete
            .collision
            .as_deref()
            .expect("checked component")
            .skeleton
            .audio
            .vehicle_impact)
    }
    fn guard_prior_component(
        &self,
        component: PhysOutComponent,
    ) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_guarded_state_output(&self) -> Result<&simple::State, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::State)?;
        Ok(self.concrete.state.as_deref().expect("checked component"))
    }
    pub(crate) fn try_guarded_grinds_output(&self) -> Result<&grinds::Grinds, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Grinds)?;
        Ok(self.concrete.grinds.as_deref().expect("checked component"))
    }
    pub(crate) fn try_guarded_motion_remainder(
        &self,
    ) -> Result<&simple::MotionRemainder, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::Motion)?;
        Ok(self.concrete.motion.as_deref().expect("checked component"))
    }
    pub(crate) fn try_guarded_skateboard_reckoning(
        &self,
    ) -> Result<&simple::SkateboardReckoning, PriorInputReadError> {
        self.guard_prior_component(PhysOutComponent::SkateboardReckoning)?;
        Ok(self
            .concrete
            .skateboard_reckoning
            .as_deref()
            .expect("checked component"))
    }
}

#[cfg(test)]
mod update1_tests {
    use super::*;
    use crate::physical_player_phys_out::construct::PhysOutAllocation;
    use crate::simulation_assembly_backend::constraint_session::physical_update1_prefix::{
        Update1PrefixError, check_for_teleport,
    };

    #[test]
    fn teleport_check_retains_false_branch_assertion_and_actual_transform() {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        check_for_teleport::<()>(&output).unwrap();
        output.concrete.state.as_mut().unwrap().is_teleport_finished = true;
        assert!(matches!(
            check_for_teleport::<()>(&output),
            Err(Update1PrefixError::TeleportNotPublished)
        ));
        let transform =
            crate::skeleton_root_transforms::SkeletonRootMatrix::known(std::array::from_fn(|i| {
                f32::from(u16::try_from(i).unwrap()) + 0.25
            }));
        let motion = output.concrete.motion.as_mut().unwrap();
        motion.did_teleport = true;
        motion.teleport_transform = transform;
        match check_for_teleport::<()>(&output) {
            Err(Update1PrefixError::TeleportPlayerNowPending(actual)) => {
                assert_eq!(actual, transform);
            }
            result => panic!("unexpected teleport branch {result:?}"),
        }
        output.lock_after_update1().unwrap();
        assert!(matches!(
            check_for_teleport::<()>(&output),
            Err(Update1PrefixError::Read(PriorInputReadError::Access(
                PhysOutAccessError::LockedForWrite
            )))
        ));
    }
}
