//! Same outer-board `BlendToAnimatedBoard82b25bd8`; no replacement blend owner.
use super::SkateboardSteeringState;
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_acos},
    reckoning_sin_cos::{ReckoningSinCosNumeric, sine::try_sine, try_cosine},
    skeleton_root_transforms::SkeletonRootMatrix,
};

#[derive(Debug)]
pub(crate) enum AnimatedBlendError<A, N, R> {
    Attribute(A),
    Numeric(N),
    Trigonometry(R),
}

/// VMX dot4 under the existing host reduction boundary: all four products
/// execute, including zero coefficients. The injectable XYZ reduction remains.
fn dot4<N: ReckoningNumeric>(a: [f32; 4], b: [f32; 4], n: &mut N) -> Result<f32, N::Error> {
    Ok(n.dot3(a, b)? + a[3] * b[3])
}
fn reciprocal_step(value: f32, estimate: f32) -> f32 {
    let residual = -estimate.mul_add(value, -1.0);
    estimate.mul_add(residual, estimate)
}

impl SkateboardSteeringState {
    /// Called only after `ApplyBoardAnimation` validates/copies processed+0.
    /// Graph is fetched at its source stage, after the initial inverse trig.
    pub(crate) fn try_blend_animated<A, N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
        &mut self,
        input: SkeletonRootMatrix,
        fast: bool,
        graph: impl FnOnce(bool, f32) -> Result<f32, A>,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<SkeletonRootMatrix, AnimatedBlendError<A, N::Error, R::Error>> {
        use AnimatedBlendError as E;
        if !self.physics_error_blending {
            return Ok(input);
        }
        let angle = try_acos(self.physics_error[3], numeric).map_err(E::Numeric)? * 2.0;
        let turns = angle * f32::from_bits(0x3e22_f983);
        // VRFIM uses host floor; denormals, NaN payloads and FP status remain
        // explicit numerical boundaries, as with the existing dot reduction.
        let fraction = turns - turns.floor();
        let wrapped = fraction - if fraction > 0.5 { 1.0 } else { 0.0 };
        let angle = (wrapped * f32::from_bits(0x40c9_0fdb)).abs();
        let blend = graph(fast, angle).map_err(E::Attribute)?;
        // Native BLT: unordered takes completion too, without changing q.
        if !matches!(
            blend.partial_cmp(&f32::from_bits(0x3f73_3333)),
            Some(std::cmp::Ordering::Less)
        ) {
            self.physics_error_blending = false;
            return Ok(input);
        }
        let identity = [0.0, 0.0, 0.0, 1.0];
        let mut quaternion = self.physics_error;
        let mut dot = dot4(quaternion, identity, numeric).map_err(E::Numeric)?;
        if 0.0 > dot {
            quaternion = quaternion.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
            dot = f32::from_bits(dot.to_bits() ^ 0x8000_0000);
        }
        let threshold =
            try_cosine(f32::from_bits(0x3db2_b8c3), rounding).map_err(E::Trigonometry)?;
        let result = if dot > threshold {
            let new_dot = dot4(quaternion, identity, numeric).map_err(E::Numeric)?;
            let result: [f32; 4] = std::array::from_fn(|i| {
                if new_dot > 0.0 {
                    (identity[i] - quaternion[i]).mul_add(blend, quaternion[i])
                } else {
                    -(identity[i] + quaternion[i]).mul_add(blend, -quaternion[i])
                }
            });
            let squared = dot4(result, result, numeric).map_err(E::Numeric)?;
            let estimate = numeric
                .reciprocal_sqrt_estimate(squared)
                .map_err(E::Numeric)?;
            // Existing board helper preserves literal VNMSUB zero signs.
            let reciprocal = super::physics_error::refine(squared, estimate);
            result.map(|v| v * reciprocal)
        } else {
            let angle = try_acos(dot, numeric).map_err(E::Numeric)?;
            let sine = try_sine(angle, rounding).map_err(E::Trigonometry)?;
            let first = numeric.reciprocal_estimate(sine).map_err(E::Numeric)?;
            let second = numeric.reciprocal_estimate(sine).map_err(E::Numeric)?;
            let first = reciprocal_step(sine, first);
            let second = reciprocal_step(sine, second);
            let identity_sine = try_sine(blend * angle, rounding).map_err(E::Trigonometry)?;
            let second = reciprocal_step(sine, second);
            let identity_weight = second * identity_sine;
            let identity_weighted = identity.map(|v| v * identity_weight);
            let source_sine = try_sine((1.0 - blend) * angle, rounding).map_err(E::Trigonometry)?;
            let first = reciprocal_step(sine, first);
            let source_weight = first * source_sine;
            std::array::from_fn(|i| quaternion[i].mul_add(source_weight, identity_weighted[i]))
        };
        self.physics_error = result; // 82b25ed0, before matrix construction.
        Ok(compose_error_rotation(input, result))
    }
}

fn compose_error_rotation(input: SkeletonRootMatrix, quaternion: [f32; 4]) -> SkeletonRootMatrix {
    let q = quaternion.map(|v| f32::from_bits(0x3fb5_04f3) * v);
    let h = q.map(|v| -v.mul_add(v, -0.5));
    let next = [q[1], q[2], q[0]];
    let other = [q[2], q[0], q[1]];
    let product: [f32; 3] = std::array::from_fn(|i| q[i] * next[i]);
    let w_product: [f32; 3] = std::array::from_fn(|i| q[3] * other[i]);
    let plus: [f32; 3] = std::array::from_fn(|i| product[i] + w_product[i]);
    let minus: [f32; 3] = std::array::from_fn(|i| product[i] - w_product[i]);
    let diagonal = [h[0] + h[1], h[1] + h[2], h[2] + h[0]];
    // Native permutation scratch W is retained even though compose consumes
    // only inner XYZ coefficients: X.W=diagonal.x, Y.W=minus.x, Z.W=plus.x.
    let rotation = SkeletonRootMatrix::known([
        diagonal[1],
        plus[0],
        minus[2],
        diagonal[0],
        minus[0],
        diagonal[2],
        plus[1],
        minus[0],
        plus[2],
        minus[1],
        diagonal[0],
        plus[0],
        0.0,
        0.0,
        0.0,
        0.0,
    ]);
    // compose executes the three zero-coefficient translation FMAs as well.
    input.compose(&rotation)
}

#[cfg(test)]
#[path = "skateboard_animated_blend_tests.rs"]
mod tests;
