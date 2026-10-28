//! Unregistered child of processed Contact: actual late inputs for the existing
//! collision/brake leaves, not the differently ordered `GroundAnimation` tail.
use super::SkeletonProcessedContact;
use crate::{
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    skateboard_brake_drag::SkateboardBrakeDragInputRead,
    skateboard_slide_collision::SkateboardSlideCollisionInputRead,
    skateboard_slide_collision_attributes::{
        SkateboardSelectedSurface, SkateboardSlideCollisionAttributeError, SlideSurfaceField,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum GroundForceInputError {
    Missing(&'static str),
    Poisoned,
    Surface(SkateboardSlideCollisionAttributeError),
}
impl std::fmt::Display for GroundForceInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "ground force missing {field}"),
            Self::Poisoned => f.write_str("ground force processed lock poisoned"),
            Self::Surface(error) => write!(f, "ground force selected surface: {error}"),
        }
    }
}

/// The pool caller must verify the same board/processed/Reckoning identities.
/// No completed-frame or toolkit-wide readiness gate replaces individual reads.
pub(crate) struct GroundForceInput<'a> {
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) reckoning: Option<&'a Reckoning<ReckoningVaultAttributes>>,
}
impl GroundForceInput<'_> {
    fn read<T>(
        &self,
        field: &'static str,
        read: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, GroundForceInputError> {
        let input = self
            .processed
            .lock()
            .map_err(|_| GroundForceInputError::Poisoned)?;
        read(&input).ok_or(GroundForceInputError::Missing(field))
    }
    fn surface(&self) -> Result<SkateboardSelectedSurface, GroundForceInputError> {
        self.read("selected surface collection", |p| {
            p.surface_projection.selected.clone()
        })
    }
    /// `ToolKit_CalcStraightenOutTorque` 286a8 loads Processed+160
    /// `mBodyZAxis`; braking alone reads the flattened +170 direction.
    pub(crate) fn body_z_axis(&self) -> Result<SkeletonRootVector, GroundForceInputError> {
        self.read("body Z axis160", |p| p.toolkit.body_z())
    }
}
impl SkateboardBrakeDragInputRead for GroundForceInput<'_> {
    type Error = GroundForceInputError;
    type Surface = SkateboardSelectedSurface;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        self.read("braking flags8c4/60000000", |p| {
            let (bits, known) = p.known_flags_8c4();
            (known & 0x6000_0000 == 0x6000_0000).then_some(bits)
        })
    }
    fn foot_brake(&mut self) -> Result<f32, Self::Error> {
        self.read("foot brake9a8", |p| p.anim_attributes.foot_brake)
    }
    fn capture_surface(&mut self) -> Result<Self::Surface, Self::Error> {
        self.surface()
    }
    fn z_speed(&mut self) -> Result<f32, Self::Error> {
        self.read("Z speed944", |p| p.prior_output.z_speed().map(|v| v.value))
    }
    fn surface_braking_scalar(&mut self, surface: &Self::Surface) -> Result<f32, Self::Error> {
        surface
            .try_float(SlideSurfaceField::BrakingScalar)
            .map_err(Self::Error::Surface)
    }
    fn abs_body_speed(&mut self) -> Result<f32, Self::Error> {
        self.read("absolute speed948", |p| p.toolkit.abs_body_speed())
    }
    fn balance(&mut self) -> Result<f32, Self::Error> {
        self.read("balance9a0", |p| p.physics_state.balance)
    }
    fn push(&mut self) -> Result<f32, Self::Error> {
        self.read("push9a4", |p| p.physics_state.push)
    }
    fn body_z_direction(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("body Z direction170", |p| p.toolkit.body_z_direction())
    }
    fn reckoning_ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.reckoning
            .map(Reckoning::ground_normal)
            .ok_or(Self::Error::Missing("actual Reckoning"))
    }
}
impl SkateboardSlideCollisionInputRead for GroundForceInput<'_> {
    type Surface = SkateboardSelectedSurface;
    type Error = GroundForceInputError;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("ground normal1d0", |p| {
            p.prior_output.ground_normal().map(|v| v.value)
        })
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("velocity190", |p| {
            p.prior_output.velocity().map(|v| v.value)
        })
    }
    fn deck_right(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("deck X40", |p| p.deck.map(|v| v.columns()[0]))
    }
    fn surface_speed(&mut self) -> Result<f32, Self::Error> {
        self.read("surface speed", |p| {
            p.prior_output.surface_speed().map(|v| v.value)
        })
    }
    fn surface_collection(&mut self) -> Result<Self::Surface, Self::Error> {
        self.surface()
    }
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error> {
        self.read("wheel hardness9c8", |p| p.animation_input.wheel_hardness)
    }
    fn is_skeleton_in_collision(&mut self) -> Result<bool, Self::Error> {
        self.read("collision8c8/40000", |p| {
            p.flags_8c8.map(|v| v & 0x40000 != 0)
        })
    }
    fn reckoning_ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.reckoning
            .map(Reckoning::ground_normal)
            .ok_or(Self::Error::Missing("actual Reckoning"))
    }
    fn collision_hook_to_hips(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("hook to hips2e0", |p| p.process_data_output.hook_to_hips)
    }
    fn surface_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("surface velocity1a0", |p| {
            p.prior_output.surface_velocity().map(|v| v.value)
        })
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.read("DT93c", |p| p.physics_state.time_delta)
    }
    fn skateboard_mass(&mut self) -> Result<f32, Self::Error> {
        self.read("mass970", |p| p.physical_mass)
    }
    fn z_axis_velocity_dir(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("Z axis velocity direction", |p| {
            p.toolkit.z_axis_velocity_dir()
        })
    }
    fn up_vector(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("up220", |p| p.prior_output.reckoning_up().map(|v| v.value))
    }
}

#[path = "skeleton_processed_ground_foot_input.rs"]
pub(crate) mod foot_input;

impl crate::skateboard_ground_force_tail::SkateboardGroundForceInput for GroundForceInput<'_> {
    fn state_timer(&mut self) -> Result<f32, GroundForceInputError> {
        self.processed
            .lock()
            .map_err(|_| GroundForceInputError::Poisoned)?
            .physics_state_fields()
            .state_timer
            .ok_or(GroundForceInputError::Missing("state timer"))
    }
}
