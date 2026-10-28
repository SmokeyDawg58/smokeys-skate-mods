//! Same-owner normal tail of Body `PostPhysics`, 0x82b37114..0x82b37498.
//! Caller completes selection/velocity/surface stages first. This method issues
//! no whole-frame readiness token and never repeats contact traversal.
use super::normal_attributes::GROUND_NORMAL_ANGLE;
use super::{SkateboardContactSurface, normal_attributes::SkateboardGroundNormalAttributeRead};
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_unsigned_angle},
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_cosine},
    simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt},
    skeleton_collision_numeric::length_from_squared_and_estimate,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{cmp::Ordering, fmt};
const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
const UP: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);

pub(crate) trait SkateboardContactNormalServices {
    type Target;
    type Error;
    fn read_free_wheel_enabled(&mut self) -> Result<bool, Self::Error>;
    fn capture_free_wheel_drag(&mut self) -> Result<Self::Target, Self::Error>;
    fn free_wheel_drag(&mut self, target: &Self::Target) -> Result<f32, Self::Error>;
    fn store_num_parts(&mut self, count: u8) -> Result<(), Self::Error>;
    fn store_num_wheels(&mut self, count: u8) -> Result<(), Self::Error>;
    /// Resolve this board's actual Part/body/inertia only at this source stage.
    fn set_wheel_angular_drag(&mut self, wheel: usize, value: f32) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkateboardContactNormalUnknown {
    InContact(usize),
    Validity(usize),
    Normal(usize),
    LastValidNormal,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkateboardContactNormalError<A, S, N, R> {
    Unknown(SkateboardContactNormalUnknown),
    InvalidBestPart(i32),
    Attribute(A),
    Service(S),
    Numeric(N),
    Rounding(R),
}
impl<A: fmt::Debug, S: fmt::Debug, N: fmt::Debug, R: fmt::Debug> fmt::Display
    for SkateboardContactNormalError<A, S, N, R>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "board contact normal: {self:?}")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkateboardContactNormalOutcome {
    pub(crate) num_parts: u8,
    pub(crate) num_wheels: u8,
    pub(crate) updated_last_valid: bool,
}
type TailError<A, S, N, R> = SkateboardContactNormalError<
    <A as SkateboardGroundNormalAttributeRead>::Error,
    <S as SkateboardContactNormalServices>::Error,
    <N as ReckoningNumeric>::Error,
    <R as ReckoningSinCosNumeric>::Error,
>;

impl SkateboardContactSurface {
    pub(crate) fn try_finish_ground_normals<A, S, N, R>(
        &mut self,
        best_part: i32,
        passed_up: SkeletonRootVector,
        attributes: &mut A,
        services: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<SkateboardContactNormalOutcome, TailError<A, S, N, R>>
    where
        A: SkateboardGroundNormalAttributeRead,
        S: SkateboardContactNormalServices,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    {
        use SkateboardContactNormalError as E;
        let mut num_parts = 0;
        for part in 0..7 {
            num_parts += u8::from(
                self.collision_state.in_contact[part]
                    .ok_or(E::Unknown(SkateboardContactNormalUnknown::InContact(part)))?,
            );
        }
        let enabled = services.read_free_wheel_enabled().map_err(E::Service)?;
        services.store_num_parts(num_parts).map_err(E::Service)?;
        let contact_drag = if enabled {
            let target = services.capture_free_wheel_drag().map_err(E::Service)?;
            services.free_wheel_drag(&target).map_err(E::Service)?
        } else {
            0.0
        };
        let global = attributes.capture_global().map_err(E::Attribute)?;
        let pointer = attributes
            .lookup(&global, GROUND_NORMAL_ANGLE)
            .map_err(E::Attribute)?;
        let angle = attributes.float(&pointer).map_err(E::Attribute)?;
        let cosine =
            try_cosine(angle * f32::from_bits(0x3c8e_fa35), rounding).map_err(E::Rounding)?;
        let mut sum = ZERO;
        let mut num_wheels = 0;
        for wheel in 0..4 {
            let in_contact = self.collision_state.in_contact[wheel]
                .ok_or(E::Unknown(SkateboardContactNormalUnknown::InContact(wheel)))?;
            let drag = if in_contact {
                num_wheels += 1;
                contact_drag
            } else {
                f32::from_bits(0x3bc4_9ba6)
            };
            self.try_accumulate_wheel::<A, S, N, R>(
                wheel, best_part, passed_up, cosine, &mut sum, numeric,
            )?;
            services
                .set_wheel_angular_drag(wheel, drag * f32::from_bits(0x426f_ffff))
                .map_err(E::Service)?;
        }
        services.store_num_wheels(num_wheels).map_err(E::Service)?;
        if num_wheels > 0 {
            let squared = numeric
                .dot3(scratch(sum), scratch(sum))
                .map_err(E::Numeric)?;
            if squared.partial_cmp(&f32::from_bits(0x3780_0000)) == Some(Ordering::Greater) {
                let estimate = numeric
                    .reciprocal_sqrt_estimate(squared)
                    .map_err(E::Numeric)?;
                let inverse = refine_drive_reciprocal_sqrt(squared, estimate);
                let normal = SkeletonRootVector {
                    xyz: sum.xyz.map(|lane| lane * inverse),
                    w: sum.w.map(|lane| lane * inverse),
                };
                self.collision_state.average_normal = Some(normal);
                self.collision_state.last_valid_normal = Some(normal);
                return Ok(SkateboardContactNormalOutcome {
                    num_parts,
                    num_wheels,
                    updated_last_valid: true,
                });
            }
        }
        self.try_fallback_normal::<A, S, N, R>(numeric)?;
        Ok(SkateboardContactNormalOutcome {
            num_parts,
            num_wheels,
            updated_last_valid: false,
        })
    }

    fn try_accumulate_wheel<A, S, N, R>(
        &self,
        wheel: usize,
        best_part: i32,
        up: SkeletonRootVector,
        cosine: f32,
        sum: &mut SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), TailError<A, S, N, R>>
    where
        A: SkateboardGroundNormalAttributeRead,
        S: SkateboardContactNormalServices,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    {
        use SkateboardContactNormalError as E;
        let valid = self
            .collision_state
            .surfaces
            .validity(wheel)
            .ok_or(E::Unknown(SkateboardContactNormalUnknown::Validity(wheel)))?;
        if valid == 0 {
            return Ok(());
        }
        let normal = self.collision_state.normal[wheel]
            .ok_or(E::Unknown(SkateboardContactNormalUnknown::Normal(wheel)))?;
        let dot = numeric
            .dot3(scratch(normal), scratch(up))
            .map_err(E::Numeric)?;
        if dot.partial_cmp(&cosine) != Some(Ordering::Greater) {
            return Ok(());
        }
        let best = if best_part == -1 {
            self.collision_state
                .last_valid_normal
                .ok_or(E::Unknown(SkateboardContactNormalUnknown::LastValidNormal))?
        } else {
            let index = usize::try_from(best_part)
                .ok()
                .filter(|index| *index < 7)
                .ok_or(E::InvalidBestPart(best_part))?;
            self.collision_state.normal[index]
                .ok_or(E::Unknown(SkateboardContactNormalUnknown::Normal(index)))?
        };
        let angle =
            try_unsigned_angle(scratch(best), scratch(normal), numeric).map_err(E::Numeric)?;
        if f32::from_bits(0x3f49_0fdb).partial_cmp(&angle) == Some(Ordering::Greater) {
            // The same exclusively borrowed state is reread at native372b0.
            let normal = self.collision_state.normal[wheel]
                .ok_or(E::Unknown(SkateboardContactNormalUnknown::Normal(wheel)))?;
            *sum = add(*sum, normal);
        }
        Ok(())
    }
    fn try_fallback_normal<A, S, N, R>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), TailError<A, S, N, R>>
    where
        A: SkateboardGroundNormalAttributeRead,
        S: SkateboardContactNormalServices,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    {
        use SkateboardContactNormalError as E;
        let mut sum = ZERO;
        for part in [6, 4, 5] {
            let contact = self.collision_state.in_contact[part]
                .ok_or(E::Unknown(SkateboardContactNormalUnknown::InContact(part)))?;
            if contact {
                sum = add(
                    sum,
                    self.collision_state.normal[part]
                        .ok_or(E::Unknown(SkateboardContactNormalUnknown::Normal(part)))?,
                );
            }
        }
        let squared = numeric
            .dot3(scratch(sum), scratch(sum))
            .map_err(E::Numeric)?;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let length = length_from_squared_and_estimate(squared, estimate);
        let normal = if length.partial_cmp(&f32::from_bits(0x3c23_d70a)) == Some(Ordering::Greater)
        {
            let seed = numeric.reciprocal_estimate(length).map_err(E::Numeric)?;
            let inverse = refine_drive_reciprocal(length, seed);
            SkeletonRootVector {
                xyz: sum.xyz.map(|lane| inverse * lane),
                w: sum.w.map(|lane| inverse * lane),
            }
        } else {
            UP
        };
        self.collision_state.average_normal = Some(normal);
        Ok(())
    }
}
// Dot/unsigned-angle do not observe W. Scratch zero never updates retained W.
fn scratch(value: SkeletonRootVector) -> [f32; 4] {
    [value.xyz[0], value.xyz[1], value.xyz[2], 0.0]
}
fn add(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| left.xyz[i] + right.xyz[i]),
        w: left.w.zip(right.w).map(|(a, b)| a + b),
    }
}
#[cfg(test)]
#[path = "skateboard_contact_normal_tail_tests.rs"]
mod tests;
