//! Native flat-board basis, full inverse landing angle, and impact shake graphs.
use crate::{
    camera_attributes::CameraParameterPublication,
    camera_compass::runtime::math::unit_fraction,
    camera_man::CameraShake,
    camera_subject::{CameraSubject, identity},
    reckoning_numeric::{
        ReckoningNumeric, cross_typed as cross, try_acos, try_asin, try_length,
        try_normalize_unsafe,
    },
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector,
    skeleton_two_bone::try_inverse_basis_from_cofactors,
};

type Vector = SkeletonRootVector;
const UP: Vector = Vector::known([0.0, 1.0, 0.0, 0.0]);

fn multiply_add(vector: Vector, scalar: f32, addend: Vector) -> Vector {
    vector.zip(addend, |value, previous| value.mul_add(scalar, previous))
}

/// `GetSkateboardFlatMtx82e3b590`; native comparison replaces W with abs(X).
pub(crate) fn flat_basis<N: ReckoningNumeric>(
    subject: &CameraSubject,
    numeric: &mut N,
) -> Result<[Vector; 3], N::Error> {
    let mut forward = subject.board_effective_transform.columns()[2];
    forward.xyz[1] = 0.0;
    let epsilon = f32::from_bits(0x3400_0000);
    if !(forward.xyz[0].abs() > epsilon || forward.xyz[2].abs() > epsilon) {
        let [x, y, z, _] = *identity().columns();
        return Ok([x, y, z]);
    }
    // The source J store is to caller scratch before normalization, not a
    // retained camera publication. Unknown forward W remains unknown.
    let z = try_normalize_unsafe(forward, numeric)?;
    Ok([cross(UP, z), UP, z])
}

/// `UpdateTrajectoryData45118..45220`: includes native negative-zero translation.
pub(crate) fn landing_incline<N: ReckoningNumeric>(
    basis: [Vector; 3],
    normal: Vector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let [x, y, z] = basis;
    let inverse =
        try_inverse_basis_from_cofactors(x, [cross(y, z), cross(z, x), cross(x, y)], numeric)?;
    let translated = inverse[0].map(|value| value * -0.0);
    let translated = multiply_add(inverse[1], -0.0, translated);
    let translated = multiply_add(inverse[2], -0.0, translated);
    let transformed = multiply_add(inverse[0], normal.xyz[0], translated);
    let transformed = multiply_add(inverse[1], normal.xyz[1], transformed);
    let transformed = multiply_add(inverse[2], normal.xyz[2], transformed);
    Ok(-try_asin(transformed.xyz[2], numeric)?)
}

/// Native FSEL lower/upper graph, including its unordered selection of +1.
pub(crate) fn signed_fraction(value: f32) -> f32 {
    let lower = if -1.0 - value >= 0.0 { -1.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

#[derive(Debug)]
pub(crate) enum TremorError<E> {
    Numeric(E),
    OwnerLock,
    UnwrittenParameter(&'static str),
}

impl<E: std::fmt::Debug> std::fmt::Display for TremorError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "camera tremor numeric boundary: {error:?}"),
            Self::OwnerLock => f.write_str("camera tremor parameter owner lock poisoned"),
            Self::UnwrittenParameter(name) => {
                write!(f, "camera tremor parameter {name} is unpublished")
            }
        }
    }
}

impl CameraShake {
    /// `ShakeEffect::Tremor82e3cf90`; parameters are read only for positive impact.
    pub(crate) fn try_tremor<N: ReckoningNumeric>(
        &mut self,
        normal: Vector,
        velocity: Vector,
        numeric: &mut N,
    ) -> Result<bool, TremorError<N::Error>> {
        let speed = try_length(velocity, numeric).map_err(TremorError::Numeric)?;
        if f32::from_bits(0x3727_c5ac) > speed {
            return Ok(false);
        }
        let estimate = numeric
            .reciprocal_estimate(speed)
            .map_err(TremorError::Numeric)?;
        let inverse = refine_drive_reciprocal(speed, estimate);
        let direction = velocity.map(|lane| lane * inverse);
        let dot = numeric
            .dot_vector(normal.map(|lane| -lane), direction)
            .map_err(TremorError::Numeric)?;
        let angle = try_acos(signed_fraction(dot), numeric).map_err(TremorError::Numeric)?;
        let fraction = unit_fraction(angle * f32::from_bits(0x3f22_f983));
        let impact = (1.0 - fraction) * speed;
        if impact.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Ok(false);
        }
        let previous = self.one_shot_effect;
        let owner = self.parameters.lock().map_err(|_| TremorError::OwnerLock)?;
        let CameraParameterPublication::Fields(fields) = &owner.parameters else {
            return Err(TremorError::UnwrittenParameter(
                "shake_one_shot_min_velocity",
            ));
        };
        let minimum = fields
            .shake_one_shot_min_velocity
            .ok_or(TremorError::UnwrittenParameter(
                "shake_one_shot_min_velocity",
            ))?;
        let maximum = fields
            .shake_one_shot_max_velocity
            .ok_or(TremorError::UnwrittenParameter(
                "shake_one_shot_max_velocity",
            ))?;
        let magnitude = fields
            .shake_one_shot_magnitude
            .ok_or(TremorError::UnwrittenParameter("shake_one_shot_magnitude"))?;
        let fraction = unit_fraction((impact - minimum) / (maximum - minimum));
        let magnitude = (fraction * fraction) * magnitude;
        if magnitude > previous {
            self.one_shot_effect = magnitude;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
#[path = "camera_man_trajectory_math_tests.rs"]
mod tests;
