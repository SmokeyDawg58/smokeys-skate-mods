//! Numeric fixtures do not construct Contact identities or solver authority.
//! The root's exporter integration tests exercise actual retained body records.
use super::*;

fn assert_words(actual: [f32; 3], expected: [f32; 3]) {
    assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
}

fn simple_jacobian() -> SimulationContactJacobian {
    let mut jacobian = SimulationContactJacobian::zeroed();
    jacobian.nb_word = 8;
    jacobian.position_impulse = [2.0, 3.0, -4.0];
    jacobian.contact_right = [1.0, 2.0, 3.0];
    jacobian.contact_up = [4.0, 5.0, 6.0];
    jacobian.contact_at = [7.0, 8.0, 9.0];
    jacobian.attachment_offset_a = [1.0, 2.0, 3.0];
    jacobian.attachment_offset_b = [-1.0, -2.0, -3.0];
    jacobian.inverse_mass_a = 1.0;
    jacobian.inverse_mass_b = 3.0;
    jacobian.tag_word = 0xabcd_1234;
    jacobian
}

#[test]
fn selection_requires_spy_bit_and_ordered_positive_normal_impulse_only() {
    let mut jacobian = simple_jacobian();
    for nb in [0, 1, 2, 4, 7, 8, 9, 12, u32::MAX] {
        jacobian.nb_word = nb;
        for impulse in [f32::NEG_INFINITY, -1.0, -0.0, 0.0, f32::NAN] {
            jacobian.position_impulse[0] = impulse;
            assert!(!SimulationContactSpy::selects(&jacobian));
        }
        for impulse in [f32::from_bits(1), 1.0, f32::INFINITY] {
            jacobian.position_impulse[0] = impulse;
            assert_eq!(SimulationContactSpy::selects(&jacobian), nb & 8 != 0);
        }
    }
    // Other impulse coordinates and denominator are not admission predicates.
    jacobian.nb_word = 8;
    jacobian.position_impulse = [1.0, f32::NAN, f32::INFINITY];
    jacobian.inverse_mass_a = f32::NAN;
    assert!(SimulationContactSpy::selects(&jacobian));
}

#[test]
fn xyz_graph_copies_basis_and_uses_original_ab_offsets_mass_weights_and_forces() {
    let jacobian = simple_jacobian();
    let result = solved_vectors(&jacobian, 4.0, [9.0, 18.0, 27.0], [21.0, 42.0, 63.0]);
    assert_words(result.frame[0], [1.0, 2.0, 3.0]);
    assert_words(result.frame[1], [4.0, 5.0, 6.0]);
    assert_words(result.frame[2], [7.0, 8.0, 9.0]);
    assert_words(result.frame[3], [17.5, 35.0, 52.5]);
    assert_words(result.normal_force, [8.0, 16.0, 24.0]);
    assert_words(result.friction_force, [-64.0, -68.0, -72.0]);
    // The kernel consumes XYZ only: packed words must not leak into axes.
    let mut packed_decoy = jacobian;
    packed_decoy.body_a = super::super::ContactBodyToken(0x7fc0_1234);
    packed_decoy.body_b = super::super::ContactBodyToken(0xff80_0000);
    packed_decoy.tag_word = 0xffff_ffff;
    let decoy = solved_vectors(&packed_decoy, 4.0, [9.0, 18.0, 27.0], [21.0, 42.0, 63.0]);
    for (actual, expected) in decoy.frame.into_iter().zip(result.frame) {
        assert_words(actual, expected);
    }
    assert_words(decoy.normal_force, result.normal_force);
    assert_words(decoy.friction_force, result.friction_force);
}

#[test]
fn friction_and_position_use_the_source_fused_addends_not_rounded_products() {
    // (1+2^-23)*(1-2^-23)-1 = -2^-46 exactly. A separate product
    // rounds to1 and loses the residual; this is an independent algebraic oracle.
    let a = f32::from_bits(0x3f80_0001);
    let b = f32::from_bits(0x3f7f_fffe);
    let mut jacobian = SimulationContactJacobian::zeroed();
    jacobian.contact_up = [a; 3];
    jacobian.contact_at = [-1.0; 3];
    jacobian.position_impulse = [1.0, b, 1.0];
    jacobian.inverse_mass_a = b;
    jacobian.inverse_mass_b = 1.0;
    let result = solved_vectors(&jacobian, 1.0, [a; 3], [-1.0; 3]);
    assert_eq!(result.friction_force.map(f32::to_bits), [0xa880_0000; 3]);
    // Denominator bits3fffffff, rounded reciprocal3f000001; multiply
    // the fused numerator by that reciprocal, not a direct quotient.
    assert_eq!(result.frame[3].map(f32::to_bits), [0xa800_0001; 3]);
    assert_eq!((a * b - 1.0).to_bits(), 0);
}

#[test]
fn reciprocal_then_position_multiply_and_two_normal_multiplies_remain_distinct() {
    let mut jacobian = SimulationContactJacobian::zeroed();
    jacobian.inverse_mass_a = f32::from_bits(0x4005_d174);
    let center = f32::from_bits(0x3e12_4925);
    jacobian.contact_right = [center; 3];
    jacobian.position_impulse[0] = f32::from_bits(0x3dba_2e8c);
    let squared_frequency = f32::from_bits(0x3ec4_ec4f);
    let result = solved_vectors(&jacobian, squared_frequency, [center; 3], [0.0; 3]);
    assert_eq!(result.frame[3].map(f32::to_bits), [0x3e12_4924; 3]);
    assert_eq!(result.normal_force.map(f32::to_bits), [0x3ba3_ad24; 3]);
    let numerator = center * jacobian.inverse_mass_a;
    assert_eq!((numerator / jacobian.inverse_mass_a).to_bits(), 0x3e12_4925);
    let regrouped = center * (jacobian.position_impulse[0] * squared_frequency);
    assert_eq!(regrouped.to_bits(), 0x3ba3_ad25);
}

#[test]
fn degenerate_and_exceptional_values_are_not_clamped_or_rejected_by_the_kernel() {
    let mut jacobian = SimulationContactJacobian::zeroed();
    let zero = solved_vectors(&jacobian, 1.0, [0.0; 3], [0.0; 3]);
    assert!(zero.frame[3].iter().all(|value| value.is_nan()));
    jacobian.inverse_mass_a = 1.0;
    jacobian.inverse_mass_b = -1.0;
    let cancelled = solved_vectors(&jacobian, 1.0, [2.0; 3], [1.0; 3]);
    assert_eq!(
        cancelled.frame[3].map(f32::to_bits),
        [f32::INFINITY.to_bits(); 3]
    );
    jacobian.inverse_mass_b = 0.0;
    jacobian.contact_right = [1.0; 3];
    jacobian.position_impulse[0] = f32::INFINITY;
    let force = solved_vectors(&jacobian, 0.0, [1.0; 3], [0.0; 3]);
    assert!(force.normal_force.iter().all(|value| value.is_nan()));
}

#[test]
fn empty_frame_uses_actual_manager_pool_and_epoch_without_fabricated_contacts() {
    let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let manager = SimulationManagerContactDomain::try_new(&mut pool, 0).unwrap();
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    let frame = SimulationContactSpyFrame::new(
        manager.assembly_pool_authority(),
        pool.authority(),
        pool.frame_identity().unwrap(),
        Vec::new(),
    );
    assert!(frame.records().is_empty());
    frame.try_validate_current(&manager, &pool).unwrap();
    let mut other_pool = SimulationRigidBodyPool::try_new(1).unwrap();
    let other_manager = SimulationManagerContactDomain::try_new(&mut other_pool, 0).unwrap();
    other_pool.begin_dynamic_update_frame(1.0 / 60.0);
    assert_eq!(
        frame.try_validate_current(&other_manager, &other_pool),
        Err(SimulationContactSpyError::ManagerMismatch)
    );
    assert_eq!(
        frame.try_validate_current(&manager, &other_pool),
        Err(SimulationContactSpyError::BodyPoolMismatch)
    );
    pool.begin_dynamic_update_frame(1.0 / 60.0);
    assert_eq!(
        frame.try_validate_current(&manager, &pool),
        Err(SimulationContactSpyError::FrameMismatch)
    );
}
