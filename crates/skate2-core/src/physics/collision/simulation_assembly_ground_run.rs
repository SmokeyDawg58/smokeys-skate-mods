//! Private value slices of Assembly/ground `RunIntersectionTests`.
//!
//! The complete live traversal owner is not reconstructed here. This module
//! starts with the source-shaped velocity-padding graph immediately before
//! `Sk8ComputeContactPoints` at `0x827cd444..0x827cd4cc`.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the reconstructed Assembly/ground Run owner"
    )
)]

const RUN_CONTACT_TIMESTEP: f32 = f32::from_bits(0x3c88_8889);
const FLOAT_SIGN_BIT: u32 = 0x8000_0000;

pub(crate) trait AssemblyGroundRunPaddingNumeric {
    type Error;

    /// Injects native `vmsum3fp128` while preserving its exact inputs.
    /// Returning `Err` and no value is safe-host/test policy; native numeric
    /// execution has no analogous failure route.
    fn sum_three_products(
        &mut self,
        bit_negated_face_normal: [f32; 3],
        timestep_scaled_velocity: [f32; 3],
    ) -> Result<f32, Self::Error>;
}

/// PowerPC `fsel` for the operand order used by Run.
///
/// Both signed zeros select `if_nonnegative`; unordered selects
/// `if_negative_or_unordered`.
fn run_fsel(selector: f32, if_nonnegative: f32, if_negative_or_unordered: f32) -> f32 {
    if selector >= -0.0 {
        if_nonnegative
    } else {
        if_negative_or_unordered
    }
}

fn select_run_velocity_padding(closing: f32, generator_padding: f32) -> f32 {
    let negated_closing = f32::from_bits(closing.to_bits() ^ FLOAT_SIGN_BIT);
    let positive_closing = run_fsel(negated_closing, 0.0, closing);
    let remaining = generator_padding - positive_closing;
    run_fsel(remaining, positive_closing, generator_padding)
}

/// Computes the exact surrounding value graph at `0x827cd444..0x827cd4cc`.
///
/// Velocity is multiplied by the literal timestep before the injected Xenon
/// reduction. The face normal uses bitwise sign negation. The final volume
/// padding add occurs only after both scalar selects. A numeric-backend error
/// is a typed host seam and not a native control-flow claim.
pub(crate) fn try_assembly_ground_effective_padding<N: AssemblyGroundRunPaddingNumeric>(
    volume_padding: f32,
    generator_padding: f32,
    velocity: [f32; 3],
    triangle_face_normal: [f32; 3],
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let bit_negated_face_normal =
        triangle_face_normal.map(|component| f32::from_bits(component.to_bits() ^ FLOAT_SIGN_BIT));
    let timestep_scaled_velocity = velocity.map(|component| component * RUN_CONTACT_TIMESTEP);
    let closing = numeric.sum_three_products(bit_negated_face_normal, timestep_scaled_velocity)?;
    Ok(select_run_velocity_padding(closing, generator_padding) + volume_padding)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PaddingTraceNumeric {
        returned: f32,
        calls: Vec<([u32; 3], [u32; 3])>,
    }

    impl AssemblyGroundRunPaddingNumeric for PaddingTraceNumeric {
        type Error = core::convert::Infallible;

        fn sum_three_products(
            &mut self,
            bit_negated_face_normal: [f32; 3],
            timestep_scaled_velocity: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.calls.push((
                bit_negated_face_normal.map(f32::to_bits),
                timestep_scaled_velocity.map(f32::to_bits),
            ));
            Ok(self.returned)
        }
    }

    fn run(
        volume_padding: f32,
        generator_padding: f32,
        returned: f32,
        velocity: [f32; 3],
        face_normal: [f32; 3],
    ) -> (f32, PaddingTraceNumeric) {
        let mut numeric = PaddingTraceNumeric {
            returned,
            calls: Vec::new(),
        };
        let result = try_assembly_ground_effective_padding(
            volume_padding,
            generator_padding,
            velocity,
            face_normal,
            &mut numeric,
        )
        .expect("infallible padding numeric");
        (result, numeric)
    }

    #[test]
    fn padding_uses_literal_timestep_bit_negated_normal_and_prescaled_velocity() {
        assert_eq!(RUN_CONTACT_TIMESTEP.to_bits(), 0x3c88_8889);
        let velocity = [f32::from_bits(0x3bed_ddd2), -60.0, 60.0];
        let face_normal = [
            f32::from_bits(0x0000_0000),
            f32::from_bits(0x8000_0000),
            f32::from_bits(0x7fc1_2345),
        ];
        let (_, numeric) = run(0.0, 0.75, 0.0, velocity, face_normal);

        assert_eq!(numeric.calls.len(), 1);
        assert_eq!(numeric.calls[0].0, [0x8000_0000, 0x0000_0000, 0xffc1_2345]);
        assert_eq!(
            numeric.calls[0].1,
            [0x38fd_b969, (-1.0_f32).to_bits(), 1.0_f32.to_bits()]
        );
        assert_eq!(
            (f32::from_bits(0xba40_65a0) * f32::from_bits(0x38fd_b969)).to_bits(),
            0xb3be_afc7
        );
    }

    #[test]
    fn padding_fsel_preserves_zero_nan_and_cap_operand_choices() {
        let k = f32::from_bits(0x3f40_0000);
        for (closing, expected) in [
            (0.0, 0x0000_0000),
            (-0.0, 0x0000_0000),
            (-0.5, 0x0000_0000),
            (0.5, 0x3f00_0000),
            (1.0, 0x3f40_0000),
            (f32::from_bits(0x3f3f_ffff), 0x3f3f_ffff),
            (f32::from_bits(0x3f40_0000), 0x3f40_0000),
            (f32::from_bits(0x3f40_0001), 0x3f40_0000),
            (f32::from_bits(0x7fc1_2345), 0x3f40_0000),
            (f32::from_bits(0xffc1_2345), 0x3f40_0000),
        ] {
            assert_eq!(
                select_run_velocity_padding(closing, k).to_bits(),
                expected,
                "closing={:08x}",
                closing.to_bits()
            );
        }

        let nan_k = f32::from_bits(0x7fc5_4321);
        assert_eq!(
            select_run_velocity_padding(0.5, nan_k).to_bits(),
            0x7fc5_4321
        );
        assert_eq!(select_run_velocity_padding(-0.0, -0.0).to_bits(), 0);
    }

    #[test]
    fn padding_caps_before_the_distinct_volume_add() {
        let (positive, positive_numeric) =
            run(0.125, 0.75, 1.0, [-60.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(positive.to_bits(), 0x3f60_0000);
        assert_eq!(positive_numeric.calls.len(), 1);

        let (negative, negative_numeric) =
            run(0.125, 0.75, -1.0, [60.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(negative.to_bits(), 0x3e00_0000);
        assert_eq!(negative_numeric.calls.len(), 1);

        let (after_cap, _) = run(0.25, 0.75, 0.75, [0.0; 3], [0.0; 3]);
        assert_eq!(after_cap.to_bits(), 1.0_f32.to_bits());
    }
}
