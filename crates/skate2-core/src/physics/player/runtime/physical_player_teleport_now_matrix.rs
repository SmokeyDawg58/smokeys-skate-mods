//! Local reset matrix made by `PhysicalPlayerHiLOD::TeleportPlayerNow`.
//! This stops before the HiLOD flag/history stores at 0x82e008ac.

use crate::{
    simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

const HORIZONTAL_LENGTH_MIN: f32 = f32::from_bits(0x3a83_126f);
const UP_A_BIT: SkeletonRootVector =
    SkeletonRootVector::known([0.0, f32::from_bits(0x3dcc_cccd), 0.0, 0.0]);
const X: SkeletonRootVector = SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]);
const Y: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
const Z: SkeletonRootVector = SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]);

/// Separate estimates correspond to Xenon `vrefp` and `vrsqrtefp`. The host
/// implementation below preserves their refinement graphs, but cannot claim
/// Xenon seed, reduction, denormal, NaN or fused-operation bit parity.
pub(crate) trait TeleportNowMatrixNumeric {
    fn dot3(&mut self, left: [f32; 3], right: [f32; 3]) -> f32;
    fn reciprocal_sqrt_estimate(&mut self, squared_length: f32) -> f32;
    fn reciprocal_estimate(&mut self, length: f32) -> f32;
}

pub(crate) struct HostTeleportNowMatrixNumeric;

impl TeleportNowMatrixNumeric for HostTeleportNowMatrixNumeric {
    fn dot3(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
        left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
    }

    fn reciprocal_sqrt_estimate(&mut self, squared_length: f32) -> f32 {
        squared_length.sqrt().recip()
    }

    fn reciprocal_estimate(&mut self, length: f32) -> f32 {
        length.recip()
    }
}

/// Raw 0x82e00640..0x82e00830 selects a yaw-only orientation from the input
/// Z axis, falling back to I/J/K when its horizontal length is not strictly
/// above 0.001. The true branch forms J cross horizontal Z and performs the
/// source reverse Gram-Schmidt pass Z, Y, X. The 0x82e008a0..bc translation
/// is the input's full fourth column plus UpABit in *all four lanes*.
pub(crate) fn adjusted_teleport_matrix(
    reset: &SkeletonRootMatrix,
    numeric: &mut impl TeleportNowMatrixNumeric,
) -> SkeletonRootMatrix {
    let input_z = reset.columns()[2];
    // vrlimi128 mask 4 inserts +0 into Y, retaining the input Z-axis W.
    let horizontal_z = SkeletonRootVector {
        xyz: [input_z.xyz[0], 0.0, input_z.xyz[2]],
        w: input_z.w,
    };
    let squared = numeric.dot3(horizontal_z.xyz, horizontal_z.xyz);
    let rsqrt = refine_drive_reciprocal_sqrt(squared, numeric.reciprocal_sqrt_estimate(squared));
    let calculated_length = squared * rsqrt;
    let length = if squared == 0.0 {
        0.0
    } else {
        calculated_length
    };

    let mut axes = [X, Y, Z];
    if length > HORIZONTAL_LENGTH_MIN {
        // The first normalization starts from vrefp(length), rather than
        // reusing the prior vrsqrtefp(squared) result.
        // 0x82e00750 and 0x82e00758 place the reciprocal seed/refined
        // reciprocal in the first multiply source, and length in the second.
        // That is the shared helper's `(-seed).mul_add(length, 1)` order,
        // including exceptional-value behavior; the rsqrt sites above and
        // below share its distinct square/half/fused-residual graph.
        let reciprocal = refine_drive_reciprocal(length, numeric.reciprocal_estimate(length));
        let forward = horizontal_z.map(|lane| lane * reciprocal);
        axes = [vmx_cross(Y, forward), Y, forward];
        axes = reverse_gram_schmidt(axes, numeric);
    }

    let translation = reset.columns()[3].zip(UP_A_BIT, |left, right| left + right);
    SkeletonRootMatrix::from_columns([axes[0], axes[1], axes[2], translation])
}

fn vmx_cross(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    // vpermwi128 0x63 is YZXW; vmulfp128 and vnmsubfp use full lanes.
    let left_yzx = [left.xyz[1], left.xyz[2], left.xyz[0]];
    let right_yzx = [right.xyz[1], right.xyz[2], right.xyz[0]];
    let difference: [f32; 3] = std::array::from_fn(|lane| {
        let product = left.xyz[lane] * right_yzx[lane];
        (-left_yzx[lane]).mul_add(right.xyz[lane], product)
    });
    SkeletonRootVector {
        xyz: [difference[1], difference[2], difference[0]],
        w: left.w.zip(right.w).map(|(a, b)| (-a).mul_add(b, a * b)),
    }
}

fn reverse_gram_schmidt(
    original: [SkeletonRootVector; 3],
    numeric: &mut impl TeleportNowMatrixNumeric,
) -> [SkeletonRootVector; 3] {
    let mut output = [X, Y, Z];
    for axis in (0..3).rev() {
        let mut candidate = original[axis];
        for previous in ((axis + 1)..3).rev() {
            // 0x82e007a4 samples the original axis, not the shortened one.
            let basis = output[previous];
            let projection = numeric.dot3(basis.xyz, original[axis].xyz);
            candidate = candidate.zip(basis.map(|lane| lane * projection), |a, b| a - b);
        }
        let squared = numeric.dot3(candidate.xyz, candidate.xyz);
        let rsqrt =
            refine_drive_reciprocal_sqrt(squared, numeric.reciprocal_sqrt_estimate(squared));
        output[axis] = candidate.map(|lane| lane * rsqrt);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct CountingNumeric {
        rsqrt: usize,
        reciprocal: usize,
    }

    impl TeleportNowMatrixNumeric for CountingNumeric {
        fn dot3(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
            HostTeleportNowMatrixNumeric.dot3(left, right)
        }

        fn reciprocal_sqrt_estimate(&mut self, squared_length: f32) -> f32 {
            self.rsqrt += 1;
            HostTeleportNowMatrixNumeric.reciprocal_sqrt_estimate(squared_length)
        }

        fn reciprocal_estimate(&mut self, length: f32) -> f32 {
            self.reciprocal += 1;
            HostTeleportNowMatrixNumeric.reciprocal_estimate(length)
        }
    }

    #[test]
    fn vertical_input_keeps_canonical_basis_and_adds_full_translation() {
        let mut reset = SkeletonRootMatrix::known([
            0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 7.0, 3.0, 4.0, 5.0, 9.0,
        ]);
        let mut translation = reset.columns()[3];
        translation.w = None;
        reset.set_column(3, translation);
        let mut numeric = CountingNumeric::default();
        let actual = adjusted_teleport_matrix(&reset, &mut numeric);
        assert_eq!(&actual.columns()[..3], &[X, Y, Z]);
        assert_eq!(actual.columns()[3].xyz, [3.0, 4.0 + UP_A_BIT.xyz[1], 5.0]);
        assert_eq!(actual.columns()[3].w, None);
        assert_eq!((numeric.rsqrt, numeric.reciprocal), (1, 0));
    }

    #[test]
    fn tilted_input_uses_reciprocal_then_reverse_rsqrt_and_preserves_unknown_w() {
        let mut reset = SkeletonRootMatrix::known([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.6, 0.8, 0.8, 2.0, 3.0, 4.0, 5.0, 6.0,
        ]);
        let mut forward = reset.columns()[2];
        forward.w = None;
        reset.set_column(2, forward);
        let mut numeric = CountingNumeric::default();
        let actual = adjusted_teleport_matrix(&reset, &mut numeric);
        let axes = actual.columns();
        for (actual, expected) in axes[0].xyz.into_iter().zip([0.8, 0.0, -0.6]) {
            assert!((actual - expected).abs() < 1.0e-5);
        }
        assert_eq!(axes[1].xyz, [0.0, 1.0, 0.0]);
        for (actual, expected) in axes[2].xyz.into_iter().zip([0.6, 0.0, 0.8]) {
            assert!((actual - expected).abs() < 1.0e-5);
        }
        assert_eq!(axes[2].w, None);
        assert_eq!(axes[3].w, Some(6.0));
        assert_eq!((numeric.rsqrt, numeric.reciprocal), (4, 1));
    }
}
