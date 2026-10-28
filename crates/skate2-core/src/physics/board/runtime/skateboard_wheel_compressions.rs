//! `CalculateAverageWheelCompressions` 0x82b2b538..82b2b800.
//! General determinant inverse, real body transforms, and late retained distance.
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_full4},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
    skeleton_two_bone::try_inverse_basis_from_cofactors,
};

#[cfg(test)]
#[path = "skateboard_wheel_compressions_tests.rs"]
mod tests;

pub(crate) trait SkateboardWheelCompressionServices {
    type Error;
    fn body_transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error>;
    fn wheel_distance(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardWheelCompressionError<S, N> {
    Service(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display
    for SkateboardWheelCompressionError<S, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wheel compression: {self:?}")
    }
}

#[derive(Default)]
pub(crate) struct SkateboardWheelCompressions {
    front: Option<f32>,
    back: Option<f32>,
    complete: bool,
}
impl SkateboardWheelCompressions {
    pub(crate) fn current(&self) -> Option<[f32; 2]> {
        self.complete.then(|| {
            [
                self.front.expect("front written"),
                self.back.expect("back written"),
            ]
        })
    }
    pub(crate) fn try_update<S: SkateboardWheelCompressionServices, N: ReckoningNumeric>(
        &mut self,
        services: &mut S,
        numeric: &mut N,
    ) -> Result<(), SkateboardWheelCompressionError<S::Error, N::Error>> {
        use SkateboardWheelCompressionError as E;
        self.complete = false;
        let deck = services.body_transform(6).map_err(E::Service)?;
        let [x, y, z, position] = *deck.columns();
        // Source independent cofactor order is C0, C2, C1.
        let cofactor_x = cross(y, z);
        let cofactor_z = cross(x, y);
        let cofactor_y = cross(z, x);
        let basis =
            try_inverse_basis_from_cofactors(x, [cofactor_x, cofactor_y, cofactor_z], numeric)
                .map_err(E::Numeric)?;
        let negative = position
            .xyz
            .map(|value| f32::from_bits(value.to_bits() ^ 0x8000_0000));
        // Source scalar-left multiply/FMA/FMA; recovered inverse W is known.
        let inverse_position = scale_left(negative[0], basis[0]);
        let inverse_position = madd_left(negative[1], basis[1], inverse_position);
        let inverse_position = madd_left(negative[2], basis[2], inverse_position);
        let mut positions = [SkeletonRootVector::known([0.0; 4]); 4];
        for (part, position) in positions.iter_mut().enumerate() {
            *position = services.body_transform(part).map_err(E::Service)?.columns()[3];
        }
        let mut local =
            positions.map(|position| madd_right(basis[0], position.xyz[0], inverse_position));
        // Capture retained source member after all X projections, before Y/Z.
        let distance = services.wheel_distance().map_err(E::Service)?;
        for (local, position) in local.iter_mut().zip(positions) {
            *local = madd_right(basis[1], position.xyz[1], *local);
        }
        for (local, position) in local.iter_mut().zip(positions) {
            *local = madd_right(basis[2], position.xyz[2], *local);
        }
        let front_sum = local[0].xyz[1] + local[1].xyz[1];
        let back_sum = local[2].xyz[1] + local[3].xyz[1];
        let front = 0.5 * front_sum - distance;
        let back = 0.5 * back_sum - distance;
        self.front = Some(front);
        self.back = Some(back);
        self.complete = true;
        Ok(())
    }
}
fn cross(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    let scratch = |v: SkeletonRootVector| [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.0)];
    let result = cross_full4(scratch(a), scratch(b));
    SkeletonRootVector {
        xyz: [result[0], result[1], result[2]],
        w: a.w.zip(b.w).map(|_| result[3]),
    }
}
fn scale_left(scalar: f32, value: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: value.xyz.map(|lane| scalar * lane),
        w: value.w.map(|lane| scalar * lane),
    }
}
fn madd_left(
    scalar: f32,
    value: SkeletonRootVector,
    addend: SkeletonRootVector,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| scalar.mul_add(value.xyz[lane], addend.xyz[lane])),
        w: value
            .w
            .zip(addend.w)
            .map(|(value, addend)| scalar.mul_add(value, addend)),
    }
}
fn madd_right(
    value: SkeletonRootVector,
    scalar: f32,
    addend: SkeletonRootVector,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| value.xyz[lane].mul_add(scalar, addend.xyz[lane])),
        w: value
            .w
            .zip(addend.w)
            .map(|(value, addend)| value.mul_add(scalar, addend)),
    }
}
