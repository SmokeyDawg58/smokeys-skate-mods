//! ComputeSkateboardThrowParameters82dcca10, using retained source reads.
use super::righting::normalize;
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_typed},
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_root_transforms::SkeletonRootVector as V,
};

pub(crate) trait ThrowSource {
    type Error;
    fn effective_forward(&mut self) -> Result<V, Self::Error>;
    fn com_velocity(&mut self) -> Result<V, Self::Error>;
    fn board_velocity(&mut self) -> Result<V, Self::Error>;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn curve(&mut self, key: u64, input: f32) -> Result<f32, Self::Error>;
    fn mass(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum ThrowError<S, N, R> {
    Source(S),
    Numeric(N),
    Rounding(R),
}

fn refine(value: f32, seed: f32) -> f32 {
    let residual = -seed.mul_add(value, -1.0);
    let first = seed.mul_add(residual, seed);
    let residual = -first.mul_add(value, -1.0);
    first.mul_add(residual, first)
}

pub(crate) fn try_compute<S: ThrowSource, N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    source: &mut S,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<V, ThrowError<S::Error, N::Error, R::Error>> {
    use ThrowError as E;
    let up = V::known([0.0, 1.0, 0.0, 0.0]);
    let forward = source.effective_forward().map_err(E::Source)?;
    let vertical = numeric.dot_vector(forward, up).map_err(E::Numeric)?;
    let com_velocity = source.com_velocity().map_err(E::Source)?;
    let flat_forward = forward.zip(up.map(|x| x * vertical), |a, b| a - b);
    let forward = normalize(flat_forward, numeric).map_err(E::Numeric)?;
    let side = normalize(cross_typed(up, forward), numeric).map_err(E::Numeric)?;
    let degrees = source.scalar(0x9e6c_8713_483c_4768).map_err(E::Source)?;
    let com_vertical = numeric.dot_vector(com_velocity, up).map_err(E::Numeric)?;
    let board_velocity = source.board_velocity().map_err(E::Source)?;
    let board_vertical = numeric.dot_vector(board_velocity, up).map_err(E::Numeric)?;
    let angle = degrees * f32::from_bits(0x3c8e_fa35);
    let board_flat = board_velocity.zip(up.map(|x| x * board_vertical), |a, b| a - b);
    let com_flat = com_velocity.zip(up.map(|x| x * com_vertical), |a, b| a - b);
    let sideways = numeric.dot_vector(board_flat, side).map_err(E::Numeric)?;
    let com_speed = numeric.dot_vector(com_flat, forward).map_err(E::Numeric)?;
    let lateral_velocity = side.map(|x| x * sideways);
    let longitudinal = board_flat.zip(lateral_velocity, |a, b| a - b);
    let squared = numeric
        .dot_vector(longitudinal, longitudinal)
        .map_err(E::Numeric)?;
    // Inline length always estimates, then selects zero only for squared==0.
    let seed = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(E::Numeric)?;
    let residual = -squared.mul_add(seed * seed, -1.0);
    let first = (seed * 0.5).mul_add(residual, seed);
    let residual = -squared.mul_add(first * first, -1.0);
    let inverse = (first * 0.5).mul_add(residual, first);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    let rotation = crate::skeleton_normal_displacement::try_rotated_identity(
        side.xyz,
        f32::from_bits(angle.to_bits() ^ 0x8000_0000),
        rounding,
    )
    .map_err(E::Rounding)?;
    let correction = length - com_speed;
    let requested = source
        .curve(0xe2f1_0c8b_6cab_532b, com_speed)
        .map_err(E::Source)?
        - correction;
    // Native calls ATan(angle), not Tan(angle). Preserve the actual debug build.
    let tangent = crate::skeleton_query_rotation::try_atan(angle, numeric).map_err(E::Numeric)?;
    let tangent_seed = numeric.reciprocal_estimate(tangent).map_err(E::Numeric)?;
    let rotated = V::known(std::array::from_fn(|i| {
        let x = rotation[0][i].mul_add(forward.xyz[0], 0.0);
        let xy = rotation[1][i].mul_add(forward.xyz[1], x);
        rotation[2][i].mul_add(forward.xyz[2], xy)
    }));
    let mass = source.mass().map_err(E::Source)?;
    let tangent_inverse = refine(tangent, tangent_seed);
    let dt = f32::from_bits(0x3c88_8889);
    let dt_seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
    let scale = tangent_inverse * requested;
    let inverse_dt = refine(dt, dt_seed);
    let primary = rotated
        .map(|x| x * scale)
        .map(|x| x * mass)
        .map(|x| inverse_dt * x);
    let conservation = source.scalar(0x2149_0bc7_264f_1fa8).map_err(E::Source)?;
    let mass = source.mass().map_err(E::Source)?;
    let lateral = lateral_velocity.map(|x| x * mass);
    let dt_seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
    let inverse_dt = refine(dt, dt_seed);
    let lateral = lateral.map(|x| inverse_dt * x);
    Ok(lateral.zip(primary, |a, b| a.mul_add(conservation, b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Source {
        calls: Vec<&'static str>,
        fail_second_mass: bool,
        masses: usize,
    }
    impl ThrowSource for Source {
        type Error = &'static str;
        fn effective_forward(&mut self) -> Result<V, Self::Error> {
            self.calls.push("forward");
            Ok(V::known([0., 0., 1., 0.]))
        }
        fn com_velocity(&mut self) -> Result<V, Self::Error> {
            self.calls.push("com");
            Ok(V::known([0., 0., 3., 0.]))
        }
        fn board_velocity(&mut self) -> Result<V, Self::Error> {
            self.calls.push("board");
            Ok(V::known([1., 0., 2., 0.]))
        }
        fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
            match key {
                0x9e6c_8713_483c_4768 => {
                    self.calls.push("angle");
                    Ok(30.)
                }
                0x2149_0bc7_264f_1fa8 => {
                    self.calls.push("conservation");
                    Ok(-0.5)
                }
                _ => panic!("unexpected attribute"),
            }
        }
        fn curve(&mut self, key: u64, input: f32) -> Result<f32, Self::Error> {
            assert_eq!(key, 0xe2f1_0c8b_6cab_532b);
            assert!((input - 3.).abs() < 1e-5);
            self.calls.push("curve");
            Ok(2.)
        }
        fn mass(&mut self) -> Result<f32, Self::Error> {
            self.calls.push("mass");
            self.masses += 1;
            if self.fail_second_mass && self.masses == 2 {
                Err("second mass")
            } else {
                Ok(2.)
            }
        }
    }
    #[test]
    fn throw_projects_lateral_velocity_and_retains_atan_launch_scale() {
        let mut source = Source {
            calls: Vec::new(),
            fail_second_mass: false,
            masses: 0,
        };
        let force = try_compute(
            &mut source,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
        .unwrap();
        assert_eq!(
            source.calls,
            [
                "forward",
                "com",
                "angle",
                "board",
                "curve",
                "mass",
                "conservation",
                "mass"
            ]
        );
        let angle = 30.0_f32.to_radians();
        let scale = 3.0 / angle.atan() * 2.0 / f32::from_bits(0x3c88_8889);
        assert!((force.xyz[0] + 60.).abs() < 0.001);
        assert!((force.xyz[1] - angle.sin() * scale).abs() < 0.02);
        assert!((force.xyz[2] - angle.cos() * scale).abs() < 0.02);
    }
    #[test]
    fn late_mass_reload_failure_is_not_replaced_by_cached_mass() {
        let mut source = Source {
            calls: Vec::new(),
            fail_second_mass: true,
            masses: 0,
        };
        assert!(matches!(
            try_compute(
                &mut source,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric
            ),
            Err(ThrowError::Source("second mass"))
        ));
        assert_eq!(source.masses, 2);
    }
}
