//! One explicit host numeric policy across modern GP and triangle line tests.
//! Separate XYZ products and left-associated sums match the existing Reckoning
//! host backend. The old Assembly/ground backend keeps its own nested FMA policy.
//! Neither policy is claimed to reproduce Xenon reduction/estimate bits.

use crate::{
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, host_dot3,
    },
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
};

pub(crate) struct HostCollisionLineNumeric;

impl AssemblyGroundNumeric for HostCollisionLineNumeric {
    fn sum_three_products(&mut self, a: [f32; 3], b: [f32; 3]) -> f32 {
        host_dot3([a[0], a[1], a[2], 0.0], [b[0], b[1], b[2], 0.0])
    }
    fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32 {
        1.0 / value.sqrt()
    }
    fn reciprocal_seed(&mut self, value: f32) -> f32 {
        1.0 / value
    }
}

impl ReckoningNumeric for HostCollisionLineNumeric {
    type Error = ReckoningHostNumericError;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        HostReckoningNumeric.dot3(a, b)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric.reciprocal_sqrt_estimate(value)
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric.reciprocal_estimate(value)
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric.truncate_i32(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_traits_share_one_reduction_with_cancellation_and_ignore_w() {
        let mut numeric = HostCollisionLineNumeric;
        let a = [33_554_432.0, -33_554_432.0, 1.0];
        let b = [1.0; 3];
        let gp = numeric.sum_three_products(a, b);
        let line = numeric
            .dot3([a[0], a[1], a[2], f32::NAN], [1.0; 4])
            .unwrap();
        assert_eq!(gp.to_bits(), 1.0_f32.to_bits());
        assert_eq!(gp.to_bits(), line.to_bits());
        // The separately retained old GP host reduction gives zero here.
        assert_eq!(
            a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
                .to_bits(),
            0
        );
    }
}
