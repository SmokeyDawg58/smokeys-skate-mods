//! Skateboard-owned force state, separate from its `SkateboardBody` constructor.
//! This is a bounded projection of 0x82b38918, not the whole Skateboard object.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained board force state awaits complete grounded/prephysics scheduling"
    )
)]

use crate::{
    attribute_runtime::AttributeRuntimeVault,
    skateboard_foot_attributes::SkateboardFootVaultAttributes,
    skateboard_force_attributes::SkateboardQueuedForceVaultAttributes,
    skateboard_force_queue::SkateboardForceQueue,
};
use std::sync::Arc;

#[path = "../runtime/skateboard_entry_state.rs"]
pub(crate) mod entry;
#[path = "skateboard_fake_torque.rs"]
pub(crate) mod fake_torque;

#[path = "../control/skateboard_speed_wobble.rs"]
pub(crate) mod speed_wobble;

pub(crate) struct SkateboardForceRuntime {
    /// Outer +60, unknown until the actual `StoreForceDisplacement` producer runs.
    pub(crate) expected_displacement: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) pumping: Arc<std::sync::Mutex<crate::physical_player_pumping::PhysicalPlayerPumpingOwner>>,
    pub(crate) speed_wobble: speed_wobble::SkateboardSpeedWobbleOwner,
    pub(crate) entry: entry::SkateboardEntryState,
    pub(crate) push_attributes: push::attributes::PushAttributes,
    pub(crate) conservation_attributes: crate::skateboard_slide_collision_attributes::speed_model::ConservationAttributes,
    // SAME Body embedded hook dynamics, retained even when AddDrive returns null.
    pub(crate) hook_dynamics: Option<Arc<std::sync::RwLock<crate::simulation_drive_pool::SimulationDriveDynamics>>>,
    pub(crate) up_state: Arc<std::sync::Mutex<crate::skateboard_update_up_vector::SkateboardUpdateUpState>>,
    pub(crate) up_physics: Arc<crate::attribute_runtime::AttributeRuntimePhysics>,
    pub(crate) ground_normal_attributes: Arc<std::sync::Mutex<crate::skateboard_contact_surface::normal_attributes::SkateboardGroundNormalVaultAttributes>>,
    pub(crate) steering: crate::skateboard_steering::SkateboardSteeringState,
    pub(crate) steering_attributes: crate::skateboard_steering::SkateboardSteeringVaultAttributes,
    jump_normal: crate::skeleton_root_transforms::SkeletonRootVector,
    /// Outer+b0: full zero constructor store82b38ae0, later `CalcGroundJump320d0`.
    pub(crate) jump_delta: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) contact_surface:
        Arc<std::sync::Mutex<crate::skateboard_contact_surface::SkateboardContactSurface>>,
    pub(crate) queue: SkateboardForceQueue,
    pub(crate) own_attributes: SkateboardQueuedForceVaultAttributes,
    pub(crate) foot_attributes: SkateboardFootVaultAttributes,
    pub(crate) wallride_trajectory: crate::skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
    pub(crate) brake_attributes: crate::skateboard_brake_attributes::SkateboardBrakeVaultAttributes,
    pub(crate) slide_collision_attributes:
        crate::skateboard_slide_collision_attributes::SkateboardSlideCollisionVaultAttributes,
}

impl SkateboardForceRuntime {
    /// Native queue construction precedes the own physicsdeck binding, which
    /// precedes the body constructor. Global feet are a separate retained read
    /// capability; binding reads no tuning value and does not merge instances.
    /// Keeps the collection capabilities already issued by the earlier `HiLOD`
    /// `SurfacePhysics` constructor when the actual board is constructed.
    pub(crate) fn new(
        vault: &Arc<AttributeRuntimeVault>,
        pumping: &Arc<std::sync::Mutex<crate::physical_player_pumping::PhysicalPlayerPumpingOwner>>,
        slide_collision_attributes: crate::skateboard_slide_collision_attributes::SkateboardSlideCollisionVaultAttributes,
    ) -> Self {
        // Selected outer constructor stores38aa0..38ad0 execute before the
        // existing jump-normal and queue projections. This is not allocation
        // alone, and Body Reset/TeleportReset must not repeat these stores.
        let mut up_state =
            crate::skateboard_update_up_vector::SkateboardUpdateUpState::uninitialized();
        up_state
            .initialize_constructor()
            .expect("new outer constructor");
        // Outer constructor 0x82b38af0, before queue and own-attribute binding.
        let jump_normal =
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
        let steering = crate::skateboard_steering::SkateboardSteeringState::new();
        let entry = entry::SkateboardEntryState::new();
        let speed_wobble = speed_wobble::SkateboardSpeedWobbleOwner::new();
        let queue = SkateboardForceQueue::new();
        let own_attributes = SkateboardQueuedForceVaultAttributes::bind(vault);
        Self {
            expected_displacement: None,
            jump_delta: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            pumping: Arc::clone(pumping),
            speed_wobble,
            entry,
            push_attributes: push::attributes::PushAttributes::bind(vault),
            conservation_attributes: crate::skateboard_slide_collision_attributes::speed_model::ConservationAttributes::bind(vault),
            hook_dynamics: None,
            up_state: Arc::new(std::sync::Mutex::new(up_state)),
            up_physics: Arc::new(vault.physics(crate::skateboard_contact_surface::normal_attributes::SkateboardGroundNormalVaultAttributes::DEFAULT_COLLECTION)),
            ground_normal_attributes: Arc::new(std::sync::Mutex::new(crate::skateboard_contact_surface::normal_attributes::SkateboardGroundNormalVaultAttributes::bind(vault))),
            jump_normal,
            steering,
            steering_attributes: crate::skateboard_steering::SkateboardSteeringVaultAttributes::bind(vault),
            contact_surface: Arc::new(std::sync::Mutex::new(crate::skateboard_contact_surface::SkateboardContactSurface::new())),
            queue,
            own_attributes,
            slide_collision_attributes,
            foot_attributes: SkateboardFootVaultAttributes::bind(vault),
            // Retained selected global+e8 read capability, distinct from HandPlant.
            wallride_trajectory: crate::skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes::bind(vault),
            brake_attributes:
                crate::skateboard_brake_attributes::SkateboardBrakeVaultAttributes::bind(vault),
        }
    }

    pub(crate) const fn jump_normal(&self) -> crate::skeleton_root_transforms::SkeletonRootVector {
        self.jump_normal
    }

    /// `PrepareToolkitInput` 0x82b26d78 commits the board before the debug gate
    /// and later processed-data copy. This is the retained outer-board value.
    pub(crate) fn set_jump_normal(
        &mut self,
        value: crate::skeleton_root_transforms::SkeletonRootVector,
    ) {
        self.jump_normal = value;
    }
}

impl crate::skeleton_processed_contact::toolkit::SkeletonProcessedToolkitJumpBoard
    for SkateboardForceRuntime
{
    type Error = std::convert::Infallible;
    fn jump_normal(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        Ok(SkateboardForceRuntime::jump_normal(self))
    }
    fn set_jump_normal(
        &mut self,
        value: crate::skeleton_root_transforms::SkeletonRootVector,
    ) -> Result<(), Self::Error> {
        SkateboardForceRuntime::set_jump_normal(self, value);
        Ok(())
    }
}

#[path = "../runtime/skateboard_wallride_force.rs"]
pub(crate) mod wallride;

#[path = "skateboard_push_acceleration.rs"]
pub(crate) mod push;

#[path = "skateboard_pumping_forces.rs"]
pub(crate) mod pumping_forces;

#[path = "../control/skateboard_straighten_torque.rs"]
pub(crate) mod straighten;

#[path = "../control/skateboard_antiflip_torque.rs"]
pub(crate) mod antiflip;
