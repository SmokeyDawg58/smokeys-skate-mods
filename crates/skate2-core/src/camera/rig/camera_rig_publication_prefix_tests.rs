use super::*;
use crate::input_cvar_runtime::InputCvarManager;

#[test]
fn disabled_debug_gates_need_no_overlay_or_position_reads() {
    let refs = InputCvarReferences::new_static_initialized();
    InputCvarManager::new().try_link(&refs).unwrap();
    try_debug_gates(&refs, &DebugOverlaySlot::default(), |_| {
        panic!("disabled position read")
    })
    .unwrap();
}
#[test]
fn debug_position_order_is_live_and_missing_overlay_stops_before_math() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkbShowIntendedPosition=1\r\n"))
        .unwrap();
    let overlay = DebugOverlaySlot::default();
    assert!(matches!(
        try_debug_gates(&refs, &overlay, |_| panic!("no overlay")),
        Err(RigDynamicsError::Overlay(_))
    ));
    overlay.try_construct().unwrap();
    let mut order = Vec::new();
    try_debug_gates(&refs, &overlay, |kind| {
        order.push(match kind {
            RigDebugBoolean::Intended => 1,
            RigDebugBoolean::Avoidance => 2,
            RigDebugBoolean::Tracker => 3,
        });
        manager
            .try_deserialize_startup(Some(
                b"[Config]\r\nkbShowAvoidancePosition=1\r\nkbShowTrackerDebug=1\r\n",
            ))
            .unwrap();
    })
    .unwrap();
    assert_eq!(order, [1, 2]);
}
#[test]
fn opacity_fsel_retains_shot_when_bound_is_unordered() {
    assert_eq!(grind_opacity(0.75, 1.0, 0.25).to_bits(), 0.25_f32.to_bits());
    assert_eq!(
        grind_opacity(0.75, f32::NAN, 0.25).to_bits(),
        0.75_f32.to_bits()
    );
    assert!(grind_opacity(f32::NAN, 1.0, 0.25).is_nan());
}
#[test]
fn fov_uses_aspect_plus_one_not_aspect_squared_and_zero_select() {
    let a = vertical_fov(17.5, 35.0, 3.0, &mut HostReckoningNumeric).unwrap();
    let b = vertical_fov(17.5, 35.0, 8.0, &mut HostReckoningNumeric).unwrap();
    assert!((a - 45.0).abs() < 0.001);
    assert!((b - 30.0).abs() < 0.001);
    assert_eq!(
        vertical_fov(17.5, 0.0, 3.0, &mut HostReckoningNumeric)
            .unwrap()
            .to_bits(),
        0.0_f32.to_bits()
    );
    assert!(
        vertical_fov(17.5, 35.0, -2.0, &mut HostReckoningNumeric)
            .unwrap()
            .is_nan()
    );
}
