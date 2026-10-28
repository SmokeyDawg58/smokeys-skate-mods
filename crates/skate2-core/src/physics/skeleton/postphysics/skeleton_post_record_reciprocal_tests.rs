//! Unregistered child of post-records; source541a0/a8 exceptional-zero policy.
use super::{records_reciprocal_residual, refine_records_reciprocal};

#[test]
fn vnmsub_residual_negates_exact_fused_cancellation() {
    assert_eq!(
        records_reciprocal_residual(0.25, 4.0).to_bits(),
        (-0.0_f32).to_bits()
    );
    // Discriminator against the previous fused-negative-operand formulation.
    assert_eq!((-4.0_f32).mul_add(0.25, 1.0).to_bits(), 0.0_f32.to_bits());
}

#[test]
fn records_refinement_preserves_exact_positive_and_negative_inverse() {
    assert_eq!(
        refine_records_reciprocal(0.25, 4.0).to_bits(),
        4.0_f32.to_bits()
    );
    assert_eq!(
        refine_records_reciprocal(-0.25, -4.0).to_bits(),
        (-4.0_f32).to_bits()
    );
}
