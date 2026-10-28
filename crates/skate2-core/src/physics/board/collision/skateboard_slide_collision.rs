//! Staged grounded slide 0x82b28260 and collision response 0x82b27530.
//! Typed full lanes, retained owners and existing numerical graphs; no queue,
//! body writes, contact synthesis or frame scheduling is performed here.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "grounded board integration pending")
)]
use crate::{
    reckoning_numeric::{
        ReckoningNumeric, length_from_squared_and_estimate, try_acos, try_normalize_angle,
        try_signed_angle_with_wind,
    },
    simulation_drive_jacobian::refine_drive_reciprocal,
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_slide_collision_attributes::{
        COLLISION_LIMIT, COLLISION_LOCAL_Y, COLLISION_NORMAL_SPEED, COLLISION_RESPONSE,
        SkateboardSlideCollisionAttributeRead, SlideCollisionCurve, SlideCollisionGlobal,
        SlideSurfaceField, WHEEL_HARDNESS_BLEND,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{fmt, marker::PhantomData};
const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);

pub(crate) trait SkateboardSlideCollisionInputRead {
    type Surface;
    type Error;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn deck_right(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn surface_speed(&mut self) -> Result<f32, Self::Error>;
    fn surface_collection(&mut self) -> Result<Self::Surface, Self::Error>;
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error>;
    fn is_skeleton_in_collision(&mut self) -> Result<bool, Self::Error>;
    fn reckoning_ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn collision_hook_to_hips(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn surface_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn skateboard_mass(&mut self) -> Result<f32, Self::Error>;
    fn z_axis_velocity_dir(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn up_vector(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SkateboardSlideCollisionError<I, A, N> {
    Input(I),
    Attribute(A),
    Numeric(N),
    NegativeWheelBlend { bits: u32 },
}
impl<I: fmt::Debug, A: fmt::Debug, N: fmt::Debug> fmt::Display
    for SkateboardSlideCollisionError<I, A, N>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(error) => write!(f, "slide/collision input: {error:?}"),
            Self::Attribute(error) => write!(f, "slide/collision attribute: {error:?}"),
            Self::Numeric(error) => write!(f, "slide/collision numeric: {error:?}"),
            Self::NegativeWheelBlend { bits } => {
                write!(f, "negative wheel blend assertion: {bits:08x}")
            }
        }
    }
}
type Error<P, A, N> = SkateboardSlideCollisionError<
    <P as SkateboardSlideCollisionInputRead>::Error,
    <A as SkateboardSlideCollisionAttributeRead>::Error,
    <N as ReckoningNumeric>::Error,
>;
pub(crate) struct SkateboardSlideCollisionServices<'a, P, A, N> {
    pub(crate) input: &'a mut P,
    pub(crate) attributes: &'a mut A,
    pub(crate) numeric: &'a mut N,
}
impl<P, A, N> SkateboardSlideCollisionServices<'_, P, A, N>
where
    P: SkateboardSlideCollisionInputRead,
    A: SkateboardSlideCollisionAttributeRead<Surface = P::Surface>,
    N: ReckoningNumeric,
{
    /// Source first dot precedes the second normal/right reads. Local output
    /// stores occur only after the final actual surface scalar load.
    pub(crate) fn try_slide(
        &mut self,
        time: f32,
        output: &mut SkateboardFootForce,
    ) -> Result<(), Error<P, A, N>> {
        use SkateboardSlideCollisionError as E;
        let normal = self.input.ground_normal().map_err(E::Input)?;
        let velocity = self.input.velocity().map_err(E::Input)?;
        let normal_speed = self
            .numeric
            .dot3(scratch(velocity), scratch(normal))
            .map_err(E::Numeric)?;
        let normal_y = self.input.ground_normal().map_err(E::Input)?.xyz[1];
        let right = self.input.deck_right().map_err(E::Input)?;
        let normal_velocity = scale(normal, normal_speed);
        // Explicit ordinary-host VMX exceptional selection boundary, matching the
        // shared angle kernel's policy, not an assertion of Xenon NaN payload parity.
        let lower = if normal_y < -1.0 { -1.0 } else { normal_y };
        let bounded = if lower > 1.0 { 1.0 } else { lower };
        let tangent = subtract(velocity, normal_velocity);
        let side_speed = self
            .numeric
            .dot3(scratch(tangent), scratch(right))
            .map_err(E::Numeric)?;
        let side = scale(right, side_speed);
        let angle = try_acos(bounded, self.numeric).map_err(E::Numeric)?;
        let slope = angle * f32::from_bits(0x3f22_f983);
        let negative_slope = -slope;
        let friction = self
            .attributes
            .capture_global(SlideCollisionGlobal::Friction)
            .map_err(E::Attribute)?;
        let lower = select(negative_slope, 0.0, slope);
        let normalized = select(1.0 - lower, lower, 1.0);
        let normal_curve = self
            .attributes
            .graph(&friction, SlideCollisionCurve::SlideVsNormal, normalized)
            .map_err(E::Attribute)?;
        let friction = self
            .attributes
            .capture_global(SlideCollisionGlobal::Friction)
            .map_err(E::Attribute)?;
        let normal_factor = 1.0 - normal_curve;
        let speed = self.input.surface_speed().map_err(E::Input)?;
        let speed_curve = self
            .attributes
            .graph(
                &friction,
                SlideCollisionCurve::SlideVsSpeed,
                speed * f32::from_bits(0x3dcc_cccd),
            )
            .map_err(E::Attribute)?;
        let negative_time = -time;
        let speed_factor = 1.0 - speed_curve;
        let heading = self
            .attributes
            .capture_global(SlideCollisionGlobal::Heading)
            .map_err(E::Attribute)?;
        let surface = self.input.surface_collection().map_err(E::Input)?;
        let lower_time = select(negative_time, 0.0, time);
        let difference = normal_factor - speed_factor;
        let force_time = self
            .attributes
            .surface_float(&surface, SlideSurfaceField::StraightenOutForceTime)
            .map_err(E::Attribute)?;
        let remaining = force_time - lower_time;
        let friction_factor = select(difference, normal_factor, speed_factor);
        let bounded_time = select(remaining, lower_time, force_time);
        let time_ratio = bounded_time / force_time;
        let heading_factor = self
            .attributes
            .graph(
                &heading,
                SlideCollisionCurve::SideFrictionFactor,
                time_ratio,
            )
            .map_err(E::Attribute)?;
        let wheels = self
            .attributes
            .capture_global(SlideCollisionGlobal::Wheels)
            .map_err(E::Attribute)?;
        let hardness = self.input.wheel_hardness().map_err(E::Input)?;
        let pointer = self
            .attributes
            .lookup(&wheels, WHEEL_HARDNESS_BLEND)
            .map_err(E::Attribute)?;
        let wheel = self.attributes.float(&pointer).map_err(E::Attribute)?;
        if wheel < 0.0 {
            return Err(E::NegativeWheelBlend {
                bits: wheel.to_bits(),
            });
        }
        let remaining = 1.0 - wheel;
        let surface = self.input.surface_collection().map_err(E::Input)?;
        let blend = remaining.mul_add(hardness, wheel);
        let scalar = self
            .attributes
            .surface_float(&surface, SlideSurfaceField::SideFrictionScalar)
            .map_err(E::Attribute)?;
        let magnitude = blend * scalar;
        output.local_space_position = ZERO;
        let magnitude = magnitude * heading_factor;
        output.local_space_position = ZERO;
        let magnitude = -(magnitude * friction_factor);
        output.world_space_force = scale(side, magnitude);
        Ok(())
    }
    /// Distinct early false (untouched outputs) and late false (all output stores
    /// and angle/torque calls still occur). Optional new velocity commits first.
    #[expect(
        clippy::too_many_lines,
        reason = "single source sequence preserves staged owner reads and output failure prefixes"
    )]
    pub(crate) fn try_collision(
        &mut self,
        output: &mut SkateboardFootForce,
        torque: &mut SkeletonRootVector,
        new_velocity: Option<&mut SkeletonRootVector>,
    ) -> Result<bool, Error<P, A, N>> {
        use SkateboardSlideCollisionError as E;
        if !self.input.is_skeleton_in_collision().map_err(E::Input)? {
            return Ok(false);
        }
        let global = self
            .attributes
            .capture_global(SlideCollisionGlobal::Collision)
            .map_err(E::Attribute)?;
        let limit_pointer = self
            .attributes
            .lookup(&global, COLLISION_LIMIT)
            .map_err(E::Attribute)?;
        let global = self
            .attributes
            .capture_global(SlideCollisionGlobal::Collision)
            .map_err(E::Attribute)?;
        let limit = self
            .attributes
            .float(&limit_pointer)
            .map_err(E::Attribute)?;
        let local_pointer = self
            .attributes
            .lookup(&global, COLLISION_LOCAL_Y)
            .map_err(E::Attribute)?;
        let local_y = self
            .attributes
            .float(&local_pointer)
            .map_err(E::Attribute)?;
        let ground = self.input.reckoning_ground_normal().map_err(E::Input)?;
        let hook = self.input.collision_hook_to_hips().map_err(E::Input)?;
        let normal_component = self
            .numeric
            .dot3(scratch(hook), scratch(ground))
            .map_err(E::Numeric)?;
        let velocity = self.input.surface_velocity().map_err(E::Input)?;
        let projected = subtract(hook, scale(ground, normal_component));
        let squared = self
            .numeric
            .dot3(scratch(projected), scratch(projected))
            .map_err(E::Numeric)?;
        let seed = self
            .numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let length = length_from_squared_and_estimate(squared, seed);
        if length < f32::from_bits(0x3780_0000) {
            return Ok(false);
        }
        let global = self
            .attributes
            .capture_global(SlideCollisionGlobal::Collision)
            .map_err(E::Attribute)?;
        let normal = scale(projected, 1.0 / length);
        let projection = self
            .numeric
            .dot3(scratch(velocity), scratch(normal))
            .map_err(E::Numeric)?;
        let response_pointer = self
            .attributes
            .lookup(&global, COLLISION_RESPONSE)
            .map_err(E::Attribute)?;
        let global = self
            .attributes
            .capture_global(SlideCollisionGlobal::Collision)
            .map_err(E::Attribute)?;
        let response = self
            .attributes
            .float(&response_pointer)
            .map_err(E::Attribute)?;
        let speed_pointer = self
            .attributes
            .lookup(&global, COLLISION_NORMAL_SPEED)
            .map_err(E::Attribute)?;
        let target_speed = self
            .attributes
            .float(&speed_pointer)
            .map_err(E::Attribute)?;
        let tangent = subtract(velocity, scale(normal, projection));
        let new = mul_add(normal, target_speed, tangent);
        if let Some(out) = new_velocity {
            *out = new;
        }
        let difference = subtract(new, velocity);
        let mut impulse = scale(difference, response);
        if projection > target_speed {
            impulse = ZERO;
        }
        let squared = self
            .numeric
            .dot3(scratch(impulse), scratch(impulse))
            .map_err(E::Numeric)?;
        let time_delta = self.input.time_delta().map_err(E::Input)?;
        let mass = self.input.skateboard_mass().map_err(E::Input)?;
        let length_seed = self
            .numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let mass_impulse = scale(impulse, mass);
        // Source interleaves pure rsqrt/reciprocal refinement instructions. Numeric
        // boundary calls retain their order: rsqrt then reciprocal, both before the
        // magnitude branch. Existing exact arithmetic DAGs are reused once each;
        // native FP-status observation between pure instructions is not modeled.
        let reciprocal_seed = self
            .numeric
            .reciprocal_estimate(time_delta)
            .map_err(E::Numeric)?;
        let length = length_from_squared_and_estimate(squared, length_seed);
        let reciprocal = refine_drive_reciprocal(time_delta, reciprocal_seed);
        let force = scale_left(reciprocal, mass_impulse);
        let (force, active) = if length < f32::from_bits(0x3c23_d70a) {
            (ZERO, false)
        } else {
            let factor = if length > limit { limit / length } else { 1.0 };
            (scale(force, factor), true)
        };
        output.world_space_force = force;
        output.local_space_position = SkeletonRootVector::known([0.0, local_y, 0.0, 0.0]);
        *torque = ZERO;
        let axis = self.input.z_axis_velocity_dir().map_err(E::Input)?;
        let input = &mut *self.input;
        let mut numeric = NumericBridge::<N, P::Error, A::Error> {
            numeric: self.numeric,
            marker: PhantomData,
        };
        let angle = try_signed_angle_with_wind(
            scratch(axis),
            scratch(normal),
            || input.up_vector().map(scratch).map_err(E::Input),
            &mut numeric,
        )?;
        let angle = try_normalize_angle(angle, self.numeric).map_err(E::Numeric)?;
        let global = self
            .attributes
            .capture_global(SlideCollisionGlobal::Collision)
            .map_err(E::Attribute)?;
        let fraction = angle.abs() * f32::from_bits(0x3ea2_f983);
        let magnitude = self
            .attributes
            .graph(
                &global,
                SlideCollisionCurve::CollisionTorqueVsAngle,
                fraction,
            )
            .map_err(E::Attribute)?;
        let negative = -magnitude;
        let up = self.input.up_vector().map_err(E::Input)?;
        let magnitude = select(angle, magnitude, negative);
        *torque = scale(up, magnitude);
        Ok(active)
    }
}
struct NumericBridge<'a, N, I, A> {
    numeric: &'a mut N,
    marker: PhantomData<fn() -> (I, A)>,
}
impl<N: ReckoningNumeric, I, A> ReckoningNumeric for NumericBridge<'_, N, I, A> {
    type Error = SkateboardSlideCollisionError<I, A, N::Error>;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.numeric
            .dot3(a, b)
            .map_err(SkateboardSlideCollisionError::Numeric)
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.numeric
            .reciprocal_sqrt_estimate(x)
            .map_err(SkateboardSlideCollisionError::Numeric)
    }
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.numeric
            .reciprocal_estimate(x)
            .map_err(SkateboardSlideCollisionError::Numeric)
    }
    fn truncate_i32(&mut self, x: f32) -> Result<i32, Self::Error> {
        self.numeric
            .truncate_i32(x)
            .map_err(SkateboardSlideCollisionError::Numeric)
    }
}
fn select(test: f32, nonnegative: f32, other: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { other }
}
// Scratch W is unobserved by dot3/angle XYZ reductions; it never becomes an
// output or retained owner's W. Full-lane result arithmetic propagates knowledge.
fn scratch(v: SkeletonRootVector) -> [f32; 4] {
    [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.0)]
}
// 0x82b27954 places refined reciprocal on the left of every product.
fn scale_left(s: f32, v: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: v.xyz.map(|x| s * x),
        w: v.w.map(|x| s * x),
    }
}
fn scale(v: SkeletonRootVector, s: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: v.xyz.map(|x| x * s),
        w: v.w.map(|x| x * s),
    }
}
fn subtract(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| a.xyz[i] - b.xyz[i]),
        w: a.w.zip(b.w).map(|(a, b)| a - b),
    }
}
fn mul_add(a: SkeletonRootVector, s: f32, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| a.xyz[i].mul_add(s, b.xyz[i])),
        w: a.w.zip(b.w).map(|(a, b)| a.mul_add(s, b)),
    }
}
#[cfg(test)]
#[path = "skateboard_slide_collision_tests.rs"]
mod tests;
