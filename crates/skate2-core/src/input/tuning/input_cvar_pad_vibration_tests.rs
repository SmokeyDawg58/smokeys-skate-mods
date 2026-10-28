use super::*;
#[test]
fn null_gate_and_retained_live_targets_preserve_all_four_defaults() {
    let refs = InputCvarReferences::new_static_initialized();
    assert!(
        refs.capture_pad_vibration(PadVibrationFloat::LowScale)
            .unwrap()
            .is_none()
    );
    let mut manager = crate::input_cvar_runtime::InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    for (field, bits) in [
        (PadVibrationFloat::LowScale, 0x3f99_999a),
        (PadVibrationFloat::LowOffset, 0xbca3_d70a),
        (PadVibrationFloat::HighScale, 0x3f86_6666),
        (PadVibrationFloat::HighOffset, 0x3d4c_cccd),
    ] {
        assert_eq!(
            refs.capture_pad_vibration(field)
                .unwrap()
                .unwrap()
                .read()
                .unwrap()
                .to_bits(),
            bits
        );
    }
    let target = refs
        .capture_pad_vibration(PadVibrationFloat::LowScale)
        .unwrap()
        .unwrap();
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkLowFreqScale=2\r\n"))
        .unwrap();
    assert_eq!(target.read().unwrap().to_bits(), 2.0f32.to_bits());
}
