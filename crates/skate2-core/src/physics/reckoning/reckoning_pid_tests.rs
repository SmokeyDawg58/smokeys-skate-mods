//! Independent source-graph oracles with controlled finite/raw inputs. These
//! are not original Vault tuning, native traces or a completed Reckoning owner.
use super::*;

fn control() -> PhysicsControllerData {
    PhysicsControllerData {
        proportional: 0.5,
        integral: 0.25,
        derivative: 0.125,
        integral_blend: 0.5,
    }
}
fn words(control: PhysicsControllerData) -> [u32; 4] {
    [
        control.proportional,
        control.integral,
        control.derivative,
        control.integral_blend,
    ]
    .map(f32::to_bits)
}

#[test]
fn constructor_copies_control_and_both_initial_vectors_without_sanitizing() {
    let control = PhysicsControllerData {
        proportional: -0.0,
        integral: f32::from_bits(0x7fc1_2345),
        derivative: f32::INFINITY,
        integral_blend: f32::from_bits(1),
    };
    let initial = [-0.0, f32::from_bits(0x7fc0_9876), f32::NEG_INFINITY, 17.0];
    let filter = VectorPidFilter::new(control, initial);
    assert_eq!(words(filter.control()), words(control));
    assert_eq!(
        filter
            .value()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        initial.map(f32::to_bits)
    );
    assert_eq!(
        filter
            .last_target()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        initial.map(f32::to_bits)
    );
    for history in [
        filter.integral(),
        filter.derivative(),
        filter.derivative_input(),
    ] {
        assert_eq!(
            history
                .known_lanes()
                .expect("controlled known lanes")
                .map(f32::to_bits),
            [0; 4]
        );
    }
}

#[test]
fn two_updates_retain_distinct_histories_and_compute_nontrivial_fourth_lane() {
    let mut filter = VectorPidFilter::new(control(), [2.0, -4.0, 0.0, 8.0]);
    let target = [6.0, 4.0, -2.0, 0.0];
    // First step: I=.5*error, filtered target delta=.5*error,
    // output=oldValue+(.125+.5+.0625)*error, all exact binary fractions.
    assert_eq!(
        filter
            .update(target)
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [4.75_f32, 1.5, -1.375, 2.5].map(f32::to_bits)
    );
    assert_eq!(
        filter
            .integral()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [2.0_f32, 4.0, -1.0, -4.0].map(f32::to_bits)
    );
    assert_eq!(
        filter
            .derivative_input()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [2.0_f32, 4.0, -1.0, -4.0].map(f32::to_bits)
    );
    assert_eq!(
        filter
            .derivative()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [2.75_f32, 5.5, -1.375, -5.5].map(f32::to_bits)
    );
    assert_eq!(
        filter
            .update(target)
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [5.5625_f32, 3.125, -1.78125, 0.875].map(f32::to_bits)
    );
    assert_eq!(
        filter
            .integral()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [1.625_f32, 3.25, -0.8125, -3.25].map(f32::to_bits)
    );
    assert_eq!(
        filter
            .derivative_input()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [1.0_f32, 2.0, -0.5, -2.0].map(f32::to_bits)
    );
    assert_eq!(
        filter
            .derivative()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [0.8125_f32, 1.625, -0.40625, -1.625].map(f32::to_bits)
    );
    assert_eq!(
        filter
            .last_target()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        target.map(f32::to_bits)
    );
    // Incorrectly subtracting old derivativeInput instead of output delta
    // would produce X5.65625 in the second step.
    assert_ne!(filter.value().xyz[0].to_bits(), 5.65625_f32.to_bits());
}

#[test]
fn override_and_control_replacement_preserve_histories_for_subsequent_update() {
    let first = PhysicsControllerData {
        proportional: 1.0,
        integral: 0.0,
        derivative: 0.0,
        integral_blend: 1.0,
    };
    let mut filter = VectorPidFilter::new(first, [0.0; 4]);
    filter.update([5.0; 4]);
    let retained = [
        filter.integral(),
        filter.derivative(),
        filter.derivative_input(),
        filter.last_target(),
    ];
    let second = PhysicsControllerData {
        proportional: 0.0,
        integral: 0.0,
        derivative: 1.0,
        integral_blend: 0.0,
    };
    filter.replace_control(second);
    filter.set_value([100.0; 4]);
    assert_eq!(words(filter.control()), words(second));
    assert_eq!(
        [
            filter.integral(),
            filter.derivative(),
            filter.derivative_input(),
            filter.last_target()
        ]
        .map(|v| v
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits)),
        retained.map(|v| v
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits))
    );
    assert_eq!(
        filter
            .update([5.0; 4])
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [100.0_f32.to_bits(); 4]
    );
    assert_eq!(
        filter
            .derivative()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [0; 4]
    );
    // Filtered target delta remains5, while previous OUTPUT delta is now0.
    assert_eq!(
        filter
            .update([5.0; 4])
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [105.0_f32.to_bits(); 4]
    );
}

#[test]
fn integral_blend_uses_rounded_error_product_then_fused_history_product() {
    let epsilon = f32::from_bits(0x3400_0000); //2^-23
    let one_plus = f32::from_bits(0x3f80_0001);
    let coefficients = PhysicsControllerData {
        proportional: 0.0,
        integral: 0.0,
        derivative: 0.0,
        integral_blend: -epsilon,
    };
    let mut filter = VectorPidFilter::new(coefficients, [0.0; 4]);
    // Explicit controlled prior integral state, not a constructor claim.
    filter.integral = Vector::known([one_plus; 4]);
    filter.update([8_388_610.0; 4]);
    // Rounded error*(-2^-23)=-(1+2^-22); fused(1+2^-23)^2
    // plus that value retains2^-46. Separately rounded products cancel to0.
    assert_eq!(
        filter
            .integral()
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [0x2880_0000; 4]
    );
    let rounded = one_plus * one_plus;
    assert_eq!((rounded - f32::from_bits(0x3f80_0002)).to_bits(), 0);
}

#[test]
fn proportional_contribution_is_fused_into_existing_value_without_dt() {
    let one_plus = f32::from_bits(0x3f80_0001);
    let old = -f32::from_bits(0x3f80_0002);
    let coefficients = PhysicsControllerData {
        proportional: one_plus,
        integral: 0.0,
        derivative: 0.0,
        integral_blend: 0.0,
    };
    let mut filter = VectorPidFilter::new(coefficients, [old; 4]);
    // target−old=1+2^-23 exactly. P*error+old is2^-46;
    // no timestep multiplier or post-normalization is present.
    assert_eq!(
        filter
            .update([-f32::from_bits(0x3400_0000); 4])
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [0x2880_0000; 4]
    );
}

#[test]
fn nonfinite_fourth_lane_is_not_replaced_and_does_not_reduce_into_xyz() {
    let coefficients = PhysicsControllerData {
        proportional: 1.0,
        integral: 0.0,
        derivative: 0.0,
        integral_blend: 1.0,
    };
    let mut filter = VectorPidFilter::new(coefficients, [0.0; 4]);
    let output = filter.update([1.0, 2.0, 3.0, f32::NAN]);
    assert_eq!(
        output.xyz.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        [1.0_f32, 2.0, 3.0].map(f32::to_bits)
    );
    assert!(output.w.expect("controlled known W").is_nan());
    assert!(filter.integral().w.expect("controlled known W").is_nan());
    assert!(
        filter
            .derivative_input()
            .w
            .expect("controlled known W")
            .is_nan()
    );
    assert!(filter.derivative().w.expect("controlled known W").is_nan());
    assert!(filter.last_target().w.expect("controlled known W").is_nan());
}

#[test]
fn unknown_w_survives_zero_coefficients_and_value_override_in_the_same_histories() {
    let mut filter = VectorPidFilter::new(
        PhysicsControllerData {
            proportional: 0.0,
            integral: 0.0,
            derivative: 0.0,
            integral_blend: 0.0,
        },
        [0.0; 4],
    );
    let result = filter.update(Vector {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    });
    assert!(result.w.is_none());
    filter.set_value([0.0; 4]);
    assert_eq!(filter.value().w.map(f32::to_bits), Some(0));
    assert!(filter.integral().w.is_none());
    assert!(filter.derivative().w.is_none());
    assert!(filter.derivative_input().w.is_none());
    assert!(filter.last_target().w.is_none());
    assert!(filter.update([0.0; 4]).w.is_none());
}
