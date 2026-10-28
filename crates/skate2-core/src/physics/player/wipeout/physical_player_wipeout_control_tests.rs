use super::*;
use crate::{input_cvar_runtime::InputCvarManager, reckoning_numeric::HostReckoningNumeric};
#[test]
fn live_swapped_angle_controls_and_body_velocity_feed_torque() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    manager.try_deserialize_startup(Some(b"[Config]\r\nkWipeControlMaxAngleRotX=2\r\nkWipeControlMaxAngleRotY=7\r\nkWipeoutTorqueForce=1\r\nkWipeControlMaxTorque=1000\r\n")).unwrap();
    let axis = Vector::known([0.0, 1.0, 0.0, 0.0]);
    let auto = AutoControl {
        side: Vector::known([1.0, 0.0, 0.0, 0.0]),
        correction: Vector::known([0.0; 4]),
    };
    let result = try_control_torque(
        axis,
        auto,
        Vector::known([0.0; 4]),
        0.0,
        1.0,
        &refs,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    let expected = ((2.0 * f32::from_bits(0x3c8e_fa35)) * f32::from_bits(0x426f_ffff))
        * f32::from_bits(0x3c88_8889);
    assert!((result.xyz[0] - expected).abs() < 1e-7);
    assert_eq!(result.xyz[1].to_bits(), 0);
    let auto = AutoControl {
        side: Vector::known([1.0, 0.0, 0.0, 0.0]),
        correction: Vector::known([0.0; 4]),
    };
    let result = try_control_torque(
        axis,
        auto,
        Vector::known([0.0; 4]),
        1.0,
        0.0,
        &refs,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    let expected = -((7.0 * f32::from_bits(0x3c8e_fa35)) * f32::from_bits(0x426f_ffff))
        * f32::from_bits(0x3c88_8889);
    assert!((result.xyz[1] - expected).abs() < 1e-7);
}
