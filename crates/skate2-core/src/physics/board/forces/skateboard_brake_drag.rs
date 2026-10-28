//! Braking 0x82b26df0 and linear drag 0x82b27078; scoped current-input leaves.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaits grounded owner integration")
)]

use crate::{
    skateboard_brake_attributes::{
        BALANCE_DRAG, BALANCE_SPEED, FOOT_COEFFICIENT, MANUAL_COEFFICIENT, NORMAL_Y_THRESHOLD,
        STOP_SPEED, SkateboardBrakeAttributeRead,
    },
    skateboard_foot_forces::SkateboardFootForce,
    skeleton_root_transforms::SkeletonRootVector,
};

/// Getters execute at source load sites. A captured surface retains the selected
/// owner across z-speed sampling, before its layout `BrakingScalar` is read.
pub(crate) trait SkateboardBrakeDragInputRead {
    type Error;
    type Surface;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error>;
    fn foot_brake(&mut self) -> Result<f32, Self::Error>;
    fn capture_surface(&mut self) -> Result<Self::Surface, Self::Error>;
    fn z_speed(&mut self) -> Result<f32, Self::Error>;
    fn surface_braking_scalar(&mut self, surface: &Self::Surface) -> Result<f32, Self::Error>;
    fn abs_body_speed(&mut self) -> Result<f32, Self::Error>;
    fn balance(&mut self) -> Result<f32, Self::Error>;
    fn push(&mut self) -> Result<f32, Self::Error>;
    fn body_z_direction(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn reckoning_ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardBrakeDragError<P, A> {
    Input(P),
    Attribute(A),
}

pub(crate) struct SkateboardBrakeDragServices<'a, P, A> {
    pub(crate) input: &'a mut P,
    pub(crate) attributes: &'a mut A,
}

type Error<P, A> = SkateboardBrakeDragError<
    <P as SkateboardBrakeDragInputRead>::Error,
    <A as SkateboardBrakeAttributeRead>::Error,
>;

impl<P: SkateboardBrakeDragInputRead, A: SkateboardBrakeAttributeRead>
    SkateboardBrakeDragServices<'_, P, A>
{
    fn attribute(&mut self, key: u64) -> Result<f32, Error<P, A>> {
        let pointer = self
            .attributes
            .lookup(key)
            .map_err(SkateboardBrakeDragError::Attribute)?;
        self.attributes
            .float(&pointer)
            .map_err(SkateboardBrakeDragError::Attribute)
    }

    /// 0x82b26df0. All lookups/surface reads precede output initialization;
    /// the final direction read follows both zero vector writes.
    pub(crate) fn try_braking(
        &mut self,
        output: &mut SkateboardFootForce,
    ) -> Result<(), Error<P, A>> {
        use SkateboardBrakeDragError::Input;
        let mut coefficient = if self.input.flags_8c4().map_err(Input)? & 0x4000_0000 != 0 {
            let value = self.attribute(FOOT_COEFFICIENT)?;
            self.input.foot_brake().map_err(Input)? * value
        } else {
            0.0
        };
        if self.input.flags_8c4().map_err(Input)? & 0x2000_0000 != 0 {
            coefficient = self.attribute(MANUAL_COEFFICIENT)?;
        }
        let surface = self.input.capture_surface().map_err(Input)?;
        let z_speed = self.input.z_speed().map_err(Input)?;
        let surface_scalar = self.input.surface_braking_scalar(&surface).map_err(Input)?;
        let magnitude = surface_scalar * coefficient;
        let sign = if z_speed < 0.0 { 1.0 } else { -1.0 };
        let signed = sign * magnitude;
        let threshold = self.attribute(STOP_SPEED)?;
        let speed = self.input.abs_body_speed().map_err(Input)?;
        // BO4/BI24: !LT includes unordered; only ordered LT zeros magnitude.
        let scalar = if speed < threshold { 0.0 } else { signed };
        output.world_space_force = SkeletonRootVector::known([0.0; 4]);
        output.local_space_position = SkeletonRootVector::known([0.0; 4]);
        let direction = self.input.body_z_direction().map_err(Input)?;
        output.world_space_force = SkeletonRootVector {
            xyz: direction.xyz.map(|lane| lane * scalar),
            w: direction.w.map(|lane| lane * scalar),
        };
        Ok(())
    }

    /// 0x82b27078. Returns drag without writing the board; the caller publishes
    /// it only after the collision/ordinary force choice.
    pub(crate) fn try_linear_drag(&mut self) -> Result<f32, Error<P, A>> {
        use SkateboardBrakeDragError::Input;
        let threshold = self.attribute(STOP_SPEED)? * 2.0;
        let speed = self.input.abs_body_speed().map_err(Input)?;
        let stopped = if speed < threshold {
            let flags = self.input.flags_8c4().map_err(Input)?;
            if flags & 0x6000_0000 != 0 {
                let push = self.input.push().map_err(Input)?;
                if push > 0.0 {
                    false
                } else {
                    self.input.balance().map_err(Input)? == 0.0
                }
            } else {
                false
            }
        } else {
            false
        };
        // This second balance read is unconditional, even after short circuit.
        let balance = self.input.balance().map_err(Input)?;
        let mut hill = false;
        if balance != 0.0 {
            let threshold = self.attribute(BALANCE_SPEED)?;
            let speed = self.input.abs_body_speed().map_err(Input)?;
            if speed < threshold {
                let threshold = self.attribute(NORMAL_Y_THRESHOLD)?;
                let normal = self.input.reckoning_ground_normal().map_err(Input)?;
                hill = normal.xyz[1] > threshold;
            }
        }
        let drag = if hill {
            self.attribute(BALANCE_DRAG)?
        } else {
            0.0
        };
        Ok(if stopped { 1.0 } else { drag })
    }
}

#[cfg(test)]
#[path = "skateboard_brake_drag_tests.rs"]
mod tests;
