//! `WriteSkater` matrix Q4 loop, 0x827d8c94..0x827d8e20.
//! This produces logical XYZW halfwords; the stream writer emits big endian.
use crate::{
    animation::AnimationMatrix, reckoning_numeric::ReckoningNumeric,
    world::try_simulation_quaternion_candidate,
};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MatrixQ4CompressionError<E> {
    DestinationBounds { required: usize, available: usize },
    MatrixBounds { ordinal: usize, index: usize },
    Numeric(E),
}
impl<E: fmt::Display> fmt::Display for MatrixQ4CompressionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DestinationBounds {
                required,
                available,
            } => write!(
                f,
                "matrix Q4 requires {required} destination records, has {available}"
            ),
            Self::MatrixBounds { ordinal, index } => write!(
                f,
                "matrix Q4 offset {ordinal} references missing matrix {index}"
            ),
            Self::Numeric(error) => write!(f, "matrix Q4 numeric service: {error}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for MatrixQ4CompressionError<E> {}

/// Borrow the actual local matrix buffer and original ordered `InitOffsets` list.
/// Empty indices follow the native loop skip8ba4. Capacity is a host admission
/// check; each matrix index is resolved at its actual iteration, preserving
/// already written records on a later source or numeric error.
pub(crate) fn try_compress_matrix_q4<N: ReckoningNumeric>(
    matrices: &[AnimationMatrix],
    indices: &[usize],
    destination: &mut [[u16; 4]],
    ratio: [f32; 4],
    numeric: &mut N,
) -> Result<(), MatrixQ4CompressionError<N::Error>> {
    use MatrixQ4CompressionError as E;
    if destination.len() < indices.len() {
        return Err(E::DestinationBounds {
            required: indices.len(),
            available: destination.len(),
        });
    }
    for (ordinal, &index) in indices.iter().enumerate() {
        let matrix = *matrices
            .get(index)
            .ok_or(E::MatrixBounds { ordinal, index })?;
        // One native vector estimate8d50 observes every magnitude, including
        // zero/negative unselected lanes. The scalar service adapter samples
        // lanes0..3; no normalization or matrix orthonormalization is added.
        let quaternion = try_simulation_quaternion_candidate(matrix, |magnitudes| {
            let mut seeds = [0.0; 4];
            for lane in 0..4 {
                seeds[lane] = numeric.reciprocal_sqrt_estimate(magnitudes[lane])?;
            }
            Ok(seeds)
        })
        .map_err(E::Numeric)?;
        let scaled: [f32; 4] = std::array::from_fn(|lane| quaternion[lane] * ratio[lane]); //8dbc
        let mut halfwords = [0; 4];
        // fctiwz X,Y,W,Z at8dcc/8dd4/8dec/8df8. All four complete before
        // destination writes; no saturating float cast or halfword clamp.
        for lane in [0, 1, 3, 2] {
            let bytes = numeric
                .truncate_i32(scaled[lane])
                .map_err(E::Numeric)?
                .to_be_bytes();
            halfwords[lane] = u16::from_be_bytes([bytes[2], bytes[3]]);
        }
        destination[ordinal][0] = halfwords[0]; //8e08
        destination[ordinal][1] = halfwords[1]; //8e0c
        destination[ordinal][3] = halfwords[3]; //8e14
        destination[ordinal][2] = halfwords[2]; //8e18
    }
    Ok(())
}

#[cfg(test)]
#[path = "skater_presentation_compression_q4_tests.rs"]
mod tests;
