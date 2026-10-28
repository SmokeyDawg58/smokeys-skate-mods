use super::*;
use crate::input_cvar_runtime::InputCvarManager;

#[test]
fn recognizer_targets_read_live_values_and_keep_captured_identity() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut provider = refs.recognizer_provider();
    for (key, bits) in [
        (InputPatternCvar::FastestRatio, 0x3fe0_0000),
        (InputPatternCvar::SlowestRatio, 0x408c_cccd),
        (InputPatternCvar::MinError, 0x3e19_999a),
        (InputPatternCvar::MaxError, 0x3e19_999a),
    ] {
        let target = provider.capture(key).unwrap();
        assert_eq!(provider.value(&target).unwrap().to_bits(), bits);
    }
    let target = provider.capture(InputPatternCvar::FastestRatio).unwrap();
    let deadzone = provider.capture_deadzone().unwrap();
    assert_eq!(
        provider.deadzone(&deadzone).unwrap().map(f32::to_bits),
        [0x3dcc_cccd; 2]
    );
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\nkFastestGestureRatio=2.25\r\nkInputHistoryDeadZone=0.25,0.5\r\n",
        ))
        .unwrap();
    assert_eq!(
        provider.value(&target).unwrap().to_bits(),
        2.25f32.to_bits()
    );
    assert_eq!(
        provider.deadzone(&deadzone).unwrap().map(f32::to_bits),
        [0.25f32.to_bits(), 0.5f32.to_bits()]
    );
    let mut other = InputCvarManager::new();
    other.try_link(&refs).unwrap();
    assert_eq!(
        provider.value(&target).unwrap().to_bits(),
        2.25f32.to_bits()
    );
    let fresh = provider.capture(InputPatternCvar::FastestRatio).unwrap();
    assert_eq!(provider.value(&fresh).unwrap().to_bits(), 0x3fe0_0000);
}
#[test]
fn gesture_reference_copies_are_shared_once_while_input_display_captures_fresh() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut first = InputCvarManager::new();
    first.try_link(&refs).unwrap();
    let mut left = refs.recognizer_provider();
    let mut right = refs.recognizer_provider();
    left.ensure_gesture_display_references().unwrap();
    first.try_deserialize_startup(Some(b"[Config]\r\nkDisplayLeftStickGestures=1\r\nkDisplayRightStickGestures=-1\r\nkDisplayLeftStickInput=1\r\n")).unwrap();
    let mut second = InputCvarManager::new();
    second.try_link(&refs).unwrap();
    right.ensure_gesture_display_references().unwrap();
    assert!(
        right
            .gesture_display_value(InputPatternStick::Left)
            .unwrap()
    );
    assert!(
        right
            .gesture_display_value(InputPatternStick::Right)
            .unwrap()
    );
    let live = right
        .capture_input_display(InputPatternStick::Left)
        .unwrap();
    assert!(!right.input_display_value(&live).unwrap());
    first
        .try_deserialize_startup(Some(b"[Config]\r\nkDisplayLeftStickGestures=0\r\n"))
        .unwrap();
    assert!(!left.gesture_display_value(InputPatternStick::Left).unwrap());
}
#[test]
fn copying_unlinked_gesture_refs_preserves_native_null_even_after_link() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut provider = refs.recognizer_provider();
    provider.ensure_gesture_display_references().unwrap();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    provider.ensure_gesture_display_references().unwrap();
    assert_eq!(
        provider.gesture_display_value(InputPatternStick::Left),
        Err(InputCvarError::Unlinked("kDisplayLeftStickGestures"))
    );
    assert_eq!(
        provider.gesture_display_value(InputPatternStick::Right),
        Err(InputCvarError::Unlinked("kDisplayRightStickGestures"))
    );
}
