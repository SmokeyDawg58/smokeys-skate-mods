//! Retained mass/frame suffix of `ComputeMassProperties` (0x82ae9a50), plus
//! `SkeletonBody`'s debug inertia adjustments (0x82e4b0e0 / 0x82e4b188).
//! Principal geometry is supplied by an actual source shape/integral producer;
//! this module does not dispatch volumes or manufacture a physical skeleton.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "shared skeleton mass state awaits actual Part construction"
    )
)]

use crate::{
    simulation_rigid_body_pool::SimulationInertia,
    skateboard_spawn_math::skateboard_spawn_rigid_inverse,
    skateboard_wheels::{
        HostSkateboardWheelMassNumeric, SkateboardWheelInertiaAxis, SkateboardWheelMassNumeric,
        apply_unit_density_mass_to_inertia,
    },
};

#[cfg(test)]
#[path = "skeleton_body_mass_tests.rs"]
mod tests;

/// Pure numeric services: callbacks must not reenter or mutate the retained
/// owner. Scalar Rust division/FMA remains the explicit host arithmetic policy;
/// VMX dot reduction/estimate, denormals and NaN payload parity are not claimed.
pub(crate) trait SkeletonBodyMassNumeric: SkateboardWheelMassNumeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> f32;
    fn debug_factor_reciprocal_estimate(&mut self, factor: f32) -> f32;
}

pub(crate) struct HostSkeletonBodyMassNumeric;
impl SkateboardWheelMassNumeric for HostSkeletonBodyMassNumeric {
    fn reciprocal_estimate(&mut self, axis: SkateboardWheelInertiaAxis, denominator: f32) -> f32 {
        HostSkateboardWheelMassNumeric.reciprocal_estimate(axis, denominator)
    }
}
impl SkeletonBodyMassNumeric for HostSkeletonBodyMassNumeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
        left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
    }
    fn debug_factor_reciprocal_estimate(&mut self, factor: f32) -> f32 {
        factor.recip()
    }
}

/// Inactive inverse-frame storage is separate from its source flag. None means
/// native bytes have never been initialized, not a zero/identity matrix. A later
/// flag-clear preserves any prior known storage. Packed inverse tensor W is
/// retained even though ordinary body solvers consume the three XYZ values.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BodyMassState {
    pub(crate) inertia: SimulationInertia,
    pub(crate) inverse_tensor_w: f32,
    pub(crate) inverse_body_transform_storage: Option<[f32; 16]>,
    pub(crate) has_inverse_body_transform: bool,
}

impl BodyMassState {
    /// `PartDefinition::Initialize`, 0x82ae9058..0x82ae90e8.
    pub(crate) const fn native_default() -> Self {
        Self {
            inertia: SimulationInertia::native_default(),
            inverse_tensor_w: 1.0,
            inverse_body_transform_storage: None,
            has_inverse_body_transform: false,
        }
    }

    pub(crate) const fn packed_inverse_tensor(&self) -> [f32; 4] {
        let [x, y, z] = self.inertia.inverse_tensor;
        [x, y, z, self.inverse_tensor_w]
    }

    /// Frame gate 0x82ae9bc0..9cd4, then existing density-one mass suffix.
    /// Supplied frame is the actual principal body-to-Part frame, not a pose to
    /// normalize. Inverse storage uses source rigid transpose even for unusual
    /// nonorthonormal input; no determinant inverse or geometry repair occurs.
    pub(crate) fn apply_principal(
        &mut self,
        diagonal: [f32; 3],
        volume: f32,
        frame: [f32; 16],
        mass: f32,
        numeric: &mut impl SkeletonBodyMassNumeric,
    ) {
        let threshold = ((diagonal[1] + diagonal[2]) + diagonal[0]) * f32::from_bits(0x3586_37be);
        let position = [frame[12], frame[13], frame[14], frame[15]];
        let displacement = numeric.sum_three_products(position, position);
        // Native NOT_LT admits equality and unordered; replacing this with
        // >= would silently discard the NaN inverse-construction branch.
        let translated = !matches!(
            displacement.partial_cmp(&threshold),
            Some(core::cmp::Ordering::Less)
        );
        self.has_inverse_body_transform = translated
            || (0..3).any(|column| {
                (0..3).any(|lane| {
                    let expected = f32::from(u8::from(column == lane));
                    (frame[column * 4 + lane] - expected).abs() > f32::from_bits(0x3a83_126f)
                })
            });
        if self.has_inverse_body_transform {
            self.inverse_body_transform_storage = Some(skateboard_spawn_rigid_inverse(&frame));
        }
        apply_unit_density_mass_to_inertia(diagonal, volume, mass, &mut self.inertia, numeric);
        self.inverse_tensor_w = self.inertia.inverse_tensor[0];
    }

    /// Debug method1, 0x82e4b0e0..b188. One factor seed, two refinements,
    /// multiply ALL four existing lanes, then later-on-tie/unordered minimum.
    pub(crate) fn debug_multiply(
        &mut self,
        factor: f32,
        numeric: &mut impl SkeletonBodyMassNumeric,
    ) {
        let mut reciprocal = numeric.debug_factor_reciprocal_estimate(factor);
        for _ in 0..2 {
            let residual = (-factor).mul_add(reciprocal, 1.0);
            reciprocal = reciprocal.mul_add(residual, reciprocal);
        }
        let packed = self.packed_inverse_tensor().map(|value| reciprocal * value);
        self.inertia.inverse_tensor = [packed[0], packed[1], packed[2]];
        self.inverse_tensor_w = packed[3];
        self.refresh_spherical();
    }

    /// Debug method2, 0x82e4b188..b254. Source separately computes inverse of
    /// factor*3, then multiplies the ordered XYZ sum. W is explicitly +0.
    pub(crate) fn debug_multiply_isotropic(&mut self, factor: f32) {
        let factor_three = factor * 3.0;
        let [x, y, z] = self.inertia.inverse_tensor;
        let xy = x + y;
        let inverse_factor = 1.0 / factor_three;
        let total = xy + z;
        let value = inverse_factor * total;
        self.inertia.inverse_tensor = [value; 3];
        self.inverse_tensor_w = 0.0;
        self.refresh_spherical();
    }

    fn refresh_spherical(&mut self) {
        let [x, y, z] = self.inertia.inverse_tensor;
        let xy = if x < y { x } else { y };
        let minimum = if xy < z { xy } else { z };
        self.inertia.spherical = 1.0 / minimum;
    }
}
