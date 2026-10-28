//! Front/rear foot force 0x82b28b70/0x82b28d98 and helper 0x82b28828.
//! This produces the source force record; the actual queue/deck torque consumer
//! and pre/post physics scheduling are separate required integration phases.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "foot kernels await the actual grounded board owner"
    )
)]

use crate::{
    reckoning_numeric::ReckoningNumeric,
    reckoning_numeric::try_normalize_safe_with_refinement,
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    skateboard_foot_attributes::{
        FOOT_OFFSET, INITIAL_COEFFICIENT, LANDING_DENOMINATOR, REAR_ABSORPTION_SCALE,
        SPEED_COEFFICIENT, SkateboardFootAttributeRead,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::fmt;

const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);

/// Both native vectors occupy sixteen bytes. Unknown source W remains unknown;
/// neither zero coefficients nor XYZ-only dot products justify fabricating it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkateboardFootForce {
    pub(crate) world_space_force: SkeletonRootVector,
    pub(crate) local_space_position: SkeletonRootVector,
}

/// Current processed fields and actual retained Reckoning up. Getters execute
/// at source load sites; the owner decides how current provenance is checked.
pub(crate) trait SkateboardFootInputRead {
    type Error;
    fn balance(&mut self) -> Result<f32, Self::Error>;
    fn surface_speed(&mut self) -> Result<f32, Self::Error>;
    fn body_y_direction(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn reckoning_up(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}

/// Native target pointer is captured before the projection dot, its bool read
/// follows that dot, and a true value requires overlay after the multiply.
pub(crate) trait SkateboardFootDebugRead {
    type Target;
    type Error;
    fn capture_draw_absorption_target(&mut self) -> Result<Self::Target, Self::Error>;
    fn draw_absorption(&mut self, target: &Self::Target) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub(crate) enum SkateboardFootForceError<P, A, D, N> {
    Input(P),
    Attribute(A),
    Debug(D),
    Numeric(N),
}
impl<P: fmt::Debug, A: fmt::Debug, D: fmt::Debug, N: fmt::Debug> fmt::Display
    for SkateboardFootForceError<P, A, D, N>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "foot force: {self:?}")
    }
}

pub(crate) struct SkateboardFootForceServices<'a, P, A, D, N> {
    pub(crate) input: &'a mut P,
    pub(crate) attributes: &'a mut A,
    pub(crate) debug: &'a mut D,
    pub(crate) numeric: &'a mut N,
}

type Error<P, A, D, N> = SkateboardFootForceError<
    <P as SkateboardFootInputRead>::Error,
    <A as SkateboardFootAttributeRead>::Error,
    <D as SkateboardFootDebugRead>::Error,
    <N as ReckoningNumeric>::Error,
>;

impl<
    P: SkateboardFootInputRead,
    A: SkateboardFootAttributeRead,
    D: SkateboardFootDebugRead,
    N: ReckoningNumeric,
> SkateboardFootForceServices<'_, P, A, D, N>
{
    /// 0x82b28b70. BO4/BI25 branches when GT is not set, including unordered;
    /// subsequent fsel yields front scalar TWO for NaN balance.
    pub(crate) fn try_front(
        &mut self,
        time_since_landing: f32,
        absorption: f32,
        output: &mut SkateboardFootForce,
    ) -> Result<(), Error<P, A, D, N>> {
        self.try_front_with_refinement(
            time_since_landing,
            absorption,
            output,
            refine_drive_reciprocal_sqrt,
        )
    }
    pub(crate) fn try_front_with_refinement(
        &mut self,
        time_since_landing: f32,
        absorption: f32,
        output: &mut SkateboardFootForce,
        refine: impl FnOnce(f32, f32) -> f32,
    ) -> Result<(), Error<P, A, D, N>> {
        let balance = self
            .input
            .balance()
            .map_err(SkateboardFootForceError::Input)?;
        let general = if balance > 0.0 {
            0.0
        } else if balance >= 0.0 {
            1.0
        } else {
            2.0
        };
        let pointer = self
            .attributes
            .lookup(FOOT_OFFSET)
            .map_err(SkateboardFootForceError::Attribute)?;
        let displacement = self
            .attributes
            .float(&pointer)
            .map_err(SkateboardFootForceError::Attribute)?;
        self.helper(
            time_since_landing,
            displacement,
            absorption,
            general,
            output,
            refine,
        )
    }

    /// 0x82b28d98. The absorption product precedes offset negation. This
    /// wrapper's final fsel selects zero for negative or unordered balance.
    pub(crate) fn try_rear(
        &mut self,
        time_since_landing: f32,
        absorption: f32,
        output: &mut SkateboardFootForce,
    ) -> Result<(), Error<P, A, D, N>> {
        self.try_rear_with_refinement(
            time_since_landing,
            absorption,
            output,
            refine_drive_reciprocal_sqrt,
        )
    }
    pub(crate) fn try_rear_with_refinement(
        &mut self,
        time_since_landing: f32,
        absorption: f32,
        output: &mut SkateboardFootForce,
        refine: impl FnOnce(f32, f32) -> f32,
    ) -> Result<(), Error<P, A, D, N>> {
        let pointer = self
            .attributes
            .lookup(REAR_ABSORPTION_SCALE)
            .map_err(SkateboardFootForceError::Attribute)?;
        let absorption_scale = self
            .attributes
            .float(&pointer)
            .map_err(SkateboardFootForceError::Attribute)?;
        let balance = self
            .input
            .balance()
            .map_err(SkateboardFootForceError::Input)?;
        let candidate = if balance > 0.0 { 2.0 } else { 1.0 };
        let general = if balance >= 0.0 { candidate } else { 0.0 };
        let pointer = self
            .attributes
            .lookup(FOOT_OFFSET)
            .map_err(SkateboardFootForceError::Attribute)?;
        let displacement = self
            .attributes
            .float(&pointer)
            .map_err(SkateboardFootForceError::Attribute)?;
        let absorption_effect = absorption_scale * absorption;
        let displacement = -displacement;
        self.helper(
            time_since_landing,
            displacement,
            absorption_effect,
            general,
            output,
            refine,
        )
    }

    /// 0x82b28828: output is not touched before the native stores at 82b28abc.
    fn helper(
        &mut self,
        time: f32,
        displacement: f32,
        absorption: f32,
        general: f32,
        output: &mut SkateboardFootForce,
        refine: impl FnOnce(f32, f32) -> f32,
    ) -> Result<(), Error<P, A, D, N>> {
        let pointer = self
            .attributes
            .lookup(LANDING_DENOMINATOR)
            .map_err(SkateboardFootForceError::Attribute)?;
        let denominator = self
            .attributes
            .float(&pointer)
            .map_err(SkateboardFootForceError::Attribute)?;
        let ratio = time / denominator;
        let balance = self
            .input
            .balance()
            .map_err(SkateboardFootForceError::Input)?;
        let active = if balance == 0.0 { 0.0 } else { 1.0 };
        let lower = if -ratio >= 0.0 { 0.0 } else { ratio };
        let upper_test = 1.0 - lower;
        let blend_time = if upper_test >= 0.0 { lower } else { 1.0 };
        let landing = if time == 0.0 { active } else { 1.0 };
        let speed = self
            .input
            .surface_speed()
            .map_err(SkateboardFootForceError::Input)?
            * 0.125;
        let speed_pointer = self
            .attributes
            .lookup(SPEED_COEFFICIENT)
            .map_err(SkateboardFootForceError::Attribute)?;
        let initial_pointer = self
            .attributes
            .lookup(INITIAL_COEFFICIENT)
            .map_err(SkateboardFootForceError::Attribute)?;
        let remaining = 1.0 - blend_time;
        let speed_coefficient = self
            .attributes
            .float(&speed_pointer)
            .map_err(SkateboardFootForceError::Attribute)?;
        let speed_force = speed_coefficient * speed;
        let initial_coefficient = self
            .attributes
            .float(&initial_pointer)
            .map_err(SkateboardFootForceError::Attribute)?;
        let initial = remaining * initial_coefficient;
        let steady = speed_force * blend_time;
        let body_y = self
            .input
            .body_y_direction()
            .map_err(SkateboardFootForceError::Input)?;
        let negative_body_y = negate(body_y);
        let blended = initial.mul_add(landing, steady);
        let magnitude = -(blended * general);
        // Native prefix: force zero, local zero, then local [0,0,d,0].
        output.world_space_force = ZERO;
        output.local_space_position = ZERO;
        output.local_space_position = SkeletonRootVector::known([0.0, 0.0, displacement, 0.0]);
        let absorption_force = scale(negative_body_y, absorption);
        let up = self
            .input
            .reckoning_up()
            .map_err(SkateboardFootForceError::Input)?;
        let raw = multiply_add(up, magnitude, absorption_force);
        let velocity = self
            .input
            .velocity()
            .map_err(SkateboardFootForceError::Input)?;
        let direction = try_normalize_safe_with_refinement(velocity, ZERO, self.numeric, refine)
            .map_err(SkateboardFootForceError::Numeric)?;
        let target = self
            .debug
            .capture_draw_absorption_target()
            .map_err(SkateboardFootForceError::Debug)?;
        // dot3 ignores W. Scratch zeros are never published as source W.
        let dot = self
            .numeric
            .dot3(xyz_scratch(direction), xyz_scratch(raw))
            .map_err(SkateboardFootForceError::Numeric)?;
        let draw = self
            .debug
            .draw_absorption(&target)
            .map_err(SkateboardFootForceError::Debug)?;
        let parallel = scale(direction, dot);
        if draw {
            self.debug
                .require_overlay()
                .map_err(SkateboardFootForceError::Debug)?;
        }
        output.world_space_force = subtract(raw, parallel);
        Ok(())
    }
}

fn xyz_scratch(value: SkeletonRootVector) -> [f32; 4] {
    [value.xyz[0], value.xyz[1], value.xyz[2], 0.0]
}
fn negate(value: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: value
            .xyz
            .map(|lane| f32::from_bits(lane.to_bits() ^ 0x8000_0000)),
        w: value
            .w
            .map(|lane| f32::from_bits(lane.to_bits() ^ 0x8000_0000)),
    }
}
fn scale(value: SkeletonRootVector, scalar: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: value.xyz.map(|lane| lane * scalar),
        w: value.w.map(|lane| lane * scalar),
    }
}
fn multiply_add(
    value: SkeletonRootVector,
    scalar: f32,
    add: SkeletonRootVector,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| value.xyz[lane].mul_add(scalar, add.xyz[lane])),
        w: value
            .w
            .zip(add.w)
            .map(|(value, add)| value.mul_add(scalar, add)),
    }
}
fn subtract(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] - right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left - right),
    }
}

#[cfg(test)]
#[path = "skateboard_foot_forces_tests.rs"]
mod tests;
