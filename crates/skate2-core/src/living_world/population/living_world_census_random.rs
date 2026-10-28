//! LW.GetRandomPos82e4e898, shared by pedestrian and vehicle census passes.
use crate::{
    native_skater_random::{NativeRandomGate, NativeRandomLocked, NativeSkaterRandom},
    reckoning_numeric::ReckoningNumeric,
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
};

#[derive(Debug)]
pub(crate) enum CensusPositionError<N> {
    Random(NativeRandomLocked),
    Numeric(N),
}

/// Three Manager RNG draws: Z, X, then radius. This is a normalized square
/// direction and a linearly distributed radius, not a uniform area sampler.
/// Source normalization is unsafe when both horizontal components are zero.
pub(crate) fn random_position<N: ReckoningNumeric>(
    random: &mut NativeSkaterRandom,
    gate: &NativeRandomGate,
    center: [f32; 4],
    inner_radius: f32,
    outer_radius: f32,
    numeric: &mut N,
) -> Result<[f32; 4], CensusPositionError<N::Error>> {
    use CensusPositionError as E;
    let scale = f32::from_bits(0x2f80_0000);
    let z = (random.try_next(gate).map_err(E::Random)? as f32 * scale) - 0.5;
    // Raw fmsubs for X, separate fmuls/fsubs for Z.
    let x = (random.try_next(gate).map_err(E::Random)? as f32).mul_add(scale, -0.5);
    let direction = [x, 0., z, 0.];
    let squared = numeric.dot3(direction, direction).map_err(E::Numeric)?;
    let seed = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(E::Numeric)?;
    // The third RNG call follows the estimate, before the final refinement.
    let radius = random
        .try_living_world_range(gate, inner_radius, outer_radius)
        .map_err(E::Random)?;
    let inverse = refine_drive_reciprocal_sqrt(squared, seed);
    Ok(std::array::from_fn(|lane| {
        (direction[lane] * inverse).mul_add(radius, center[lane])
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::{HostReckoningNumeric, host_dot3};

    // UNRUN: source RandValue reset words already pinned by the RNG tests.
    #[test]
    fn horizontal_spawn_uses_three_draws_and_preserves_height_and_w() {
        let gate = NativeRandomGate::from_static_initializer();
        let mut random = NativeSkaterRandom::reset();
        let position = random_position(
            &mut random,
            &gate,
            [10., 7., 20., 3.],
            15.,
            15.,
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(random.index, 3);
        assert_eq!(random.try_next(&gate).unwrap(), 0x76b6_5632);
        assert_eq!((position[1], position[3]), (7., 3.));
        let x = position[0] - 10.;
        let z = position[2] - 20.;
        assert!(x < z && z < 0.); // X uses SECOND word, Z uses FIRST.
        assert!(((x * x + z * z).sqrt() - 15.).abs() < 0.00001);
    }

    struct FailEstimate;
    impl ReckoningNumeric for FailEstimate {
        type Error = ();
        fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, ()> {
            Ok(host_dot3(a, b))
        }
        fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, ()> {
            Err(())
        }
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, ()> {
            panic!("unused")
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, ()> {
            panic!("unused")
        }
    }
    #[test]
    fn numeric_failure_retains_two_draw_prefix_without_consuming_radius() {
        let gate = NativeRandomGate::from_static_initializer();
        let mut random = NativeSkaterRandom::reset();
        assert!(matches!(
            random_position(&mut random, &gate, [0.; 4], 8., 80., &mut FailEstimate),
            Err(CensusPositionError::Numeric(()))
        ));
        assert_eq!(random.index, 2);
        assert_eq!(random.try_next(&gate).unwrap(), 0xe827_f7e0);
    }
}
