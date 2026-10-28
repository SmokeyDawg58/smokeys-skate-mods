//! Registered child of `ground_force_input`: live feet helper82b28828 reads.
use super::{GroundForceInput, GroundForceInputError};
use crate::{
    reckoning::Reckoning, skateboard_foot_forces::SkateboardFootInputRead,
    skeleton_root_transforms::SkeletonRootVector,
};

impl SkateboardFootInputRead for GroundForceInput<'_> {
    type Error = GroundForceInputError;
    fn balance(&mut self) -> Result<f32, Self::Error> {
        self.read("balance9a0", |p| p.physics_state_fields().balance)
    }
    fn surface_speed(&mut self) -> Result<f32, Self::Error> {
        // Raw2892c reads96c, not deck speed968 or absolute body speed948.
        self.read("surface speed96c", |p| {
            p.prior_output().surface_speed().map(|v| v.value)
        })
    }
    fn body_y_direction(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        // Raw28a6c/84 selects Processed180, the existing normalized body Y.
        self.read("body Y direction180", |p| {
            p.toolkit_state().body_y_direction()
        })
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("velocity190", |p| {
            p.prior_output().velocity().map(|v| v.value)
        })
    }
    fn reckoning_up(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.reckoning
            .map(Reckoning::up)
            .ok_or(Self::Error::Missing("actual Reckoning up1b0"))
    }
}
