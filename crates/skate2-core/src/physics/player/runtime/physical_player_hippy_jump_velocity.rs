//! Actual `Skateboard::Toolkit_CalcHippyJumpVel` 0x82b290f8.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::jump_events::{
        SkeletonJumpAttributes, SkeletonJumpGlobal, SkeletonJumpVaultError,
    },
    skeleton_root_transforms::SkeletonRootVector as V,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum HippyJumpVelocityError<N> {
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    Poisoned,
    Missing(&'static str),
    Attribute(SkeletonJumpVaultError),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for HippyJumpVelocityError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hippy jump velocity: {self:?}")
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_calculate_pair_hippy_jump_velocity<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<V, HippyJumpVelocityError<N::Error>> {
        use HippyJumpVelocityError as E;
        if !self
            .try_controller_pair_matches_owner(owner, board, skeleton)
            .map_err(E::Pair)?
        {
            return Err(E::ForeignOwner);
        }
        // Native separately captures global+d0 before each GetAttributePointer.
        // These are live reads from the SAME physics_jump.default owner.
        let attribute = |key| {
            let mut attrs = owner.jump_attributes().lock().map_err(|_| E::Poisoned)?;
            let global = attrs
                .capture(SkeletonJumpGlobal::Adjustment)
                .map_err(E::Attribute)?;
            let pointer = attrs.lookup(&global, key).map_err(E::Attribute)?;
            attrs.float(&pointer).map_err(E::Attribute)
        };
        let first_height = attribute(0x7461_bf1d_ef25_99d0)?; //29140..174
        let second_height = attribute(0xebdb_4644_f9ac_d7bb)?; //29198..1d4
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let deck = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .deck_transform()
            .ok_or(E::Missing("Processed deck transform+40"))?
            .columns()[3];
        let com = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .centre_of_mass()
            .ok_or(E::Missing("Processed COM+250"))?
            .value;
        let offset = V {
            xyz: std::array::from_fn(|i| com.xyz[i] - deck.xyz[i]),
            w: com.w.zip(deck.w).map(|(a, b)| a - b),
        };
        let up = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .reckoning_up()
            .ok_or(E::Missing("Processed up+220"))?
            .value;
        let strength = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .jump_events()
            .strength()
            .ok_or(E::Missing("Processed jump strength+950"))?;
        let first_term = strength * first_height;
        let remaining = 1.0 - strength;
        let height = numeric.dot_vector(up, offset).map_err(E::Numeric)?; //29254
        let velocity = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .velocity()
            .ok_or(E::Missing("Processed velocity+190"))?
            .value;
        let vertical_velocity = numeric.dot_vector(up, velocity).map_err(E::Numeric)?; //2925c
        let target_height = remaining.mul_add(second_height, first_term); //29260
        finish_velocity(
            up,
            velocity,
            vertical_velocity,
            target_height - height,
            numeric,
        )
        .map_err(E::Numeric)
    }
}

fn finish_velocity<N: ReckoningNumeric>(
    up: V,
    velocity: V,
    vertical_velocity: f32,
    height_difference: f32,
    numeric: &mut N,
) -> Result<V, N::Error> {
    // Source literal 19.6, not a Processed/cvar gravity read. Do not clamp
    // negative height: the native rsqrt and following graph execute unchanged.
    let squared = height_difference * f32::from_bits(0x419c_cccd);
    let projected = up.map(|lane| lane * vertical_velocity);
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    let tangent = V {
        xyz: std::array::from_fn(|i| velocity.xyz[i] - projected.xyz[i]),
        w: velocity.w.zip(projected.w).map(|(a, b)| a - b),
    };
    let square = seed * seed;
    let half = seed * 0.5;
    let residual = -squared.mul_add(square, -1.0);
    let once = half.mul_add(residual, seed);
    let square = once * once;
    let half = once * 0.5;
    let residual = -squared.mul_add(square, -1.0);
    let twice = half.mul_add(residual, once);
    let root = squared * twice;
    //29298 compares against ZERO (actual VA62), selecting +0 only afterward.
    let root = if squared == 0.0 { 0.0 } else { root };
    Ok(V {
        xyz: std::array::from_fn(|i| up.xyz[i].mul_add(root, tangent.xyz[i])),
        w: up.w.zip(tangent.w).map(|(u, t)| u.mul_add(root, t)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    struct FailEstimate;
    impl ReckoningNumeric for FailEstimate {
        type Error = &'static str;
        fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
            unreachable!()
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
            unreachable!()
        }
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            unreachable!()
        }
        fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
            assert_eq!(value.to_bits(), 0);
            Err("estimate failed")
        }
    }
    #[test]
    fn zero_height_does_not_skip_the_numeric_estimate() {
        let up = V::known([0.0, 1.0, 0.0, 0.0]);
        let velocity = V::known([3.0, 7.0, 4.0, 5.0]);
        assert_eq!(
            finish_velocity(up, velocity, 7.0, 0.0, &mut FailEstimate),
            Err("estimate failed")
        );
    }
    #[test]
    fn replaces_up_velocity_preserving_tangent_and_unknown_w() {
        let up = V::known([0.0, 1.0, 0.0, 0.0]);
        let velocity = V {
            xyz: [3.0, -7.0, 4.0],
            w: None,
        };
        let result = finish_velocity(up, velocity, -7.0, 1.0, &mut HostReckoningNumeric).unwrap();
        assert_eq!((result.xyz[0], result.xyz[2]), (3.0, 4.0));
        assert!((result.xyz[1] - 19.6f32.sqrt()).abs() < 0.000001);
        assert_eq!(result.w, None);
    }
    #[test]
    fn zero_height_selects_zero_after_estimate_and_negative_height_is_unclamped() {
        let up = V::known([0.0, 1.0, 0.0, 0.0]);
        let velocity = V::known([3.0, 7.0, 4.0, 5.0]);
        for height in [0.0, -0.0] {
            let result =
                finish_velocity(up, velocity, 7.0, height, &mut HostReckoningNumeric).unwrap();
            assert_eq!(result, V::known([3.0, 0.0, 4.0, 5.0]));
        }
        let result = finish_velocity(up, velocity, 7.0, -1.0, &mut HostReckoningNumeric).unwrap();
        assert!(result.xyz.into_iter().all(f32::is_nan));
        assert!(result.w.unwrap().is_nan());
    }
}
